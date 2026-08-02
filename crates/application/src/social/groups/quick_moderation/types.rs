use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum GroupQuickModerationAction {
    Kick,
    Ban,
}

impl GroupQuickModerationAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Ban => "ban",
        }
    }
}

impl std::fmt::Display for GroupQuickModerationAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuickModerationInput {
    #[serde(default)]
    pub current_user_id: String,
    #[serde(default)]
    pub target_user_id: String,
    #[serde(default)]
    pub endpoint: String,
}

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuickModerationActionInput {
    #[serde(default)]
    pub current_user_id: String,
    #[serde(default)]
    pub target_user_id: String,
    #[serde(default)]
    pub group_id: String,
    #[serde(default)]
    pub endpoint: String,
    pub action: GroupQuickModerationAction,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuickModerationGroup {
    pub group_id: String,
    pub name: String,
    pub short_code: String,
    pub icon_url: String,
    pub owner_id: String,
    pub membership_label: String,
    pub role_label: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuickModerationOutput {
    pub current_user_id: String,
    pub target_user_id: String,
    pub stale: bool,
    pub kick_groups: Vec<GroupQuickModerationGroup>,
    pub ban_groups: Vec<GroupQuickModerationGroup>,
    pub membership_error_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupQuickModerationActionOutput {
    pub group_id: String,
    pub target_user_id: String,
    pub action: GroupQuickModerationAction,
    pub status: i32,
}
