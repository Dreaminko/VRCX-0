use crate::open_string_enum::open_string_enum;

open_string_enum! {
    pub enum ReleaseStatus {
        Public => "public",
        Private => "private",
        Hidden => "hidden",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ReleaseStatus;

    #[test]
    fn serde_maps_known_world_release_statuses() {
        for (value, expected) in [
            ("public", ReleaseStatus::Public),
            ("private", ReleaseStatus::Private),
            ("hidden", ReleaseStatus::Hidden),
        ] {
            let status: ReleaseStatus = serde_json::from_value(json!(value)).unwrap();

            assert_eq!(status, expected, "{value}");
            assert_eq!(serde_json::to_value(status).unwrap(), json!(value));
        }
    }

    #[test]
    fn serde_preserves_unknown_world_release_status() {
        let status: ReleaseStatus = serde_json::from_value(json!("future")).unwrap();

        assert_eq!(status, ReleaseStatus::Unknown("future".into()));
        assert_eq!(serde_json::to_value(status).unwrap(), json!("future"));
    }

    #[test]
    fn query_only_all_value_is_not_an_entity_status() {
        let status: ReleaseStatus = serde_json::from_value(json!("all")).unwrap();

        assert_eq!(status, ReleaseStatus::Unknown("all".into()));
    }
}
