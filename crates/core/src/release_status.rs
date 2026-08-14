use compact_str::CompactString;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(from = "CompactString")]
pub enum WorldReleaseStatus {
    Public,
    Private,
    Hidden,
    Unknown(CompactString),
}

impl WorldReleaseStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
            Self::Hidden => "hidden",
            Self::Unknown(value) => value,
        }
    }

    fn known(value: &str) -> Option<Self> {
        match value {
            "public" => Some(Self::Public),
            "private" => Some(Self::Private),
            "hidden" => Some(Self::Hidden),
            _ => None,
        }
    }
}

impl Default for WorldReleaseStatus {
    fn default() -> Self {
        Self::Unknown(CompactString::new(""))
    }
}

impl Serialize for WorldReleaseStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl From<&str> for WorldReleaseStatus {
    fn from(value: &str) -> Self {
        Self::known(value).unwrap_or_else(|| Self::Unknown(value.into()))
    }
}

impl From<String> for WorldReleaseStatus {
    fn from(value: String) -> Self {
        Self::known(&value).unwrap_or_else(|| Self::Unknown(value.into()))
    }
}

impl From<CompactString> for WorldReleaseStatus {
    fn from(value: CompactString) -> Self {
        Self::known(&value).unwrap_or(Self::Unknown(value))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::WorldReleaseStatus;

    #[test]
    fn serde_maps_known_world_release_statuses() {
        for (value, expected) in [
            ("public", WorldReleaseStatus::Public),
            ("private", WorldReleaseStatus::Private),
            ("hidden", WorldReleaseStatus::Hidden),
        ] {
            let status: WorldReleaseStatus = serde_json::from_value(json!(value)).unwrap();

            assert_eq!(status, expected, "{value}");
            assert_eq!(serde_json::to_value(status).unwrap(), json!(value));
        }
    }

    #[test]
    fn serde_preserves_unknown_world_release_status() {
        let status: WorldReleaseStatus = serde_json::from_value(json!("future")).unwrap();

        assert_eq!(status, WorldReleaseStatus::Unknown("future".into()));
        assert_eq!(serde_json::to_value(status).unwrap(), json!("future"));
    }

    #[test]
    fn query_only_all_value_is_not_an_entity_status() {
        let status: WorldReleaseStatus = serde_json::from_value(json!("all")).unwrap();

        assert_eq!(status, WorldReleaseStatus::Unknown("all".into()));
    }
}
