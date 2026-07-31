use vrcx_0_persistence::realtime::RealtimePersistenceBatch;

use super::projection::{
    FriendProjection, RealtimeCurrentUserProjection, RealtimeInstanceClosedProjection,
    RealtimeNotificationProjection,
};
use super::runtime_types::PendingOfflineTimerAction;

#[derive(Clone, Debug, PartialEq)]
pub struct RealtimeFriendOutput {
    pub owner_user_id: String,
    pub projection: FriendProjection,
    pub persistence: RealtimePersistenceBatch,
    pub timer_action: PendingOfflineTimerAction,
    pub profile_refetch_user_ids: Vec<String>,
    pub friend_note_changed: bool,
}

impl RealtimeFriendOutput {
    pub(crate) fn new(owner_user_id: String, generation: u64, baseline_revision: u64) -> Self {
        Self::from_projection(
            owner_user_id,
            FriendProjection::new(generation, baseline_revision),
        )
    }

    pub(crate) fn from_projection(owner_user_id: String, projection: FriendProjection) -> Self {
        Self {
            owner_user_id,
            projection,
            persistence: RealtimePersistenceBatch::default(),
            timer_action: PendingOfflineTimerAction::None,
            profile_refetch_user_ids: Vec::new(),
            friend_note_changed: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RealtimeNotificationOutput {
    pub owner_user_id: String,
    pub projection: RealtimeNotificationProjection,
    pub persistence: RealtimePersistenceBatch,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RealtimeCurrentUserOutput {
    pub owner_user_id: String,
    pub projection: RealtimeCurrentUserProjection,
    pub persistence: RealtimePersistenceBatch,
    pub timer_action: PendingOfflineTimerAction,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RealtimeInstanceClosedOutput {
    pub projection: RealtimeInstanceClosedProjection,
    pub persistence: RealtimePersistenceBatch,
}
