use vrcx_0_persistence::config::ConfigRepository;

use super::generic_webhook::{default_webhook_fields, is_default_webhook_field};
use super::{
    NotificationDeliveryCondition, NotificationDeliveryPreferences, NotificationTtsNameMode,
    NotificationWebhookFormat,
};

pub(super) struct NotificationWebhookPreferences {
    pub enabled: bool,
    pub url: String,
    pub format: NotificationWebhookFormat,
    pub fields: Vec<String>,
    pub show_instance_id_in_location: bool,
}

pub(super) fn load_webhook_preferences(
    config: &ConfigRepository,
) -> NotificationWebhookPreferences {
    NotificationWebhookPreferences {
        enabled: config_bool(config, "webhookEnabled", false),
        url: config_string(config, "webhookUrl", ""),
        format: NotificationWebhookFormat::from_config(&config_string(
            config,
            "webhookFormat",
            "generic",
        )),
        fields: parse_webhook_fields(&config_string(config, "webhookFields", "")),
        show_instance_id_in_location: config_bool(config, "VRCX_showInstanceIdInLocation", false),
    }
}

pub fn load_preferences(config: &ConfigRepository) -> NotificationDeliveryPreferences {
    NotificationDeliveryPreferences {
        desktop_toast: NotificationDeliveryCondition::from_config(&config_string(
            config,
            "desktopToast",
            "Never",
        )),
        desktop_notification_sound: config_bool(config, "desktopNotificationSound", false),
        notification_tts: NotificationDeliveryCondition::from_config(&config_string(
            config,
            "notificationTTS",
            "Never",
        )),
        notification_tts_name_mode: config_tts_name_mode(config),
        notification_tts_voice_native: config_string(config, "notificationTTSVoiceNative", ""),
        xs_notifications: config_bool_with_legacy(config, "xsNotifications", false),
        ovrt_hud_notifications: config_bool_with_legacy(config, "ovrtHudNotifications", false),
        ovrt_wrist_notifications: config_bool_with_legacy(config, "ovrtWristNotifications", false),
        image_notifications: config_bool_with_legacy(config, "imageNotifications", true),
        notification_timeout_ms: config_int_with_legacy(config, "notificationTimeout", 3000),
        notification_opacity_percent: config_int_with_legacy(config, "notificationOpacity", 100),
        webhook_enabled: config_bool(config, "webhookEnabled", false),
        webhook_url: config_string(config, "webhookUrl", ""),
        webhook_format: NotificationWebhookFormat::from_config(&config_string(
            config,
            "webhookFormat",
            "generic",
        )),
        webhook_fields: parse_webhook_fields(&config_string(config, "webhookFields", "")),
        show_instance_id_in_location: config_bool(config, "VRCX_showInstanceIdInLocation", false),
    }
}

pub fn config_tts_name_mode(config: &ConfigRepository) -> NotificationTtsNameMode {
    let configured = config_string(config, "notificationTTSNameMode", "");
    if !configured.trim().is_empty() {
        return notification_tts_name_mode(&configured);
    }
    if config_bool(config, "notificationTTSNickName", false) {
        NotificationTtsNameMode::Note
    } else {
        NotificationTtsNameMode::Username
    }
}

fn config_string(config: &ConfigRepository, key: &str, default_value: &str) -> String {
    config
        .get_string(key, default_value)
        .unwrap_or_else(|_| default_value.to_string())
}

pub fn config_bool(config: &ConfigRepository, key: &str, default_value: bool) -> bool {
    config.get_bool(key, default_value).unwrap_or(default_value)
}

fn config_bool_with_legacy(config: &ConfigRepository, key: &str, default_value: bool) -> bool {
    if config.get_raw(key).ok().flatten().is_some() {
        return config_bool(config, key, default_value);
    }
    if let Some(legacy_key) = legacy_overlay_notification_key(key) {
        if config.get_raw(legacy_key).ok().flatten().is_some() {
            return config_bool(config, legacy_key, default_value);
        }
    }
    default_value
}

pub fn seed_hmd_notifications_default(
    config: &ConfigRepository,
) -> Result<Option<bool>, vrcx_0_persistence::Error> {
    if config.get_raw("hmdNotificationsEnabled")?.is_some() {
        return Ok(None);
    }
    let external_overlay_enabled = [
        "xsNotifications",
        "ovrtHudNotifications",
        "ovrtWristNotifications",
    ]
    .into_iter()
    .any(|key| config_bool_with_legacy(config, key, false));
    let enabled = !external_overlay_enabled;
    config.set_bool("hmdNotificationsEnabled", enabled)?;
    Ok(Some(enabled))
}

fn config_int_with_legacy(config: &ConfigRepository, key: &str, default_value: i32) -> i32 {
    if let Some(raw) = config.get_raw(key).ok().flatten() {
        return parse_config_int(&raw, default_value);
    }
    if let Some(legacy_key) = legacy_overlay_notification_key(key) {
        if let Some(raw) = config.get_raw(legacy_key).ok().flatten() {
            return parse_config_int(&raw, default_value);
        }
    }
    default_value
}

fn parse_config_int(value: &str, default_value: i32) -> i32 {
    value.trim().parse::<i32>().unwrap_or(default_value)
}

fn legacy_overlay_notification_key(key: &str) -> Option<&'static str> {
    match key {
        "xsNotifications" => Some("VRCX-0_xsNotifications"),
        "ovrtHudNotifications" => Some("VRCX-0_ovrtHudNotifications"),
        "ovrtWristNotifications" => Some("VRCX-0_ovrtWristNotifications"),
        "imageNotifications" => Some("VRCX-0_imageNotifications"),
        "notificationTimeout" => Some("VRCX-0_notificationTimeout"),
        "notificationOpacity" => Some("VRCX-0_notificationOpacity"),
        _ => None,
    }
}

pub fn parse_webhook_fields(value: &str) -> Vec<String> {
    let fields = value.trim();
    if fields.is_empty() {
        return default_webhook_fields();
    }
    let parsed = if fields.starts_with('[') {
        serde_json::from_str::<Vec<String>>(fields).unwrap_or_default()
    } else {
        fields.split(',').map(str::to_string).collect()
    };
    let mut selected = Vec::new();
    for field in parsed {
        let field = field.trim();
        if is_default_webhook_field(field) && !selected.iter().any(|item| item == field) {
            selected.push(field.to_string());
        }
    }
    if selected.is_empty() {
        default_webhook_fields()
    } else {
        selected
    }
}

pub fn notification_tts_name_mode(value: &str) -> NotificationTtsNameMode {
    match value {
        "note" => NotificationTtsNameMode::Note,
        "usernameAndNote" => NotificationTtsNameMode::UsernameAndNote,
        _ => NotificationTtsNameMode::Username,
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use vrcx_0_persistence::{config::ConfigRepository, DatabaseService};

    use super::seed_hmd_notifications_default;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-notification-preferences-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_config(name: &str) -> (TestDir, ConfigRepository) {
        let dir = TestDir::new(name);
        let db = Arc::new(DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap());
        (dir, ConfigRepository::new(db))
    }

    #[test]
    fn hmd_default_seed_preserves_legacy_forwarding_contract() {
        let (_dir, config) = test_config("legacy-enabled");
        config.set_bool("VRCX-0_xsNotifications", true).unwrap();

        assert_eq!(
            seed_hmd_notifications_default(&config).unwrap(),
            Some(false)
        );
        assert!(!config.get_bool("hmdNotificationsEnabled", true).unwrap());
    }

    #[test]
    fn hmd_default_seed_runs_once() {
        let (_dir, config) = test_config("existing-value");
        config.set_bool("hmdNotificationsEnabled", false).unwrap();

        assert_eq!(seed_hmd_notifications_default(&config).unwrap(), None);
        assert!(!config.get_bool("hmdNotificationsEnabled", true).unwrap());
    }
}
