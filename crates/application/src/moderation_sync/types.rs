use serde::{Deserialize, Serialize};
use vrcx_0_persistence::local_moderation::{LocalModerationOutput, RemoteModerationInput};
use vrcx_0_persistence::DatabaseService;

use vrcx_0_application_core::{HostSessionRuntime, RuntimeAuthScope, WebClient};

pub struct ModerationSyncDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub session: &'a HostSessionRuntime,
    pub auth_scope: &'a RuntimeAuthScope,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModerationSyncRefreshInput {
    pub user_id: String,
    #[serde(default)]
    pub endpoint: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModerationSyncMutationInput {
    #[serde(default)]
    pub(crate) owner_user_id: String,
    #[serde(default)]
    pub(crate) endpoint: String,
    pub(crate) target_user_id: String,
    #[serde(default)]
    pub(crate) target_display_name: String,
    pub(crate) r#type: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModerationSyncRefreshOutput {
    pub accepted: bool,
    pub user_id: String,
    pub remote_count: usize,
    pub local_count: usize,
    pub rows: Vec<RemoteModerationRow>,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteModerationRow {
    pub(crate) id: String,
    pub(crate) r#type: String,
    pub(crate) source_user_id: String,
    pub(crate) source_display_name: String,
    pub(crate) target_user_id: String,
    pub(crate) target_display_name: String,
    pub(crate) created: String,
}

impl RemoteModerationRow {
    pub(crate) fn to_local_input(&self) -> RemoteModerationInput {
        RemoteModerationInput {
            r#type: self.r#type.clone(),
            target_user_id: self.target_user_id.clone(),
            target_display_name: self.target_display_name.clone(),
            created: self.created.clone(),
        }
    }
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModerationSyncMutationOutput {
    pub target_user_id: String,
    pub r#type: String,
    pub enabled: bool,
    pub local: Option<LocalModerationOutput>,
}
