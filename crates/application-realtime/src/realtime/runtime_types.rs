use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
use vrcx_0_core::friends::FriendRecord;
pub use vrcx_0_core::realtime::{
    RealtimeSessionContext, RealtimeWsMessagePayload, RealtimeWsStatus, RealtimeWsStatusPayload,
};

use super::output::RealtimeFriendOutput;

pub(crate) const PENDING_OFFLINE_DELAY_MS: u64 = 170_000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RealtimeFriendSnapshot {
    pub current_user_id: String,
    pub endpoint: String,
    pub websocket: String,
    pub generation: u64,
    pub baseline_revision: u64,
    pub friends_by_id: HashMap<String, FriendRecord>,
}

#[derive(Debug, PartialEq)]
pub struct RealtimeFriendRosterSnapshot {
    pub current_user_id: String,
    pub endpoint: String,
    pub websocket: String,
    pub friend_count: usize,
    pub snapshot: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FriendBaselineResult {
    pub accepted: bool,
    pub generation: u64,
    pub baseline_revision: u64,
    pub friend_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FriendBaselineCausalWatermark {
    pub generation: Option<u64>,
    pub baseline_revision: Option<u64>,
    pub friend_state_sequence: u64,
    pub friend_log_sequence: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendBaselineSyncOutcome {
    pub result: FriendBaselineResult,
    pub snapshot: Option<RealtimeFriendSnapshot>,
    pub friend_log_changed: bool,
}

impl FriendBaselineSyncOutcome {
    pub(crate) fn rejected(result: FriendBaselineResult) -> Self {
        Self {
            result,
            snapshot: None,
            friend_log_changed: false,
        }
    }

    pub(crate) fn accepted(
        result: FriendBaselineResult,
        snapshot: RealtimeFriendSnapshot,
        friend_log_changed: bool,
    ) -> Self {
        Self {
            result,
            snapshot: Some(snapshot),
            friend_log_changed,
        }
    }

    pub(crate) fn into_result(self) -> FriendBaselineResult {
        self.result
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeTransportStartResult {
    pub generation: u64,
    pub client_run_id: u64,
    pub session_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealtimeTransportTermination {
    Stopped,
    AuthExpired {
        reason: String,
        status_code: Option<i32>,
    },
    UnexpectedExit {
        reason: String,
        connected_secs: Option<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealtimeTransportLifecycleEvent {
    Connected(RealtimeTransportStartResult),
    Finished {
        transport: RealtimeTransportStartResult,
        termination: RealtimeTransportTermination,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RealtimeCurrentUserGameLogContext {
    pub location: String,
    pub destination: String,
    pub world_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealtimeCurrentUserAuthority {
    Unavailable,
    Available {
        is_game_running: bool,
        game_log: Option<RealtimeCurrentUserGameLogContext>,
    },
}

impl Default for RealtimeCurrentUserAuthority {
    fn default() -> Self {
        Self::Available {
            is_game_running: false,
            game_log: None,
        }
    }
}

impl RealtimeCurrentUserAuthority {
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }

    pub const fn is_game_running(&self) -> bool {
        matches!(
            self,
            Self::Available {
                is_game_running: true,
                ..
            }
        )
    }

    pub fn game_log(&self) -> Option<&RealtimeCurrentUserGameLogContext> {
        match self {
            Self::Available { game_log, .. } => game_log.as_ref(),
            Self::Unavailable => None,
        }
    }

    pub fn with_game_running(mut self, value: bool) -> Self {
        if let Self::Available {
            is_game_running, ..
        } = &mut self
        {
            *is_game_running = value;
        }
        self
    }
}

pub enum RealtimeFriendApplyResult {
    Output(Box<RealtimeFriendOutput>),
    MissingBaseline,
    Ignored,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PendingOfflineTimerAction {
    #[default]
    None,
    Schedule {
        user_id: String,
        token: u64,
        delay_ms: u64,
    },
}
