#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use serde_json::json;
    use vrcx_0_core::friends::FriendRecord;
    use vrcx_0_persistence::cache_entities::CacheEntityInput;
    use vrcx_0_persistence::config as config_store;
    use vrcx_0_persistence::favorites::favorite_add;
    use vrcx_0_persistence::notifications::{notification_list_query, NotificationListQueryInput};
    use vrcx_0_persistence::realtime::{NotificationV2Update, RealtimePersistenceBatch};
    use vrcx_0_persistence::storage::StorageService;
    use vrcx_0_persistence::worlds::world_cache_upsert;
    use vrcx_0_persistence::DatabaseService;

    use crate::overlay_activity::{
        OverlayActivityCandidate, OverlayActivityFilters, OverlayActivityRuntime,
    };
    use crate::world_enrich::PendingEntryCorrection;
    use crate::{
        HostSessionRuntime, PrintCleanupQueue, RealtimeNotificationUpsert, RuntimeEventBus,
        RuntimeSnapshot, RuntimeSyncEngine, TaskSupervisor, WebClient,
    };

    use super::super::types::{
        ActiveRealtimeContext, PendingFriendBaseline, RealtimeHostRuntimeMessageSink,
        RealtimeHostRuntimeState,
    };
    use super::super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-realtime-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn runtime_with_active_session(
        name: &str,
    ) -> Result<(TestDir, Arc<RealtimeHostRuntime>, RealtimeSessionContext)> {
        let dir = TestDir::new(name);
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?);
        let storage = StorageService::new(&dir.path.join("storage.json"))?;
        let web = Arc::new(WebClient::new(
            &storage,
            db.as_ref(),
            "wss://pipeline.vrchat.cloud".to_string(),
            env!("CARGO_PKG_VERSION"),
        )?);
        let session = HostSessionRuntime::new();
        let host_session_generation =
            session.set_realtime_context(crate::session::RealtimeSessionContext::new(
                "usr_self".into(),
                "https://api.vrchat.cloud/api/1".into(),
                "wss://pipeline.vrchat.cloud".into(),
            ));
        let world_cache = Arc::new(crate::world_cache::WorldCache::new(
            Arc::clone(&db),
            512,
            Duration::from_secs(30 * 60),
        ));
        let runtime = Arc::new(RealtimeHostRuntime::new(RealtimeHostRuntimeDeps {
            db,
            web,
            event_bus: RuntimeEventBus::new(),
            sync: RuntimeSyncEngine::new(),
            tasks: TaskSupervisor::new(),
            session,
            auth_scope: RuntimeAuthScope::new(),
            game_log_snapshot: Arc::new(Mutex::new(RuntimeSnapshot::default())),
            overlay_activity: OverlayActivityRuntime::default(),
            world_cache,
            print_cleanup: PrintCleanupQueue::new(),
        }));
        let active_session = RealtimeSessionContext::new(
            "usr_self".into(),
            "https://api.vrchat.cloud/api/1".into(),
            "wss://pipeline.vrchat.cloud".into(),
        );
        {
            let mut state = runtime.state.lock().unwrap();
            *state = RealtimeHostRuntimeState {
                generation: 7,
                active_context: Some(ActiveRealtimeContext {
                    session: active_session.clone(),
                    generation: 7,
                    client_run_id: 1,
                    session_generation: host_session_generation,
                }),
                ..RealtimeHostRuntimeState::default()
            };
        }
        Ok((dir, runtime, active_session))
    }

    fn cached_world_entry(id: &str, name: &str, updated_at: &str) -> CacheEntityInput {
        CacheEntityInput {
            id: json!(id),
            author_id: json!(null),
            author_name: json!(null),
            created_at: json!("2026-01-01T00:00:00.000Z"),
            description: json!(null),
            image_url: json!("image.png"),
            name: json!(name),
            release_status: json!("public"),
            thumbnail_image_url: json!("thumb.png"),
            updated_at: json!(updated_at),
            version: json!(1),
        }
    }

    #[test]
    fn sync_friend_snapshot_updates_overlay_friend_scope() -> Result<()> {
        let dir = TestDir::new("overlay-friend-scope");
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?);
        let storage = StorageService::new(&dir.path.join("storage.json"))?;
        let web = Arc::new(WebClient::new(
            &storage,
            db.as_ref(),
            "wss://pipeline.vrchat.cloud".to_string(),
            env!("CARGO_PKG_VERSION"),
        )?);
        let session = HostSessionRuntime::new();
        let host_session_generation =
            session.set_realtime_context(crate::session::RealtimeSessionContext::new(
                "usr_self".into(),
                "https://api.vrchat.cloud/api/1".into(),
                "wss://pipeline.vrchat.cloud".into(),
            ));
        let overlay_activity =
            OverlayActivityRuntime::with_filters(OverlayActivityFilters::from_json(json!({
                "version": 1,
                "wrist": {
                    "types": {
                        "invite": {
                            "scope": "friends",
                            "favoriteGroupKeys": "all"
                        }
                    }
                }
            })));
        let world_cache = Arc::new(crate::world_cache::WorldCache::new(
            Arc::clone(&db),
            512,
            Duration::from_secs(30 * 60),
        ));
        let runtime = Arc::new(RealtimeHostRuntime::new(RealtimeHostRuntimeDeps {
            db,
            web,
            event_bus: RuntimeEventBus::new(),
            sync: RuntimeSyncEngine::new(),
            tasks: TaskSupervisor::new(),
            session,
            auth_scope: RuntimeAuthScope::new(),
            game_log_snapshot: Arc::new(Mutex::new(RuntimeSnapshot::default())),
            overlay_activity: overlay_activity.clone(),
            world_cache,
            print_cleanup: PrintCleanupQueue::new(),
        }));
        let active_session = RealtimeSessionContext::new(
            "usr_self".into(),
            "https://api.vrchat.cloud/api/1".into(),
            "wss://pipeline.vrchat.cloud".into(),
        );
        {
            let mut state = runtime.state.lock().unwrap();
            *state = RealtimeHostRuntimeState {
                generation: 7,
                active_context: Some(ActiveRealtimeContext {
                    session: active_session.clone(),
                    generation: 7,
                    client_run_id: 1,
                    session_generation: host_session_generation,
                }),
                ..RealtimeHostRuntimeState::default()
            };
        }
        let mut friends_by_id = HashMap::new();
        friends_by_id.insert(
            "usr_new".to_string(),
            FriendRecord {
                id: "usr_new".to_string(),
                display_name: "New Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                ..FriendRecord::default()
            },
        );

        let result = runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            friends_by_id,
        )?;

        assert!(result.accepted);
        assert!(overlay_activity
            .ingest_candidate(invite_candidate("usr_new"))
            .is_some());
        Ok(())
    }

    #[test]
    fn sync_friend_snapshot_debounces_online_to_offline() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("baseline-projection")?;
        let mut initial_friends = HashMap::new();
        initial_friends.insert(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".to_string(),
                display_name: "Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                location: "wrld_old:123".to_string(),
                ..FriendRecord::default()
            },
        );
        runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            initial_friends,
        )?;
        runtime.deps.event_bus.take_events_for_test();

        let mut refreshed_friends = HashMap::new();
        refreshed_friends.insert(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".to_string(),
                display_name: "Friend".to_string(),
                state: "offline".to_string(),
                state_bucket: "offline".to_string(),
                location: "offline".to_string(),
                ..FriendRecord::default()
            },
        );
        let result = runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            refreshed_friends,
        )?;

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeFriendProjection")
            .expect("baseline refresh should emit a friend projection");
        assert!(result.accepted);
        assert_eq!(result.baseline_revision, 1);
        assert_eq!(projection.payload["generation"], 7);
        assert_eq!(projection.payload["baselineRevision"], 1);
        assert_eq!(projection.payload["patches"].as_array().unwrap().len(), 1);
        assert_eq!(projection.payload["patches"][0]["userId"], "usr_friend");
        assert_eq!(projection.payload["patches"][0]["stateBucket"], "online");
        assert_eq!(
            projection.payload["patches"][0]["patch"]["stateBucket"],
            "online"
        );
        assert_eq!(
            projection.payload["patches"][0]["patch"]["location"],
            "wrld_old:123"
        );
        assert_eq!(
            projection.payload["patches"][0]["patch"]["pendingOffline"],
            true
        );
        Ok(())
    }

    #[test]
    fn sync_friend_snapshot_emits_projection_for_active_removals() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("baseline-removal")?;
        let mut initial_friends = HashMap::new();
        initial_friends.insert(
            "usr_removed".to_string(),
            FriendRecord {
                id: "usr_removed".to_string(),
                display_name: "Removed Friend".to_string(),
                state: "offline".to_string(),
                state_bucket: "offline".to_string(),
                ..FriendRecord::default()
            },
        );
        runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            initial_friends,
        )?;
        runtime.deps.event_bus.take_events_for_test();

        let result = runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            HashMap::new(),
        )?;

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeFriendProjection")
            .expect("baseline removal should emit a friend projection");
        assert!(result.accepted);
        assert_eq!(result.baseline_revision, 1);
        assert!(projection.payload["patches"].as_array().unwrap().is_empty());
        assert_eq!(
            projection.payload["removals"].as_array().unwrap(),
            &vec![json!("usr_removed")]
        );
        Ok(())
    }

    #[test]
    fn apply_friend_profile_refresh_updates_existing_friend_only() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("profile-refresh")?;
        let mut friends_by_id = HashMap::new();
        friends_by_id.insert(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".to_string(),
                display_name: "Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                location: "wrld_old:123".to_string(),
                ..FriendRecord::default()
            },
        );
        runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(7),
            friends_by_id,
        )?;

        let updated = runtime.apply_friend_profile_refresh(
            active_session.endpoint.clone(),
            "usr_friend".into(),
            json!({
                "id": "usr_friend",
                "displayName": "Fresh Friend",
                "state": "online",
                "location": "wrld_fresh:456"
            }),
        )?;
        let stranger_added = runtime.apply_friend_profile_refresh(
            active_session.endpoint.clone(),
            "usr_stranger".into(),
            json!({
                "id": "usr_stranger",
                "displayName": "Stranger",
                "state": "online"
            }),
        )?;

        let snapshot = runtime.friend_snapshot().unwrap();
        let friend = snapshot.friends_by_id.get("usr_friend").unwrap();
        assert!(updated);
        assert!(!stranger_added);
        assert_eq!(friend.display_name, "Fresh Friend");
        assert_eq!(friend.location, "wrld_fresh:456");
        assert!(!snapshot.friends_by_id.contains_key("usr_stranger"));
        Ok(())
    }

    #[test]
    fn enrich_projection_world_names_returns_unresolved_world_ids() -> Result<()> {
        let (_dir, runtime, _active_session) =
            runtime_with_active_session("world-name-enrichment")?;
        let mut entries = vec![json!({
            "type": "GPS",
            "created_at": "2026-06-21T00:00:00.000Z",
            "userId": "usr_location",
            "location": "wrld_missing:123",
            "worldName": "wrld_missing"
        })];

        let unresolved_world_ids = runtime.enrich_projection_world_names(&mut entries);

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
                super::super::lifecycle_enrichment::feed_entry_correction_id(object),
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
            crate::world_cache::WorldCache::new(Arc::clone(&db), 1, Duration::from_secs(60));

        assert_eq!(cache.get_name("wrld_db_only"), None);
        drop(dir);
        Ok(())
    }

    #[test]
    fn world_cache_init_pins_favorites_and_bounds_working_set() -> Result<()> {
        let (dir, db) = {
            let dir = TestDir::new("world-cache-init-bounds");
            let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?);
            (dir, db)
        };
        world_cache_upsert(
            db.as_ref(),
            cached_world_entry(
                "wrld_favorite",
                "Favorite World",
                "2026-01-01T00:00:00.000Z",
            ),
        )?;
        world_cache_upsert(
            db.as_ref(),
            cached_world_entry("wrld_recent", "Recent World", "2026-03-01T00:00:00.000Z"),
        )?;
        world_cache_upsert(
            db.as_ref(),
            cached_world_entry("wrld_old", "Old World", "2026-02-01T00:00:00.000Z"),
        )?;
        favorite_add(
            db.as_ref(),
            "world".into(),
            "wrld_favorite".into(),
            "Favorites".into(),
        )?;
        let cache =
            crate::world_cache::WorldCache::new(Arc::clone(&db), 1, Duration::from_secs(60));

        cache.init_load();

        assert_eq!(
            cache.get_name("wrld_favorite").as_deref(),
            Some("Favorite World")
        );
        assert_eq!(
            cache.get_name("wrld_recent").as_deref(),
            Some("Recent World")
        );
        assert_eq!(cache.get_name("wrld_old"), None);
        drop(dir);
        Ok(())
    }

    #[test]
    fn notification_cache_hits_enrich_projection_and_persistence() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-cache-hit")?;
        world_cache_upsert(
            runtime.deps.db.as_ref(),
            cached_world_entry("wrld_cached", "Cached World", "2026-01-01T00:00:00.000Z"),
        )?;
        runtime.world_cache.init_load();
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_sender",
                "displayName": "Cached Sender"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-cache-hit",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "invite",
            "senderUserId": "usr_sender",
            "senderUsername": "usr_sender",
            "message": "Join me",
            "details": {
                "worldId": "wrld_cached",
                "worldName": "wrld_cached"
            }
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("cache-hit notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert_eq!(projected["senderDisplayName"], "Cached Sender");
        assert_eq!(projected["senderUsername"], "Cached Sender");
        assert_eq!(projected["details"]["worldName"], "Cached World");

        let rows = notification_list_query(
            runtime.deps.db.as_ref(),
            NotificationListQueryInput {
                user_id: active_session.user_id,
                search: String::new(),
                filters: Vec::new(),
                per_table_limit: 10,
                limit: 10,
                include_unseen: false,
            },
        )?;
        let row = rows
            .iter()
            .find(|row| row.id == "notif-cache-hit")
            .expect("notification should be persisted");
        assert_eq!(row.sender_username, "Cached Sender");
        assert_eq!(row.details["worldName"], "Cached World");
        Ok(())
    }

    #[test]
    fn notification_cache_hit_enriches_avatar_image_for_runtime_delivery() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-cache-hit")?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_sender",
                "displayName": "Cached Sender",
                "userIcon": "https://images.example/user-icon.png",
                "profilePicOverride": "https://images.example/profile.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/avatar-thumb.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-avatar-cache-hit",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "friendRequest",
            "senderUserId": "usr_sender",
            "senderUsername": "usr_sender",
            "message": "Friend request"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("cache-hit notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert_eq!(
            projected["imageUrl"],
            "https://images.example/user-icon.png"
        );

        let entries = runtime.deps.overlay_activity.snapshot().entries;
        let entry = entries
            .iter()
            .find(|entry| entry.source_id == "notification:notif-avatar-cache-hit")
            .expect("runtime delivery should be projected to overlay activity");
        assert_eq!(
            entry.content.image_url,
            "https://images.example/user-icon.png"
        );
        Ok(())
    }

    #[test]
    fn notification_avatar_resolves_from_user_id_when_sender_field_absent() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-user-id")?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_sender",
                "displayName": "Cached Sender",
                "userIcon": "https://images.example/user-icon.png",
                "profilePicOverride": "https://images.example/profile.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/avatar-thumb.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-avatar-user-id",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "friendRequest",
            "userId": "usr_sender",
            "senderUsername": "usr_sender",
            "message": "Friend request"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("user-id sender notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert_eq!(
            projected["imageUrl"],
            "https://images.example/user-icon.png"
        );
        Ok(())
    }

    #[test]
    fn notification_avatar_fallback_skips_owner_receiver_when_sender_is_absent() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-receiver")?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_self",
                "displayName": "Self",
                "userIcon": "https://images.example/self-icon.png",
                "profilePicOverride": "https://images.example/self-profile.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/self-avatar.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-avatar-receiver",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "group.announcement",
            "receiverUserId": "usr_self",
            "userId": "usr_self",
            "message": "Group announcement"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("receiver-only notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert!(projected["imageUrl"].is_null());

        let entries = runtime.deps.overlay_activity.snapshot().entries;
        let entry = entries
            .iter()
            .find(|entry| entry.source_id == "notification:notif-avatar-receiver")
            .expect("runtime delivery should be projected to overlay activity");
        assert!(entry.actor_user_id.is_empty());
        assert!(entry.content.image_url.is_empty());
        Ok(())
    }

    #[test]
    fn notification_avatar_fallback_skips_current_user_sender() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-self-sender")?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_self",
                "displayName": "Self",
                "userIcon": "https://images.example/self-icon.png",
                "profilePicOverride": "https://images.example/self-profile.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/self-avatar.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-avatar-self-sender",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "friendRequest",
            "senderUserId": "usr_self",
            "senderUsername": "Self",
            "message": "Friend request"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id,
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("self-sender notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert!(projected["imageUrl"].is_null());

        let entries = runtime.deps.overlay_activity.snapshot().entries;
        let entry = entries
            .iter()
            .find(|entry| entry.source_id == "notification:notif-avatar-self-sender")
            .expect("runtime delivery should be projected to overlay activity");
        assert_eq!(entry.actor_user_id, "usr_self");
        assert!(entry.content.image_url.is_empty());
        Ok(())
    }

    #[test]
    fn notification_avatar_fallback_respects_vrc_plus_icon_preference() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-vrc-plus-disabled")?;
        config_store::set_bool(
            runtime.deps.db.as_ref(),
            "displayVRCPlusIconsAsAvatar",
            false,
        )?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_sender",
                "displayName": "Cached Sender",
                "userIcon": "https://images.example/user-icon.png",
                "profilePicOverride": "https://images.example/profile.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/avatar-thumb.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let notification = json!({
            "id": "notif-avatar-vrc-plus-disabled",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "friendRequest",
            "senderUserId": "usr_sender",
            "senderUsername": "usr_sender",
            "message": "Friend request"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id,
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("cache-hit notification should emit a realtime projection");
        let projected = &projection.payload["upserts"][0]["notification"];
        assert_eq!(projected["imageUrl"], "https://images.example/profile.png");
        Ok(())
    }

    #[test]
    fn notification_avatar_fallback_preserves_existing_image_and_skips_group_sender() -> Result<()>
    {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-avatar-existing-and-group")?;
        runtime.ingest_user_facts(vec![json!({
            "user": {
                "id": "usr_sender",
                "displayName": "Cached Sender",
                "userIcon": "https://images.example/user-icon.png",
                "currentAvatarThumbnailImageUrl": "https://images.example/avatar-thumb.png"
            },
            "source": "test",
            "isFriend": false
        })]);
        runtime.deps.event_bus.take_events_for_test();
        let existing_image = json!({
            "id": "notif-avatar-existing",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "friendRequest",
            "senderUserId": "usr_sender",
            "senderUsername": "Cached Sender",
            "message": "Friend request",
            "imageUrl": "https://images.example/existing.png"
        });
        let group_sender = json!({
            "id": "notif-avatar-group",
            "createdAt": "2026-06-21T00:00:01.000Z",
            "type": "friendRequest",
            "senderUserId": "grp_sender",
            "senderUsername": "Group Sender",
            "message": "Group request"
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id,
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![
                    RealtimeNotificationUpsert {
                        notification: existing_image.clone(),
                        insert_defaults: None,
                        notify_menu: true,
                        deliver_runtime: true,
                        run_automation: false,
                    },
                    RealtimeNotificationUpsert {
                        notification: group_sender.clone(),
                        insert_defaults: None,
                        notify_menu: true,
                        deliver_runtime: true,
                        run_automation: false,
                    },
                ],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![existing_image, group_sender],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeNotificationProjection")
            .expect("notifications should emit a realtime projection");
        let upserts = projection.payload["upserts"]
            .as_array()
            .expect("projection upserts");
        let existing = upserts
            .iter()
            .find(|upsert| upsert["notification"]["id"] == "notif-avatar-existing")
            .expect("existing image notification");
        let group = upserts
            .iter()
            .find(|upsert| upsert["notification"]["id"] == "notif-avatar-group")
            .expect("group notification");
        assert_eq!(
            existing["notification"]["imageUrl"],
            "https://images.example/existing.png"
        );
        assert!(group["notification"]["imageUrl"].is_null());
        Ok(())
    }

    #[test]
    fn failed_world_name_warm_drains_pending_corrections_without_emit() -> Result<()> {
        let (_dir, runtime, _active_session) =
            runtime_with_active_session("world-warm-failure-drain")?;
        {
            let mut state = runtime.state.lock().unwrap();
            state.world_name_fetch_inflight.insert("wrld_fail".into());
            state.pending_world_name_corrections.insert(
                "wrld_fail".into(),
                vec![PendingEntryCorrection {
                    stream: RealtimeEntryCorrectionStream::Feed,
                    id: "GPS:2026-06-21T00:00:00.000Z:usr_location:wrld_fail:123:".into(),
                    location: "wrld_fail:123".into(),
                    group_name: String::new(),
                }],
            );
        }

        runtime.resolve_pending_world_corrections("wrld_fail", None);

        let state = runtime.state.lock().unwrap();
        assert!(!state.world_name_fetch_inflight.contains("wrld_fail"));
        assert!(!state
            .pending_world_name_corrections
            .contains_key("wrld_fail"));
        drop(state);
        assert!(runtime.deps.event_bus.take_events_for_test().is_empty());
        Ok(())
    }

    #[test]
    fn unresolved_person_location_notification_persists_without_runtime_projection() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-unresolved-basic")?;
        let notification = json!({
            "id": "notif-unresolved",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "invite",
            "senderUserId": "usr_missing",
            "senderUsername": "usr_missing",
            "message": "Join me",
            "details": {
                "worldId": "wrld_missing",
                "worldName": "wrld_missing"
            }
        });

        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: notification.clone(),
                    insert_defaults: None,
                    notify_menu: true,
                    deliver_runtime: true,
                    run_automation: true,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![notification],
                ..RealtimePersistenceBatch::default()
            },
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        assert!(
            events
                .iter()
                .all(|event| event.name != "realtimeNotificationProjection"),
            "unresolved notification should not be emitted to runtime/UI projection"
        );

        let rows = notification_list_query(
            runtime.deps.db.as_ref(),
            NotificationListQueryInput {
                user_id: active_session.user_id,
                search: String::new(),
                filters: Vec::new(),
                per_table_limit: 10,
                limit: 10,
                include_unseen: false,
            },
        )?;
        let row = rows
            .iter()
            .find(|row| row.id == "notif-unresolved")
            .expect("unresolved notification should still be persisted");
        assert_eq!(row.sender_user_id, "usr_missing");
        assert_eq!(row.sender_username, "");
        assert_eq!(row.details["worldId"], "wrld_missing");
        assert_eq!(row.details["worldName"], "");
        assert!(
            runtime
                .state
                .lock()
                .unwrap()
                .world_name_fetches
                .contains_key("wrld_missing"),
            "notification resolver failures should register async world warm"
        );
        Ok(())
    }

    #[test]
    fn notification_v2_update_sanitizes_id_like_names_before_persistence() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("notification-update-sanitize")?;
        let initial = json!({
            "id": "notif-update-sanitize",
            "createdAt": "2026-06-21T00:00:00.000Z",
            "type": "invite",
            "senderUserId": "usr_sender",
            "senderUsername": "Sender",
            "message": "Join me",
            "details": {
                "worldId": "wrld_initial",
                "worldName": "Initial World"
            }
        });
        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: initial.clone(),
                    insert_defaults: None,
                    notify_menu: false,
                    deliver_runtime: false,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_upserts: vec![initial],
                ..RealtimePersistenceBatch::default()
            },
        });
        runtime.deps.event_bus.take_events_for_test();

        let update = json!({
            "id": "notif-update-sanitize",
            "senderUserId": "usr_missing",
            "senderUsername": "usr_missing",
            "details": {
                "worldId": "wrld_missing",
                "worldName": "wrld_missing"
            }
        });
        runtime.apply_notification_output(RealtimeNotificationOutput {
            owner_user_id: active_session.user_id.clone(),
            projection: RealtimeNotificationProjection {
                generation: 7,
                upserts: vec![RealtimeNotificationUpsert {
                    notification: update.clone(),
                    insert_defaults: Some(json!({
                        "createdAt": "2026-06-21T00:01:00.000Z",
                        "created_at": "2026-06-21T00:01:00.000Z",
                        "seen": false
                    })),
                    notify_menu: false,
                    deliver_runtime: false,
                    run_automation: false,
                }],
                ..RealtimeNotificationProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                notification_v2_updates: vec![NotificationV2Update {
                    id: "notif-update-sanitize".into(),
                    updates: update,
                    received_at: "2026-06-21T00:01:00.000Z".into(),
                }],
                ..RealtimePersistenceBatch::default()
            },
        });

        let rows = notification_list_query(
            runtime.deps.db.as_ref(),
            NotificationListQueryInput {
                user_id: active_session.user_id,
                search: String::new(),
                filters: Vec::new(),
                per_table_limit: 10,
                limit: 10,
                include_unseen: false,
            },
        )?;
        let row = rows
            .iter()
            .find(|row| row.id == "notif-update-sanitize")
            .expect("notification update should be persisted");
        assert_eq!(row.sender_user_id, "usr_missing");
        assert_eq!(row.sender_username, "");
        assert_eq!(row.details["worldId"], "wrld_missing");
        assert_eq!(row.details["worldName"], "");
        Ok(())
    }

    #[test]
    fn friend_projection_clears_feed_entries_when_persistence_fails() -> Result<()> {
        let (_dir, runtime, active_session) =
            runtime_with_active_session("friend-persist-failure-clears-feed")?;
        let feed_entry = json!({
            "created_at": "2026-06-21T00:00:00.000Z",
            "type": "NewFeedType",
            "userId": "usr_friend",
            "displayName": "Friend"
        });

        runtime.apply_friend_output(RealtimeFriendOutput {
            owner_user_id: active_session.user_id,
            projection: FriendProjection {
                generation: 7,
                feed_entries: vec![feed_entry.clone()],
                ..FriendProjection::default()
            },
            persistence: RealtimePersistenceBatch {
                feed_entries: vec![feed_entry],
                ..RealtimePersistenceBatch::default()
            },
            ..RealtimeFriendOutput::default()
        });

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeFriendProjection")
            .expect("friend projection should still be emitted after persistence failure");
        assert_eq!(
            projection.payload["feedEntries"].as_array().unwrap().len(),
            0
        );
        assert!(events.iter().all(|event| {
            event.name != "backendRuntimeTelemetry" || event.payload["kind"] != "wsPersisted"
        }));
        Ok(())
    }

    #[test]
    fn connected_after_reconnect_without_snapshot_resumes_queued_friend_events() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("reconnect-drain")?;
        let active = runtime
            .state
            .lock()
            .unwrap()
            .active_context
            .clone()
            .unwrap();
        let mut friends_by_id = HashMap::new();
        friends_by_id.insert(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".to_string(),
                display_name: "Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                location: "wrld_old:123".to_string(),
                ..FriendRecord::default()
            },
        );
        runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(active.generation),
            friends_by_id,
        )?;
        runtime.deps.event_bus.take_events_for_test();

        let sink = RealtimeHostRuntimeMessageSink {
            runtime: Arc::clone(&runtime),
        };
        sink.handle_realtime_transport_status(
            active.generation,
            active.session_generation,
            &active_session,
            "reconnecting",
        );
        sink.handle_realtime_ws_message(
            active.generation,
            active.session_generation,
            &active_session,
            &RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_new:456"
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-06-08T10:05:00Z".into(),
            },
        );
        assert!(runtime.state.lock().unwrap().friend_messages_paused);

        sink.handle_realtime_transport_status(
            active.generation,
            active.session_generation,
            &active_session,
            "connected",
        );

        let events = runtime.deps.event_bus.take_events_for_test();
        let projection = events
            .iter()
            .find(|event| event.name == "realtimeFriendProjection")
            .expect("queued friend event should be drained after reconnect");
        assert!(!runtime.state.lock().unwrap().friend_messages_paused);
        assert_eq!(projection.payload["patches"][0]["userId"], "usr_friend");
        assert_eq!(
            projection.payload["patches"][0]["patch"]["location"],
            "wrld_new:456"
        );
        Ok(())
    }

    #[test]
    fn passive_reconnect_resumes_stream_without_refetching_roster() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("reconnect-no-refetch")?;
        let active = runtime
            .state
            .lock()
            .unwrap()
            .active_context
            .clone()
            .unwrap();
        let mut friends_by_id = HashMap::new();
        friends_by_id.insert(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".to_string(),
                display_name: "Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                location: "wrld_1:123".to_string(),
                ..FriendRecord::default()
            },
        );
        runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            Some(active.generation),
            friends_by_id,
        )?;
        {
            let mut state = runtime.state.lock().unwrap();
            state.pending_friend_baseline = Some(PendingFriendBaseline {
                session: active_session.clone(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".to_string(),
                        display_name: "Polluted Baseline".to_string(),
                        state: "offline".to_string(),
                        state_bucket: "offline".to_string(),
                        location: "offline".to_string(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
            });
        }

        let sink = RealtimeHostRuntimeMessageSink {
            runtime: Arc::clone(&runtime),
        };
        sink.handle_realtime_transport_status(
            active.generation,
            active.session_generation,
            &active_session,
            "reconnecting",
        );
        assert!(runtime.state.lock().unwrap().friend_messages_paused);
        sink.handle_realtime_ws_message(
            active.generation,
            active.session_generation,
            &active_session,
            &RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_2:456"
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-06-08T10:05:00Z".into(),
            },
        );
        sink.handle_realtime_transport_status(
            active.generation,
            active.session_generation,
            &active_session,
            "connected",
        );

        assert!(!runtime.state.lock().unwrap().friend_messages_paused);
        let snapshot = runtime.friend_snapshot().unwrap();
        let friend = snapshot.friends_by_id.get("usr_friend").unwrap();
        assert_eq!(friend.state_bucket, "online");
        assert_eq!(friend.display_name, "Friend");
        assert_eq!(friend.location, "wrld_2:456");
        assert!(runtime
            .state
            .lock()
            .unwrap()
            .pending_friend_baseline
            .is_some());
        Ok(())
    }

    #[test]
    fn sync_friend_snapshot_caches_pre_active_baseline() -> Result<()> {
        let (_dir, runtime, active_session) = runtime_with_active_session("pre-active-baseline")?;
        {
            let mut state = runtime.state.lock().unwrap();
            state.active_context = None;
        }
        let mut friends_by_id = HashMap::new();
        friends_by_id.insert(
            "usr_cached".to_string(),
            FriendRecord {
                id: "usr_cached".to_string(),
                display_name: "Cached Friend".to_string(),
                state: "online".to_string(),
                state_bucket: "online".to_string(),
                ..FriendRecord::default()
            },
        );

        let result = runtime.sync_friend_snapshot(
            active_session.user_id.clone(),
            active_session.endpoint.clone(),
            active_session.websocket.clone(),
            None,
            friends_by_id,
        )?;

        let state = runtime.state.lock().unwrap();
        let pending = state.pending_friend_baseline.as_ref().unwrap();
        assert!(result.accepted);
        assert_eq!(result.friend_count, 1);
        assert_eq!(pending.session, active_session);
        assert!(pending.friends_by_id.contains_key("usr_cached"));
        Ok(())
    }

    fn invite_candidate(user_id: &str) -> OverlayActivityCandidate {
        OverlayActivityCandidate {
            source_id: format!("invite:{user_id}"),
            activity_type: "invite".to_string(),
            created_at: "2026-06-01T00:00:00.000Z".to_string(),
            actor_user_id: user_id.to_string(),
            actor_display_name: "Friend".to_string(),
            current_instance: false,
            payload: json!({}),
        }
    }
}
