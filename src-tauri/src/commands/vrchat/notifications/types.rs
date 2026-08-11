use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatNotificationSendInput {
    #[serde(default)]
    pub(crate) receiver_user_id: String,
    #[serde(default)]
    pub(crate) params: Value,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatNotificationPhotoSendInput {
    #[serde(default)]
    pub(crate) receiver_user_id: String,
    #[serde(default)]
    pub(crate) params: Value,
    #[serde(default)]
    pub(crate) image_data: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatBoopInput {
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) emoji_id: String,
}
