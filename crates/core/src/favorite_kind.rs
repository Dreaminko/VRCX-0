use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FavoriteEntityKind {
    Avatar,
    World,
    Friend,
}

impl FavoriteEntityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::World => "world",
            Self::Friend => "friend",
        }
    }

    pub const fn entity_id_prefix(self) -> &'static str {
        match self {
            Self::Avatar => "avtr_",
            Self::World => "wrld_",
            Self::Friend => "usr_",
        }
    }

    pub fn from_remote_type(value: &str) -> Option<Self> {
        match value.trim() {
            "avatar" => Some(Self::Avatar),
            "world" | "vrcPlusWorld" => Some(Self::World),
            "friend" => Some(Self::Friend),
            _ => None,
        }
    }

    pub fn matches_remote_type(self, value: &str) -> bool {
        Self::from_remote_type(value) == Some(self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FavoriteChangeScope {
    Avatar,
    World,
    Friend,
    #[serde(rename = "unknown")]
    All,
}

impl FavoriteChangeScope {
    pub fn from_remote_type(value: &str) -> Self {
        FavoriteEntityKind::from_remote_type(value)
            .map(Self::from)
            .unwrap_or(Self::All)
    }
}

impl From<FavoriteEntityKind> for FavoriteChangeScope {
    fn from(value: FavoriteEntityKind) -> Self {
        match value {
            FavoriteEntityKind::Avatar => Self::Avatar,
            FavoriteEntityKind::World => Self::World,
            FavoriteEntityKind::Friend => Self::Friend,
        }
    }
}
