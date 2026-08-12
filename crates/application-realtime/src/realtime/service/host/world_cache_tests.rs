use super::test_support::*;
use super::*;
use vrcx_0_application_core::{RuntimeTask, RuntimeTaskExecutor, RuntimeTaskHandle};

#[derive(Clone, Copy)]
struct DiscardWorldCacheTaskExecutor;

struct FinishedWorldCacheTaskHandle;

impl RuntimeTaskExecutor for DiscardWorldCacheTaskExecutor {
    fn spawn(&self, _task: RuntimeTask) -> Box<dyn RuntimeTaskHandle> {
        Box::new(FinishedWorldCacheTaskHandle)
    }
}

impl RuntimeTaskHandle for FinishedWorldCacheTaskHandle {
    fn abort(&self) {}

    fn is_finished(&self) -> bool {
        true
    }

    fn join_or_abort(&mut self, _timeout: Duration) {}
}

#[test]
fn enrich_projection_world_names_returns_unresolved_world_ids() -> Result<()> {
    let (_dir, runtime, _active_session) = runtime_with_active_session("world-name-enrichment")?;
    let mut entries = vec![json!({
        "type": "GPS",
        "created_at": "2026-06-21T00:00:00.000Z",
        "userId": "usr_location",
        "location": "wrld_missing:123",
        "worldName": "wrld_missing"
    })];

    let unresolved_world_ids = runtime
        .runtime()
        .enrich_projection_world_names(&mut entries);

    assert_eq!(unresolved_world_ids.len(), 1);
    assert_eq!(unresolved_world_ids[0].world_id, "wrld_missing");
    let entry = unresolved_world_ids[0].entry.as_ref().unwrap();
    assert_eq!(entry.stream, RealtimeEntryCorrectionStream::Feed);
    assert_eq!(
        entry.id,
        "GPS:2026-06-21T00:00:00.000Z:usr_location:wrld_missing:123:"
    );
    assert_eq!(entries[0]["worldName"], "wrld_missing");
    Ok(())
}

#[test]
fn feed_entry_correction_id_matches_frontend_golden_vectors() {
    let vectors = [
        (
            json!({
                "id": "feed-entry-1",
                "type": "GPS",
                "rowId": "10",
                "sourceRank": "2"
            }),
            "id:feed-entry-1",
        ),
        (
            json!({
                "type": "GPS",
                "rowId": "10",
                "sourceRank": "2"
            }),
            "row:GPS:2:10",
        ),
        (
            json!({
                "type": "Online",
                "row_id": "11",
                "source_rank": "3"
            }),
            "row:Online:3:11",
        ),
        (
            json!({
                "type": "invite",
                "created_at": "2026-06-21T00:00:00.000Z",
                "userId": "usr_sender",
                "details": {
                    "location": "wrld_world:123"
                },
                "message": "Join me"
            }),
            "invite:2026-06-21T00:00:00.000Z:usr_sender:wrld_world:123:Join me",
        ),
    ];

    for (input, expected) in vectors {
        let object = input.as_object().unwrap();
        assert_eq!(
            super::enrichment::feed_entry_correction_id(object),
            expected
        );
    }
}

#[test]
fn world_cache_name_lookup_does_not_fallback_to_db_hot_path() -> Result<()> {
    let (dir, db) = {
        let dir = TestDir::new("world-cache-fast-path");
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?);
        (dir, db)
    };
    world_cache_upsert(
        db.as_ref(),
        cached_world_entry("wrld_db_only", "DB Only World", "2026-01-01T00:00:00.000Z"),
    )?;
    let cache =
        vrcx_0_application_core::WorldCache::new(Arc::clone(&db), 1, Duration::from_secs(60));

    assert_eq!(cache.get_name("wrld_db_only"), None);
    drop(dir);
    Ok(())
}

#[test]
fn realtime_start_does_not_preload_world_cache_rows() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("world-cache-starts-empty")?;
    world_cache_upsert(
        runtime.database(),
        cached_world_entry("wrld_db_only", "DB Only World", "2026-01-01T00:00:00.000Z"),
    )?;
    runtime.set_task_executor_for_test(DiscardWorldCacheTaskExecutor);

    runtime.runtime().start(
        active_session.user_id,
        active_session.endpoint,
        active_session.websocket,
        1,
        json!({"id": "usr_self"}),
        Default::default(),
    )?;

    assert_eq!(runtime.runtime().world_cache.get_name("wrld_db_only"), None);
    Ok(())
}

#[test]
fn failed_world_name_warm_drains_pending_corrections_without_emit() -> Result<()> {
    let (_dir, runtime, _active_session) = runtime_with_active_session("world-warm-failure-drain")?;
    {
        let mut state = runtime.runtime().state.lock().unwrap();
        state.world_enrichment.inflight.insert("wrld_fail".into());
        state.world_enrichment.pending_corrections.insert(
            "wrld_fail".into(),
            vec![PendingEntryCorrection {
                stream: RealtimeEntryCorrectionStream::Feed,
                id: "GPS:2026-06-21T00:00:00.000Z:usr_location:wrld_fail:123:".into(),
                location: "wrld_fail:123".into(),
                group_name: String::new(),
            }],
        );
    }

    runtime
        .runtime()
        .resolve_pending_world_corrections("wrld_fail", None);

    let state = runtime.runtime().state.lock().unwrap();
    assert!(!state.world_enrichment.inflight.contains("wrld_fail"));
    assert!(!state
        .world_enrichment
        .pending_corrections
        .contains_key("wrld_fail"));
    drop(state);
    assert!(runtime
        .runtime()
        .deps
        .event_bus
        .take_events_for_test()
        .is_empty());
    Ok(())
}

#[test]
fn resolved_feed_world_name_patches_the_rust_cache_and_emits_feed_projection() -> Result<()> {
    let (_dir, runtime, _active_session) =
        runtime_with_active_session("world-warm-feed-correction")?;
    runtime.runtime().emit_feed_entries(
        7,
        "usr_self",
        vec![json!({
            "type": "GPS",
            "created_at": "2026-06-21T00:00:00.000Z",
            "userId": "usr_location",
            "location": "wrld_pending:123",
            "worldName": "wrld_pending"
        })],
    );
    runtime.runtime().deps.event_bus.take_events_for_test();
    {
        let mut state = runtime.runtime().state.lock().unwrap();
        state
            .world_enrichment
            .inflight
            .insert("wrld_pending".into());
        state.world_enrichment.pending_corrections.insert(
            "wrld_pending".into(),
            vec![PendingEntryCorrection {
                stream: RealtimeEntryCorrectionStream::Feed,
                id: "GPS:2026-06-21T00:00:00.000Z:usr_location:wrld_pending:123:".into(),
                location: "wrld_pending:123".into(),
                group_name: String::new(),
            }],
        );
    }

    runtime
        .runtime()
        .resolve_pending_world_corrections("wrld_pending", Some("Resolved World"));

    let events = runtime.runtime().deps.event_bus.take_events_for_test();
    let projection = events
        .iter()
        .find(|event| event.name == "realtimeFeedProjection")
        .expect("resolved Feed world should emit a Feed correction");
    assert_eq!(projection.payload["patches"][0]["sequence"], 2);
    assert_eq!(
        projection.payload["patches"][0]["fields"]["worldName"],
        "Resolved World"
    );
    assert!(events
        .iter()
        .all(|event| event.name != "realtimeEntryCorrection"));
    Ok(())
}

#[test]
fn notify_favorites_changed_emits_event_and_normalizes_vrc_plus_world() -> Result<()> {
    let (_dir, runtime, _active_session) = runtime_with_active_session("favorites-changed-notify")?;

    runtime.runtime().notify_favorites_changed(
        vrcx_0_application_core::FavoritesChangedPayload::invalidated(
            &vrcx_0_application_core::RuntimeAuthScopeSnapshot {
                current_user_id: "usr_self".into(),
                endpoint: "https://api.vrchat.cloud/api/1".into(),
                generation: 1,
                active: true,
            },
            vrcx_0_application_core::FavoriteChangeScope::World,
            true,
            false,
        ),
    );

    let events = runtime.runtime().deps.event_bus.take_events_for_test();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].name, "favoritesChanged");
    assert_eq!(events[0].payload["kind"], "world");
    assert_eq!(events[0].payload["local"], true);
    assert_eq!(events[0].payload["remote"], false);
    Ok(())
}
