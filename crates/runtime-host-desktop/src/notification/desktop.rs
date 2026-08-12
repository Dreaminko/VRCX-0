use std::sync::{Arc, Mutex};

use vrcx_0_core::vrchat_ids::is_user_id;
use vrcx_0_runtime_host::notification::RenderedNotification;

use super::NotificationDeliveryPreferences;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopNotificationAction {
    pub owner_user_id: String,
    pub user_id: String,
}

impl DesktopNotificationAction {
    pub fn open_user_profile(owner_user_id: &str, user_id: &str) -> Option<Self> {
        if !is_user_id(owner_user_id) || !is_user_id(user_id) {
            return None;
        }
        Some(Self {
            owner_user_id: owner_user_id.to_string(),
            user_id: user_id.to_string(),
        })
    }
}

pub trait DesktopNotifier: Send + Sync {
    fn show(
        &self,
        title: &str,
        body: Option<&str>,
        image: Option<&str>,
        play_sound: bool,
        action: Option<&DesktopNotificationAction>,
    ) -> Result<(), String>;
}

#[derive(Clone, Default)]
pub struct DesktopNotifierSlot {
    inner: Arc<Mutex<Option<Arc<dyn DesktopNotifier>>>>,
}

impl DesktopNotifierSlot {
    pub fn set(&self, notifier: Arc<dyn DesktopNotifier>) {
        match self.inner.lock() {
            Ok(mut slot) => {
                *slot = Some(notifier);
            }
            Err(error) => {
                tracing::warn!("failed to set desktop notification bridge: {error}");
            }
        }
    }
}

impl DesktopNotifier for DesktopNotifierSlot {
    fn show(
        &self,
        title: &str,
        body: Option<&str>,
        image: Option<&str>,
        play_sound: bool,
        action: Option<&DesktopNotificationAction>,
    ) -> Result<(), String> {
        let notifier = self
            .inner
            .lock()
            .map_err(|error| format!("desktop notification bridge lock poisoned: {error}"))?
            .clone();
        let Some(notifier) = notifier else {
            return Ok(());
        };
        notifier.show(title, body, image, play_sound, action)
    }
}

pub(super) fn send_desktop_notification(
    notifier: &dyn DesktopNotifier,
    render: &RenderedNotification,
    preferences: &NotificationDeliveryPreferences,
    local_image: Option<&str>,
    action: Option<&DesktopNotificationAction>,
) {
    if let Err(error) = notifier.show(
        &render.title,
        non_empty(&render.body),
        local_image,
        preferences.desktop_notification_sound,
        action,
    ) {
        tracing::warn!("[Desktop] notification send failed: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopNotificationAction;

    #[test]
    fn open_user_profile_action_requires_canonical_owner_and_actor_ids() {
        let action = DesktopNotificationAction::open_user_profile(
            "usr_12345678-1234-1234-1234-1234567890ab",
            "usr_abcdefab-cdef-abcd-efab-cdefabcdefab",
        );

        assert_eq!(
            action,
            Some(DesktopNotificationAction {
                owner_user_id: "usr_12345678-1234-1234-1234-1234567890ab".into(),
                user_id: "usr_abcdefab-cdef-abcd-efab-cdefabcdefab".into(),
            })
        );
        assert!(DesktopNotificationAction::open_user_profile(
            "usr_invalid",
            "usr_abcdefab-cdef-abcd-efab-cdefabcdefab"
        )
        .is_none());
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}
