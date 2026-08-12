use rusqlite::types::Value as SqlValue;

pub(super) fn json_to_sql(val: &serde_json::Value) -> SqlValue {
    match val {
        serde_json::Value::Null => SqlValue::Null,
        serde_json::Value::Bool(b) => SqlValue::Integer(if *b { 1 } else { 0 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

pub(super) fn sqlite_value_to_json(val: SqlValue) -> serde_json::Value {
    match val {
        SqlValue::Null => serde_json::Value::Null,
        SqlValue::Integer(i) => serde_json::json!(i),
        SqlValue::Real(f) => serde_json::json!(f),
        SqlValue::Text(s) => serde_json::json!(s),
        SqlValue::Blob(b) => serde_json::json!(base64_encode(&b)),
    }
}

fn base64_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 4 / 3 + 4);

    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        s.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        s.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            s.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            s.push('=');
        }
        if chunk.len() > 2 {
            s.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            s.push('=');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_to_sql_maps_null_to_sql_null() {
        assert_eq!(json_to_sql(&serde_json::json!(null)), SqlValue::Null);
    }

    #[test]
    fn json_to_sql_maps_bools_to_zero_or_one() {
        assert_eq!(json_to_sql(&serde_json::json!(true)), SqlValue::Integer(1));
        assert_eq!(json_to_sql(&serde_json::json!(false)), SqlValue::Integer(0));
    }

    #[test]
    fn json_to_sql_keeps_integers_as_integers_and_floats_as_reals() {
        assert_eq!(json_to_sql(&serde_json::json!(42)), SqlValue::Integer(42));
        assert_eq!(json_to_sql(&serde_json::json!(1.5)), SqlValue::Real(1.5));
    }

    #[test]
    fn json_to_sql_degrades_integers_beyond_i64_range_to_a_float_approximation() {
        let huge = serde_json::from_str::<serde_json::Value>("18446744073709551616").unwrap();
        assert_eq!(json_to_sql(&huge), SqlValue::Real(18446744073709551616.0));
    }

    #[test]
    fn json_to_sql_passes_strings_through_unquoted() {
        assert_eq!(
            json_to_sql(&serde_json::json!("usr_alice")),
            SqlValue::Text("usr_alice".into())
        );
    }

    #[test]
    fn json_to_sql_stringifies_arrays_and_objects_as_json_text() {
        assert_eq!(
            json_to_sql(&serde_json::json!([1, 2])),
            SqlValue::Text("[1,2]".into())
        );
        assert_eq!(
            json_to_sql(&serde_json::json!({"a": 1})),
            SqlValue::Text("{\"a\":1}".into())
        );
    }

    #[test]
    fn sqlite_value_to_json_round_trips_scalar_variants() {
        assert_eq!(
            sqlite_value_to_json(SqlValue::Null),
            serde_json::json!(null)
        );
        assert_eq!(
            sqlite_value_to_json(SqlValue::Integer(7)),
            serde_json::json!(7)
        );
        assert_eq!(
            sqlite_value_to_json(SqlValue::Real(2.5)),
            serde_json::json!(2.5)
        );
        assert_eq!(
            sqlite_value_to_json(SqlValue::Text("hi".into())),
            serde_json::json!("hi")
        );
    }

    #[test]
    fn sqlite_value_to_json_base64_encodes_blobs() {
        assert_eq!(
            sqlite_value_to_json(SqlValue::Blob(b"Man".to_vec())),
            serde_json::json!("TWFu")
        );
    }

    #[test]
    fn base64_encode_matches_known_vectors_across_padding_lengths() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Many"), "TWFueQ==");
    }
}
