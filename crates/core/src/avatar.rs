use crate::open_string_enum::open_string_enum;

open_string_enum! {
    pub enum PerformanceRating {
        Excellent => "Excellent",
        Good => "Good",
        Medium => "Medium",
        None => "None",
        Poor => "Poor",
        VeryPoor => "VeryPoor",
    }
}

pub fn avatar_name_from_file_name(file_name: &str) -> Option<String> {
    let lower = file_name.to_ascii_lowercase();
    let start = lower.find("avatar - ")? + "avatar - ".len();
    let end = lower.rfind(" - image -")?;
    if end < start {
        return None;
    }
    let name = file_name[start..end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{avatar_name_from_file_name, PerformanceRating};

    #[test]
    fn avatar_name_from_file_name_extracts_name() {
        let raw = "Avatar - Name - Image - 2022․3․22f1_1_standalonewindows_Release";

        assert_eq!(avatar_name_from_file_name(raw).as_deref(), Some("Name"));
        assert_eq!(avatar_name_from_file_name("just a name"), None);
    }

    #[test]
    fn performance_rating_maps_known_values_and_preserves_unknown_values() {
        for (value, expected) in [
            ("Excellent", PerformanceRating::Excellent),
            ("Good", PerformanceRating::Good),
            ("Medium", PerformanceRating::Medium),
            ("None", PerformanceRating::None),
            ("Poor", PerformanceRating::Poor),
            ("VeryPoor", PerformanceRating::VeryPoor),
        ] {
            let rating: PerformanceRating = serde_json::from_value(json!(value)).unwrap();

            assert_eq!(rating, expected, "{value}");
            assert_eq!(serde_json::to_value(rating).unwrap(), json!(value));
        }

        let rating: PerformanceRating = serde_json::from_value(json!("future")).unwrap();
        assert_eq!(rating, PerformanceRating::Unknown("future".into()));
        assert_eq!(serde_json::to_value(rating).unwrap(), json!("future"));
    }
}
