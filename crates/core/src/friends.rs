use std::collections::HashMap;

use compact_str::CompactString;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::text::first_non_empty;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateBucket {
    Online,
    Active,
    Offline,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptionalCompactString(Option<Option<CompactString>>);

impl OptionalCompactString {
    pub fn null() -> Self {
        Self(Some(None))
    }

    pub fn is_missing(&self) -> bool {
        self.0.is_none()
    }

    pub fn is_null(&self) -> bool {
        matches!(self.0, Some(None))
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.as_ref().and_then(Option::as_deref)
    }
}

impl From<&str> for OptionalCompactString {
    fn from(value: &str) -> Self {
        Self(Some(Some(value.into())))
    }
}

impl Serialize for OptionalCompactString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Some(Some(value)) => serializer.serialize_str(value),
            None | Some(None) => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for OptionalCompactString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<CompactString>::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}

impl StateBucket {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Active => "active",
            Self::Offline => "offline",
        }
    }

    pub fn from_exact(value: &str) -> Option<Self> {
        match value {
            "online" => Some(Self::Online),
            "active" => Some(Self::Active),
            "offline" => Some(Self::Offline),
            _ => None,
        }
    }

    pub fn normalize(value: &str) -> Option<Self> {
        Self::from_exact(value.trim().to_ascii_lowercase().as_str())
    }

    pub fn matches(self, value: &str) -> bool {
        value == self.as_str()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FriendRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    #[specta(type = String)]
    pub display_name: CompactString,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    #[specta(type = String)]
    pub state: CompactString,
    #[serde(default)]
    #[specta(type = String)]
    pub state_bucket: CompactString,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub traveling_to_location: String,
    #[serde(default)]
    pub world_id: String,
    #[serde(default)]
    #[specta(type = String)]
    pub platform: CompactString,
    #[serde(default, alias = "last_platform")]
    #[specta(type = String)]
    pub last_platform: CompactString,
    #[serde(default)]
    #[specta(type = String)]
    pub status: CompactString,
    #[serde(default)]
    #[specta(type = String)]
    pub status_description: CompactString,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub current_avatar_image_url: String,
    #[serde(default)]
    pub current_avatar_thumbnail_image_url: String,
    #[serde(default)]
    pub current_avatar_author_id: String,
    #[serde(default)]
    pub current_avatar_name: String,
    #[serde(
        rename = "date_joined",
        skip_serializing_if = "OptionalCompactString::is_missing",
        default
    )]
    #[specta(optional, type = Option<String>)]
    pub date_joined: OptionalCompactString,
    #[serde(
        rename = "last_activity",
        skip_serializing_if = "OptionalCompactString::is_missing",
        default
    )]
    #[specta(optional, type = Option<String>)]
    pub last_activity: OptionalCompactString,
    #[serde(
        rename = "last_login",
        skip_serializing_if = "OptionalCompactString::is_missing",
        default
    )]
    #[specta(optional, type = Option<String>)]
    pub last_login: OptionalCompactString,
    #[serde(
        rename = "last_mobile",
        skip_serializing_if = "OptionalCompactString::is_missing",
        default
    )]
    #[specta(optional, type = Option<String>)]
    pub last_mobile: OptionalCompactString,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl FriendRecord {
    pub fn normalized(mut self, fallback_user_id: &str) -> Option<Self> {
        self.id = normalize_user_id(first_non_empty([self.id.as_str(), fallback_user_id]));
        if self.id.is_empty() {
            return None;
        }

        self.state_bucket = StateBucket::normalize(first_non_empty([
            self.state_bucket.as_str(),
            self.state.as_str(),
        ]))
        .unwrap_or(StateBucket::Offline)
        .as_str()
        .into();
        self.state = self.state_bucket.clone();
        Some(self)
    }

    pub fn resolved_state_bucket(&self) -> Option<StateBucket> {
        [self.state_bucket.as_str(), self.state.as_str()]
            .into_iter()
            .find_map(StateBucket::normalize)
    }

    pub fn is_placeholder(&self) -> bool {
        self.extra.get("$profileSource").and_then(Value::as_str) == Some("placeholder")
    }

    pub fn display_name_or_id(&self) -> String {
        first_non_empty([
            self.display_name.as_str(),
            self.username.as_str(),
            self.id.as_str(),
        ])
        .to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FriendRosterBaseline {
    pub current_user_id: String,
    pub endpoint: String,
    pub websocket: String,
    #[serde(default)]
    pub friends_by_id: HashMap<String, FriendRecord>,
}

impl FriendRosterBaseline {
    pub fn normalized(mut self) -> Self {
        self.current_user_id = normalize_user_id(&self.current_user_id);
        self.endpoint = self.endpoint.trim().to_string();
        self.websocket = self.websocket.trim().to_string();
        self.friends_by_id = self
            .friends_by_id
            .into_iter()
            .filter_map(|(user_id, record)| {
                let normalized_user_id = normalize_user_id(&user_id);
                record
                    .normalized(&normalized_user_id)
                    .map(|record| (record.id.clone(), record))
            })
            .collect();
        self
    }
}

pub const DEFAULT_AVATAR_FILE_ID: &str = "file_0e8c4e32-7444-44ea-ade4-313c010d4bae";

pub fn strip_default_avatar_image(object: &mut Map<String, Value>) {
    let is_default = object
        .get("currentAvatarImageUrl")
        .and_then(Value::as_str)
        .is_some_and(|url| url.contains(DEFAULT_AVATAR_FILE_ID));
    if is_default {
        object.remove("currentAvatarImageUrl");
        object.remove("currentAvatarThumbnailImageUrl");
    }
}

pub fn normalize_user_id(value: &str) -> String {
    value.trim().to_string()
}

pub fn normalize_state_bucket(value: &str) -> Option<String> {
    StateBucket::normalize(value).map(|bucket| bucket.as_str().to_string())
}

pub fn meaningful_display_name(
    display_name: &str,
    username: &str,
    user_id: &str,
) -> Option<String> {
    let user_id = user_id.trim();
    for candidate in [display_name, username] {
        let candidate = candidate.trim();
        if !candidate.is_empty()
            && candidate != user_id
            && candidate != "Unknown"
            && !candidate.starts_with("usr_")
        {
            return Some(candidate.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        meaningful_display_name, strip_default_avatar_image, FriendRecord, FriendRosterBaseline,
        DEFAULT_AVATAR_FILE_ID,
    };
    use crate::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;
    use serde_json::{json, Value};

    #[test]
    fn strips_default_avatar_image_and_thumbnail() {
        let mut object = json!({
            "currentAvatarImageUrl": format!("{VRCHAT_API_DEFAULT_ENDPOINT}/file/{DEFAULT_AVATAR_FILE_ID}/1/file"),
            "currentAvatarThumbnailImageUrl": format!("{VRCHAT_API_DEFAULT_ENDPOINT}/file/{DEFAULT_AVATAR_FILE_ID}/1/256"),
            "displayName": "Friend"
        })
        .as_object()
        .cloned()
        .unwrap();

        strip_default_avatar_image(&mut object);

        assert!(!object.contains_key("currentAvatarImageUrl"));
        assert!(!object.contains_key("currentAvatarThumbnailImageUrl"));
        assert_eq!(
            object.get("displayName"),
            Some(&Value::String("Friend".into()))
        );
    }

    #[test]
    fn keeps_real_avatar_image() {
        let mut object = json!({
            "currentAvatarImageUrl": "https://api.vrchat.cloud/api/1/file/file_real/1/file",
            "currentAvatarThumbnailImageUrl": "https://api.vrchat.cloud/api/1/file/file_real/1/256"
        })
        .as_object()
        .cloned()
        .unwrap();

        strip_default_avatar_image(&mut object);

        assert!(object.contains_key("currentAvatarImageUrl"));
        assert!(object.contains_key("currentAvatarThumbnailImageUrl"));
    }

    #[test]
    fn normalizes_baseline_friend_records() {
        let baseline = FriendRosterBaseline {
            current_user_id: " usr_self ".into(),
            endpoint: " https://api.example.test ".into(),
            websocket: " wss://ws.example.test ".into(),
            friends_by_id: [(
                " usr_friend ".to_string(),
                FriendRecord {
                    display_name: "Friend".into(),
                    state: "online".into(),
                    ..FriendRecord::default()
                },
            )]
            .into_iter()
            .collect(),
        }
        .normalized();

        assert_eq!(baseline.current_user_id, "usr_self");
        assert_eq!(baseline.endpoint, "https://api.example.test");
        assert_eq!(baseline.websocket, "wss://ws.example.test");
        let friend = baseline.friends_by_id.get("usr_friend").unwrap();
        assert_eq!(friend.id, "usr_friend");
        assert_eq!(friend.state_bucket, "online");
        assert_eq!(friend.display_name_or_id(), "Friend");
    }

    #[test]
    fn compact_friend_fields_keep_string_serialization_and_unknown_fields() {
        let status_description = "a status description longer than twenty-four bytes";
        let record: FriendRecord = serde_json::from_value(json!({
            "id": "usr_friend",
            "displayName": "Friend",
            "state": "online",
            "stateBucket": "online",
            "platform": "standalonewindows",
            "last_platform": "android",
            "status": "join me",
            "statusDescription": status_description,
            "futureField": "preserved"
        }))
        .unwrap();

        assert!(!record.display_name.is_heap_allocated());
        assert!(!record.state.is_heap_allocated());
        assert!(!record.state_bucket.is_heap_allocated());
        assert!(!record.platform.is_heap_allocated());
        assert!(!record.last_platform.is_heap_allocated());
        assert!(!record.status.is_heap_allocated());
        assert!(record.status_description.is_heap_allocated());

        let serialized = serde_json::to_value(record).unwrap();
        assert_eq!(serialized["displayName"], "Friend");
        assert_eq!(serialized["state"], "online");
        assert_eq!(serialized["stateBucket"], "online");
        assert_eq!(serialized["platform"], "standalonewindows");
        assert_eq!(serialized["lastPlatform"], "android");
        assert_eq!(serialized["status"], "join me");
        assert_eq!(serialized["statusDescription"], status_description);
        assert_eq!(serialized["futureField"], "preserved");
    }

    #[test]
    fn compact_friend_dates_preserve_missing_null_and_string_states() {
        let record: FriendRecord = serde_json::from_value(json!({
            "id": "usr_friend",
            "date_joined": "2026-01-01",
            "last_activity": "2026-01-02T03:04:05.000Z",
            "last_login": null,
            "futureField": "preserved"
        }))
        .unwrap();

        assert_eq!(record.date_joined.as_str(), Some("2026-01-01"));
        assert_eq!(
            record.last_activity.as_str(),
            Some("2026-01-02T03:04:05.000Z")
        );
        assert!(record.last_login.is_null());
        assert!(record.last_mobile.is_missing());
        assert!(!record.extra.contains_key("date_joined"));
        assert!(!record.extra.contains_key("last_activity"));
        assert!(!record.extra.contains_key("last_login"));

        let serialized = serde_json::to_value(record).unwrap();
        assert_eq!(serialized["date_joined"], "2026-01-01");
        assert_eq!(
            serialized["last_activity"],
            "2026-01-02T03:04:05.000Z"
        );
        assert_eq!(serialized["last_login"], Value::Null);
        assert!(serialized.get("last_mobile").is_none());
        assert_eq!(serialized["futureField"], "preserved");
    }

    #[test]
    fn meaningful_display_name_skips_placeholders() {
        assert_eq!(
            meaningful_display_name("Nagisa", "naginagi", "usr_1"),
            Some("Nagisa".to_string())
        );
        assert_eq!(
            meaningful_display_name("  ", "naginagi", "usr_1"),
            Some("naginagi".to_string())
        );
        assert_eq!(meaningful_display_name("Unknown", "", "usr_1"), None);
        assert_eq!(meaningful_display_name("usr_1", "", "usr_1"), None);
        assert_eq!(meaningful_display_name("usr_other", "", "usr_1"), None);
        assert_eq!(meaningful_display_name("", "", "usr_1"), None);
    }
}
