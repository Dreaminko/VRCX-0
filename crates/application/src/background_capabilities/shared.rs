use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct BackgroundCapabilitySession {
    pub current_user_id: String,
    pub endpoint: String,
    pub websocket: String,
    pub current_user_snapshot: Value,
}

pub(super) fn parse_response_json(data: &str) -> Option<Value> {
    serde_json::from_str(data).ok()
}
