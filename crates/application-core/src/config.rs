use serde_json::{json, Value};

use vrcx_0_persistence::config::{get_json, set_json};
use vrcx_0_persistence::DatabaseService;

use crate::Result;

pub fn read_config_string_array(db: &DatabaseService, key: &str) -> Result<Vec<String>> {
    let parsed = get_json(db, key, Value::Null)?;
    let mut values = parsed
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(config_value_to_string)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    values.sort();
    values.dedup();
    Ok(values)
}

pub fn write_config_string_array(db: &DatabaseService, key: &str, values: &[String]) -> Result<()> {
    set_json(db, key, &json!(values))?;
    Ok(())
}

fn config_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}
