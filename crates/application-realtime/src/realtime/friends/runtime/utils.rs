use super::*;

use super::event_patch::record_string;

pub(super) fn string_or_previous(patch: &Value, previous: &FriendRecord, key: &str) -> String {
    let value = patch.text_field(key);
    if value.is_empty() {
        record_string(previous, key)
    } else {
        value
    }
}

pub(super) use vrcx_0_core::json::JsonExt;

pub(super) fn first_string<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> String {
    values
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub(super) use vrcx_0_core::text::{first_non_empty, first_owned};

pub(super) use vrcx_0_core::location::parse_location;

pub(super) struct EventTime {
    pub(super) iso: String,
    pub(super) timestamp_ms: i64,
}

impl EventTime {
    pub(super) fn from_received_at(received_at: &str) -> Self {
        let timestamp_ms = DateTime::parse_from_rfc3339(received_at)
            .map(|value| value.timestamp_millis())
            .unwrap_or_else(|_| Utc::now().timestamp_millis());
        Self {
            iso: received_at.to_string(),
            timestamp_ms,
        }
    }
}
