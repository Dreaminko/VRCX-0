use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use vrcx_0_core::friends::{normalize_state_bucket, FriendRecord, FriendRosterBaseline};
use vrcx_0_core::realtime::RealtimeWsMessagePayload;
use vrcx_0_persistence::realtime::{FriendLogDelete, FriendLogUpsert};

use super::super::{
    FriendBaselineResult, FriendProjection, FriendProjectionPatch, PendingOfflineTimerAction,
    RealtimeFriendApplyResult, RealtimeFriendOutput, RealtimeFriendSnapshot,
};

#[path = "event_patch.rs"]
mod event_patch;
#[path = "persistence.rs"]
mod persistence;
#[path = "projection.rs"]
mod projection;
#[path = "state.rs"]
mod state;
#[path = "utils.rs"]
mod utils;

#[cfg(test)]
#[path = "../../../tests/realtime/friends/tests_baseline.rs"]
mod tests_baseline;
#[cfg(test)]
#[path = "../../../tests/realtime/friends/tests_feed.rs"]
mod tests_feed;
#[cfg(test)]
#[path = "../../../tests/realtime/friends/tests_location.rs"]
mod tests_location;
#[cfg(test)]
#[path = "../../../tests/realtime/friends/tests_presence.rs"]
mod tests_presence;
#[cfg(test)]
#[path = "../../../tests/realtime/friends/tests_profile.rs"]
mod tests_profile;

pub use event_patch::is_friend_event_type;
pub use state::RealtimeFriendsRuntime;
