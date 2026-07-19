use super::reconnect_reconcile::{
    confirmed_non_friend, diff_friend_roster_against_auth_user, profile_reports_non_friend,
    resolve_bucket_message_type,
};
use super::test_support::*;
use super::*;
use crate::social_baseline::service::FriendStateBuckets;

fn buckets(
    has_friend_list: bool,
    state_by_id: &[(&str, &str)],
    expected_ids: &[&str],
) -> FriendStateBuckets {
    FriendStateBuckets {
        has_friend_list,
        state_by_id: state_by_id
            .iter()
            .map(|(id, bucket)| (id.to_string(), bucket.to_string()))
            .collect(),
        expected_ids: expected_ids.iter().map(|id| id.to_string()).collect(),
    }
}

fn friend(state_bucket: &str) -> FriendRecord {
    FriendRecord {
        id: "usr_friend".into(),
        display_name: "Friend".into(),
        state: state_bucket.into(),
        state_bucket: state_bucket.into(),
        ..FriendRecord::default()
    }
}

#[test]
fn diff_puts_stale_online_bucket_into_suspicious_only() {
    let buckets = buckets(true, &[("usr_friend", "online")], &["usr_friend"]);
    let local = HashMap::from([("usr_friend".to_string(), friend("offline"))]);

    let diff = diff_friend_roster_against_auth_user(&buckets, &local).expect("diff");

    assert_eq!(diff.suspicious, vec!["usr_friend".to_string()]);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
}

#[test]
fn diff_matches_local_state_produce_no_suspicious_entry() {
    let buckets = buckets(true, &[("usr_friend", "online")], &["usr_friend"]);
    let local = HashMap::from([("usr_friend".to_string(), friend("online"))]);

    let diff = diff_friend_roster_against_auth_user(&buckets, &local).expect("diff");

    assert!(diff.is_empty());
}

#[test]
fn diff_detects_added_and_removed_members_both_directions() {
    let buckets = buckets(
        true,
        &[("usr_new", "online"), ("usr_kept", "offline")],
        &["usr_new", "usr_kept"],
    );
    let local = HashMap::from([
        ("usr_kept".to_string(), friend("offline")),
        ("usr_gone".to_string(), friend("online")),
    ]);

    let diff = diff_friend_roster_against_auth_user(&buckets, &local).expect("diff");

    assert_eq!(diff.added, vec!["usr_new".to_string()]);
    assert_eq!(diff.removed, vec!["usr_gone".to_string()]);
}

#[test]
fn diff_abandons_reconcile_when_auth_user_friend_list_is_incomplete() {
    let buckets = buckets(false, &[], &[]);
    let local = HashMap::from([("usr_friend".to_string(), friend("online"))]);

    assert!(diff_friend_roster_against_auth_user(&buckets, &local).is_none());
}

#[test]
fn resolve_bucket_message_type_is_none_when_getuser_confirms_local_bucket() {
    let profile = json!({ "id": "usr_friend", "state": "offline" });
    assert_eq!(resolve_bucket_message_type("offline", &profile), None);
}

#[test]
fn resolve_bucket_message_type_emits_friend_online_when_getuser_disagrees() {
    let profile = json!({ "id": "usr_friend", "state": "online" });
    assert_eq!(
        resolve_bucket_message_type("offline", &profile),
        Some("friend-online")
    );
}

#[test]
fn resolve_bucket_message_type_emits_friend_offline_when_getuser_reports_offline() {
    let profile = json!({ "id": "usr_friend", "state": "offline" });
    assert_eq!(
        resolve_bucket_message_type("online", &profile),
        Some("friend-offline")
    );
}

#[test]
fn confirmed_non_friend_true_on_404() {
    assert!(confirmed_non_friend(404, None));
}

#[test]
fn confirmed_non_friend_true_when_getuser_says_not_friend() {
    let body = json!({ "id": "usr_friend", "isFriend": false });
    assert!(confirmed_non_friend(200, Some(&body)));
}

#[test]
fn confirmed_non_friend_false_when_getuser_still_reports_friend() {
    let body = json!({ "id": "usr_friend", "isFriend": true });
    assert!(!confirmed_non_friend(200, Some(&body)));
}

#[test]
fn confirmed_non_friend_false_when_ambiguous_or_failed() {
    assert!(!confirmed_non_friend(200, None));
    assert!(!confirmed_non_friend(500, None));
}

#[test]
fn profile_non_friend_gate_blocks_only_explicit_is_friend_false() {
    assert!(profile_reports_non_friend(&json!({ "isFriend": false })));
    assert!(!profile_reports_non_friend(&json!({ "isFriend": true })));
    assert!(!profile_reports_non_friend(&json!({ "state": "online" })));
}

#[test]
fn landing_applies_correction_when_sequence_matches() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("reconcile-sequence-match")?;
    let active = runtime
        .state
        .lock()
        .unwrap()
        .connection
        .active_context
        .clone()
        .unwrap();
    runtime.sync_friend_snapshot(
        active_session.user_id.clone(),
        active_session.endpoint.clone(),
        active_session.websocket.clone(),
        Some(active.generation),
        HashMap::from([("usr_friend".to_string(), friend("offline"))]),
    )?;
    let expected_sequence = runtime
        .friends
        .friend_state_sequence_for_user(active.generation, "usr_friend");

    let outcome = runtime.apply_synthetic_friend_event_if_sequence(
        &active_session.user_id,
        &active_session.endpoint,
        active.generation,
        "usr_friend",
        expected_sequence,
        "friend-online",
        json!({ "userId": "usr_friend", "user": { "id": "usr_friend", "displayName": "Friend" } }),
        "2026-07-20T00:00:00Z".into(),
        false,
    );

    assert_eq!(outcome, SyntheticFriendEventOutcome::Applied);
    let snapshot = runtime.friend_snapshot().expect("friend snapshot");
    assert_eq!(snapshot.friends_by_id["usr_friend"].state_bucket, "online");
    Ok(())
}

#[test]
fn landing_skips_correction_when_ws_advanced_the_sequence() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("reconcile-sequence-stale")?;
    let active = runtime
        .state
        .lock()
        .unwrap()
        .connection
        .active_context
        .clone()
        .unwrap();
    runtime.sync_friend_snapshot(
        active_session.user_id.clone(),
        active_session.endpoint.clone(),
        active_session.websocket.clone(),
        Some(active.generation),
        HashMap::from([("usr_friend".to_string(), friend("offline"))]),
    )?;
    let stale_sequence = runtime
        .friends
        .friend_state_sequence_for_user(active.generation, "usr_friend");

    // A genuine WS event lands in between the reconcile's getUser snapshot and its
    // guarded apply, advancing the per-user sequence past `stale_sequence`.
    runtime.apply_synthetic_friend_event(
        &active_session.user_id,
        &active_session.endpoint,
        "friend-online",
        json!({ "userId": "usr_friend", "user": { "id": "usr_friend", "displayName": "Friend" } }),
        "2026-07-20T00:00:00Z".into(),
    );
    assert_eq!(
        runtime.friend_snapshot().unwrap().friends_by_id["usr_friend"].state_bucket,
        "online"
    );

    let outcome = runtime.apply_synthetic_friend_event_if_sequence(
        &active_session.user_id,
        &active_session.endpoint,
        active.generation,
        "usr_friend",
        stale_sequence,
        "friend-active",
        json!({ "userId": "usr_friend" }),
        "2026-07-20T00:00:05Z".into(),
        false,
    );

    assert_eq!(outcome, SyntheticFriendEventOutcome::Ignored);
    // The WS-driven online state must survive the skipped, stale correction.
    assert_eq!(
        runtime.friend_snapshot().unwrap().friends_by_id["usr_friend"].state_bucket,
        "online"
    );
    Ok(())
}

#[test]
fn landing_abandons_correction_when_generation_no_longer_matches() -> Result<()> {
    let (_dir, runtime, active_session) =
        runtime_with_active_session("reconcile-generation-mismatch")?;
    let active = runtime
        .state
        .lock()
        .unwrap()
        .connection
        .active_context
        .clone()
        .unwrap();
    runtime.sync_friend_snapshot(
        active_session.user_id.clone(),
        active_session.endpoint.clone(),
        active_session.websocket.clone(),
        Some(active.generation),
        HashMap::from([("usr_friend".to_string(), friend("offline"))]),
    )?;

    let outcome = runtime.apply_synthetic_friend_event_if_sequence(
        &active_session.user_id,
        &active_session.endpoint,
        active.generation.wrapping_add(1),
        "usr_friend",
        Some(0),
        "friend-online",
        json!({ "userId": "usr_friend" }),
        "2026-07-20T00:00:00Z".into(),
        false,
    );

    assert_eq!(outcome, SyntheticFriendEventOutcome::MissingBaseline);
    assert_eq!(
        runtime.friend_snapshot().unwrap().friends_by_id["usr_friend"].state_bucket,
        "offline"
    );
    Ok(())
}

#[test]
fn landing_confirms_member_disappearance_only_when_watermark_still_present() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("reconcile-member-removed")?;
    let active = runtime
        .state
        .lock()
        .unwrap()
        .connection
        .active_context
        .clone()
        .unwrap();
    runtime.sync_friend_snapshot(
        active_session.user_id.clone(),
        active_session.endpoint.clone(),
        active_session.websocket.clone(),
        Some(active.generation),
        HashMap::from([("usr_gone".to_string(), friend("offline"))]),
    )?;
    let expected_sequence = runtime
        .friends
        .friend_state_sequence_for_user(active.generation, "usr_gone");

    let outcome = runtime.apply_synthetic_friend_event_if_sequence(
        &active_session.user_id,
        &active_session.endpoint,
        active.generation,
        "usr_gone",
        expected_sequence,
        "friend-delete",
        json!({ "userId": "usr_gone" }),
        "2026-07-20T00:00:00Z".into(),
        false,
    );

    assert_eq!(outcome, SyntheticFriendEventOutcome::Applied);
    assert!(!runtime
        .friend_snapshot()
        .unwrap()
        .friends_by_id
        .contains_key("usr_gone"));
    Ok(())
}
