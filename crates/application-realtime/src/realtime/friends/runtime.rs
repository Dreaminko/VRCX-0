#[cfg(test)]
use serde_json::{json, Value};
#[cfg(test)]
use vrcx_0_core::friends::{FriendRecord, FriendRosterBaseline};
#[cfg(test)]
use vrcx_0_core::realtime::RealtimeWsMessagePayload;

#[cfg(test)]
use super::super::{
    FriendStateBucketAuthority, PendingOfflineTimerAction, RealtimeFriendApplyResult,
    RealtimeFriendOutput,
};

mod event_patch;
mod persistence;
mod state;
mod utils;

#[cfg(test)]
mod baseline_tests;
#[cfg(test)]
mod event_field_ownership_tests;
#[cfg(test)]
mod feed_tests;
#[cfg(test)]
mod location_embedded_user_tests;
#[cfg(test)]
mod location_feed_tests;
#[cfg(test)]
mod location_offline_tests;
#[cfg(test)]
mod location_state_tests;
#[cfg(test)]
mod presence_tests;
#[cfg(test)]
mod profile_tests;
#[cfg(test)]
mod ws_trace_replay_test;

pub use event_patch::is_friend_event_type;
pub(crate) use persistence::{player_joining_feed_entry, trust_level_feed_entry};
pub use state::RealtimeFriendsRuntime;
pub(crate) use state::{PendingOfflineSchedule, SyntheticFriendEvent};
