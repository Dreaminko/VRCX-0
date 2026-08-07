use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiAvatarSearchInput {
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) vrcx_id: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiYoutubeVideoInput {
    #[serde(default)]
    pub(crate) video_id: String,
    #[serde(default)]
    pub(crate) api_key: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiUrlInput {
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiImageInput {
    #[serde(default)]
    pub(crate) url: String,
}
