use super::*;

#[test]
fn hmd_toast_refresh_hint_finishes_fade_in_before_waiting_for_expiry() {
    let runtime = VrOverlayRuntime::new_for_test();
    let appeared_at = Instant::now();
    let timeout = Duration::from_secs(5);
    runtime.enqueue_hmd_toast(hmd_entry("fade-in"), appeared_at, timeout);
    let last_animating_frame = appeared_at + HMD_TOAST_FADE_IN - Duration::from_millis(1);
    let after_fade_in = appeared_at + HMD_TOAST_FADE_IN + Duration::from_millis(1);

    let opacity = runtime.hmd_toast_views(last_animating_frame)[0].opacity;
    assert!(opacity < 1.0);
    assert_eq!(
        runtime.hmd_toast_refresh_hint(after_fade_in),
        Some(Duration::ZERO)
    );

    let opacity = runtime.hmd_toast_views(after_fade_in)[0].opacity;
    assert_eq!(opacity, 1.0);
    assert!(runtime
        .hmd_toast_refresh_hint(after_fade_in)
        .is_some_and(|hint| !hint.is_zero()));
}

#[test]
fn hmd_toast_refresh_hint_removes_toast_at_fade_out_boundary() {
    let runtime = VrOverlayRuntime::new_for_test();
    let appeared_at = Instant::now();
    let timeout = Duration::from_secs(5);
    runtime.enqueue_hmd_toast(hmd_entry("fade-out"), appeared_at, timeout);
    let fade_out_ends_at = appeared_at + timeout + HMD_TOAST_FADE_OUT;
    let last_animating_frame = fade_out_ends_at - Duration::from_millis(1);
    let after_fade_out = fade_out_ends_at + Duration::from_millis(1);

    let opacity = runtime.hmd_toast_views(last_animating_frame)[0].opacity;
    assert!(opacity > 0.0);
    assert_eq!(
        runtime.hmd_toast_refresh_hint(after_fade_out),
        Some(Duration::ZERO)
    );

    assert!(runtime.hmd_toast_views(after_fade_out).is_empty());
    assert_eq!(runtime.hmd_toast_refresh_hint(after_fade_out), None);
}

fn hmd_entry(source_id: &str) -> OverlayActivityEntry {
    OverlayActivityEntry {
        sequence: 1,
        source_id: source_id.to_string(),
        activity_type: "Status".to_string(),
        category: vrcx_0_application_activity::OverlayActivityCategory::CurrentInstance,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        actor_user_id: "usr_actor".to_string(),
        actor_display_name: source_id.to_string(),
        content: vrcx_0_application_activity::OverlayActivityContent {
            title: vrcx_0_application_activity::OverlayActivityText::literal(source_id),
            body: vrcx_0_application_activity::OverlayActivityText::literal("Status"),
            location: "wrld_a:123".to_string(),
            ..vrcx_0_application_activity::OverlayActivityContent::default()
        },
        actor_relation: OverlayActivityActorRelation::Favorite,
        payload: serde_json::json!({}),
    }
}
