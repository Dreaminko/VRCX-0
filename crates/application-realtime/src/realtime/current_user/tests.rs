use serde_json::json;
use vrcx_0_core::realtime::RealtimeWsMessagePayload;
use vrcx_0_persistence::game_log::{GameLogLocationEntry, GameLogLocationTimeUpdate};

use crate::realtime::{PendingOfflineTimerAction, RealtimeCurrentUserAuthority};

use super::runtime::RealtimeCurrentUserRuntime;
use super::state::{
    CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS,
    CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS,
};

fn remote_authority(game_log_enabled: bool) -> RealtimeCurrentUserAuthority {
    RealtimeCurrentUserAuthority {
        local_game_context_available: true,
        is_game_running: false,
        game_log_enabled,
        ..RealtimeCurrentUserAuthority::default()
    }
}

fn current_user_location_message(
    location: &str,
    traveling_to_location: &str,
    received_at: &str,
) -> RealtimeWsMessagePayload {
    RealtimeWsMessagePayload {
        json: json!({
            "type": "user-location",
            "content": {
                "userId": "usr_self",
                "location": location,
                "travelingToLocation": traveling_to_location
            }
        }),
        raw: String::new(),
        received_at: received_at.into(),
    }
}

#[test]
fn current_user_projection_serializes_object_shape() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "displayName": "Self",
            "location": "offline"
        }),
    );

    let output = runtime
        .apply_ws_message(
            7,
            &RealtimeWsMessagePayload {
                json: json!({
                    "type": "user-location",
                    "content": {
                        "userId": "usr_self",
                        "location": "wrld_1:123~group(grp_1)",
                        "travelingToLocation": "",
                        "worldId": "wrld_1"
                    }
                }),
                raw: String::new(),
                received_at: "2026-05-15T00:00:00Z".into(),
            },
            RealtimeCurrentUserAuthority::default(),
        )
        .expect("current user location output");

    let serialized = serde_json::to_value(&output.projection).unwrap();
    assert_eq!(serialized["patch"]["id"], json!("usr_self"));
    assert_eq!(
        serialized["snapshot"]["location"],
        json!("wrld_1:123~group(grp_1)")
    );
    assert_eq!(
        serialized["gameStatePatch"]["currentLocation"],
        json!("wrld_1:123~group(grp_1)")
    );
    assert_eq!(
        serialized["patch"]["$location"]["tag"],
        json!("wrld_1:123~group(grp_1)")
    );
    assert_eq!(serialized["patch"]["$location"]["worldId"], json!("wrld_1"));
    assert_eq!(
        serialized["patch"]["$location"]["accessType"],
        json!("group")
    );
    assert_eq!(serialized["patch"]["$location"]["groupId"], json!("grp_1"));
    assert_eq!(
        serialized["patch"]["$travelingToLocation"]["isRealInstance"],
        json!(false)
    );
}

#[test]
fn refreshed_current_user_snapshot_preserves_local_authority_fields() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "displayName": "Self",
            "location": "wrld_local:1",
            "worldId": "wrld_local",
            "instanceId": "1",
            "state": "online",
            "stateBucket": "online",
            "status": "join me",
            "statusDescription": "Local status",
            "worldName": "Local World",
            "bio": "old bio"
        }),
    );

    let output = runtime
        .apply_refreshed_snapshot(
            7,
            json!({
                "id": "usr_self",
                "displayName": "Self Fresh",
                "location": "offline",
                "worldId": "offline",
                "instanceId": "offline",
                "state": "offline",
                "stateBucket": "offline",
                "status": "busy",
                "statusDescription": "REST status",
                "worldName": "REST World",
                "bio": "fresh bio"
            }),
            json!({}),
            RealtimeCurrentUserAuthority {
                is_game_running: true,
                game_log_enabled: true,
                game_log_location: "wrld_auth:123".into(),
                game_log_world_name: "Authoritative World".into(),
                ..RealtimeCurrentUserAuthority::default()
            },
        )
        .expect("refreshed snapshot should update profile fields");

    assert_eq!(
        output.projection.snapshot["displayName"],
        json!("Self Fresh")
    );
    assert_eq!(output.projection.snapshot["bio"], json!("fresh bio"));
    assert_eq!(output.projection.snapshot["status"], json!("join me"));
    assert_eq!(
        output.projection.snapshot["statusDescription"],
        json!("Local status")
    );
    assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
    assert_eq!(
        output.projection.snapshot["location"],
        json!("wrld_auth:123")
    );
    assert_eq!(output.projection.snapshot["worldId"], json!("wrld_auth"));
    assert_eq!(output.projection.snapshot["instanceId"], json!("123"));
    assert_eq!(
        output.projection.snapshot["worldName"],
        json!("Authoritative World")
    );
    assert_eq!(output.projection.patch["location"], json!("wrld_auth:123"));
    assert_eq!(
        output.projection.patch["$location"]["tag"],
        json!("wrld_auth:123")
    );
}

#[test]
fn refreshed_snapshot_with_stale_sequence_is_dropped() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({ "id": "usr_self", "bio": "old bio" }),
    );
    let stale_sequence = runtime.snapshot_sequence(7).expect("sequence");

    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("interleaved location apply");

    assert!(runtime
        .apply_refreshed_snapshot_if_sequence(
            7,
            stale_sequence,
            json!({ "id": "usr_self", "bio": "stale bio" }),
            json!({}),
            &[],
            remote_authority(true),
        )
        .is_none());
    let fresh_sequence = runtime.snapshot_sequence(7).expect("sequence");
    let output = runtime
        .apply_refreshed_snapshot_if_sequence(
            7,
            fresh_sequence,
            json!({ "id": "usr_self", "bio": "fresh bio" }),
            json!({}),
            &[],
            remote_authority(true),
        )
        .expect("fresh sequence applies");
    assert_eq!(output.projection.snapshot["bio"], json!("fresh bio"));
}

#[test]
fn interleaved_avatar_and_fallback_selection_drops_the_stale_response() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "currentAvatar": "avtr_old",
            "fallbackAvatar": "avtr_old_fallback"
        }),
    );
    let shared_sequence = runtime.snapshot_sequence(7).expect("sequence");

    let avatar_output = runtime
        .apply_refreshed_snapshot_if_sequence(
            7,
            shared_sequence,
            json!({
                "id": "usr_self",
                "currentAvatar": "avtr_new",
                "fallbackAvatar": "avtr_old_fallback"
            }),
            json!({}),
            CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS,
            remote_authority(true),
        )
        .expect("avatar selection response applies");
    assert_eq!(
        avatar_output.projection.snapshot["currentAvatar"],
        json!("avtr_new")
    );

    assert!(runtime
        .apply_refreshed_snapshot_if_sequence(
            7,
            shared_sequence,
            json!({
                "id": "usr_self",
                "currentAvatar": "avtr_old",
                "fallbackAvatar": "avtr_new_fallback"
            }),
            json!({}),
            CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS,
            remote_authority(true),
        )
        .is_none());
    let snapshot = runtime.snapshot_value().expect("snapshot");
    assert_eq!(snapshot["currentAvatar"], json!("avtr_new"));
    assert_eq!(snapshot["fallbackAvatar"], json!("avtr_old_fallback"));
}

#[test]
fn response_authority_fields_override_the_local_authority_strip() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({ "id": "usr_self", "status": "join me" }),
    );
    let sequence = runtime.snapshot_sequence(7).expect("sequence");

    let output = runtime
        .apply_refreshed_snapshot_if_sequence(
            7,
            sequence,
            json!({ "id": "usr_self", "status": "busy" }),
            json!({}),
            &["status"],
            remote_authority(true),
        )
        .expect("response authority field applies");

    assert_eq!(output.projection.snapshot["status"], json!("busy"));
}

#[test]
fn unavailable_local_game_context_skips_game_dependent_side_effects() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "currentAvatar": "avtr_current",
            "$previousAvatarSwapTime": 1_000
        }),
    );
    let authority = RealtimeCurrentUserAuthority {
        local_game_context_available: false,
        ..RealtimeCurrentUserAuthority::default()
    };

    let output = runtime
        .apply_ws_message(
            7,
            &RealtimeWsMessagePayload {
                json: json!({
                    "type": "user-location",
                    "content": {
                        "userId": "usr_self",
                        "location": "wrld_1:123",
                        "travelingToLocation": "",
                        "worldId": "wrld_1"
                    }
                }),
                raw: String::new(),
                received_at: "2026-05-15T00:00:02Z".into(),
            },
            authority.clone(),
        )
        .expect("current user location output");

    assert_eq!(output.projection.snapshot["location"], json!("wrld_1:123"));
    assert_eq!(
        output.projection.snapshot["$previousAvatarSwapTime"],
        json!(1_000)
    );
    assert!(output.projection.game_state_patch.is_none());
    assert!(output.persistence.is_empty());
    assert!(runtime.apply_game_running_state(7, authority).is_none());
}

#[test]
fn running_local_game_keeps_authoritative_location_above_remote_ws_location() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "location": "wrld_local:123",
            "worldId": "wrld_local",
            "instanceId": "123",
            "state": "online",
            "stateBucket": "online"
        }),
    );

    let output = runtime
        .apply_ws_message(
            7,
            &RealtimeWsMessagePayload {
                json: json!({
                    "type": "user-location",
                    "content": {
                        "userId": "usr_self",
                        "location": "wrld_remote:456",
                        "travelingToLocation": "",
                        "worldId": "wrld_remote"
                    }
                }),
                raw: String::new(),
                received_at: "2026-05-15T00:00:00Z".into(),
            },
            RealtimeCurrentUserAuthority {
                is_game_running: true,
                game_log_enabled: true,
                game_log_location: "wrld_local:123".into(),
                game_log_world_name: "Local World".into(),
                ..RealtimeCurrentUserAuthority::default()
            },
        )
        .expect("current user location output");

    assert_eq!(
        output.projection.snapshot["location"],
        json!("wrld_local:123")
    );
    assert_eq!(output.projection.snapshot["worldId"], json!("wrld_local"));
    assert!(output.projection.game_state_patch.is_none());
    assert!(output.persistence.game_log_locations.is_empty());
}

#[test]
fn stopped_local_game_projects_remote_location_as_online_and_starts_gamelog_interval() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot(
        "usr_self".into(),
        7,
        json!({
            "id": "usr_self",
            "status": "busy",
            "location": "offline",
            "state": "offline",
            "stateBucket": "offline"
        }),
    );

    let output = runtime
        .apply_ws_message(
            7,
            &current_user_location_message(
                "wrld_remote:456~group(grp_remote)",
                "",
                "2026-05-15T00:00:00Z",
            ),
            remote_authority(true),
        )
        .expect("remote location output");

    assert_eq!(output.projection.snapshot["state"], json!("online"));
    assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
    assert_eq!(
        output.projection.snapshot["location"],
        json!("wrld_remote:456~group(grp_remote)")
    );
    assert!(output.projection.snapshot.get("pendingOffline").is_none());
    assert_eq!(output.persistence.game_log_locations.len(), 1);
    assert_eq!(
        output.persistence.game_log_locations[0],
        GameLogLocationEntry {
            created_at: "2026-05-15T00:00:00Z".into(),
            location: "wrld_remote:456~group(grp_remote)".into(),
            world_id: "wrld_remote".into(),
            world_name: "".into(),
            time: 0,
            group_name: "grp_remote".into(),
        }
    );
    assert_eq!(output.timer_action, PendingOfflineTimerAction::None);
}

#[test]
fn false_remote_offline_keeps_location_until_same_location_cancels_pending() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");

    let pending = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("offline:offline", "", "2026-05-15T00:00:10Z"),
            remote_authority(true),
        )
        .expect("remote offline pending output");
    let PendingOfflineTimerAction::Schedule {
        user_id,
        token,
        delay_ms,
    } = pending.timer_action
    else {
        panic!("remote offline should schedule pending timer");
    };

    assert_eq!(user_id, "usr_self");
    assert_eq!(delay_ms, 170_000);
    assert_eq!(
        pending.projection.snapshot["location"],
        json!("wrld_remote:456")
    );
    assert_eq!(pending.projection.snapshot["stateBucket"], json!("online"));
    assert!(pending.persistence.is_empty());

    let resumed = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:10.004Z"),
            remote_authority(true),
        )
        .expect("same remote location should cancel pending");

    assert_eq!(resumed.timer_action, PendingOfflineTimerAction::None);
    assert!(resumed.persistence.is_empty());
    assert!(runtime
        .fire_pending_offline(
            7,
            token,
            "2026-05-15T00:03:00Z".into(),
            remote_authority(true),
        )
        .is_none());
}

#[test]
fn confirmed_remote_offline_ends_interval_and_same_location_can_start_again() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");
    let pending = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
            remote_authority(true),
        )
        .expect("remote offline pending output");
    let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
        panic!("remote offline should schedule pending timer");
    };

    let confirmed = runtime
        .fire_pending_offline(
            7,
            token,
            "2026-05-15T00:03:00Z".into(),
            remote_authority(true),
        )
        .expect("pending remote offline should fire");

    assert_eq!(confirmed.projection.snapshot["state"], json!("active"));
    assert_eq!(
        confirmed.projection.snapshot["stateBucket"],
        json!("active")
    );
    assert_eq!(confirmed.projection.snapshot["location"], json!("offline"));
    assert_eq!(
        confirmed.persistence.game_log_location_time_updates,
        vec![GameLogLocationTimeUpdate {
            created_at: "2026-05-15T00:00:00Z".into(),
            time: 180_000,
        }]
    );

    let restarted = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:03:20Z"),
            remote_authority(true),
        )
        .expect("same location after confirmed offline starts a new interval");
    assert_eq!(restarted.persistence.game_log_locations.len(), 1);
    assert_eq!(
        restarted.persistence.game_log_locations[0].created_at,
        "2026-05-15T00:03:20Z"
    );
}

#[test]
fn remote_presence_remains_visible_when_gamelog_is_disabled_without_writes() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));

    let output = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(false),
        )
        .expect("remote presence output");

    assert_eq!(output.projection.snapshot["stateBucket"], json!("online"));
    assert!(output.persistence.is_empty());
}

#[test]
fn local_game_start_invalidates_remote_offline_timer_and_keeps_local_authority() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");
    let pending = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
            remote_authority(true),
        )
        .expect("remote offline pending output");
    let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
        panic!("remote offline should schedule pending timer");
    };
    let local_authority = RealtimeCurrentUserAuthority {
        local_game_context_available: true,
        is_game_running: true,
        game_log_enabled: true,
        game_log_location: "wrld_local:123".into(),
        game_log_world_name: "Local World".into(),
        ..RealtimeCurrentUserAuthority::default()
    };

    let local = runtime
        .apply_game_running_state(7, local_authority.clone())
        .expect("local game state output");

    assert_eq!(
        local.projection.snapshot["location"],
        json!("wrld_local:123")
    );
    assert_eq!(local.projection.snapshot["stateBucket"], json!("online"));
    assert!(runtime
        .fire_pending_offline(7, token, "2026-05-15T00:03:00Z".into(), local_authority,)
        .is_none());
}

#[test]
fn reconnect_preserves_remote_interval_and_invalidates_old_pending_timer() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");
    let pending = runtime
        .apply_ws_message(
            7,
            &current_user_location_message("offline", "", "2026-05-15T00:00:10Z"),
            remote_authority(true),
        )
        .expect("remote offline pending output");
    let PendingOfflineTimerAction::Schedule {
        token: old_token, ..
    } = pending.timer_action
    else {
        panic!("remote offline should schedule pending timer");
    };

    runtime.set_snapshot(
        "usr_self".into(),
        8,
        json!({
            "id": "usr_self",
            "location": "wrld_remote:456",
            "state": "online",
            "stateBucket": "online"
        }),
    );

    assert!(runtime
        .fire_pending_offline(
            7,
            old_token,
            "2026-05-15T00:03:00Z".into(),
            remote_authority(true),
        )
        .is_none());
    let duplicate = runtime
        .apply_ws_message(
            8,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:20Z"),
            remote_authority(true),
        )
        .expect("reconnected remote location output");
    assert!(duplicate.persistence.game_log_locations.is_empty());

    let pending = runtime
        .apply_ws_message(
            8,
            &current_user_location_message("offline", "", "2026-05-15T00:00:30Z"),
            remote_authority(true),
        )
        .expect("remote offline after reconnect");
    let PendingOfflineTimerAction::Schedule { token, .. } = pending.timer_action else {
        panic!("remote offline should schedule pending timer");
    };
    let confirmed = runtime
        .fire_pending_offline(
            8,
            token,
            "2026-05-15T00:03:20Z".into(),
            remote_authority(true),
        )
        .expect("remote offline should close original interval");

    assert_eq!(
        confirmed.persistence.game_log_location_time_updates,
        vec![GameLogLocationTimeUpdate {
            created_at: "2026-05-15T00:00:00Z".into(),
            time: 200_000,
        }]
    );
}

#[test]
fn transport_interruption_does_not_end_remote_interval_or_change_presence() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");

    let finalized = runtime
        .interrupt_transport(7, remote_authority(true))
        .expect("transport finalization output");

    assert_eq!(
        finalized.projection.snapshot["location"],
        json!("wrld_remote:456")
    );
    assert_eq!(
        finalized.projection.snapshot["stateBucket"],
        json!("online")
    );
    assert!(finalized
        .persistence
        .game_log_location_time_updates
        .is_empty());
}

#[test]
fn explicit_transport_finalization_ends_remote_interval() {
    let runtime = RealtimeCurrentUserRuntime::new();
    runtime.set_snapshot("usr_self".into(), 7, json!({ "id": "usr_self" }));
    runtime
        .apply_ws_message(
            7,
            &current_user_location_message("wrld_remote:456", "", "2026-05-15T00:00:00Z"),
            remote_authority(true),
        )
        .expect("remote interval start");

    let finalized = runtime
        .finalize_transport(7, remote_authority(true))
        .expect("explicit transport finalization output");

    assert_eq!(
        finalized.persistence.game_log_location_time_updates.len(),
        1
    );
    assert_eq!(
        finalized.persistence.game_log_location_time_updates[0].created_at,
        "2026-05-15T00:00:00Z"
    );
    assert!(finalized.persistence.game_log_location_time_updates[0].time > 0);
}
