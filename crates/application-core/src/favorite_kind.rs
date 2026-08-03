use serde::{Deserialize, Serialize};

pub use vrcx_0_core::{FavoriteChangeScope, FavoriteEntityKind};

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

impl From<VrchatFavoriteType> for FavoriteChangeScope {
    fn from(value: VrchatFavoriteType) -> Self {
        FavoriteEntityKind::from(value).into()
    }
}
