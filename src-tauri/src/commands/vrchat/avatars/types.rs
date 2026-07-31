use serde::{Deserialize, Serialize};
use serde_json::Value;
use vrcx_0_application_core::vrchat_api::VrchatApiResponse;

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarSelectionOutcome {
    pub applied: bool,
    pub response: VrchatApiResponse,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarIdInput {
    #[serde(default)]
    pub(crate) avatar_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarListByUserInput {
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) user: String,
    #[serde(default)]
    pub(crate) n: i64,
    #[serde(default)]
    pub(crate) offset: i64,
    #[serde(default)]
    pub(crate) sort: String,
    #[serde(default)]
    pub(crate) order: String,
    #[serde(default)]
    pub(crate) release_status: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarFileInput {
    #[serde(default)]
    pub(crate) file_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarSaveInput {
    #[serde(default)]
    pub(crate) avatar_id: String,
    pub(crate) params: Option<Value>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatAvatarModerationInput {
    #[serde(default)]
    pub(crate) avatar_id: String,
    #[serde(default, rename = "type")]
    pub(crate) type_name: String,
}
