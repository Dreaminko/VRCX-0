use std::collections::HashMap;
use std::iter::once_with;

use chrono::Utc;
use serde_json::{json, Map, Value};
use vrcx_0_core::friends::{FriendRecord, StateBucket};
use vrcx_0_persistence::realtime::FriendLogUpsert;

use crate::realtime::RealtimeFriendOutput;

use super::event_patch::{record_string, record_value};
use super::utils::{first_non_empty, first_owned, parse_location, string_or_previous, JsonExt};

mod feed_entry;

use feed_entry::{
    feed_duration_ms, feed_entry_value, AvatarFeedEntry, BioFeedEntry, FeedEntryType,
    FriendRelationshipFeedEntry, GpsFeedEntry, OfflineFeedEntry, OnlineFeedEntry,
    PlayerJoiningFeedEntry, StatusFeedEntry, TrustLevelFeedEntry,
};

struct ResolvedLocationNames {
    world_name: String,
    group_name: String,
}

#[derive(Clone, Debug)]
pub(super) struct OfflineFeedPrevious {
    display_name: String,
    username: String,
    location: String,
    world_name: String,
    group_name: String,
    location_updated_at: i64,
}

impl OfflineFeedPrevious {
    pub(super) fn from_record(record: &FriendRecord) -> Self {
        Self {
            display_name: record.display_name.clone(),
            username: record.username.clone(),
            location: record.location.clone(),
            world_name: record_string(record, "worldName"),
            group_name: record_string(record, "groupName"),
            location_updated_at: record
                .extra
                .i64_field("locationUpdatedAt")
                .or_else(|| record.extra.i64_field("$location_at"))
                .unwrap_or(0),
        }
    }

    fn meaningful_name(&self, user_id: &str) -> String {
        vrcx_0_core::friends::meaningful_display_name(&self.display_name, &self.username, user_id)
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum FriendRelationshipFeedKind {
    Friend,
    Unfriend,
}

impl FriendRelationshipFeedKind {
    fn feed_type(self) -> FeedEntryType {
        match self {
            Self::Friend => FeedEntryType::Friend,
            Self::Unfriend => FeedEntryType::Unfriend,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct FriendFieldChange {
    next: Value,
    previous: Value,
}

#[derive(Clone, Debug, Default)]
pub(super) struct FriendChangedProps {
    changes: HashMap<String, FriendFieldChange>,
}

impl FriendChangedProps {
    pub(super) fn from_patch(patch: &Value, previous: Option<&FriendRecord>) -> Self {
        let Some(previous) = previous else {
            return Self::default();
        };
        let Some(patch_object) = patch.as_object() else {
            return Self::default();
        };
        let mut changes = HashMap::new();
        for (key, next) in patch_object {
            let previous = previous_value_for_diff(previous, key);
            if value_equal_for_diff(next, &previous) {
                continue;
            }
            changes.insert(
                key.clone(),
                FriendFieldChange {
                    next: next.clone(),
                    previous,
                },
            );
        }
        Self { changes }
    }

    fn get(&self, key: &str) -> Option<&FriendFieldChange> {
        self.changes.get(key)
    }

    pub(super) fn has(&self, key: &str) -> bool {
        self.changes.contains_key(key)
    }
}

fn previous_value_for_diff(previous: &FriendRecord, key: &str) -> Value {
    let value = record_value(previous, key);
    if !value.is_null() {
        return value;
    }
    match key {
        "currentAvatarTags" => json!([]),
        _ => Value::String(String::new()),
    }
}

pub(super) fn value_equal_for_diff(next: &Value, previous: &Value) -> bool {
    if next == previous {
        return true;
    }
    let next_empty_string = next.as_str().map(|value| value.is_empty()).unwrap_or(false);
    let previous_empty_string = previous
        .as_str()
        .map(|value| value.is_empty())
        .unwrap_or(false);
    next.is_null() && previous_empty_string || previous.is_null() && next_empty_string
}

pub(super) fn friend_log_upsert(
    user_id: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
    _state_bucket: &str,
    created_at: &str,
) -> FriendLogUpsert {
    FriendLogUpsert {
        target_user_id: user_id.to_string(),
        display_name: display_name(user_id, patch, previous),
        trust_level: first_owned([
            patch.text_field("$trustLevel"),
            patch.text_field("trustLevel"),
            previous
                .map(|previous| record_string(previous, "$trustLevel"))
                .unwrap_or_default(),
            previous
                .map(|previous| record_string(previous, "trustLevel"))
                .unwrap_or_default(),
        ]),
        friend_number: patch
            .i64_field("$friendNumber")
            .or_else(|| patch.i64_field("friendNumber"))
            .or_else(|| previous.and_then(|previous| previous.extra.i64_field("$friendNumber")))
            .or_else(|| previous.and_then(|previous| previous.extra.i64_field("friendNumber")))
            .unwrap_or(0),
        created_at: created_at.to_string(),
        force_history: false,
    }
}

pub(crate) fn trust_level_feed_entry(
    created_at: &str,
    user_id: &str,
    display_name: &str,
    trust_level: &str,
    previous_trust_level: &str,
    friend_number: i64,
) -> Value {
    feed_entry_value(&TrustLevelFeedEntry {
        created_at,
        entry_type: FeedEntryType::TrustLevel,
        user_id,
        display_name,
        trust_level,
        previous_trust_level,
        friend_number,
    })
}

pub(super) fn add_profile_diff_feed_entries(
    output: &mut RealtimeFriendOutput,
    user_id: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
    changes: &FriendChangedProps,
    created_at: &str,
) {
    let Some(previous) = previous.filter(|previous| is_online_state(previous)) else {
        return;
    };
    let status_changed = changes.has("status");
    let status_description_changed = changes.has("statusDescription");
    let next_status = string_or_previous(patch, previous, "status");
    let previous_status = previous.status.clone();
    if (status_changed || status_description_changed)
        && next_status != "offline"
        && previous_status != "offline"
    {
        output
            .persistence
            .feed_entries
            .push(feed_entry_value(&StatusFeedEntry {
                created_at,
                entry_type: FeedEntryType::Status,
                user_id,
                display_name: display_name(user_id, patch, Some(previous)),
                status: next_status,
                status_description: string_or_previous(patch, previous, "statusDescription"),
                previous_status,
                previous_status_description: &previous.status_description,
            }));
    }
    if changes.has("bio") && !patch.text_field("bio").is_empty() && !previous.bio.is_empty() {
        output
            .persistence
            .feed_entries
            .push(feed_entry_value(&BioFeedEntry {
                created_at,
                entry_type: FeedEntryType::Bio,
                user_id,
                display_name: display_name(user_id, patch, Some(previous)),
                bio: patch.text_field("bio"),
                previous_bio: &previous.bio,
            }));
    }
    let avatar_image_changed =
        changes.has("currentAvatarImageUrl") || changes.has("currentAvatarThumbnailImageUrl");
    let avatar_tags_changed = changes.has("currentAvatarTags");
    let profile_pic_override = string_or_previous(patch, previous, "profilePicOverride");
    let should_write_avatar =
        (avatar_image_changed && profile_pic_override.is_empty()) || avatar_tags_changed;
    let current_avatar = first_owned([
        string_or_previous(patch, previous, "currentAvatarImageUrl"),
        string_or_previous(patch, previous, "currentAvatarThumbnailImageUrl"),
    ]);
    let previous_avatar = first_owned([
        previous.current_avatar_image_url.clone(),
        previous.current_avatar_thumbnail_image_url.clone(),
    ]);
    if should_write_avatar && !previous_avatar.is_empty() && !current_avatar.is_empty() {
        let current_avatar_tags = changes
            .get("currentAvatarTags")
            .map(|change| change.next.clone())
            .or_else(|| previous.extra.get("currentAvatarTags").cloned())
            .unwrap_or_else(|| json!([]));
        let previous_avatar_tags = changes
            .get("currentAvatarTags")
            .map(|change| change.previous.clone())
            .or_else(|| previous.extra.get("currentAvatarTags").cloned())
            .unwrap_or_else(|| json!([]));
        output
            .persistence
            .feed_entries
            .push(feed_entry_value(&AvatarFeedEntry {
                created_at,
                entry_type: FeedEntryType::Avatar,
                user_id,
                display_name: display_name(user_id, patch, Some(previous)),
                owner_id: first_owned([
                    patch.text_field("currentAvatarAuthorId"),
                    patch.text_field("authorId"),
                    previous.current_avatar_author_id.clone(),
                    record_string(previous, "authorId"),
                ]),
                previous_owner_id: first_owned([
                    previous.current_avatar_author_id.clone(),
                    record_string(previous, "authorId"),
                ]),
                avatar_name: first_owned([
                    patch.text_field("currentAvatarName"),
                    patch.text_field("avatarName"),
                    previous.current_avatar_name.clone(),
                    record_string(previous, "avatarName"),
                ]),
                previous_avatar_name: first_owned([
                    previous.current_avatar_name.clone(),
                    record_string(previous, "avatarName"),
                ]),
                current_avatar_image_url: string_or_previous(
                    patch,
                    previous,
                    "currentAvatarImageUrl",
                ),
                current_avatar_thumbnail_image_url: string_or_previous(
                    patch,
                    previous,
                    "currentAvatarThumbnailImageUrl",
                ),
                previous_current_avatar_image_url: &previous.current_avatar_image_url,
                previous_current_avatar_thumbnail_image_url: &previous
                    .current_avatar_thumbnail_image_url,
                current_avatar_tags,
                previous_current_avatar_tags: previous_avatar_tags,
            }));
    }
}

pub(super) fn friend_relationship_feed_entry(
    relationship: FriendRelationshipFeedKind,
    user_id: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
    created_at: &str,
) -> Value {
    feed_entry_value(&FriendRelationshipFeedEntry {
        created_at,
        entry_type: relationship.feed_type(),
        user_id,
        display_name: display_name(user_id, patch, previous),
    })
}

pub(super) fn gps_feed_entry(
    user_id: &str,
    patch: &Value,
    previous: &FriendRecord,
    created_at: &str,
) -> Option<Value> {
    let previous_location = resolve_gps_previous_location(previous);
    let location = patch.text_field("location");
    if !is_gps_feed_location(&previous_location)
        || !is_gps_feed_location(&location)
        || previous_location == location
    {
        return None;
    }
    let location_names = if is_real_location(&location) {
        resolve_location_name(&location, patch, Some(previous))
    } else {
        ResolvedLocationNames {
            world_name: String::new(),
            group_name: String::new(),
        }
    };
    Some(feed_entry_value(&GpsFeedEntry {
        created_at,
        entry_type: FeedEntryType::Gps,
        user_id,
        display_name: display_name(user_id, patch, Some(previous)),
        location,
        world_name: location_names.world_name,
        previous_location,
        time: json!(resolve_gps_duration(previous)),
        group_name: location_names.group_name,
    }))
}

pub(crate) fn player_joining_feed_entry(
    user_id: &str,
    was_traveling: bool,
    current: &FriendRecord,
    created_at: &str,
) -> Option<Value> {
    if was_traveling
        || !parse_location(&current.location).is_traveling
        || current.traveling_to_location.trim().is_empty()
    {
        return None;
    }
    Some(feed_entry_value(&PlayerJoiningFeedEntry {
        created_at,
        entry_type: FeedEntryType::OnPlayerJoining,
        user_id,
        display_name: &current.display_name,
        location: &current.location,
        traveling_to_location: &current.traveling_to_location,
    }))
}

pub(super) fn online_feed_entry(
    user_id: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
    location: &str,
    time: i64,
    created_at: &str,
) -> Value {
    let location_names = if is_real_location(location) {
        resolve_location_name(location, patch, previous)
    } else {
        ResolvedLocationNames {
            world_name: String::new(),
            group_name: String::new(),
        }
    };
    feed_entry_value(&OnlineFeedEntry {
        created_at,
        entry_type: FeedEntryType::Online,
        user_id,
        display_name: display_name(user_id, patch, previous),
        location,
        world_name: location_names.world_name,
        group_name: location_names.group_name,
        time: feed_duration_ms(time),
    })
}

pub(super) fn offline_feed_entry(
    user_id: &str,
    current: &FriendRecord,
    previous: &OfflineFeedPrevious,
    created_at: &str,
    timestamp_ms: i64,
) -> Value {
    let location = previous.location.clone();
    let location_names = if is_real_location(&location) {
        resolve_record_location_name(&location, current, Some(previous))
    } else {
        ResolvedLocationNames {
            world_name: String::new(),
            group_name: String::new(),
        }
    };
    let time = if previous.location_updated_at > 0 {
        timestamp_ms.saturating_sub(previous.location_updated_at)
    } else {
        0
    };
    feed_entry_value(&OfflineFeedEntry {
        created_at,
        entry_type: FeedEntryType::Offline,
        user_id,
        display_name: first_owned([
            meaningful_record_name(current, user_id),
            previous.meaningful_name(user_id),
            "Unknown".to_string(),
        ]),
        location: &location,
        world_name: location_names.world_name,
        group_name: location_names.group_name,
        time: feed_duration_ms(time),
    })
}

pub(super) fn add_location_metadata(
    patch: &mut Map<String, Value>,
    previous: Option<&FriendRecord>,
    timestamp_ms: i64,
) {
    let location = patch.text_field("location");
    if location.eq_ignore_ascii_case("traveling") {
        if previous
            .map(|previous| previous.location.eq_ignore_ascii_case("traveling"))
            .unwrap_or(false)
        {
            return;
        }
        let previous_location = previous.map(resolve_previous_location).unwrap_or_default();
        let previous_timestamp = previous
            .and_then(|previous| {
                previous
                    .extra
                    .i64_field("locationUpdatedAt")
                    .or_else(|| previous.extra.i64_field("$location_at"))
            })
            .unwrap_or(0);
        patch.insert("locationUpdatedAt".into(), Value::from(timestamp_ms));
        patch.insert("$location_at".into(), Value::from(timestamp_ms));
        patch.insert("$travelingToTime".into(), Value::from(timestamp_ms));
        patch.insert("travelingToTime".into(), Value::from(timestamp_ms));
        if is_real_location(&previous_location) {
            patch.insert("$previousLocation".into(), Value::String(previous_location));
            patch.insert(
                "$previousLocation_at".into(),
                Value::from(previous_timestamp),
            );
        }
        return;
    }

    let previous_travel_location = previous
        .map(|previous| record_string(previous, "$previousLocation"))
        .unwrap_or_default();
    let previous_location_timestamp = previous
        .and_then(|previous| previous.extra.i64_field("$previousLocation_at"))
        .unwrap_or(0);
    let returned_to_previous_location =
        !previous_travel_location.is_empty() && previous_travel_location == location;
    let location_timestamp = if returned_to_previous_location && previous_location_timestamp > 0 {
        previous_location_timestamp
    } else {
        timestamp_ms
    };
    patch.insert("locationUpdatedAt".into(), Value::from(location_timestamp));
    patch.insert("$location_at".into(), Value::from(location_timestamp));
    patch.insert("$previousLocation".into(), Value::String(String::new()));
    patch.insert("$previousLocation_at".into(), Value::String(String::new()));
    patch.insert("$travelingToTime".into(), Value::String(String::new()));
    patch.insert("travelingToTime".into(), Value::String(String::new()));
}

pub(super) fn display_name(
    user_id: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
) -> String {
    first_owned([
        meaningful_name(patch, user_id),
        previous
            .map(|previous| meaningful_record_name(previous, user_id))
            .unwrap_or_default(),
        "Unknown".to_string(),
    ])
}

pub(super) fn meaningful_record_name(record: &FriendRecord, user_id: &str) -> String {
    vrcx_0_core::friends::meaningful_display_name(&record.display_name, &record.username, user_id)
        .unwrap_or_default()
}

pub(super) fn meaningful_name(value: &Value, user_id: &str) -> String {
    vrcx_0_core::friends::meaningful_display_name(
        &value.text_field("displayName"),
        &value.text_field("username"),
        user_id,
    )
    .unwrap_or_default()
}

fn resolve_location_name(
    location: &str,
    patch: &Value,
    previous: Option<&FriendRecord>,
) -> ResolvedLocationNames {
    let parsed = parse_location(location);
    ResolvedLocationNames {
        world_name: first_owned(
            once_with(|| patch.text_field("worldName"))
                .chain(once_with(|| {
                    patch
                        .get("world")
                        .and_then(|world| world.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                }))
                .chain(once_with(|| {
                    previous
                        .map(|previous| record_string(previous, "worldName"))
                        .unwrap_or_default()
                }))
                .chain(once_with(|| parsed.world_id.clone()))
                .chain(once_with(|| location.to_string())),
        ),
        group_name: first_owned(
            once_with(|| patch.text_field("groupName"))
                .chain(once_with(|| {
                    previous
                        .map(|previous| record_string(previous, "groupName"))
                        .unwrap_or_default()
                }))
                .chain(once_with(|| parsed.group_id.clone().unwrap_or_default())),
        ),
    }
}

fn resolve_record_location_name(
    location: &str,
    current: &FriendRecord,
    previous: Option<&OfflineFeedPrevious>,
) -> ResolvedLocationNames {
    let parsed = parse_location(location);
    ResolvedLocationNames {
        world_name: first_owned(
            once_with(|| record_string(current, "worldName"))
                .chain(once_with(|| {
                    current
                        .extra
                        .get("world")
                        .and_then(|world| world.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                }))
                .chain(once_with(|| {
                    previous
                        .map(|previous| previous.world_name.clone())
                        .unwrap_or_default()
                }))
                .chain(once_with(|| parsed.world_id.clone()))
                .chain(once_with(|| location.to_string())),
        ),
        group_name: first_owned(
            once_with(|| record_string(current, "groupName"))
                .chain(once_with(|| {
                    previous
                        .map(|previous| previous.group_name.clone())
                        .unwrap_or_default()
                }))
                .chain(once_with(|| parsed.group_id.unwrap_or_default())),
        ),
    }
}

pub(super) fn resolve_previous_location(previous: &FriendRecord) -> String {
    first_non_empty([
        previous.location.as_str(),
        previous
            .extra
            .get("$location")
            .and_then(|location| location.get("tag"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    ])
    .to_string()
}

pub(super) fn resolve_gps_previous_location(previous: &FriendRecord) -> String {
    let previous_location = previous.location.clone();
    if previous_location.eq_ignore_ascii_case("traveling") {
        return record_string(previous, "$previousLocation");
    }
    previous_location
}

pub(super) fn resolve_gps_duration(previous: &FriendRecord) -> i64 {
    if previous.location.eq_ignore_ascii_case("traveling") {
        let previous_timestamp = previous
            .extra
            .i64_field("$previousLocation_at")
            .unwrap_or(0);
        return if previous_timestamp > 0 {
            Utc::now().timestamp_millis() - previous_timestamp
        } else {
            0
        };
    }
    duration_ms(previous, Utc::now().timestamp_millis())
}

pub(super) fn duration_ms(previous: &FriendRecord, now_ms: i64) -> i64 {
    let timestamp = previous
        .extra
        .i64_field("locationUpdatedAt")
        .or_else(|| previous.extra.i64_field("$location_at"))
        .unwrap_or(0);
    if timestamp > 0 {
        now_ms.saturating_sub(timestamp)
    } else {
        0
    }
}

pub(super) fn is_online_state(record: &FriendRecord) -> bool {
    StateBucket::Online.matches(&record.state_bucket) || StateBucket::Online.matches(&record.state)
}

pub(super) fn is_real_location(location: &str) -> bool {
    let location = location.trim().to_ascii_lowercase();
    if location.is_empty() || location.starts_with("local") {
        return false;
    }
    !matches!(
        location.as_str(),
        ":" | "offline"
            | "offline:offline"
            | "traveling"
            | "traveling:traveling"
            | "private"
            | "private:private"
    )
}

pub(super) fn is_private_location(location: &str) -> bool {
    matches!(
        location.trim().to_ascii_lowercase().as_str(),
        "private" | "private:private"
    )
}

fn is_gps_feed_location(location: &str) -> bool {
    is_real_location(location) || is_private_location(location)
}
