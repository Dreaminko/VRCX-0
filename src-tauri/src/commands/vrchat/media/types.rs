use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use vrcx_0_application_core::vrchat_api::media::MediaAssetKind;

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaParamsInput {
    #[serde(default)]
    pub(crate) params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaFileIdInput {
    #[serde(default)]
    pub(crate) file_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaImageUploadInput {
    #[serde(default)]
    pub(crate) image_data: String,
    #[serde(default)]
    pub(crate) params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaAvatarGalleryImageUploadInput {
    #[serde(default)]
    pub(crate) image_data: String,
    pub(crate) avatar_id: Value,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaPrintUploadInput {
    #[serde(default)]
    pub(crate) image_data: String,
    #[serde(default)]
    pub(crate) crop_white_border: bool,
    #[serde(default)]
    pub(crate) params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaAssetUploadInput {
    pub(crate) asset_kind: MediaAssetKind,
    #[serde(default)]
    pub(crate) image_data: String,
    #[serde(default)]
    pub(crate) crop_white_border: bool,
    #[serde(default)]
    pub(crate) params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaPrintsInput {
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) n: i64,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaPrintIdInput {
    #[serde(default)]
    pub(crate) print_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatPrintFavoriteSetInput {
    #[serde(default)]
    pub(crate) print_id: String,
    #[serde(default)]
    pub(crate) favorite: bool,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaUserInventoryItemInput {
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) inventory_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaInventoryItemInput {
    #[serde(default)]
    pub(crate) inventory_id: String,
    #[serde(default)]
    pub(crate) params: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaInventoryTemplateInput {
    #[serde(default)]
    pub(crate) inventory_template_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaProfileDecorationEquipInput {
    #[serde(default)]
    pub(crate) expected_user_id: String,
    #[serde(default)]
    pub(crate) inventory_id: String,
    #[serde(default)]
    pub(crate) equip_slot: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaProfileDecorationUnequipInput {
    #[serde(default)]
    pub(crate) expected_user_id: String,
    #[serde(default)]
    pub(crate) equip_slot: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaRewardRedeemInput {
    #[serde(default)]
    pub(crate) code: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatMediaLegacyImageUploadInput {
    #[serde(default)]
    pub(crate) entity_id: String,
    #[serde(default)]
    pub(crate) image_url: String,
    #[serde(default)]
    pub(crate) base64_file: String,
    #[serde(default)]
    pub(crate) file_size_in_bytes: Option<i64>,
}
