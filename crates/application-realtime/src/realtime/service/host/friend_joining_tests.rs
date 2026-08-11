use super::test_support::*;
use super::*;
use vrcx_0_application_core::HostSessionGameProcessStatus as GameProcessStatus;
use vrcx_0_core::friends::FriendRecord;

fn joining_output(
    owner_user_id: &str,
    baseline_revision: u64,
    destination: &str,
) -> RealtimeFriendOutput {
    RealtimeFriendOutput::from_projection(
        owner_user_id.to_string(),
        FriendProjection {
            generation: 7,
            baseline_revision,
            feed_entries: vec![json!({
                "created_at": "2026-07-13T10:00:00Z",
                "type": "OnPlayerJoining",
                "userId": "usr_friend",
                "displayName": "Friend",
                "location": "traveling",
                "travelingToLocation": destination,
            })],
            ..FriendProjection::new(7, baseline_revision)
        },
    )
}

#[test]
fn player_joining_only_reaches_overlay_for_current_instance_absent_player() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("player-joining")?;
    let local_game_context = runtime.local_game_context_for_test();
    let activity_sink = runtime.activity_sink_for_test();
    let baseline = runtime.runtime().sync_friend_snapshot(
        active_session.clone(),
        Some(7),
        [(
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
    )?;
    runtime.runtime().deps.event_bus.take_events_for_test();
    activity_sink.take_friend_projections();
    local_game_context.set_location("wrld_current:456");
    let apply_joining = |destination: &str| {
        runtime.runtime().apply_friend_output(joining_output(
            &active_session.user_id,
            baseline.baseline_revision,
            destination,
        ));
    };

    apply_joining("wrld_current:456");
    assert!(activity_sink
        .take_friend_projections()
        .iter()
        .all(|projection| projection.feed_entries.is_empty()));

    runtime
        .runtime()
        .deps
        .session
        .apply_game_process_status(GameProcessStatus {
            is_game_running: true,
            is_steamvr_running: true,
            changed_at: "2026-07-13T09:59:00Z".into(),
        });
    apply_joining("wrld_other:789");
    assert!(activity_sink
        .take_friend_projections()
        .iter()
        .all(|projection| projection.feed_entries.is_empty()));

    local_game_context.set_player_user_ids(vec!["usr_friend".into()]);
    apply_joining("wrld_current:456");
    assert!(activity_sink
        .take_friend_projections()
        .iter()
        .all(|projection| projection.feed_entries.is_empty()));

    local_game_context.set_player_user_ids(Vec::new());
    apply_joining("wrld_current:456");

    let entries = activity_sink
        .take_friend_projections()
        .into_iter()
        .flat_map(|projection| projection.feed_entries)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["type"], "OnPlayerJoining");
    assert_eq!(entries[0]["userId"], "usr_friend");
    let events = runtime.runtime().deps.event_bus.take_events_for_test();
    assert!(events
        .iter()
        .all(|event| event.name != "realtimeFeedProjection"));
    Ok(())
}

#[test]
fn initial_traveling_baseline_emits_player_joining() -> Result<()> {
    let (_dir, runtime, active_session) = runtime_with_active_session("baseline-player-joining")?;
    let local_game_context = runtime.local_game_context_for_test();
    let activity_sink = runtime.activity_sink_for_test();
    runtime
        .runtime()
        .deps
        .session
        .apply_game_process_status(GameProcessStatus {
            is_game_running: true,
            is_steamvr_running: true,
            changed_at: "2026-07-13T09:59:00Z".into(),
        });
    local_game_context.set_location("wrld_current:456");

    runtime.runtime().sync_friend_snapshot(
        active_session,
        Some(7),
        [(
            "usr_friend".to_string(),
            FriendRecord {
                id: "usr_friend".into(),
                display_name: "Friend".into(),
                state: "online".into(),
                state_bucket: "online".into(),
                location: "traveling".into(),
                traveling_to_location: "wrld_current:456".into(),
                ..FriendRecord::default()
            },
        )]
        .into_iter()
        .collect(),
    )?;

    let entries = activity_sink
        .take_friend_projections()
        .into_iter()
        .flat_map(|projection| projection.feed_entries)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["type"], "OnPlayerJoining");
    assert_eq!(entries[0]["userId"], "usr_friend");
    let events = runtime.runtime().deps.event_bus.take_events_for_test();
    let projection = events
        .iter()
        .find(|event| event.name == "realtimeFriendProjection")
        .expect("traveling baseline should emit a friend projection");
    assert!(projection.payload["feedEntries"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(events
        .iter()
        .all(|event| event.name != "realtimeFeedProjection"));
    Ok(())
}
