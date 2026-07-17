use chrono::Utc;
use vrcx_0_persistence::config as config_store;
use vrcx_0_persistence::DatabaseService;

use crate::game_log::host::GameLogHostActions;
use crate::game_log::runtime_state::parse_event_time_ms;
use crate::Result;
use crate::RuntimeEventBus;
use crate::RuntimeGameEventBusExt;

pub fn set_game_no_vr(
    db: &DatabaseService,
    event_bus: &RuntimeEventBus,
    no_vr: bool,
) -> Result<()> {
    config_store::set_bool(db, "isGameNoVR", no_vr)?;
    event_bus.emit_game_log_side_effect(
        "gameNoVR",
        serde_json::json!({
            "isGameNoVR": no_vr,
        }),
    );
    Ok(())
}

pub fn handle_vrc_quit(
    db: &DatabaseService,
    host_actions: &dyn GameLogHostActions,
    event_bus: &RuntimeEventBus,
    created_at: &str,
    is_game_running: bool,
) {
    if !is_game_running {
        return;
    }
    if !config_store::get_bool(db, "vrcQuitFix", true).unwrap_or(true) {
        return;
    }

    let Some(created_at_ms) = parse_event_time_ms(created_at) else {
        return;
    };
    if created_at_ms + 3000 < Utc::now().timestamp_millis() {
        return;
    }

    let killed = host_actions.quit_game();
    if killed > 0 {
        event_bus.emit_game_log_side_effect(
            "notification",
            serde_json::json!({
                "level": "info",
                "title": "VRChat quit cleanup",
                "message": format!("Closed {killed} lingering VRChat process(es)."),
            }),
        );
    }
}

pub fn emit_video_sync(event_bus: &RuntimeEventBus, timestamp: &str, created_at: &str) {
    let position = timestamp
        .replace(',', "")
        .parse::<i64>()
        .ok()
        .filter(|value| *value >= 0)
        .unwrap_or(0);

    event_bus.emit_game_log_side_effect(
        "nowPlaying",
        serde_json::json!({
            "position": position,
            "startedAt": created_at,
            "updatedAt": Utc::now().to_rfc3339(),
        }),
    );
}
