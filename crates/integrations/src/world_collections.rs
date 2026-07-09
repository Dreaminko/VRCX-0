use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const WORLD_COLLECTIONS_SITE_ORIGIN: &str = "https://worlds.vrcx-0.dev";
pub const WORLD_COLLECTIONS_API_ENDPOINT: &str = "https://worlds.vrcx-0.dev/api/collections";
const WORLD_COLLECTIONS_UPLOAD_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldCollectionCreatePayload {
    pub schema: i64,
    pub owner_key: String,
    pub title: String,
    pub listed: bool,
    pub access: String,
    pub updated_at: i64,
    pub worlds: Vec<WorldCollectionPayloadWorld>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorldCollectionPayloadWorld {
    pub world_id: String,
    pub author_id: String,
    pub name: String,
    pub author_name: String,
    pub created_at: String,
    pub image_url: String,
    pub description: String,
    pub release_status: String,
    pub thumbnail_image_url: String,
    pub comment: String,
    pub updated_at: String,
    pub version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct WorldCollectionCreateResponse {
    pub id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WorldCollectionShareError {
    #[error("{0}")]
    Custom(String),
}

pub async fn create_world_collection(
    payload: &WorldCollectionCreatePayload,
) -> Result<WorldCollectionCreateResponse, WorldCollectionShareError> {
    let client = reqwest::Client::builder()
        .timeout(WORLD_COLLECTIONS_UPLOAD_TIMEOUT)
        .build()
        .map_err(|error| {
            WorldCollectionShareError::Custom(format!(
                "share collection upload client failed: {error}"
            ))
        })?;
    let response = client
        .post(WORLD_COLLECTIONS_API_ENDPOINT)
        .json(payload)
        .send()
        .await
        .map_err(|error| {
            WorldCollectionShareError::Custom(format!("share collection upload failed: {error}"))
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let detail = body.trim();
        let message = if detail.is_empty() {
            format!("share collection upload returned HTTP {status}")
        } else {
            format!("share collection upload returned HTTP {status}: {detail}")
        };
        return Err(WorldCollectionShareError::Custom(message));
    }
    response.json().await.map_err(|error| {
        WorldCollectionShareError::Custom(format!("share collection response is invalid: {error}"))
    })
}
