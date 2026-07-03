use std::sync::{Arc, Mutex};

use serde_json::json;
use vrcx_0_application::{
    OverlayActivityActorRelation, OverlayActivityCategory, OverlayActivityContent,
    OverlayActivityDelivery, OverlayActivityEntry,
};
use vrcx_0_runtime_host::notification::{
    decide_notification_plan, DesktopNotifier, DesktopNotifierSlot, NotificationDeliveryGameState,
    NotificationDeliveryPreferences,
};

#[test]
fn webhook_delivery_ignores_game_state_conditions() {
    let preferences = NotificationDeliveryPreferences {
        desktop_toast: "Game Running".into(),
        notification_tts: "Game Running".into(),
        webhook_enabled: true,
        webhook_url: "https://example.com/webhook".into(),
        ..NotificationDeliveryPreferences::default()
    };
    let game = NotificationDeliveryGameState {
        is_game_running: false,
        is_steamvr_running: false,
        is_game_no_vr: false,
    };

    let plan = decide_notification_plan(&delivery(true, true, true), &preferences, &game);

    assert!(!plan.desktop);
    assert!(!plan.tts);
    assert!(plan.webhook);
}

#[test]
fn vr_delivery_requires_steamvr_and_enabled_channels() {
    let preferences = NotificationDeliveryPreferences {
        xs_notifications: true,
        ovrt_hud_notifications: true,
        ovrt_wrist_notifications: true,
        ..NotificationDeliveryPreferences::default()
    };

    let not_in_vr = decide_notification_plan(
        &delivery(false, true, false),
        &preferences,
        &NotificationDeliveryGameState {
            is_game_running: true,
            is_steamvr_running: false,
            is_game_no_vr: true,
        },
    );
    assert!(!not_in_vr.xs);
    assert!(!not_in_vr.ovrt);

    let in_vr = decide_notification_plan(
        &delivery(false, true, false),
        &preferences,
        &NotificationDeliveryGameState {
            is_game_running: true,
            is_steamvr_running: true,
            is_game_no_vr: false,
        },
    );
    assert!(in_vr.xs);
    assert!(in_vr.ovrt);
    assert!(in_vr.ovrt_hud);
    assert!(in_vr.ovrt_wrist);
}

#[test]
fn desktop_notifier_slot_noops_until_tauri_injects_notifier() {
    let slot = DesktopNotifierSlot::default();

    slot.show("Title", Some("Body"), None, true).unwrap();

    let recorder = Arc::new(RecordingDesktopNotifier::default());
    slot.set(recorder.clone());
    slot.show("Title", Some("Body"), Some("image.png"), true)
        .unwrap();

    assert_eq!(
        recorder.entries.lock().unwrap().as_slice(),
        &[DesktopNotificationRecord {
            title: "Title".into(),
            body: Some("Body".into()),
            image: Some("image.png".into()),
            play_sound: true,
        }]
    );
}

fn delivery(desktop: bool, vr: bool, webhook: bool) -> OverlayActivityDelivery {
    OverlayActivityDelivery {
        entry: OverlayActivityEntry {
            sequence: 1,
            source_id: "notification:1".into(),
            activity_type: "Online".into(),
            category: OverlayActivityCategory::FavoriteMovement,
            created_at: "2026-06-18T08:30:00.000Z".into(),
            actor_user_id: "usr_123".into(),
            actor_display_name: "Pizza".into(),
            content: OverlayActivityContent::default(),
            actor_relation: OverlayActivityActorRelation::Friend,
            payload: json!({}),
        },
        desktop,
        vr,
        hmd: false,
        webhook,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DesktopNotificationRecord {
    title: String,
    body: Option<String>,
    image: Option<String>,
    play_sound: bool,
}

#[derive(Default)]
struct RecordingDesktopNotifier {
    entries: Mutex<Vec<DesktopNotificationRecord>>,
}

impl DesktopNotifier for RecordingDesktopNotifier {
    fn show(
        &self,
        title: &str,
        body: Option<&str>,
        image: Option<&str>,
        play_sound: bool,
    ) -> Result<(), String> {
        self.entries
            .lock()
            .unwrap()
            .push(DesktopNotificationRecord {
                title: title.into(),
                body: body.map(str::to_string),
                image: image.map(str::to_string),
                play_sound,
            });
        Ok(())
    }
}
