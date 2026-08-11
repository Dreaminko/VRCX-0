mod desktop;
mod dispatcher;
mod overlay_transport;
#[cfg(any(windows, target_os = "linux"))]
mod ovrt;
mod tts;
#[cfg(any(windows, target_os = "linux"))]
mod xs_overlay;

pub use desktop::{DesktopNotificationAction, DesktopNotifier, DesktopNotifierSlot};
pub use dispatcher::{NotificationDispatcher, NotificationDispatcherDeps};
