use vrcx_0_application_core::{GameProcessEvent, GameProcessEventSink};
use vrcx_0_overlay_runtime::VrOverlayRuntime;

#[test]
fn runtime_starts_panel_listener_before_wrist_overlay_is_enabled() {
    let runtime = VrOverlayRuntime::new_for_test();

    runtime
        .on_game_process_event(GameProcessEvent {
            is_game_running: true,
            is_steamvr_running: true,
            game_changed: true,
        })
        .expect("record process status");
    assert!(runtime.is_running());

    runtime.set_enabled(true);
    assert!(runtime.is_running());

    runtime
        .on_game_process_event(GameProcessEvent {
            is_game_running: false,
            is_steamvr_running: true,
            game_changed: true,
        })
        .expect("record process status");
    assert!(runtime.is_running());

    runtime
        .on_game_process_event(GameProcessEvent {
            is_game_running: false,
            is_steamvr_running: false,
            game_changed: true,
        })
        .expect("record process status");
    assert!(!runtime.is_running());
}

#[test]
fn runtime_does_not_start_noop_overlay_when_backend_is_unavailable() {
    let runtime = VrOverlayRuntime::new_for_test_with_backend_available(false);

    runtime
        .on_game_process_event(GameProcessEvent {
            is_game_running: true,
            is_steamvr_running: true,
            game_changed: true,
        })
        .expect("record process status");
    runtime.set_enabled(true);

    assert!(runtime.is_enabled());
    assert!(!runtime.is_running());
}
