mod delivery;
mod desktop;
mod dispatcher;
mod overlay_transport;
#[cfg(any(windows, target_os = "linux"))]
mod ovrt;
mod preferences;
mod tts;
#[cfg(any(windows, target_os = "linux"))]
mod xs_overlay;

pub use delivery::{
    decide_notification_plan, NotificationDeliveryCondition, NotificationDeliveryGameState,
    NotificationDeliveryPlan, NotificationDeliveryPreferences, NotificationTtsNameMode,
};
pub use desktop::{DesktopNotificationAction, DesktopNotifier, DesktopNotifierSlot};
pub use dispatcher::{NotificationDispatcher, NotificationDispatcherDeps};
pub use preferences::{
    config_tts_name_mode, load_preferences, notification_tts_name_mode,
    seed_hmd_notifications_default,
};
