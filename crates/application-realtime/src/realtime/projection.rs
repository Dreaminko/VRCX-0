use serde::Serialize;
use vrcx_0_application_core::RuntimeEventPayload;
pub use vrcx_0_application_core::{
    FriendProjection, FriendProjectionPatch, FriendStateBucketAuthority,
    RealtimeCurrentUserProjection, RealtimeEntryCorrection, RealtimeEntryCorrectionFields,
    RealtimeEntryCorrectionStream, RealtimeInstanceClosedProjection, RealtimeInstanceQueueKind,
    RealtimeInstanceQueueProjection, RealtimeNotificationProjection, RealtimeNotificationUpsert,
    RealtimeUserProjection,
};
use vrcx_0_core::json::RawJson;

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeFeedUpsert {
    pub sequence: i64,
    pub entry: RawJson,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeFeedPatch {
    pub sequence: i64,
    pub id: String,
    pub fields: RealtimeEntryCorrectionFields,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeFeedProjection {
    pub generation: u64,
    pub owner_user_id: String,
    #[serde(default)]
    pub upserts: Vec<RealtimeFeedUpsert>,
    #[serde(default)]
    pub patches: Vec<RealtimeFeedPatch>,
}

impl RuntimeEventPayload for RealtimeFeedProjection {
    const EVENT_NAME: &'static str = "realtimeFeedProjection";
}
