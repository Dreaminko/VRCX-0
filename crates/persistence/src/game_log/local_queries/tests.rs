use super::*;
use serde_json::json;
use std::path::PathBuf;
use vrcx_0_core::json::RawJson;

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

struct TestDatabase {
    _dir: TestDir,
    db: DatabaseService,
}

fn test_db(name: &str) -> Result<TestDatabase, crate::Error> {
    let dir = TestDir::new(name);
    let db = DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?;
    ensure_game_log_tables(&db)?;
    Ok(TestDatabase { _dir: dir, db })
}

fn query(db: &DatabaseService, kind: &str, params: serde_json::Value) -> Result<Value, Error> {
    game_log_query(
        db,
        GameLogQueryInput {
            kind: kind.into(),
            params: RawJson::from(params),
        },
    )
}

fn rows(value: Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}

fn row_texts(rows: &[Value], key: &str) -> Vec<String> {
    rows.iter()
        .filter_map(|row| {
            row.get(key)
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

fn seed_fixture(db: &DatabaseService) -> Result<(), Error> {
    write_game_log_batch(
        db,
        &GameLogWriteBatch {
            locations: vec![
                GameLogLocationEntry {
                    created_at: "2026-05-14T08:00:00Z".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    world_id: "wrld_alpha".into(),
                    world_name: "Alpha World".into(),
                    time: 60_000,
                    group_name: "Group Alpha".into(),
                },
                GameLogLocationEntry {
                    created_at: "2026-05-14T09:00:00Z".into(),
                    location: "wrld_beta:inst-b".into(),
                    world_id: "wrld_beta".into(),
                    world_name: "Beta World".into(),
                    time: 120_000,
                    group_name: String::new(),
                },
                GameLogLocationEntry {
                    created_at: "2026-05-14T10:00:00Z".into(),
                    location: "wrld_alpha:inst-c~group(grp_alpha)".into(),
                    world_id: "wrld_alpha".into(),
                    world_name: "Alpha World".into(),
                    time: 90_000,
                    group_name: "Group Alpha".into(),
                },
            ],
            join_leave: vec![
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T08:01:00Z".into(),
                    event_type: "OnPlayerJoined".into(),
                    display_name: "Vip Friend".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_vip".into(),
                    world_name: "Alpha World".into(),
                    time: 0,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T08:02:00Z".into(),
                    event_type: "OnPlayerJoined".into(),
                    display_name: "Self User".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_self".into(),
                    world_name: "Alpha World".into(),
                    time: 0,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T08:03:00Z".into(),
                    event_type: "OnPlayerJoined".into(),
                    display_name: "Other Friend".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_other".into(),
                    world_name: "Alpha World".into(),
                    time: 0,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T08:30:00Z".into(),
                    event_type: "OnPlayerLeft".into(),
                    display_name: "Vip Friend".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_vip".into(),
                    world_name: "Alpha World".into(),
                    time: 1_800_000,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T09:10:00Z".into(),
                    event_type: "OnPlayerJoined".into(),
                    display_name: "Old Target".into(),
                    location: "wrld_beta:inst-b".into(),
                    user_id: "usr_target".into(),
                    world_name: "Beta World".into(),
                    time: 0,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T09:20:00Z".into(),
                    event_type: "OnPlayerLeft".into(),
                    display_name: "New Target".into(),
                    location: "wrld_beta:inst-b".into(),
                    user_id: "usr_target".into(),
                    world_name: "Beta World".into(),
                    time: 900_000,
                },
                GameLogJoinLeaveEntry {
                    created_at: "2026-05-14T10:05:00Z".into(),
                    event_type: "OnPlayerJoined".into(),
                    display_name: "Late Friend".into(),
                    location: "wrld_alpha:inst-c~group(grp_alpha)".into(),
                    user_id: "usr_late".into(),
                    world_name: "Alpha World".into(),
                    time: 0,
                },
            ],
            portal_spawns: vec![
                GameLogPortalSpawnEntry {
                    created_at: "2026-05-14T08:04:00Z".into(),
                    display_name: "Vip Friend".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_vip".into(),
                    instance_id: "wrld_portal:123".into(),
                    world_name: "Portal World".into(),
                },
                GameLogPortalSpawnEntry {
                    created_at: "2026-05-14T08:05:00Z".into(),
                    display_name: "Other Friend".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    user_id: "usr_other".into(),
                    instance_id: "wrld_other:123".into(),
                    world_name: "Other World".into(),
                },
            ],
            video_plays: vec![
                GameLogVideoPlayEntry {
                    created_at: "2026-05-14T08:06:00Z".into(),
                    video_url: "https://video.example/needle".into(),
                    video_name: "Needle Video".into(),
                    video_id: "vid_needle".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                    display_name: "Vip Friend".into(),
                    user_id: "usr_vip".into(),
                },
                GameLogVideoPlayEntry {
                    created_at: "2026-05-14T09:06:00Z".into(),
                    video_url: "https://video.example/other".into(),
                    video_name: "Other Video".into(),
                    video_id: "vid_other".into(),
                    location: "wrld_beta:inst-b".into(),
                    display_name: "Other Friend".into(),
                    user_id: "usr_other".into(),
                },
            ],
            resource_loads: vec![
                GameLogResourceLoadEntry {
                    created_at: "2026-05-14T08:07:00Z".into(),
                    resource_url: "https://assets.example/needle.png".into(),
                    resource_type: "ImageLoad".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                },
                GameLogResourceLoadEntry {
                    created_at: "2026-05-14T08:08:00Z".into(),
                    resource_url: "https://assets.example/string.json".into(),
                    resource_type: "StringLoad".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                },
                GameLogResourceLoadEntry {
                    created_at: "2026-05-14T09:08:00Z".into(),
                    resource_url: "https://assets.example/beta.png".into(),
                    resource_type: "ImageLoad".into(),
                    location: "wrld_beta:inst-b".into(),
                },
            ],
            events: vec![GameLogEventEntry {
                created_at: "2026-05-14T08:09:00Z".into(),
                data: "Needle Event".into(),
            }],
            externals: vec![
                GameLogExternalEntry {
                    created_at: "2026-05-14T08:10:00Z".into(),
                    message: "Needle External".into(),
                    display_name: "Vip Friend".into(),
                    user_id: "usr_vip".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                },
                GameLogExternalEntry {
                    created_at: "2026-05-14T08:11:00Z".into(),
                    message: "Other External".into(),
                    display_name: "Other Friend".into(),
                    user_id: "usr_other".into(),
                    location: "wrld_alpha:inst-a~group(grp_alpha)".into(),
                },
            ],
            ..GameLogWriteBatch::default()
        },
    )?;
    Ok(())
}

#[test]
fn rejects_unknown_game_log_entry_kind() {
    let error = game_log_batch_for_kind(
        "UnknownKind",
        vec![json!({
            "created_at": "2026-05-15T00:00:00Z"
        })],
    )
    .unwrap_err();

    assert!(matches!(error, crate::Error::InvalidData(_)));
}

#[test]
fn recent_database_sorts_across_tables_and_clamps_page_size() -> Result<(), crate::Error> {
    let test_db = test_db("local-query-recent")?;
    seed_fixture(&test_db.db)?;

    let recent = rows(query(
        &test_db.db,
        "recentDatabase",
        json!({
            "dateOffset": "2026-05-14",
            "maxTableSize": 2
        }),
    )?);

    assert_eq!(
        row_texts(&recent, "created_at"),
        vec![
            "2026-05-14T10:00:00Z".to_string(),
            "2026-05-14T10:05:00Z".to_string(),
        ]
    );

    let empty_page = rows(query(
        &test_db.db,
        "recentDatabase",
        json!({
            "dateOffset": "2026-05-14",
            "maxTableSize": -1
        }),
    )?);
    assert!(empty_page.is_empty());
    Ok(())
}

#[test]
fn lookup_rows_respects_filters_vip_and_limit() -> Result<(), crate::Error> {
    let test_db = test_db("local-query-lookup")?;
    seed_fixture(&test_db.db)?;

    let result = rows(query(
        &test_db.db,
        "lookupRows",
        json!({
            "filters": ["OnPlayerJoined", "PortalSpawn", "External", "VideoPlay"],
            "vipList": ["usr_vip"],
            "maxEntries": 10
        }),
    )?);
    let types = row_texts(&result, "type");
    let user_ids = row_texts(&result, "userId");

    assert_eq!(
        types,
        vec![
            "External".to_string(),
            "VideoPlay".to_string(),
            "PortalSpawn".to_string(),
            "OnPlayerJoined".to_string(),
        ]
    );
    assert!(user_ids.iter().all(|user_id| user_id == "usr_vip"));

    let limited = rows(query(
        &test_db.db,
        "lookupRows",
        json!({
            "filters": ["OnPlayerJoined", "OnPlayerLeft"],
            "maxEntries": 1
        }),
    )?);
    assert_eq!(limited.len(), 1);
    assert_eq!(limited[0]["created_at"], "2026-05-14T10:05:00Z");
    Ok(())
}

#[test]
fn rows_by_location_filters_current_user_resource_kind_and_empty_filters(
) -> Result<(), crate::Error> {
    let test_db = test_db("local-query-location")?;
    seed_fixture(&test_db.db)?;

    let result = rows(query(
        &test_db.db,
        "rowsByLocation",
        json!({
            "instanceId": "inst-a",
            "currentUserId": "usr_self",
            "filters": ["OnPlayerJoined", "ImageLoad"],
            "maxEntries": 20
        }),
    )?);
    let types = row_texts(&result, "type");
    let user_ids = row_texts(&result, "userId");

    assert_eq!(
        types,
        vec![
            "ImageLoad".to_string(),
            "OnPlayerJoined".to_string(),
            "OnPlayerJoined".to_string(),
        ]
    );
    assert!(!user_ids.contains(&"usr_self".to_string()));
    assert!(!types.contains(&"StringLoad".to_string()));

    let empty = rows(query(
        &test_db.db,
        "rowsByLocation",
        json!({
            "instanceId": "inst-a",
            "filters": ["Event"]
        }),
    )?);
    assert!(empty.is_empty());
    Ok(())
}

#[test]
fn search_rows_matches_all_searchable_event_families() -> Result<(), crate::Error> {
    let test_db = test_db("local-query-search")?;
    seed_fixture(&test_db.db)?;

    let result = rows(query(
        &test_db.db,
        "searchRows",
        json!({
            "search": "Needle",
            "currentUserId": "usr_self",
            "filters": ["Location", "Event", "External", "VideoPlay", "ImageLoad"],
            "maxEntries": 20
        }),
    )?);
    let types = row_texts(&result, "type");

    assert_eq!(
        types,
        vec![
            "External".to_string(),
            "Event".to_string(),
            "ImageLoad".to_string(),
            "VideoPlay".to_string(),
        ]
    );
    assert_eq!(result[0]["message"], "Needle External");
    assert_eq!(result[1]["data"], "Needle Event");
    assert_eq!(
        result[2]["resourceUrl"],
        "https://assets.example/needle.png"
    );
    assert_eq!(result[3]["videoName"], "Needle Video");
    Ok(())
}

#[test]
fn world_and_user_stat_queries_return_defaults_and_skip_current_matches() -> Result<(), crate::Error>
{
    let test_db = test_db("local-query-stats")?;
    seed_fixture(&test_db.db)?;

    assert_eq!(
        query(
            &test_db.db,
            "lastVisit",
            json!({
                "worldId": "wrld_alpha",
                "currentWorldMatch": true
            }),
        )?["created_at"],
        "2026-05-14T08:00:00Z"
    );
    assert_eq!(
        query(
            &test_db.db,
            "visitCount",
            json!({ "worldId": "wrld_alpha" })
        )?["visitCount"],
        2
    );
    assert_eq!(
        query(
            &test_db.db,
            "timeSpentInWorld",
            json!({ "worldId": "wrld_alpha" }),
        )?["timeSpent"],
        150_000
    );
    assert_eq!(
        query(
            &test_db.db,
            "lastGroupVisit",
            json!({ "groupId": "grp_alpha" })
        )?["created_at"],
        "2026-05-14T10:00:00Z"
    );
    assert_eq!(
        query(
            &test_db.db,
            "lastSeen",
            json!({
                "userId": "usr_vip",
                "displayName": "Vip Friend",
                "inCurrentWorld": true
            }),
        )?["created_at"],
        "2026-05-14T08:01:00Z"
    );
    assert_eq!(
        query(
            &test_db.db,
            "joinCount",
            json!({
                "userId": "usr_vip",
                "displayName": "Vip Friend"
            }),
        )?["joinCount"],
        1
    );
    assert_eq!(
        query(
            &test_db.db,
            "timeSpent",
            json!({
                "userId": "usr_vip",
                "displayName": "Vip Friend"
            }),
        )?["timeSpent"],
        1_800_000
    );

    let stats = query(
        &test_db.db,
        "userStats",
        json!({
            "userId": "usr_target",
            "displayName": "New Target"
        }),
    )?;
    assert_eq!(stats["joinCount"], 1);
    assert_eq!(stats["timeSpent"], 900_000);
    assert_eq!(
        stats["previousDisplayNames"][0]["displayName"],
        "Old Target"
    );

    assert_eq!(
        query(
            &test_db.db,
            "lastVisit",
            json!({ "worldId": "wrld_missing" })
        )?,
        json!({ "created_at": "", "worldId": "" })
    );
    Ok(())
}

#[test]
fn aggregate_and_lookup_queries_cover_group_world_player_and_dates() -> Result<(), crate::Error> {
    let test_db = test_db("local-query-aggregates")?;
    seed_fixture(&test_db.db)?;

    let group_rows = rows(query(
        &test_db.db,
        "previousInstancesByGroupId",
        json!({ "groupId": "grp_alpha" }),
    )?);
    assert_eq!(group_rows.len(), 2);
    assert_eq!(
        group_rows[0]["location"],
        "wrld_alpha:inst-c~group(grp_alpha)"
    );

    let all_stats = rows(query(
        &test_db.db,
        "allUserStats",
        json!({
            "userIds": ["usr_vip"],
            "displayNames": ["New Target"]
        }),
    )?);
    assert_eq!(all_stats.len(), 2);

    assert_eq!(
        query(&test_db.db, "lastDate", json!({}))?,
        json!("2026-05-14T10:05:00Z")
    );

    let previous_by_user = rows(query(
        &test_db.db,
        "previousInstancesByUserIdRows",
        json!({
            "userId": "usr_vip",
            "dateFrom": "2026-05-14T08:00:00Z",
            "dateTo": "2026-05-14T08:10:00Z"
        }),
    )?);
    assert_eq!(previous_by_user.len(), 1);
    assert_eq!(previous_by_user[0]["worldName"], "Alpha World");

    let world_rows = rows(query(
        &test_db.db,
        "previousInstancesByWorldId",
        json!({ "worldId": "wrld_alpha" }),
    )?);
    assert_eq!(world_rows.len(), 2);

    let players = rows(query(
        &test_db.db,
        "playersFromInstanceRows",
        json!({ "location": "wrld_alpha:inst-a~group(grp_alpha)" }),
    )?);
    assert_eq!(players.len(), 4);

    assert_eq!(
        query(
            &test_db.db,
            "locationBeforeOrAt",
            json!({ "createdAt": "2026-05-14T09:30:00Z" }),
        )?["worldId"],
        "wrld_beta"
    );

    let range = rows(query(
        &test_db.db,
        "joinLeaveRange",
        json!({
            "location": "wrld_alpha:inst-a~group(grp_alpha)",
            "afterDate": "2026-05-14T08:00:00Z",
            "beforeDate": "2026-05-14T08:02:00Z"
        }),
    )?);
    assert_eq!(range.len(), 2);

    let detail = rows(query(
        &test_db.db,
        "playerDetailFromInstance",
        json!({ "location": "wrld_alpha:inst-a~group(grp_alpha)" }),
    )?);
    assert_eq!(detail[0]["display_name"], "Vip Friend");

    let names = rows(query(
        &test_db.db,
        "previousDisplayNamesByUserId",
        json!({ "userId": "usr_target" }),
    )?);
    assert_eq!(
        row_texts(&names, "displayName"),
        vec!["New Target".to_string(), "Old Target".to_string(),]
    );

    let instance_times = rows(query(&test_db.db, "instanceTimes", json!({}))?);
    assert_eq!(instance_times.len(), 3);

    let online = rows(query(
        &test_db.db,
        "onlineSessions",
        json!({
            "fromDate": "2026-05-14T09:30:00Z",
            "toDate": "2026-05-14T10:30:00Z"
        }),
    )?);
    assert_eq!(
        row_texts(&online, "created_at"),
        vec![
            "2026-05-14T09:00:00Z".to_string(),
            "2026-05-14T10:00:00Z".to_string(),
        ]
    );

    let after = rows(query(
        &test_db.db,
        "onlineSessionsAfter",
        json!({
            "afterCreatedAt": "2026-05-14T09:00:00Z",
            "inclusive": false
        }),
    )?);
    assert_eq!(
        row_texts(&after, "created_at"),
        vec!["2026-05-14T10:00:00Z".to_string(),]
    );

    let top = rows(query(
        &test_db.db,
        "topWorlds",
        json!({
            "limit": 1,
            "sortBy": "count",
            "excludeWorldId": "wrld_beta"
        }),
    )?);
    assert_eq!(top[0]["worldId"], "wrld_alpha");
    Ok(())
}

#[test]
fn activity_and_session_queries_cover_empty_and_cursor_edges() -> Result<(), crate::Error> {
    let test_db = test_db("local-query-sessions")?;
    seed_fixture(&test_db.db)?;

    let activity = rows(query(
        &test_db.db,
        "instanceActivityRows",
        json!({
            "startDate": "2026-05-14T08:00:00Z",
            "endDate": "2026-05-14T08:10:00Z"
        }),
    )?);
    assert_eq!(activity[0]["display_name"], "Vip Friend");

    assert_eq!(
        rows(query(
            &test_db.db,
            "dateOfInstanceActivity",
            json!({ "userId": "usr_vip" }),
        )?),
        vec![json!("2026-05-14T08:01:00Z"), json!("2026-05-14T08:30:00Z")]
    );

    let join_history = rows(query(
        &test_db.db,
        "instanceJoinHistory",
        json!({
            "userId": "usr_vip",
            "createdAt": "2026-05-14T08:00:00Z"
        }),
    )?);
    assert_eq!(
        join_history[0]["location"],
        "wrld_alpha:inst-a~group(grp_alpha)"
    );

    assert_eq!(
        query(
            &test_db.db,
            "worldNameByWorldId",
            json!({ "worldId": "wrld_alpha" }),
        )?,
        json!("Alpha World")
    );
    assert_eq!(
        query(
            &test_db.db,
            "userIdFromDisplayName",
            json!({ "displayName": "Vip Friend" }),
        )?,
        json!("usr_vip")
    );

    Ok(())
}
