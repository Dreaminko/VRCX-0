use serde_json::Value;
use vrcx_0_core::json::JsonExt;

pub(super) use vrcx_0_application_core::BackgroundCapabilitySession;

pub(super) fn string_field(value: &Value, key: &str) -> Option<String> {
    value.trimmed_string(key)
}

pub(super) fn non_empty(value: &str, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value.trim().to_string()
    }
}

pub(super) fn parse_response_json(data: &str) -> Option<Value> {
    serde_json::from_str(data).ok()
}
