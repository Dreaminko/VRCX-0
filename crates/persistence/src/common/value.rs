use serde_json::Value;
use vrcx_0_core::json::text_of;

use super::DbParams;

pub(crate) use vrcx_0_core::time::now_iso;

pub(crate) fn normalize_text(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

pub(crate) fn value_as_string(value: &Value) -> String {
    text_of(Some(value))
}

pub(crate) fn value_as_i64(value: &Value) -> i64 {
    if let Some(value) = value.as_i64() {
        return value;
    }
    value_as_string(value).parse::<i64>().unwrap_or(0)
}

pub(crate) fn parse_json_value(value: &Value, fallback: Value) -> Value {
    let text = value_as_string(value);
    if text.trim().is_empty() {
        return fallback;
    }
    serde_json::from_str(&text).unwrap_or(fallback)
}

pub(crate) fn add_list_params(
    params: &mut DbParams,
    values: &[String],
    prefix: &str,
) -> Vec<String> {
    values
        .iter()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .enumerate()
        .map(|(index, value)| {
            let key = format!("@{prefix}_{index}");
            params.insert(key.clone(), Value::String(value));
            key
        })
        .collect()
}

pub(crate) fn query_param_string(params: &Value, key: &str) -> String {
    params
        .get(key)
        .map(value_as_string)
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub(crate) fn query_param_i64(params: &Value, key: &str, fallback: i64) -> i64 {
    params.get(key).map(value_as_i64).unwrap_or(fallback)
}

pub(crate) fn query_param_bool(params: &Value, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn query_param_string_array(params: &Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(value_as_string)
                .filter(|value| !value.trim().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn object_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object().and_then(|object| object.get(key))
}

pub(crate) fn object_field_string(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(field) = object_field(value, key) {
            return value_as_string(field);
        }
    }
    String::new()
}

pub(crate) fn object_field_optional_string(value: &Value, keys: &[&str]) -> Value {
    for key in keys {
        if let Some(field) = object_field(value, key) {
            return match field {
                Value::Null => Value::Null,
                Value::String(value) => Value::String(value.clone()),
                other => Value::String(other.to_string()),
            };
        }
    }
    Value::Null
}

pub(crate) fn object_field_bool(value: &Value, key: &str) -> bool {
    object_field(value, key)
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn is_json_value_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn object_field_json(value: &Value, key: &str, fallback: Value) -> String {
    object_field(value, key)
        .filter(|value| is_json_value_truthy(value))
        .cloned()
        .unwrap_or(fallback)
        .to_string()
}
