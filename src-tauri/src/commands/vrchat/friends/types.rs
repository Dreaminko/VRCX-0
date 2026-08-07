use serde::Deserialize;

#[derive(Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VrchatFriendUserInput {
    #[serde(default)]
    pub(crate) user_id: String,
}
