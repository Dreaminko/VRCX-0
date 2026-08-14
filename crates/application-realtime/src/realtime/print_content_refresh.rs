use serde_json::Value;
use vrcx_0_core::realtime::RealtimeWsMessagePayload;

use super::event_kind::RealtimeWsEventKind;

pub fn is_print_created_content_refresh(payload: &RealtimeWsMessagePayload) -> bool {
    let Some(event_kind) = RealtimeWsEventKind::from_payload(payload) else {
        return false;
    };
    is_print_created_content_refresh_event(&event_kind, payload)
}

pub(crate) fn is_print_created_content_refresh_event(
    event_kind: &RealtimeWsEventKind,
    payload: &RealtimeWsMessagePayload,
) -> bool {
    if event_kind != &RealtimeWsEventKind::ContentRefresh {
        return false;
    }
    let content = payload.json.get("content").unwrap_or(&Value::Null);
    trimmed_text_field(content, "contentType") == "print"
        && trimmed_text_field(content, "actionType") == "created"
}

fn trimmed_text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}
