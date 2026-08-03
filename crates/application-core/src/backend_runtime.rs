use std::any::Any;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use vrcx_0_core::realtime::{RealtimeWsStatus, RealtimeWsStatusPayload};
use vrcx_0_core::time::now_iso;

use crate::event_bus::{
    BackendRuntimeCountKind, BackendRuntimeCountTelemetry, BackendRuntimeMessageTelemetry,
};
use crate::events::FriendProfileLoadStatusPayload;
use crate::ports::HostSessionProjection;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimeMode {
    #[default]
    Foreground,
    Background,
    Headless,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimePhase {
    #[default]
    Idle,
    Starting,
    Authenticating,
    Running,
    Stopping,
    Error,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimeAuthStatus {
    #[default]
    Unknown,
    Authenticating,
    Authenticated,
    InteractionRequired,
    Error,
    SignedOut,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimeGameLogStatus {
    #[default]
    Idle,
    Running,
    Persisted,
    Unavailable,
}

impl BackendRuntimeGameLogStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Persisted => "persisted",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimeProcessStatus {
    #[default]
    Unknown,
    VrchatRunning,
    VrchatStopped,
}

impl BackendRuntimeProcessStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::VrchatRunning => "vrchatRunning",
            Self::VrchatStopped => "vrchatStopped",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BackendRuntimeTelemetryKind {
    WsStatus,
    ProcessStatus,
    WsMessage,
    WsPersisted,
    GameLogPersisted,
    RuntimeStarted,
    RuntimeStopped,
    ModeChanged,
    AuthCleared,
    AuthSuccess,
    AuthRecoveryStarted,
    AuthRecoveryFailed,
    GameLogWatcher,
    BackgroundInfo,
    BackgroundWarning,
    BackgroundError,
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackendRuntimeSnapshot {
    pub mode: BackendRuntimeMode,
    pub phase: BackendRuntimePhase,
    pub auth_status: BackendRuntimeAuthStatus,
    pub auth_user_id: String,
    pub auth_display_name: String,
    pub ws_status: RealtimeWsStatus,
    pub game_log_status: BackendRuntimeGameLogStatus,
    pub process_status: BackendRuntimeProcessStatus,
    pub ws_message_counts: BTreeMap<String, u64>,
    pub ws_persisted_count: u64,
    pub game_log_persisted_count: u64,
    pub last_error: Option<String>,
    pub updated_at: String,
    pub friend_profile_load: FriendProfileLoadStatusPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackendRuntimeTelemetry {
    pub kind: BackendRuntimeTelemetryKind,
    pub detail: String,
    pub snapshot: BackendRuntimeSnapshot,
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeProjectionSync {
    pub snapshot: BackendRuntimeSnapshot,
}

#[derive(Clone, Debug)]
struct BackendRuntimeState {
    mode: BackendRuntimeMode,
    phase: BackendRuntimePhase,
    auth_status: BackendRuntimeAuthStatus,
    auth_user_id: String,
    auth_display_name: String,
    ws_status: RealtimeWsStatus,
    game_log_status: BackendRuntimeGameLogStatus,
    process_status: BackendRuntimeProcessStatus,
    ws_message_counts: BTreeMap<String, u64>,
    ws_persisted_count: u64,
    game_log_persisted_count: u64,
    last_error: Option<String>,
    updated_at: String,
    friend_profile_load: FriendProfileLoadStatusPayload,
}

impl Default for BackendRuntimeState {
    fn default() -> Self {
        Self {
            mode: BackendRuntimeMode::Foreground,
            phase: BackendRuntimePhase::Idle,
            auth_status: BackendRuntimeAuthStatus::Unknown,
            auth_user_id: String::new(),
            auth_display_name: String::new(),
            ws_status: RealtimeWsStatus::Idle,
            game_log_status: BackendRuntimeGameLogStatus::Idle,
            process_status: BackendRuntimeProcessStatus::Unknown,
            ws_message_counts: BTreeMap::new(),
            ws_persisted_count: 0,
            game_log_persisted_count: 0,
            last_error: None,
            updated_at: now_iso(),
            friend_profile_load: FriendProfileLoadStatusPayload::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct BackendRuntime {
    state: Arc<Mutex<BackendRuntimeState>>,
}

impl BackendRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_mode(&self, mode: BackendRuntimeMode) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.mode = mode;
        })
    }

    pub fn set_phase(&self, phase: BackendRuntimePhase) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.phase = phase;
            if phase != BackendRuntimePhase::Error {
                state.last_error = None;
            }
        })
    }

    pub fn set_error(&self, message: impl Into<String>) -> BackendRuntimeSnapshot {
        let message = message.into();
        self.update(|state| {
            state.phase = BackendRuntimePhase::Error;
            state.last_error = Some(message);
        })
    }

    pub fn set_authenticating(&self) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.phase = BackendRuntimePhase::Authenticating;
            state.auth_status = BackendRuntimeAuthStatus::Authenticating;
            state.last_error = None;
        })
    }

    pub fn set_auth_success(
        &self,
        user_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.auth_status = BackendRuntimeAuthStatus::Authenticated;
            state.auth_user_id = user_id.into();
            state.auth_display_name = display_name.into();
            state.last_error = None;
        })
    }

    pub fn set_auth_interaction_required(
        &self,
        reason: impl Into<String>,
    ) -> BackendRuntimeSnapshot {
        let reason = reason.into();
        self.update(|state| {
            state.phase = BackendRuntimePhase::Error;
            state.auth_status = BackendRuntimeAuthStatus::InteractionRequired;
            state.last_error = Some(reason);
        })
    }

    pub fn set_auth_error(&self, reason: impl Into<String>) -> BackendRuntimeSnapshot {
        let reason = reason.into();
        self.update(|state| {
            state.phase = BackendRuntimePhase::Error;
            state.auth_status = BackendRuntimeAuthStatus::Error;
            state.last_error = Some(reason);
        })
    }

    pub fn clear_authentication(&self) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.phase = BackendRuntimePhase::Idle;
            state.auth_status = BackendRuntimeAuthStatus::SignedOut;
            state.auth_user_id.clear();
            state.auth_display_name.clear();
            state.ws_status = RealtimeWsStatus::Idle;
            state.last_error = None;
        })
    }

    pub fn set_ws_status(&self, status: RealtimeWsStatus) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.ws_status = status;
        })
    }

    pub fn record_ws_message(&self, message_type: impl Into<String>) -> BackendRuntimeSnapshot {
        let message_type = message_type.into();
        self.update(|state| {
            *state.ws_message_counts.entry(message_type).or_insert(0) += 1;
        })
    }

    pub fn add_ws_persisted(&self, count: u64) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.ws_persisted_count = state.ws_persisted_count.saturating_add(count);
        })
    }

    pub fn set_game_log_status(
        &self,
        status: BackendRuntimeGameLogStatus,
    ) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.game_log_status = status;
        })
    }

    pub fn add_game_log_persisted(&self, count: u64) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.game_log_status = BackendRuntimeGameLogStatus::Persisted;
            state.game_log_persisted_count = state.game_log_persisted_count.saturating_add(count);
        })
    }

    pub fn set_process_status(
        &self,
        status: BackendRuntimeProcessStatus,
    ) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.process_status = status;
        })
    }

    pub fn set_friend_profile_load_state(
        &self,
        payload: FriendProfileLoadStatusPayload,
    ) -> BackendRuntimeSnapshot {
        self.update(|state| {
            state.friend_profile_load = payload;
        })
    }

    pub fn observe_runtime_event(&self, payload: &dyn Any) -> Option<BackendRuntimeTelemetry> {
        if let Some(status) = payload.downcast_ref::<RealtimeWsStatusPayload>() {
            let snapshot = self.set_ws_status(status.status);
            return Some(BackendRuntimeTelemetry {
                kind: BackendRuntimeTelemetryKind::WsStatus,
                detail: status.status.as_str().into(),
                snapshot,
            });
        }
        if let Some(status) = payload.downcast_ref::<FriendProfileLoadStatusPayload>() {
            self.set_friend_profile_load_state(status.clone());
            return None;
        }
        if let Some(projection) = payload.downcast_ref::<HostSessionProjection>() {
            let status = if projection.is_game_running {
                BackendRuntimeProcessStatus::VrchatRunning
            } else {
                BackendRuntimeProcessStatus::VrchatStopped
            };
            let snapshot = self.set_process_status(status);
            return Some(BackendRuntimeTelemetry {
                kind: BackendRuntimeTelemetryKind::ProcessStatus,
                detail: status.as_str().into(),
                snapshot,
            });
        }
        if let Some(telemetry) = payload.downcast_ref::<BackendRuntimeMessageTelemetry>() {
            let message_type = telemetry.message_type.clone();
            let snapshot = self.record_ws_message(message_type.clone());
            return Some(BackendRuntimeTelemetry {
                kind: BackendRuntimeTelemetryKind::WsMessage,
                detail: message_type,
                snapshot,
            });
        }
        if let Some(telemetry) = payload.downcast_ref::<BackendRuntimeCountTelemetry>() {
            let (kind, snapshot) = match telemetry.kind {
                BackendRuntimeCountKind::WsPersisted => (
                    BackendRuntimeTelemetryKind::WsPersisted,
                    self.add_ws_persisted(telemetry.count),
                ),
                BackendRuntimeCountKind::GameLogPersisted => (
                    BackendRuntimeTelemetryKind::GameLogPersisted,
                    self.add_game_log_persisted(telemetry.count),
                ),
            };
            return Some(BackendRuntimeTelemetry {
                kind,
                detail: telemetry.count.to_string(),
                snapshot,
            });
        }
        None
    }

    pub fn snapshot(&self) -> BackendRuntimeSnapshot {
        self.state_to_snapshot(&self.lock_state())
    }

    fn update(&self, update: impl FnOnce(&mut BackendRuntimeState)) -> BackendRuntimeSnapshot {
        let mut state = self.lock_state();
        update(&mut state);
        state.updated_at = now_iso();
        self.state_to_snapshot(&state)
    }

    fn state_to_snapshot(&self, state: &BackendRuntimeState) -> BackendRuntimeSnapshot {
        BackendRuntimeSnapshot {
            mode: state.mode,
            phase: state.phase,
            auth_status: state.auth_status,
            auth_user_id: state.auth_user_id.clone(),
            auth_display_name: state.auth_display_name.clone(),
            ws_status: state.ws_status,
            game_log_status: state.game_log_status,
            process_status: state.process_status,
            ws_message_counts: state.ws_message_counts.clone(),
            ws_persisted_count: state.ws_persisted_count,
            game_log_persisted_count: state.game_log_persisted_count,
            last_error: state.last_error.clone(),
            updated_at: state.updated_at.clone(),
            friend_profile_load: state.friend_profile_load.clone(),
        }
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, BackendRuntimeState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}
