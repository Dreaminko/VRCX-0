#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn friend_location_with_embedded_user_without_online_location_preserves_previous_bucket() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "location": "offline"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert_eq!(
            output.projection.patches[0]
                .state_bucket_authority
                .as_deref(),
            Some("preserve")
        );
        assert!(output.persistence.feed_entries.is_empty());
        assert_eq!(output.projection.patches[0].patch["stateBucket"], "online");
        assert_eq!(output.profile_refetch_user_ids, vec!["usr_friend"]);
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .friends_by_id
                .get("usr_friend")
                .unwrap()
                .state_bucket,
            "online"
        );
    }

    #[test]
    fn friend_location_missing_embedded_user_preserves_previous_state() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_2:456"
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert_eq!(output.persistence.feed_entries[0]["type"], "GPS");
        assert_eq!(patch["stateBucket"], "online");
        assert_eq!(patch["location"], "wrld_2:456");
    }

    #[test]
    fn friend_location_offline_with_real_location_requests_profile_refetch() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "offline".into(),
                        state_bucket: "offline".into(),
                        location: "offline".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_2:456"
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        assert_eq!(output.projection.patches[0].state_bucket, "offline");
        assert_eq!(
            output.projection.patches[0]
                .state_bucket_authority
                .as_deref(),
            Some("preserve")
        );
        assert_eq!(output.projection.patches[0].patch["location"], "wrld_2:456");
        assert_eq!(output.profile_refetch_user_ids, vec!["usr_friend"]);
    }

    #[test]
    fn friend_location_with_state_change_does_not_emit_gps_feed() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "active".into(),
                        state_bucket: "active".into(),
                        location: "wrld_old:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_new:456",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "online"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:01Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert_eq!(
            output.projection.patches[0].patch["location"],
            "wrld_new:456"
        );
        assert!(output.persistence.feed_entries.is_empty());
        assert!(output.projection.feed_entries.is_empty());
    }

    #[test]
    fn duplicate_friend_location_payload_after_repeat_window_does_not_write_gps_again() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_old:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let payload = json!({
            "type": "friend-location",
            "content": {
                "userId": "usr_friend",
                "location": "wrld_new:456",
                "user": {
                    "id": "usr_friend",
                    "displayName": "Friend",
                    "state": "online"
                }
            }
        });

        let RealtimeFriendApplyResult::Output(first) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: payload.clone(),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:01Z".into(),
            })
        else {
            panic!("first friend-location should produce an output");
        };
        assert_eq!(first.persistence.feed_entries[0]["type"], "GPS");

        let RealtimeFriendApplyResult::Output(second) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: payload,
                raw: "{}".into(),
                received_at: "2026-05-15T00:06:01Z".into(),
            })
        else {
            panic!("duplicate friend-location should still produce a projection output");
        };
        assert!(second.persistence.feed_entries.is_empty());
        assert!(second.projection.feed_entries.is_empty());
    }

    #[test]
    fn friend_location_missing_embedded_user_without_previous_is_ignored() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let result = runtime.apply_ws_message(&RealtimeWsMessagePayload {
            json: json!({
                "type": "friend-location",
                "content": {
                    "userId": "usr_friend",
                    "location": "wrld_2:456"
                }
            }),
            raw: "{}".into(),
            received_at: "2026-05-15T00:00:00Z".into(),
        });

        assert!(matches!(result, RealtimeFriendApplyResult::Ignored));
    }

    #[test]
    fn friend_location_embedded_state_does_not_override_real_location() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_2:456",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "offline"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert_eq!(output.persistence.feed_entries[0]["type"], "GPS");
        assert_eq!(patch["stateBucket"], "online");
        assert_eq!(patch["location"], "wrld_2:456");
        assert!(output.profile_refetch_user_ids.is_empty());
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .friends_by_id
                .get("usr_friend")
                .unwrap()
                .location,
            "wrld_2:456"
        );
    }

    #[test]
    fn friend_location_embedded_user_keeps_online_bucket_for_offline_location() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "offline:offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "stateBucket": "online"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert!(output.persistence.feed_entries.is_empty());
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .friends_by_id
                .get("usr_friend")
                .unwrap()
                .state_bucket,
            "online"
        );
    }

    #[test]
    fn friend_location_embedded_user_location_matches_vue_spread_order() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "stateBucket": "online",
                            "location": "wrld_stale:456"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert_eq!(output.persistence.feed_entries[0]["type"], "GPS");
        assert_eq!(output.profile_refetch_user_ids, vec!["usr_friend"]);
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .friends_by_id
                .get("usr_friend")
                .unwrap()
                .state_bucket,
            "online"
        );
    }

    #[test]
    fn friend_location_embedded_user_without_online_location_preserves_pending_offline() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(_) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-offline",
                    "content": { "userId": "usr_friend" }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-offline should produce an output");
        };

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "active"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:01Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert!(output.persistence.feed_entries.is_empty());
        assert_eq!(patch["pendingOffline"], true);
        assert_eq!(output.profile_refetch_user_ids, vec!["usr_friend"]);
        assert!(runtime
            .fire_pending_offline("usr_friend", 1, "2026-05-15T00:03:00Z".into())
            .is_some());
    }

    #[test]
    fn friend_location_embedded_user_without_online_location_does_not_revive_offline_friend() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "offline".into(),
                        state_bucket: "offline".into(),
                        location: "offline".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "online",
                            "status": "join me"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:03:01Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        assert_eq!(output.projection.patches[0].state_bucket, "offline");
        assert_eq!(patch["stateBucket"], "offline");
        assert_eq!(output.profile_refetch_user_ids, vec!["usr_friend"]);
        assert_eq!(
            runtime
                .snapshot()
                .unwrap()
                .friends_by_id
                .get("usr_friend")
                .unwrap()
                .state_bucket,
            "offline"
        );
    }

    #[test]
    fn friend_location_missing_embedded_user_preserves_pending_offline() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(offline_output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-offline",
                    "content": { "userId": "usr_friend" }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-offline should produce an output");
        };
        let PendingOfflineTimerAction::Schedule { token, .. } = offline_output.timer_action else {
            panic!("offline should schedule pending timer");
        };

        let RealtimeFriendApplyResult::Output(location_output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "wrld_2:456"
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:01Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &location_output.projection.patches[0].patch;
        assert_eq!(location_output.projection.patches[0].state_bucket, "online");
        assert_eq!(
            location_output.projection.patches[0]
                .state_bucket_authority
                .as_deref(),
            Some("preserve")
        );
        assert_eq!(patch["pendingOffline"], true);
        assert_eq!(patch["location"], "wrld_2:456");
        assert!(runtime
            .fire_pending_offline("usr_friend", token, "2026-05-15T00:03:00Z".into())
            .is_some());
    }

    #[test]
    fn friend_location_embedded_user_offline_location_starts_pending_offline() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "location": "offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "active",
                            "location": "offline"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        let PendingOfflineTimerAction::Schedule { token, .. } = output.timer_action else {
            panic!("offline location should schedule pending timer");
        };
        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert!(output.persistence.feed_entries.is_empty());
        assert_eq!(patch["location"], "offline");
        assert_eq!(patch["pendingOffline"], true);
        let fired = runtime
            .fire_pending_offline("usr_friend", token, "2026-05-15T00:03:00Z".into())
            .unwrap();
        assert_eq!(fired.projection.patches[0].state_bucket, "offline");
    }

    #[test]
    fn friend_location_embedded_user_offline_location_ignores_nested_active_state() {
        let runtime = RealtimeFriendsRuntime::new();
        runtime.set_baseline(
            FriendRosterBaseline {
                current_user_id: "usr_self".into(),
                friends_by_id: [(
                    "usr_friend".to_string(),
                    FriendRecord {
                        id: "usr_friend".into(),
                        display_name: "Friend".into(),
                        state: "online".into(),
                        state_bucket: "online".into(),
                        location: "wrld_1:123".into(),
                        ..FriendRecord::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..FriendRosterBaseline::default()
            },
            1,
            0,
        );

        let RealtimeFriendApplyResult::Output(output) =
            runtime.apply_ws_message(&RealtimeWsMessagePayload {
                json: json!({
                    "type": "friend-location",
                    "content": {
                        "userId": "usr_friend",
                        "stateBucket": "online",
                        "location": "offline",
                        "user": {
                            "id": "usr_friend",
                            "displayName": "Friend",
                            "state": "active",
                            "location": "offline"
                        }
                    }
                }),
                raw: "{}".into(),
                received_at: "2026-05-15T00:00:00Z".into(),
            })
        else {
            panic!("friend-location should produce an output");
        };

        let patch = &output.projection.patches[0].patch;
        let PendingOfflineTimerAction::Schedule { token, .. } = output.timer_action else {
            panic!("offline location should schedule pending timer");
        };
        assert_eq!(output.projection.patches[0].state_bucket, "online");
        assert!(output.persistence.feed_entries.is_empty());
        assert_eq!(patch["location"], "offline");
        assert_eq!(patch["pendingOffline"], true);
        let fired = runtime
            .fire_pending_offline("usr_friend", token, "2026-05-15T00:03:00Z".into())
            .unwrap();
        assert_eq!(fired.projection.patches[0].state_bucket, "offline");
    }
}
