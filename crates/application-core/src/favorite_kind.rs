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

    pub fn from_remote_type(value: &str) -> Option<Self> {
        VrchatFavoriteType::from_remote_type(value).map(Self::from)
    }

    pub fn matches_remote_type(self, value: &str) -> bool {
        Self::from_remote_type(value) == Some(self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum VrchatFavoriteType {
    Avatar,
    World,
    #[serde(rename = "vrcPlusWorld")]
    VrcPlusWorld,
    Friend,
}

impl VrchatFavoriteType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Avatar => "avatar",
            Self::World => "world",
            Self::VrcPlusWorld => "vrcPlusWorld",
            Self::Friend => "friend",
        }
    }

    pub fn from_remote_type(value: &str) -> Option<Self> {
        match value.trim() {
            "avatar" => Some(Self::Avatar),
            "world" => Some(Self::World),
            "vrcPlusWorld" => Some(Self::VrcPlusWorld),
            "friend" => Some(Self::Friend),
            _ => None,
        }
    }
}

impl From<FavoriteEntityKind> for VrchatFavoriteType {
    fn from(value: FavoriteEntityKind) -> Self {
        match value {
            FavoriteEntityKind::Avatar => Self::Avatar,
            FavoriteEntityKind::World => Self::World,
            FavoriteEntityKind::Friend => Self::Friend,
        }
    }
}

impl From<VrchatFavoriteType> for FavoriteEntityKind {
    fn from(value: VrchatFavoriteType) -> Self {
        match value {
            VrchatFavoriteType::Avatar => Self::Avatar,
            VrchatFavoriteType::World | VrchatFavoriteType::VrcPlusWorld => Self::World,
            VrchatFavoriteType::Friend => Self::Friend,
        }
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

impl From<VrchatFavoriteType> for FavoriteChangeScope {
    fn from(value: VrchatFavoriteType) -> Self {
        FavoriteEntityKind::from(value).into()
    }
}
