use std::sync::{Arc, Mutex};

use vrcx_0_application_core::{
    BackendRuntime, BackendRuntimePhase, BackendRuntimeStatusPublisher,
    BackendRuntimeTelemetryKind, BackgroundCapabilitySession, BackgroundCapabilitySessionIdentity,
    RuntimeBackgroundJobs,
};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_runtime_host::{AuthenticatedSessionProjection, RuntimeHostContext};

mod discord;
mod presence;

pub(in crate::state) use discord::run_background_discord_tick;
pub(in crate::state) use presence::run_background_presence_tick;

pub(in crate::state) const BACKGROUND_PRESENCE_AUTOMATION_JOB: &str =
    "backgroundPresenceAutomation";
pub(in crate::state) const BACKGROUND_DISCORD_PRESENCE_JOB: &str = "backgroundDiscordPresence";
pub(in crate::state) const BACKGROUND_PRESENCE_CADENCE_SECONDS: u64 = 3;
pub(in crate::state) const BACKGROUND_DISCORD_CADENCE_SECONDS: u64 = 3;

pub(in crate::state) struct BackgroundTickContext<'a> {
    pub(in crate::state) db: &'a Arc<vrcx_0_persistence::DatabaseService>,
    pub(in crate::state) web: &'a Arc<vrcx_0_application_core::WebClient>,
    pub(in crate::state) session_slot: &'a Arc<Mutex<AuthenticatedSessionProjection>>,
    pub(in crate::state) realtime_runtime: &'a Arc<RealtimeHostRuntime>,
    pub(in crate::state) runtime_context: &'a Arc<RuntimeHostContext>,
    pub(in crate::state) desktop_services: &'a Arc<crate::DesktopRuntimeServices>,
    pub(in crate::state) backend_runtime: &'a BackendRuntime,
    pub(in crate::state) background_jobs: &'a RuntimeBackgroundJobs,
}

pub(in crate::state) fn background_capability_session(
    session_slot: &Arc<Mutex<AuthenticatedSessionProjection>>,
) -> Option<BackgroundCapabilitySession> {
    let slot = session_slot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    slot.session
        .as_ref()
        .map(|session| BackgroundCapabilitySession {
            auth_scope_generation: session.auth_scope_generation,
            current_user_id: session.user_id.clone(),
            endpoint: session.endpoint.clone(),
            websocket: session.websocket.clone(),
            current_user_snapshot: session.current_user_snapshot.clone(),
        })
}

pub(in crate::state) fn background_capability_session_identity(
    session_slot: &Arc<Mutex<AuthenticatedSessionProjection>>,
) -> Option<BackgroundCapabilitySessionIdentity> {
    let slot = session_slot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    slot.session
        .as_ref()
        .map(|session| BackgroundCapabilitySessionIdentity {
            auth_scope_generation: session.auth_scope_generation,
            current_user_id: session.user_id.clone(),
            endpoint: session.endpoint.clone(),
            websocket: session.websocket.clone(),
        })
}

pub(in crate::state) fn background_capability_session_matches(
    session_slot: &Arc<Mutex<AuthenticatedSessionProjection>>,
    expected: &BackgroundCapabilitySessionIdentity,
) -> bool {
    let slot = session_slot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    slot.session
        .as_ref()
        .map(|current| {
            current.auth_scope_generation == expected.auth_scope_generation
                && current.user_id == expected.current_user_id
                && current.endpoint == expected.endpoint
                && current.websocket == expected.websocket
        })
        .unwrap_or(false)
}

pub(in crate::state) fn emit_background_info(
    runtime_context: &Arc<RuntimeHostContext>,
    backend_runtime: &BackendRuntime,
    detail: impl Into<String>,
) {
    emit_background_output(
        runtime_context,
        backend_runtime,
        BackendRuntimeTelemetryKind::BackgroundInfo,
        detail,
    );
}

pub(in crate::state) fn emit_background_error(
    runtime_context: &Arc<RuntimeHostContext>,
    backend_runtime: &BackendRuntime,
    detail: impl Into<String>,
) {
    emit_background_output(
        runtime_context,
        backend_runtime,
        BackendRuntimeTelemetryKind::BackgroundError,
        detail,
    );
}

pub(in crate::state) fn emit_background_warning(
    runtime_context: &Arc<RuntimeHostContext>,
    backend_runtime: &BackendRuntime,
    detail: impl Into<String>,
) {
    emit_background_output(
        runtime_context,
        backend_runtime,
        BackendRuntimeTelemetryKind::BackgroundWarning,
        detail,
    );
}

pub(in crate::state) fn emit_background_info_if_changed(
    runtime_context: &Arc<RuntimeHostContext>,
    backend_runtime: &BackendRuntime,
    last_detail: &mut Option<String>,
    detail: impl Into<String>,
) {
    let detail = detail.into();
    if !remember_background_output_if_changed(last_detail, &detail) {
        return;
    }
    emit_background_info(runtime_context, backend_runtime, detail);
}

pub(in crate::state) fn remember_background_output_if_changed(
    last_detail: &mut Option<String>,
    detail: &str,
) -> bool {
    if last_detail.as_deref() == Some(detail) {
        return false;
    }
    *last_detail = Some(detail.into());
    true
}

fn emit_background_output(
    runtime_context: &Arc<RuntimeHostContext>,
    backend_runtime: &BackendRuntime,
    kind: BackendRuntimeTelemetryKind,
    detail: impl Into<String>,
) {
    let snapshot = backend_runtime.snapshot();
    if snapshot.phase != BackendRuntimePhase::Running {
        return;
    }
    BackendRuntimeStatusPublisher::new(backend_runtime.clone(), runtime_context.event_bus.clone())
        .publish_telemetry(kind, detail, snapshot);
}
