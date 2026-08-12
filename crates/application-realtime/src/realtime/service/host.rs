use serde_json::Value;

use vrcx_0_persistence::realtime::{lookup_game_log_world_name, RealtimePersistenceBatch};

use crate::realtime::{
    RealtimeCurrentUserOutput, RealtimeEntryCorrectionStream, RealtimeInstanceQueueProjection,
    RealtimeNotificationOutput, RealtimeNotificationProjection, RealtimeNotificationUpsert,
};
#[cfg(test)]
use crate::social_baseline::service::friend_log_relationship_candidates;

#[cfg(test)]
use vrcx_0_application_core::Result;
#[cfg(test)]
use vrcx_0_core::realtime::RealtimeWsMessagePayload;
#[cfg(test)]
use vrcx_0_persistence::config as config_store;
#[cfg(test)]
use vrcx_0_persistence::realtime::write_realtime_batch;

#[cfg(test)]
use crate::realtime::connection::RealtimeMessageSink;
#[cfg(test)]
use crate::realtime::{
    PendingOfflineTimerAction, RealtimeFriendApplyResult, RealtimeFriendOutput,
    RealtimeTransportStartResult, RealtimeTransportTermination,
};
#[cfg(test)]
use crate::social_baseline::service::{reconcile_friend_roster_records, FriendStatusVerdicts};

mod automation;
mod baseline;
mod connection;
mod current_user;
mod enrichment;
mod fanout;
mod feed;
#[cfg(test)]
mod friend_baseline_tests;
#[cfg(test)]
mod friend_joining_tests;
mod friend_mutation;
mod friend_profile;
mod friend_profile_bulk_load;
#[cfg(test)]
mod friend_profile_bulk_load_tests;
mod friend_queue;
mod game_process;
mod message_dispatch;
#[cfg(test)]
mod notification_enrichment_tests;
mod state;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_support;
#[cfg(test)]
mod transport_lifecycle_tests;
mod world_cache;
#[cfg(test)]
mod world_cache_tests;

pub use current_user::RealtimeCurrentUserRefreshExpectation;
pub use friend_mutation::SyntheticFriendEventOutcome;
pub use friend_profile_bulk_load::{FriendProfileBulkLoadStatus, FriendProfileLoadStatusPayload};
pub use state::{
    RealtimeCurrentUserSnapshotSink, RealtimeHostRuntime, RealtimeHostRuntimeDeps,
    RealtimeStopRequest,
};
