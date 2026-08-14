use serde_json::{Map, Value};
use vrcx_0_application_core::{FriendProjectionPatch, FriendStateBucketAuthority};
use vrcx_0_core::friends::FriendRecord;

use super::super::utils::parse_location;
use vrcx_0_core::json::JsonExt;

#[derive(Clone, Debug)]
pub(in crate::realtime::friends::runtime) enum FriendRecordPatch {
    Fields(Map<String, Value>),
    Full(Box<FriendRecord>),
}

impl FriendRecordPatch {
    pub(in crate::realtime::friends::runtime) fn from_value(value: &Value) -> Self {
        Self::Fields(value.as_object().cloned().unwrap_or_default())
    }

    pub(in crate::realtime::friends::runtime) fn from_record(record: &FriendRecord) -> Self {
        Self::Full(Box::new(record.clone()))
    }

    pub(in crate::realtime::friends::runtime) fn set_pending_offline(
        &mut self,
        pending_offline: bool,
    ) {
        let extra = match self {
            Self::Fields(fields) => fields,
            Self::Full(record) => &mut record.extra,
        };
        extra.insert("pendingOffline".into(), Value::Bool(pending_offline));
    }

    fn apply_to(&self, target: &mut FriendRecord) {
        match self {
            Self::Fields(fields) => apply_fields(target, fields),
            Self::Full(record) => {
                let mut existing_extra = std::mem::take(&mut target.extra);
                *target = record.as_ref().clone();
                existing_extra.extend(target.extra.clone());
                target.extra = existing_extra;
            }
        }
    }
}

pub(super) struct FriendRecordTransition {
    pub(super) next: FriendRecord,
    pub(super) projection: FriendProjectionPatch,
    pub(super) was_traveling: bool,
}

pub(super) fn apply_friend_patch(
    previous: Option<&FriendRecord>,
    user_id: &str,
    patch: &FriendRecordPatch,
    state_bucket: &str,
    state_bucket_authority: FriendStateBucketAuthority,
) -> FriendRecordTransition {
    let mut next = previous.cloned().unwrap_or_default();
    let was_traveling = parse_location(&next.location).is_traveling;
    patch.apply_to(&mut next);
    next.id = user_id.to_string();
    next.state = state_bucket.into();
    next.state_bucket = state_bucket.into();
    sanitize_extra(&mut next);

    FriendRecordTransition {
        projection: FriendProjectionPatch {
            user_id: user_id.to_string(),
            patch: next.clone(),
            state_bucket: state_bucket.into(),
            state_bucket_authority,
        },
        next,
        was_traveling,
    }
}

const FRIEND_NAMED_FIELD_KEYS: &[&str] = &[
    "id",
    "displayName",
    "username",
    "state",
    "stateBucket",
    "location",
    "travelingToLocation",
    "worldId",
    "platform",
    "lastPlatform",
    "last_platform",
    "status",
    "statusDescription",
    "bio",
    "currentAvatarImageUrl",
    "currentAvatarThumbnailImageUrl",
    "currentAvatarAuthorId",
    "currentAvatarName",
];

fn patch_str<'a>(patch: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        match patch.get(*key) {
            Some(Value::String(value)) => return Some(value),
            Some(Value::Null) | None => {}
            Some(other) => tracing::warn!(
                "friend patch field `{}` has non-string value: {}",
                *key,
                other
            ),
        }
    }
    None
}

fn apply_fields(record: &mut FriendRecord, patch: &Map<String, Value>) {
    let compact_fields = [
        (&mut record.display_name, &["displayName"][..]),
        (&mut record.platform, &["platform"]),
        (
            &mut record.last_platform,
            &["lastPlatform", "last_platform"],
        ),
        (&mut record.status, &["status"]),
        (&mut record.status_description, &["statusDescription"]),
    ];
    for (target, keys) in compact_fields {
        if let Some(value) = patch_str(patch, keys) {
            *target = value.into();
        }
    }
    let string_fields = [
        (&mut record.username, &["username"][..]),
        (&mut record.location, &["location"]),
        (&mut record.traveling_to_location, &["travelingToLocation"]),
        (&mut record.world_id, &["worldId"]),
        (&mut record.bio, &["bio"]),
        (
            &mut record.current_avatar_image_url,
            &["currentAvatarImageUrl"],
        ),
        (
            &mut record.current_avatar_thumbnail_image_url,
            &["currentAvatarThumbnailImageUrl"],
        ),
        (
            &mut record.current_avatar_author_id,
            &["currentAvatarAuthorId"],
        ),
        (&mut record.current_avatar_name, &["currentAvatarName"]),
    ];
    for (target, keys) in string_fields {
        if let Some(value) = patch_str(patch, keys) {
            *target = value.to_string();
        }
    }
    record.extra.extend(
        patch
            .iter()
            .filter(|(key, _)| !FRIEND_NAMED_FIELD_KEYS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
}

fn sanitize_extra(record: &mut FriendRecord) {
    record
        .extra
        .retain(|key, _| !FRIEND_NAMED_FIELD_KEYS.contains(&key.as_str()));
}

pub(in crate::realtime::friends::runtime) fn record_string(
    record: &FriendRecord,
    key: &str,
) -> String {
    match key {
        "id" => record.id.clone(),
        "displayName" => record.display_name.to_string(),
        "username" => record.username.clone(),
        "state" => record.state.to_string(),
        "stateBucket" => record.state_bucket.to_string(),
        "location" => record.location.clone(),
        "travelingToLocation" => record.traveling_to_location.clone(),
        "worldId" => record.world_id.clone(),
        "platform" => record.platform.to_string(),
        "lastPlatform" | "last_platform" => record.last_platform.to_string(),
        "status" => record.status.to_string(),
        "statusDescription" => record.status_description.to_string(),
        "bio" => record.bio.clone(),
        "currentAvatarImageUrl" => record.current_avatar_image_url.clone(),
        "currentAvatarThumbnailImageUrl" => record.current_avatar_thumbnail_image_url.clone(),
        "currentAvatarAuthorId" => record.current_avatar_author_id.clone(),
        "currentAvatarName" => record.current_avatar_name.clone(),
        _ => record.extra.text_field(key),
    }
}

pub(in crate::realtime::friends::runtime) fn record_value(
    record: &FriendRecord,
    key: &str,
) -> Value {
    if FRIEND_NAMED_FIELD_KEYS.contains(&key) {
        Value::String(record_string(record, key))
    } else {
        record.extra.get(key).cloned().unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn transition_normalizes_aliases_and_preserves_unknown_fields() {
        let previous = FriendRecord {
            id: "usr_x".into(),
            state: "active".into(),
            state_bucket: "active".into(),
            location: "offline".into(),
            status_description: "hi".into(),
            ..FriendRecord::default()
        };
        let patch = FriendRecordPatch::from_value(&json!({
            "last_platform": "standalonewindows",
            "location": "traveling",
            "statusDescription": Value::Null,
            "$location": { "tag": "traveling" }
        }));
        let transition = apply_friend_patch(
            Some(&previous),
            "usr_x",
            &patch,
            "online",
            FriendStateBucketAuthority::Explicit,
        );

        assert_eq!(transition.next.last_platform, "standalonewindows");
        assert_eq!(transition.next.location, "traveling");
        assert_eq!(transition.next.status_description, "hi");
        assert_eq!(transition.next.extra["$location"]["tag"], "traveling");
        assert!(transition
            .projection
            .patch
            .extra
            .get("last_platform")
            .is_none());
    }
}
