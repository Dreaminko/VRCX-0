use serde_json::{json, Value};

pub trait GroupOrderSource: Send + Sync {
    fn read_group_order(&self, user_id: &str) -> Value;
}

pub struct UnavailableGroupOrderSource;

impl GroupOrderSource for UnavailableGroupOrderSource {
    fn read_group_order(&self, _user_id: &str) -> Value {
        json!([])
    }
}
