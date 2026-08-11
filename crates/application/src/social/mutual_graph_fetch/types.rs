use serde::{Deserialize, Serialize};
use vrcx_0_persistence::DatabaseService;

use crate::{RuntimeAuthScope, WebClient};

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MutualGraphFetchStartInput {
    pub owner_user_id: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub friend_ids: Vec<String>,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MutualGraphFetchCancelInput {
    #[serde(default)]
    pub owner_user_id: String,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MutualGraphFetchStatus {
    pub run_id: u64,
    pub revision: u64,
    pub status: MutualGraphFetchState,
    pub owner_user_id: String,
    pub total_friends: usize,
    pub processed_friends: usize,
    pub current_friend_id: String,
    pub fetched_friends: usize,
    pub opted_out_friends: usize,
    pub failed_friends: usize,
    pub cancel_requested: bool,
    pub started_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MutualGraphFetchState {
    Idle,
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Error,
}

impl MutualGraphFetchState {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }
}

#[derive(Clone, Copy)]
pub struct MutualGraphRequestDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub auth_scope: &'a RuntimeAuthScope,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MutualGraphFriendRefreshInput {
    pub owner_user_id: String,
    pub friend_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MutualGraphFriendRefreshStatus {
    Refreshed,
    OptedOut,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MutualGraphFriendRefreshOutput {
    pub status: MutualGraphFriendRefreshStatus,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserMutualFriendsListInput {
    pub user_id: String,
}
