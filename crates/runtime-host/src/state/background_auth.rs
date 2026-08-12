use crate::notification::{
    auth_webhook_should_recover, AuthWebhookEvent, AuthWebhookEventKind,
};

use super::{
    normalize_vrchat_api_endpoint, AtomicFlagGuard, AuthenticatedRuntimeSession,
    AuthenticatedSessionSnapshot, BackendRuntimeMode, BackendRuntimeSnapshot,
    BackendRuntimeTelemetryKind, NonInteractiveAuthError, RuntimeHostState,
};
use vrcx_0_core::time::now_iso;
use vrcx_0_application_core::RuntimeAuthScope;

#[derive(Clone, Debug)]
struct BackgroundAuthRecoveryContext {
    user_id: String,
    display_name: String,
    endpoint: String,
    reason: String,
    mode: BackendRuntimeMode,
    timestamp: String,
}

impl RuntimeHostState {
    pub async fn recover_background_auth_after_failure(
        &self,
        reason: impl Into<String>,
    ) -> BackendRuntimeSnapshot {
        let snapshot = self.backend_runtime.snapshot();
        if !auth_webhook_should_recover(&snapshot) {
            return snapshot;
        }
        let Some(_guard) = AtomicFlagGuard::try_acquire(&self.background_auth_recovery_running)
        else {
            return snapshot;
        };

        let snapshot = self.backend_runtime.snapshot();
        if !auth_webhook_should_recover(&snapshot) {
            return snapshot;
        }

        let Some(session) = self.authenticated_session_projection().session else {
            return snapshot;
        };
        let context = BackgroundAuthRecoveryContext::from_session(
            snapshot.mode,
            &session,
            reason.into(),
        );
        self.emit_backend_runtime_telemetry_snapshot(
            BackendRuntimeTelemetryKind::AuthRecoveryStarted,
            context.reason.clone(),
            snapshot,
        );
        invalidate_background_auth_scope(&self.runtime_context.auth_scope);
        self.clear_authenticated_session_projection();
        self.backend_runtime.set_authenticating();

        match self
            .authenticate_non_interactive_for_saved_user(&context.user_id, &context.endpoint)
            .await
        {
            Ok(session) => {
                if !context.matches_session(&session) {
                    let reason = "Recovered session does not match dropped background auth scope."
                        .to_string();
                    let snapshot = self.backend_runtime.set_auth_error(reason.clone());
                    self.emit_backend_runtime_telemetry_snapshot(
                        BackendRuntimeTelemetryKind::AuthRecoveryFailed,
                        reason.clone(),
                        snapshot.clone(),
                    );
                    self.send_background_auth_recovery_webhook(context.failed_event(reason));
                    return snapshot;
                }
                match self.start_authenticated_runtime_session(session) {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        let reason = error.to_string();
                        let snapshot = self.backend_runtime.set_auth_error(reason.clone());
                        self.emit_backend_runtime_telemetry_snapshot(
                            BackendRuntimeTelemetryKind::AuthRecoveryFailed,
                            reason.clone(),
                            snapshot.clone(),
                        );
                        self.send_background_auth_recovery_webhook(context.failed_event(reason));
                        snapshot
                    }
                }
            }
            Err(NonInteractiveAuthError::InteractionRequired(reason)) => {
                let snapshot = self
                    .backend_runtime
                    .set_auth_interaction_required(reason.clone());
                self.emit_backend_runtime_telemetry_snapshot(
                    BackendRuntimeTelemetryKind::AuthRecoveryFailed,
                    reason.clone(),
                    snapshot.clone(),
                );
                self.send_background_auth_recovery_webhook(context.failed_event(reason));
                snapshot
            }
            Err(NonInteractiveAuthError::SessionInvalidated {
                user_id, reason, ..
            }) => {
                let snapshot = self.clear_invalid_non_interactive_auth_session(&user_id, &reason);
                self.emit_backend_runtime_telemetry_snapshot(
                    BackendRuntimeTelemetryKind::AuthRecoveryFailed,
                    reason.clone(),
                    snapshot.clone(),
                );
                self.send_background_auth_recovery_webhook(context.failed_event(reason));
                snapshot
            }
            Err(NonInteractiveAuthError::Failed(reason)) => {
                let snapshot = self.backend_runtime.set_auth_error(reason.clone());
                self.emit_backend_runtime_telemetry_snapshot(
                    BackendRuntimeTelemetryKind::AuthRecoveryFailed,
                    reason.clone(),
                    snapshot.clone(),
                );
                self.send_background_auth_recovery_webhook(context.failed_event(reason));
                snapshot
            }
        }
    }

    fn send_background_auth_recovery_webhook(&self, event: AuthWebhookEvent) {
        self.runtime_context.enqueue_auth_webhook(event);
    }
}

fn invalidate_background_auth_scope(auth_scope: &RuntimeAuthScope) {
    auth_scope.set("", "");
}

impl BackgroundAuthRecoveryContext {
    fn from_session(
        mode: BackendRuntimeMode,
        session: &AuthenticatedSessionSnapshot,
        reason: String,
    ) -> Self {
        Self {
            user_id: session.user_id.trim().to_string(),
            display_name: session.display_name.trim().to_string(),
            endpoint: normalize_vrchat_api_endpoint(Some(&session.endpoint)),
            reason: normalize_recovery_reason(reason),
            mode,
            timestamp: now_iso(),
        }
    }

    fn matches_session(&self, session: &AuthenticatedRuntimeSession) -> bool {
        self.user_id == session.user_id.trim()
            && self.endpoint == normalize_vrchat_api_endpoint(Some(&session.endpoint))
    }

    fn failed_event(&self, reason: String) -> AuthWebhookEvent {
        AuthWebhookEvent {
            kind: AuthWebhookEventKind::ReloginFailed,
            user_id: self.user_id.clone(),
            display_name: self.display_name.clone(),
            reason: normalize_recovery_reason(reason),
            mode: self.mode,
            timestamp: self.timestamp.clone(),
        }
    }
}

fn normalize_recovery_reason(reason: String) -> String {
    let reason = reason.trim();
    if reason.is_empty() {
        "Background realtime auth failed.".into()
    } else {
        reason.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn background_auth_recovery_invalidates_the_dropped_login_generation() {
        let auth_scope = RuntimeAuthScope::new();
        let dropped = auth_scope.set("usr_before", "https://api.example.test/api/1");

        invalidate_background_auth_scope(&auth_scope);
        assert!(!auth_scope.snapshot().active);
        let recovered = auth_scope.set("usr_before", "https://api.example.test/api/1");

        assert!(recovered.generation > dropped.generation);
    }

    #[test]
    fn recovery_context_matches_only_dropped_user_and_endpoint() {
        let context = BackgroundAuthRecoveryContext::from_session(
            BackendRuntimeMode::Background,
            &projected_session("usr_before", "https://api.example.test/api/1/"),
            "auth failed".into(),
        );

        assert!(context.matches_session(&session("usr_before", "https://api.example.test/api/1")));
        assert!(!context.matches_session(&session("usr_after", "https://api.example.test/api/1")));
        assert!(!context.matches_session(&session("usr_before", "https://api.other.test/api/1")));
    }

    fn projected_session(user_id: &str, endpoint: &str) -> AuthenticatedSessionSnapshot {
        AuthenticatedSessionSnapshot {
            auth_scope_generation: 1,
            user_id: user_id.into(),
            display_name: "Pizza".into(),
            endpoint: endpoint.into(),
            websocket: "wss://pipeline.vrchat.cloud".into(),
            current_user_snapshot: json!({
                "id": user_id,
                "displayName": "Pizza"
            }),
        }
    }

    fn session(user_id: &str, endpoint: &str) -> AuthenticatedRuntimeSession {
        AuthenticatedRuntimeSession::from_user(
            json!({
                "id": user_id,
                "displayName": "Pizza"
            }),
            endpoint.into(),
            "wss://pipeline.vrchat.cloud".into(),
        )
    }
}
