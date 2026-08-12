use chrono::{DateTime, Duration, Utc};
use vrcx_0_application_core::RuntimeAuthIdentity;
use vrcx_0_persistence::config as config_store;
use vrcx_0_persistence::game_log;
use vrcx_0_persistence::DatabaseService;

use crate::game_log::host::GameLogHostActions;
use crate::game_log::ingest::ScreenshotInput;
use crate::game_log::runtime_state::world_id_from_location;
use crate::screenshots as screenshot_domain;
use crate::{Error, Result};
use crate::{GameLogSideEffectEvent, GameLogSideEffectSink, ScreenshotProcessedPayload};

const FALLBACK_LOCATION_MAX_AGE_MS: i64 = 15 * 60 * 1000;

#[derive(Clone, Debug, Default)]
struct ScreenshotContext {
    location: String,
    world_name: String,
    players: Vec<ScreenshotPlayer>,
}

#[derive(Clone, Debug, Default)]
struct ScreenshotPlayer {
    user_id: String,
    display_name: String,
}

pub async fn handle_screenshot(
    db: &DatabaseService,
    host_actions: &dyn GameLogHostActions,
    side_effect_sink: &GameLogSideEffectSink,
    author: &RuntimeAuthIdentity,
    input: ScreenshotInput,
) -> Result<()> {
    let screenshot_path = input.path.trim().to_string();
    if screenshot_path.is_empty() {
        return Ok(());
    }

    let screenshot_helper = config_store::get_bool(db, "screenshotHelper", true)?;
    let modify_filename = config_store::get_bool(db, "screenshotHelperModifyFilename", false)?;
    let copy_to_clipboard = config_store::get_bool(db, "screenshotHelperCopyToClipboard", false)?;

    let mut next_path = screenshot_path.clone();
    if screenshot_helper {
        if let Some(context) = screenshot_context(db, &author.user_id, &input)? {
            let world_id = world_id_from_location(&context.location);
            let metadata = build_metadata(author, &context, &world_id);
            let metadata_json = serde_json::to_string(&metadata)?;
            let path_for_task = screenshot_path.clone();
            let world_id_for_task = world_id.clone();
            let written = tokio::task::spawn_blocking(move || {
                screenshot_domain::add_screenshot_metadata(
                    &path_for_task,
                    &metadata_json,
                    &world_id_for_task,
                    modify_filename,
                )
            })
            .await
            .map_err(|error| Error::Custom(format!("screenshot metadata task: {error}")))?;
            if !written.is_empty() {
                next_path = written;
            }
        }
    }

    if copy_to_clipboard {
        if let Err(error) = host_actions.copy_image_to_clipboard(&next_path) {
            tracing::warn!("failed to copy GameLog screenshot to clipboard: {error}");
        }
    }

    side_effect_sink.emit(GameLogSideEffectEvent::ScreenshotProcessed(
        ScreenshotProcessedPayload { path: next_path },
    ));
    Ok(())
}

fn screenshot_context(
    db: &DatabaseService,
    owner_user_id: &str,
    input: &ScreenshotInput,
) -> Result<Option<ScreenshotContext>> {
    if !input.snapshot.location.is_empty() {
        return Ok(Some(ScreenshotContext {
            location: input.snapshot.location.clone(),
            world_name: input.snapshot.world_name.clone(),
            players: input
                .snapshot
                .players
                .iter()
                .map(|player| ScreenshotPlayer {
                    user_id: player.user_id.clone(),
                    display_name: player.display_name.clone(),
                })
                .collect(),
        }));
    }

    game_log::ensure_game_log_tables(db)?;
    let Some(location_entry) =
        game_log::get_location_before_or_at(db, owner_user_id, &input.created_at)?
    else {
        return Ok(None);
    };

    let screenshot_time = DateTime::parse_from_rfc3339(&input.created_at)
        .map(|date| date.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let location_time = DateTime::parse_from_rfc3339(&location_entry.created_at)
        .map(|date| date.with_timezone(&Utc))
        .unwrap_or_else(|_| {
            screenshot_time - Duration::milliseconds(FALLBACK_LOCATION_MAX_AGE_MS + 1)
        });
    if screenshot_time.timestamp_millis() - location_time.timestamp_millis()
        > FALLBACK_LOCATION_MAX_AGE_MS
    {
        return Ok(None);
    }

    let mut players = Vec::<ScreenshotPlayer>::new();
    for entry in game_log::get_join_leave_entries_for_location_range(
        db,
        owner_user_id,
        &location_entry.location,
        &location_entry.created_at,
        &input.created_at,
    )? {
        let key = if entry.user_id.is_empty() {
            format!("display:{}", entry.display_name)
        } else {
            entry.user_id.clone()
        };
        if entry.event_type == "OnPlayerJoined" {
            players.retain(|player| {
                let existing_key = if player.user_id.is_empty() {
                    format!("display:{}", player.display_name)
                } else {
                    player.user_id.clone()
                };
                existing_key != key
            });
            players.push(ScreenshotPlayer {
                user_id: entry.user_id,
                display_name: entry.display_name,
            });
        } else if entry.event_type == "OnPlayerLeft" {
            players.retain(|player| {
                let existing_key = if player.user_id.is_empty() {
                    format!("display:{}", player.display_name)
                } else {
                    player.user_id.clone()
                };
                existing_key != key
            });
        }
    }

    Ok(Some(ScreenshotContext {
        location: location_entry.location,
        world_name: location_entry.world_name,
        players,
    }))
}

fn build_metadata(
    author: &RuntimeAuthIdentity,
    context: &ScreenshotContext,
    world_id: &str,
) -> serde_json::Value {
    serde_json::json!({
        "application": "VRCX-0",
        "version": 1,
        "author": {
            "id": &author.user_id,
            "displayName": &author.display_name,
        },
        "world": {
            "name": &context.world_name,
            "id": world_id,
            "instanceId": &context.location,
        },
        "players": context.players.iter().map(|player| serde_json::json!({
            "id": &player.user_id,
            "displayName": &player.display_name,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vrcx_0_persistence::game_log::{
        write_batch, GameLogJoinLeaveEntry, GameLogLocationEntry, GameLogWriteBatch,
    };

    use crate::game_log::runtime_state::{PlayerState, RuntimeSnapshot};

    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("vrcx-0-{name}-{}-{nonce}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn test_db(name: &str) -> (TestDir, DatabaseService) {
        let dir = TestDir::new(name);
        let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap();
        (dir, db)
    }

    const LOCATION: &str = "wrld_live:123";
    const LOCATION_AT: &str = "2026-04-30T10:00:00.000Z";
    const SHOT_AT: &str = "2026-04-30T10:15:00.000Z";

    fn join_leave_entry(
        created_at: &str,
        event_type: &str,
        display_name: &str,
        user_id: &str,
    ) -> GameLogJoinLeaveEntry {
        GameLogJoinLeaveEntry {
            created_at: created_at.to_string(),
            event_type: event_type.to_string(),
            display_name: display_name.to_string(),
            location: LOCATION.to_string(),
            user_id: user_id.to_string(),
            world_name: "Live World".to_string(),
            time: 0,
        }
    }

    fn write_visited_location(db: &DatabaseService, join_leave: Vec<GameLogJoinLeaveEntry>) {
        let batch = GameLogWriteBatch {
            locations: vec![GameLogLocationEntry {
                created_at: LOCATION_AT.to_string(),
                location: LOCATION.to_string(),
                world_id: "wrld_live".to_string(),
                world_name: "Live World".to_string(),
                time: 0,
                group_name: String::new(),
            }],
            join_leave,
            ..Default::default()
        };
        write_batch(db, "", &batch).unwrap();
    }

    fn context_at(db: &DatabaseService, created_at: &str) -> Option<ScreenshotContext> {
        let input = ScreenshotInput {
            created_at: created_at.to_string(),
            path: "screenshot.png".to_string(),
            snapshot: RuntimeSnapshot::default(),
        };
        screenshot_context(db, "", &input).unwrap()
    }

    #[test]
    fn snapshot_location_short_circuits_the_database_lookup() {
        let (_dir, db) = test_db("screenshot-snapshot-shortcircuit");
        let input = ScreenshotInput {
            created_at: SHOT_AT.to_string(),
            path: "screenshot.png".to_string(),
            snapshot: RuntimeSnapshot {
                location: LOCATION.to_string(),
                world_name: "Live World".to_string(),
                destination: String::new(),
                started_at: String::new(),
                players: vec![PlayerState {
                    user_id: "usr_a".to_string(),
                    display_name: "Alice".to_string(),
                    join_time_ms: None,
                }],
            },
        };

        let context = screenshot_context(&db, "", &input).unwrap().unwrap();
        assert_eq!(context.location, LOCATION);
        assert_eq!(context.world_name, "Live World");
        assert_eq!(context.players.len(), 1);
        assert_eq!(context.players[0].user_id, "usr_a");
    }

    #[test]
    fn no_location_history_returns_none() {
        let (_dir, db) = test_db("screenshot-no-history");

        assert!(context_at(&db, SHOT_AT).is_none());
    }

    #[test]
    fn location_exactly_at_max_age_boundary_is_still_used() {
        let (_dir, db) = test_db("screenshot-boundary-included");
        write_visited_location(&db, vec![]);

        let context = context_at(&db, SHOT_AT).unwrap();

        assert_eq!(context.location, LOCATION);
        assert!(context.players.is_empty());
    }

    #[test]
    fn location_one_ms_past_max_age_boundary_is_rejected() {
        let (_dir, db) = test_db("screenshot-boundary-excluded");
        write_visited_location(&db, vec![]);

        assert!(context_at(&db, "2026-04-30T10:15:00.001Z").is_none());
    }

    #[test]
    fn join_events_dedupe_by_user_id_key() {
        let (_dir, db) = test_db("screenshot-join-dedupe-id");
        write_visited_location(
            &db,
            vec![
                join_leave_entry(
                    "2026-04-30T10:01:00.000Z",
                    "OnPlayerJoined",
                    "Alice",
                    "usr_a",
                ),
                join_leave_entry(
                    "2026-04-30T10:02:00.000Z",
                    "OnPlayerJoined",
                    "Alice",
                    "usr_a",
                ),
            ],
        );

        let context = context_at(&db, SHOT_AT).unwrap();

        assert_eq!(context.players.len(), 1);
        assert_eq!(context.players[0].user_id, "usr_a");
    }

    #[test]
    fn join_events_dedupe_by_display_name_when_user_id_missing() {
        let (_dir, db) = test_db("screenshot-join-dedupe-name");
        write_visited_location(
            &db,
            vec![
                join_leave_entry("2026-04-30T10:01:00.000Z", "OnPlayerJoined", "NoId", ""),
                join_leave_entry("2026-04-30T10:02:00.000Z", "OnPlayerJoined", "NoId", ""),
            ],
        );

        let context = context_at(&db, SHOT_AT).unwrap();

        assert_eq!(context.players.len(), 1);
        assert_eq!(context.players[0].display_name, "NoId");
    }

    #[test]
    fn distinct_user_ids_with_the_same_display_name_are_not_merged() {
        let (_dir, db) = test_db("screenshot-join-distinct-ids");
        write_visited_location(
            &db,
            vec![
                join_leave_entry(
                    "2026-04-30T10:01:00.000Z",
                    "OnPlayerJoined",
                    "Twin",
                    "usr_a",
                ),
                join_leave_entry(
                    "2026-04-30T10:02:00.000Z",
                    "OnPlayerJoined",
                    "Twin",
                    "usr_b",
                ),
            ],
        );

        let context = context_at(&db, SHOT_AT).unwrap();

        assert_eq!(context.players.len(), 2);
    }

    #[test]
    fn leave_removes_player_by_user_id_key() {
        let (_dir, db) = test_db("screenshot-leave-by-id");
        write_visited_location(
            &db,
            vec![
                join_leave_entry(
                    "2026-04-30T10:01:00.000Z",
                    "OnPlayerJoined",
                    "Alice",
                    "usr_a",
                ),
                join_leave_entry("2026-04-30T10:02:00.000Z", "OnPlayerLeft", "Alice", "usr_a"),
            ],
        );

        let context = context_at(&db, SHOT_AT).unwrap();

        assert!(context.players.is_empty());
    }

    #[test]
    fn leave_removes_anonymous_player_by_display_name_key() {
        let (_dir, db) = test_db("screenshot-leave-by-name");
        write_visited_location(
            &db,
            vec![
                join_leave_entry("2026-04-30T10:01:00.000Z", "OnPlayerJoined", "Bob", ""),
                join_leave_entry("2026-04-30T10:02:00.000Z", "OnPlayerLeft", "Bob", ""),
            ],
        );

        let context = context_at(&db, SHOT_AT).unwrap();

        assert!(context.players.is_empty());
    }

    #[test]
    fn captured_author_does_not_follow_a_later_account_switch() {
        let scope = crate::RuntimeAuthScope::new();
        scope.set_identity("usr_first", "First User", "");
        let author = scope.identity();
        scope.set_identity("usr_second", "Second User", "");

        let metadata = build_metadata(&author, &ScreenshotContext::default(), "wrld_example");

        assert_eq!(metadata["author"]["id"], "usr_first");
        assert_eq!(metadata["author"]["displayName"], "First User");
    }
}
