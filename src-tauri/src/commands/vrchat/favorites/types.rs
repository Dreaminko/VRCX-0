use serde::Deserialize;
use vrcx_0_application_core::{FavoriteEntityKind, VrchatFavoriteType};

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteWorldsInput {
    #[serde(default)]
    pub(crate) n: i64,
    #[serde(default)]
    pub(crate) offset: i64,
    #[serde(default)]
    pub(crate) owner_id: String,
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) tag: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteGroupsInput {
    #[serde(default)]
    pub(crate) n: i64,
    #[serde(default)]
    pub(crate) offset: i64,
    #[serde(default)]
    pub(crate) owner_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteAddInput {
    #[serde(rename = "type")]
    pub(crate) type_name: VrchatFavoriteType,
    #[serde(default)]
    pub(crate) favorite_id: String,
    #[serde(default)]
    pub(crate) tags: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteDeleteInput {
    #[serde(default)]
    pub(crate) object_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteGroupSaveInput {
    #[serde(default, rename = "type")]
    pub(crate) type_name: String,
    #[serde(default)]
    pub(crate) group: String,
    pub(crate) display_name: Option<String>,
    pub(crate) visibility: Option<String>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFavoriteGroupClearInput {
    #[serde(default, rename = "type")]
    pub(crate) type_name: String,
    #[serde(default)]
    pub(crate) group: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalFavoriteInput {
    pub(crate) kind: FavoriteEntityKind,
    #[serde(default)]
    pub(crate) entity_id: String,
    #[serde(default)]
    pub(crate) group_name: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalFavoriteGroupInput {
    pub(crate) kind: FavoriteEntityKind,
    #[serde(default)]
    pub(crate) group_name: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalFavoriteGroupRenameInput {
    pub(crate) kind: FavoriteEntityKind,
    #[serde(default)]
    pub(crate) group_name: String,
    #[serde(default)]
    pub(crate) new_group_name: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vrcx_0_application_core::VrchatFavoriteType;

    use super::VrchatFavoriteAddInput;

    #[test]
    fn favorite_add_accepts_vrc_plus_world_from_ipc() {
        let input: VrchatFavoriteAddInput = serde_json::from_value(json!({
            "type": "vrcPlusWorld",
            "favoriteId": "wrld_1",
            "tags": "worlds4",
        }))
        .unwrap();

        assert_eq!(input.type_name, VrchatFavoriteType::VrcPlusWorld);
    }
}
