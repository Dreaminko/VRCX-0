use serde_json::{json, Value};
use vrcx_0_application_core::{
    FriendProjection, RealtimeInstanceClosedProjection, RealtimeInstanceQueueProjection,
    RealtimeNotificationProjection,
};

use super::content::nested_string;
use super::definitions::known_definition_for_type;
use super::types::{OverlayActivityCandidate, OverlayActivityEntry};
use super::OverlayActivityRuntime;
use vrcx_0_core::json::JsonExt;
use vrcx_0_core::text::first_owned;

impl OverlayActivityRuntime {
    pub fn ingest_friend_projection(
        &self,
        projection: &FriendProjection,
    ) -> Vec<OverlayActivityEntry> {
        self.apply_friend_membership_projection(projection);
        projection
            .feed_entries
            .iter()
            .filter_map(friend_feed_candidate)
            .filter_map(|candidate| self.ingest_candidate(candidate))
            .collect()
    }

    pub fn ingest_notification_projection(
        &self,
        projection: &RealtimeNotificationProjection,
    ) -> Vec<OverlayActivityEntry> {
        projection
            .upserts
            .iter()
            .filter(|upsert| upsert.deliver_runtime)
            .filter_map(|upsert| notification_candidate(&upsert.notification))
            .filter_map(|candidate| self.ingest_candidate(candidate))
            .collect()
    }

    pub fn ingest_instance_queue_projection(
        &self,
        projection: &RealtimeInstanceQueueProjection,
    ) -> Vec<OverlayActivityEntry> {
        if projection.kind != "ready" {
            return Vec::new();
        }
        let candidate = OverlayActivityCandidate {
            source_id: format!(
                "queue-ready:{}:{}",
                projection.instance_location, projection.received_at
            ),
            activity_type: "group.queueReady".to_string(),
            created_at: projection.received_at.clone(),
            actor_user_id: String::new(),
            actor_display_name: String::new(),
            current_instance: false,
            payload: json!({
                "instanceLocation": projection.instance_location,
                "worldId": projection.world_id,
                "worldName": projection.world_name,
                "position": projection.position,
                "queueSize": projection.queue_size,
            }),
        };
        self.ingest_candidate(candidate).into_iter().collect()
    }

    pub fn ingest_instance_closed_projection(
        &self,
        projection: &RealtimeInstanceClosedProjection,
    ) -> Vec<OverlayActivityEntry> {
        let notification = &projection.notification;
        let location = notification.trimmed_text("location");
        let created_at = first_owned([
            notification.trimmed_text("createdAt"),
            notification.trimmed_text("created_at"),
        ]);
        let candidate = OverlayActivityCandidate {
            source_id: format!("instance-closed:{location}:{created_at}"),
            activity_type: "instance.closed".to_string(),
            created_at,
            actor_user_id: String::new(),
            actor_display_name: String::new(),
            current_instance: false,
            payload: notification.clone(),
        };
        self.ingest_candidate(candidate).into_iter().collect()
    }

    fn apply_friend_membership_projection(&self, projection: &FriendProjection) {
        for patch in &projection.patches {
            self.insert_friend_user_id(patch.user_id.clone());
        }
        for user_id in &projection.removals {
            self.remove_friend_user_id(user_id);
        }
    }
}

fn friend_feed_candidate(value: &Value) -> Option<OverlayActivityCandidate> {
    let activity_type = value.trimmed_text("type");
    known_definition_for_type(&activity_type)?;
    let created_at = first_owned([
        value.trimmed_text("created_at"),
        value.trimmed_text("createdAt"),
    ]);
    let user_id = value.trimmed_text("userId");
    let current_instance = activity_type == "OnPlayerJoining";
    Some(OverlayActivityCandidate {
        source_id: format!("friend-feed:{activity_type}:{user_id}:{created_at}"),
        activity_type,
        created_at,
        actor_user_id: user_id,
        actor_display_name: value.trimmed_text("displayName"),
        current_instance,
        payload: value.clone(),
    })
}

fn notification_candidate(value: &Value) -> Option<OverlayActivityCandidate> {
    let activity_type = value.trimmed_text("type");
    known_definition_for_type(&activity_type)?;
    let id = first_owned([
        value.trimmed_text("id"),
        value.trimmed_text("notificationId"),
    ]);
    let created_at = first_owned([
        value.trimmed_text("createdAt"),
        value.trimmed_text("created_at"),
    ]);
    let actor_user_id = value.trimmed_text("senderUserId");
    let actor_user_id = if actor_user_id.starts_with("usr_") {
        actor_user_id
    } else {
        String::new()
    };
    let actor_display_name = notification_actor_display_name(value);
    let source_id = if id.trim().is_empty() {
        format!(
            "notification:{activity_type}:{actor_user_id}:{created_at}:{}",
            stable_json_hash(value)
        )
    } else {
        format!("notification:{id}")
    };
    Some(OverlayActivityCandidate {
        source_id,
        activity_type,
        created_at,
        actor_user_id,
        actor_display_name,
        current_instance: false,
        payload: value.clone(),
    })
}

fn notification_actor_display_name(value: &Value) -> String {
    first_owned([
        value.trimmed_text("senderDisplayName"),
        value.trimmed_text("displayName"),
        value.trimmed_text("senderUsername"),
        nested_string(value, &["details", "senderDisplayName"]),
        nested_string(value, &["details", "displayName"]),
        nested_string(value, &["data", "senderDisplayName"]),
        nested_string(value, &["data", "displayName"]),
    ])
}

fn stable_json_hash(value: &Value) -> String {
    let payload = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());
    let mut hash = 0xcbf29ce484222325u64;
    for byte in payload.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
