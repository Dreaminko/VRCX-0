use serde_json::Value;
use vrcx_0_core::realtime::{RealtimeWsStatus, RealtimeWsStatusPayload};

use crate::{
    BackendRuntimeProcessStatus, BackendRuntimeTelemetry, BackendRuntimeTelemetryKind,
    RuntimeEventPayload,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeOutputMode {
    Background,
    Headless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeOutputLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeOutputLine {
    pub level: RuntimeOutputLevel,
    pub message: String,
    pub fatal_reason: Option<String>,
}

pub fn format_runtime_output_event(
    mode: RuntimeOutputMode,
    event: &str,
    payload: &Value,
) -> Option<RuntimeOutputLine> {
    if event == RealtimeWsStatusPayload::EVENT_NAME {
        return serde_json::from_value::<RealtimeWsStatusPayload>(payload.clone())
            .ok()
            .and_then(|status| format_realtime_ws_status(mode, &status));
    }
    if event == BackendRuntimeTelemetry::EVENT_NAME {
        return serde_json::from_value::<BackendRuntimeTelemetry>(payload.clone())
            .ok()
            .and_then(|telemetry| format_backend_runtime_telemetry(mode, &telemetry));
    }
    None
}

fn format_realtime_ws_status(
    mode: RuntimeOutputMode,
    payload: &RealtimeWsStatusPayload,
) -> Option<RuntimeOutputLine> {
    let status = payload.status.as_str();
    let reason = payload.reason.as_deref().unwrap_or_default().trim();
    let detail = if reason.is_empty() {
        format!("ws status: {status}")
    } else {
        format!("ws status: {status} ({reason})")
    };
    let is_auth_failure = payload.status == RealtimeWsStatus::AuthFailure;
    Some(RuntimeOutputLine {
        level: if is_auth_failure {
            RuntimeOutputLevel::Error
        } else {
            RuntimeOutputLevel::Info
        },
        message: with_mode_prefix(mode, detail),
        fatal_reason: is_auth_failure.then(|| {
            if reason.is_empty() {
                "websocket auth failure".into()
            } else {
                reason.to_string()
            }
        }),
    })
}

fn format_backend_runtime_telemetry(
    mode: RuntimeOutputMode,
    payload: &BackendRuntimeTelemetry,
) -> Option<RuntimeOutputLine> {
    let detail = payload.detail.as_str();
    let snapshot = &payload.snapshot;
    match payload.kind {
        BackendRuntimeTelemetryKind::AuthSuccess => info(
            mode,
            format!(
                "login success: {} ({})",
                empty_fallback(&snapshot.auth_display_name, "unknown user"),
                empty_fallback(&snapshot.auth_user_id, "unknown id")
            ),
        ),
        BackendRuntimeTelemetryKind::WsStatus => None,
        BackendRuntimeTelemetryKind::ProcessStatus => match snapshot.process_status {
            BackendRuntimeProcessStatus::VrchatRunning => info(mode, "vrchat started"),
            BackendRuntimeProcessStatus::VrchatStopped => info(mode, "vrchat stopped"),
            BackendRuntimeProcessStatus::Unknown => {
                info(mode, format!("vrchat process status: {detail}"))
            }
        },
        BackendRuntimeTelemetryKind::GameLogPersisted => {
            let total = snapshot.game_log_persisted_count;
            info(
                mode,
                format!("gamelog persisted to db: count={detail}, total={total}"),
            )
        }
        BackendRuntimeTelemetryKind::GameLogWatcher => {
            info(mode, format!("gamelog watcher: {detail}"))
        }
        BackendRuntimeTelemetryKind::RuntimeStopped => Some(RuntimeOutputLine {
            level: RuntimeOutputLevel::Info,
            message: match mode {
                RuntimeOutputMode::Background => format!("background mode exited: {detail}"),
                RuntimeOutputMode::Headless => format!("headless runtime exited: {detail}"),
            },
            fatal_reason: None,
        }),
        BackendRuntimeTelemetryKind::BackgroundInfo => info(mode, detail),
        BackendRuntimeTelemetryKind::BackgroundWarning => {
            output(mode, RuntimeOutputLevel::Warn, detail)
        }
        BackendRuntimeTelemetryKind::BackgroundError => {
            output(mode, RuntimeOutputLevel::Error, detail)
        }
        BackendRuntimeTelemetryKind::RuntimeStarted
        | BackendRuntimeTelemetryKind::ModeChanged
        | BackendRuntimeTelemetryKind::AuthCleared
        | BackendRuntimeTelemetryKind::AuthRecoveryStarted
        | BackendRuntimeTelemetryKind::AuthRecoveryFailed => None,
    }
}

fn info(mode: RuntimeOutputMode, message: impl Into<String>) -> Option<RuntimeOutputLine> {
    output(mode, RuntimeOutputLevel::Info, message)
}

fn output(
    mode: RuntimeOutputMode,
    level: RuntimeOutputLevel,
    message: impl Into<String>,
) -> Option<RuntimeOutputLine> {
    Some(RuntimeOutputLine {
        level,
        message: with_mode_prefix(mode, message.into()),
        fatal_reason: None,
    })
}

fn with_mode_prefix(mode: RuntimeOutputMode, message: impl Into<String>) -> String {
    let message = message.into();
    match mode {
        RuntimeOutputMode::Background => format!("background mode {message}"),
        RuntimeOutputMode::Headless => message,
    }
}

fn empty_fallback<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn telemetry(
        kind: BackendRuntimeTelemetryKind,
        detail: impl Into<String>,
    ) -> BackendRuntimeTelemetry {
        BackendRuntimeTelemetry {
            kind,
            detail: detail.into(),
            snapshot: crate::BackendRuntime::new(crate::RuntimeHostProfile::Desktop).snapshot(),
        }
    }

    fn formatted<T: RuntimeEventPayload>(
        mode: RuntimeOutputMode,
        payload: &T,
    ) -> RuntimeOutputLine {
        format_runtime_output_event(mode, T::EVENT_NAME, &serde_json::to_value(payload).unwrap())
            .unwrap()
    }

    #[test]
    fn formats_shared_runtime_info_for_background_and_headless() {
        let mut payload = telemetry(BackendRuntimeTelemetryKind::AuthSuccess, "Example");
        payload.snapshot.auth_display_name = "Example".into();
        payload.snapshot.auth_user_id = "usr_test".into();

        let background = formatted(RuntimeOutputMode::Background, &payload);
        assert_eq!(background.level, RuntimeOutputLevel::Info);
        assert_eq!(
            background.message,
            "background mode login success: Example (usr_test)"
        );

        let headless = formatted(RuntimeOutputMode::Headless, &payload);
        assert_eq!(headless.message, "login success: Example (usr_test)");
    }

    #[test]
    fn formats_background_error_as_error_output() {
        let payload = telemetry(
            BackendRuntimeTelemetryKind::BackgroundError,
            "Discord SetAssets failed: pipe closed.",
        );

        let output = formatted(RuntimeOutputMode::Background, &payload);
        assert_eq!(output.level, RuntimeOutputLevel::Error);
        assert_eq!(
            output.message,
            "background mode Discord SetAssets failed: pipe closed."
        );
        assert_eq!(output.fatal_reason, None);
    }

    #[test]
    fn formats_background_warning_as_warning_output() {
        let payload = telemetry(
            BackendRuntimeTelemetryKind::BackgroundWarning,
            "Discord SetAssets failed: pipe closed.",
        );

        let output = formatted(RuntimeOutputMode::Background, &payload);
        assert_eq!(output.level, RuntimeOutputLevel::Warn);
        assert_eq!(
            output.message,
            "background mode Discord SetAssets failed: pipe closed."
        );
        assert_eq!(output.fatal_reason, None);
    }

    #[test]
    fn websocket_auth_failure_is_error_and_fatal() {
        let payload = RealtimeWsStatusPayload {
            status: RealtimeWsStatus::AuthFailure,
            websocket_domain: String::new(),
            at: String::new(),
            client_run_id: None,
            generation: None,
            session_generation: None,
            reason: Some("token expired".into()),
            status_code: None,
        };

        let output = formatted(RuntimeOutputMode::Headless, &payload);
        assert_eq!(output.level, RuntimeOutputLevel::Error);
        assert_eq!(output.message, "ws status: authFailure (token expired)");
        assert_eq!(output.fatal_reason.as_deref(), Some("token expired"));
    }
}
