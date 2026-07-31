use serde_json::{Map, Value};
use vrcx_0_core::json::JsonExt;

use crate::realtime::PendingOfflineTimerAction;

use super::utils::normalize_id;

#[derive(Clone, Debug, Default)]
pub(super) struct RealtimeCurrentUserState {
    pub(super) generation: u64,
    pub(super) sequence: u64,
    pub(super) current_user_id: String,
    pub(super) snapshot: RealtimeCurrentUserStateSnapshot,
    pub(super) remote_snapshot: RealtimeCurrentUserStateSnapshot,
    pub(super) pending_offline: Option<PendingCurrentUserOffline>,
    pub(super) next_pending_token: u64,
    pub(super) remote_game_log_interval: Option<RemoteGameLogInterval>,
}

#[derive(Clone, Debug)]
pub(super) struct PendingCurrentUserOffline {
    pub(super) token: u64,
    pub(super) patch: Map<String, Value>,
}

#[derive(Clone, Debug)]
pub(super) struct RemoteGameLogInterval {
    pub(super) created_at: String,
    pub(super) started_at_ms: i64,
    pub(super) location: String,
}

#[derive(Default)]
pub(super) struct CurrentUserPatchOptions {
    pub(super) applies_local_game_authority: bool,
    pub(super) reconciles_remote_location: bool,
    pub(super) records_current_avatar_history: bool,
    pub(super) timer_action: PendingOfflineTimerAction,
}

#[derive(Clone, Debug, Default)]
pub(super) struct RealtimeCurrentUserStateSnapshot {
    pub(super) raw: Map<String, Value>,
    pub(super) user_id: String,
    pub(super) display_name: String,
    pub(super) location: String,
    pub(super) traveling_to_location: String,
    pub(super) world_id: String,
    pub(super) instance_id: String,
    pub(super) status: String,
    pub(super) status_description: String,
    pub(super) bio: String,
    pub(super) current_avatar: String,
    pub(super) current_avatar_image_url: String,
    pub(super) state_bucket: String,
    pub(super) world_name: String,
    pub(super) previous_avatar_swap_time: i64,
}

impl RealtimeCurrentUserStateSnapshot {
    pub(super) fn from_value(snapshot: serde_json::Value, current_user_id: &str) -> Self {
        Self::from_map(
            snapshot.as_object().cloned().unwrap_or_default(),
            current_user_id,
        )
    }

    pub(super) fn from_map(mut raw: Map<String, Value>, current_user_id: &str) -> Self {
        if !current_user_id.is_empty() {
            raw.insert("id".into(), Value::String(current_user_id.to_string()));
        }
        let mut snapshot = Self {
            raw,
            ..Self::default()
        };
        snapshot.refresh_typed_fields();
        snapshot
    }

    pub(super) fn to_map(&self) -> Map<String, Value> {
        let mut raw = self.raw.clone();
        if !self.user_id.is_empty() {
            raw.insert("id".into(), Value::String(self.user_id.clone()));
        }
        raw
    }

    pub(super) fn set_previous_avatar_swap_time(&mut self, value: Option<i64>) {
        self.previous_avatar_swap_time = value.unwrap_or_default();
        self.raw.insert(
            "$previousAvatarSwapTime".into(),
            value.map(Value::from).unwrap_or(Value::Null),
        );
    }

    fn refresh_typed_fields(&mut self) {
        self.user_id = normalize_id(&self.raw.text_field("id"));
        self.display_name = self.raw.text_field("displayName");
        self.location = self.raw.text_field("location");
        self.traveling_to_location = self.raw.text_field("travelingToLocation");
        self.world_id = self.raw.text_field("worldId");
        self.instance_id = self.raw.text_field("instanceId");
        self.status = self.raw.text_field("status");
        self.status_description = self.raw.text_field("statusDescription");
        self.bio = self.raw.text_field("bio");
        self.current_avatar = normalize_id(&self.raw.text_field("currentAvatar"));
        self.current_avatar_image_url = self.raw.text_field("currentAvatarImageUrl");
        self.state_bucket = self.raw.text_field("stateBucket");
        self.world_name = self.raw.text_field("worldName");
        self.previous_avatar_swap_time = self
            .raw
            .i64_field("$previousAvatarSwapTime")
            .unwrap_or_default();
    }
}

pub(super) const CURRENT_USER_REFRESH_LOCAL_AUTHORITY_FIELDS: &[&str] = &[
    "friends",
    "onlineFriends",
    "activeFriends",
    "offlineFriends",
    "status",
    "statusDescription",
    "state",
    "stateBucket",
    "pendingOffline",
    "location",
    "$location",
    "$location_at",
    "locationUpdatedAt",
    "worldId",
    "instanceId",
    "travelingToLocation",
    "travelingToWorld",
    "travelingToInstance",
    "$travelingToLocation",
    "$travelingToTime",
    "travelingToTime",
    "$previousLocation",
    "$previousLocation_at",
];

pub const CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS: &[&str] = &[
    "currentAvatar",
    "currentAvatarImageUrl",
    "currentAvatarName",
    "currentAvatarTags",
    "currentAvatarThumbnailImageUrl",
];

pub const CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS: &[&str] = &["fallbackAvatar"];

pub(super) const CURRENT_USER_REMOTE_PRESENCE_FIELDS: &[&str] = &[
    "location",
    "$location",
    "$location_at",
    "locationUpdatedAt",
    "worldId",
    "instanceId",
    "travelingToLocation",
    "travelingToWorld",
    "travelingToInstance",
    "$travelingToLocation",
    "$travelingToTime",
    "worldName",
    "state",
    "stateBucket",
];
