use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Serialize)]
pub(super) enum FeedEntryType {
    Online,
    Offline,
    #[serde(rename = "GPS")]
    Gps,
    Status,
    Bio,
    Avatar,
    TrustLevel,
    Friend,
    Unfriend,
    OnPlayerJoining,
}

pub(super) fn feed_entry_value<T: Serialize>(entry: &T) -> Value {
    serde_json::to_value(entry).expect("feed entry is always serializable")
}

pub(super) fn feed_duration_ms(duration_ms: i64) -> Value {
    if duration_ms > 0 {
        json!(duration_ms)
    } else {
        json!("")
    }
}

#[derive(Serialize)]
pub(super) struct TrustLevelFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: &'a str,
    #[serde(rename = "trustLevel")]
    pub(super) trust_level: &'a str,
    #[serde(rename = "previousTrustLevel")]
    pub(super) previous_trust_level: &'a str,
    #[serde(rename = "friendNumber")]
    pub(super) friend_number: i64,
}

#[derive(Serialize)]
pub(super) struct StatusFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    pub(super) status: String,
    #[serde(rename = "statusDescription")]
    pub(super) status_description: String,
    #[serde(rename = "previousStatus")]
    pub(super) previous_status: String,
    #[serde(rename = "previousStatusDescription")]
    pub(super) previous_status_description: &'a str,
}

#[derive(Serialize)]
pub(super) struct BioFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    pub(super) bio: String,
    #[serde(rename = "previousBio")]
    pub(super) previous_bio: &'a str,
}

#[derive(Serialize)]
pub(super) struct AvatarFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    #[serde(rename = "ownerId")]
    pub(super) owner_id: String,
    #[serde(rename = "previousOwnerId")]
    pub(super) previous_owner_id: String,
    #[serde(rename = "avatarName")]
    pub(super) avatar_name: String,
    #[serde(rename = "previousAvatarName")]
    pub(super) previous_avatar_name: String,
    #[serde(rename = "currentAvatarImageUrl")]
    pub(super) current_avatar_image_url: String,
    #[serde(rename = "currentAvatarThumbnailImageUrl")]
    pub(super) current_avatar_thumbnail_image_url: String,
    #[serde(rename = "previousCurrentAvatarImageUrl")]
    pub(super) previous_current_avatar_image_url: &'a str,
    #[serde(rename = "previousCurrentAvatarThumbnailImageUrl")]
    pub(super) previous_current_avatar_thumbnail_image_url: &'a str,
    #[serde(rename = "currentAvatarTags")]
    pub(super) current_avatar_tags: Value,
    #[serde(rename = "previousCurrentAvatarTags")]
    pub(super) previous_current_avatar_tags: Value,
}

#[derive(Serialize)]
pub(super) struct FriendRelationshipFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
}

#[derive(Serialize)]
pub(super) struct GpsFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    pub(super) location: String,
    #[serde(rename = "worldName")]
    pub(super) world_name: String,
    #[serde(rename = "previousLocation")]
    pub(super) previous_location: String,
    pub(super) time: Value,
    #[serde(rename = "groupName")]
    pub(super) group_name: String,
}

#[derive(Serialize)]
pub(super) struct PlayerJoiningFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: &'a str,
    pub(super) location: &'a str,
    #[serde(rename = "travelingToLocation")]
    pub(super) traveling_to_location: &'a str,
}

#[derive(Serialize)]
pub(super) struct OnlineFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    pub(super) location: &'a str,
    #[serde(rename = "worldName")]
    pub(super) world_name: String,
    #[serde(rename = "groupName")]
    pub(super) group_name: String,
    pub(super) time: Value,
}

#[derive(Serialize)]
pub(super) struct OfflineFeedEntry<'a> {
    pub(super) created_at: &'a str,
    #[serde(rename = "type")]
    pub(super) entry_type: FeedEntryType,
    #[serde(rename = "userId")]
    pub(super) user_id: &'a str,
    #[serde(rename = "displayName")]
    pub(super) display_name: String,
    pub(super) location: &'a str,
    #[serde(rename = "worldName")]
    pub(super) world_name: String,
    #[serde(rename = "groupName")]
    pub(super) group_name: String,
    pub(super) time: Value,
}
