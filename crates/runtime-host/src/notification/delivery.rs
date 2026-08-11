use serde::{Deserialize, Serialize};
use vrcx_0_application_activity::OverlayActivityDelivery;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationTtsNameMode {
    #[default]
    Username,
    Note,
    UsernameAndNote,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationDeliveryCondition {
    #[default]
    Never,
    Always,
    InsideVr,
    OutsideVr,
    GameClosed,
    GameRunning,
    DesktopMode,
}

impl NotificationDeliveryCondition {
    pub(super) fn from_config(value: &str) -> Self {
        match value {
            "Always" => Self::Always,
            "Inside VR" => Self::InsideVr,
            "Outside VR" => Self::OutsideVr,
            "Game Closed" => Self::GameClosed,
            "Game Running" => Self::GameRunning,
            "Desktop Mode" => Self::DesktopMode,
            _ => Self::Never,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum NotificationWebhookFormat {
    #[default]
    Generic,
    Discord,
}

impl NotificationWebhookFormat {
    pub(super) fn from_config(value: &str) -> Self {
        match value {
            "discord" => Self::Discord,
            _ => Self::Generic,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationDeliveryPreferences {
    pub desktop_toast: NotificationDeliveryCondition,
    pub desktop_notification_sound: bool,
    pub notification_tts: NotificationDeliveryCondition,
    pub notification_tts_name_mode: NotificationTtsNameMode,
    pub notification_tts_voice_native: String,
    pub xs_notifications: bool,
    pub ovrt_hud_notifications: bool,
    pub ovrt_wrist_notifications: bool,
    pub image_notifications: bool,
    pub notification_timeout_ms: i32,
    pub notification_opacity_percent: i32,
    pub show_instance_id_in_location: bool,
}

impl Default for NotificationDeliveryPreferences {
    fn default() -> Self {
        Self {
            desktop_toast: NotificationDeliveryCondition::Never,
            desktop_notification_sound: false,
            notification_tts: NotificationDeliveryCondition::Never,
            notification_tts_name_mode: NotificationTtsNameMode::Username,
            notification_tts_voice_native: String::new(),
            xs_notifications: false,
            ovrt_hud_notifications: false,
            ovrt_wrist_notifications: false,
            image_notifications: true,
            notification_timeout_ms: 3000,
            notification_opacity_percent: 100,
            show_instance_id_in_location: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotificationDeliveryGameState {
    pub is_game_running: bool,
    pub is_steamvr_running: bool,
    pub is_game_no_vr: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotificationDeliveryPlan {
    pub desktop: bool,
    pub xs: bool,
    pub ovrt: bool,
    pub ovrt_hud: bool,
    pub ovrt_wrist: bool,
    pub tts: bool,
}

impl NotificationDeliveryPlan {
    pub fn has_local_transport(self) -> bool {
        self.desktop || self.xs || self.ovrt || self.tts
    }

    pub fn needs_local_image(self) -> bool {
        self.desktop || self.xs || self.ovrt
    }
}

pub fn decide_notification_plan(
    delivery: &OverlayActivityDelivery,
    preferences: &NotificationDeliveryPreferences,
    game: &NotificationDeliveryGameState,
) -> NotificationDeliveryPlan {
    let desktop = delivery.desktop && should_play_for_condition(preferences.desktop_toast, game);
    let vr = delivery.vr && game.is_steamvr_running;
    let xs = vr && preferences.xs_notifications;
    let ovrt_hud = vr && preferences.ovrt_hud_notifications;
    let ovrt_wrist = vr && preferences.ovrt_wrist_notifications;
    let ovrt = ovrt_hud || ovrt_wrist;
    let tts = delivery.tts && should_play_for_condition(preferences.notification_tts, game);

    NotificationDeliveryPlan {
        desktop,
        xs,
        ovrt,
        ovrt_hud,
        ovrt_wrist,
        tts,
    }
}

fn should_play_for_condition(
    condition: NotificationDeliveryCondition,
    game: &NotificationDeliveryGameState,
) -> bool {
    match condition {
        NotificationDeliveryCondition::Never => false,
        NotificationDeliveryCondition::Always => true,
        NotificationDeliveryCondition::InsideVr => game.is_steamvr_running,
        NotificationDeliveryCondition::OutsideVr => !game.is_steamvr_running,
        NotificationDeliveryCondition::GameClosed => !game.is_game_running,
        NotificationDeliveryCondition::GameRunning => game.is_game_running,
        NotificationDeliveryCondition::DesktopMode => game.is_game_no_vr && game.is_game_running,
    }
}
