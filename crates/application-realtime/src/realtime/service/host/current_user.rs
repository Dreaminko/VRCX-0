use std::sync::Arc;

use serde_json::Value;
use tokio::sync::watch;
use vrcx_0_application_core::{Error, LocalGameContextSnapshot, Result};
use vrcx_0_vrchat_client::auth::current_user_get_input;
use vrcx_0_vrchat_client::http_api::ApiScope;

use crate::realtime::{
    PendingOfflineTimerAction, RealtimeCurrentUserAuthority, RealtimeCurrentUserGameLogContext,
    RealtimeCurrentUserOutput, RealtimeSessionContext,
};

use super::state::{ActiveRealtimeContext, CurrentUserRefreshStatus};
use super::RealtimeHostRuntime;

#[derive(Clone, Copy, Debug)]
pub struct RealtimeCurrentUserRefreshExpectation {
    generation: u64,
    session_generation: u64,
    sequence: u64,
}

impl RealtimeHostRuntime {
    pub(super) fn schedule_current_user_pending_offline(
        self: &Arc<Self>,
        generation: u64,
        timer_action: PendingOfflineTimerAction,
    ) {
        let PendingOfflineTimerAction::Schedule {
            token, delay_ms, ..
        } = timer_action
        else {
            return;
        };
        let runtime = Arc::clone(self);
        self.deps.tasks.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            let now = chrono::Utc::now().to_rfc3339();
            let Some(output) = runtime.current_user.fire_pending_offline(
                generation,
                token,
                now,
                runtime.current_user_authority(),
            ) else {
                return;
            };
            runtime.apply_current_user_output(output);
        });
    }

    pub fn sync_current_user_snapshot(
        &self,
        requested_session: RealtimeSessionContext,
        auth_scope_generation: u64,
        generation: Option<u64>,
        snapshot: serde_json::Value,
        overlay_patch: serde_json::Value,
    ) -> Result<bool> {
        let active = {
            let state = self
                .state
                .lock()
                .map_err(|error| Error::Custom(format!("realtime state lock: {error}")))?;
            let Some(active) = state.connection.active_context.clone() else {
                return Ok(false);
            };
            let auth_scope = self.deps.auth_scope.snapshot();
            if active.auth_scope_generation != auth_scope_generation
                || !auth_scope.active
                || auth_scope.generation != auth_scope_generation
                || auth_scope.current_user_id != requested_session.user_id
                || auth_scope.endpoint != requested_session.endpoint
                || active.session != requested_session
                || generation
                    .map(|generation| generation != active.generation)
                    .unwrap_or(false)
                || !self
                    .deps
                    .session
                    .is_realtime_generation_active(active.session_generation)
            {
                return Ok(false);
            }
            active
        };

        let Some(output) = self.current_user.apply_refreshed_snapshot(
            active.generation,
            snapshot,
            overlay_patch,
            self.current_user_authority(),
        ) else {
            return Ok(false);
        };
        self.apply_current_user_output(output);
        Ok(true)
    }

    pub(super) fn refresh_current_user_snapshot_after_update(
        self: &Arc<Self>,
        generation: u64,
        session: RealtimeSessionContext,
        overlay_patch: serde_json::Map<String, Value>,
    ) {
        let runtime = Arc::clone(self);
        self.deps.tasks.spawn(async move {
            let response = match runtime
                .deps
                .web
                .execute_api(
                    current_user_get_input(session.endpoint.clone()),
                    ApiScope::Vrchat,
                    &runtime.deps.db,
                )
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    tracing::warn!("Realtime current user refresh failed: {error}");
                    return;
                }
            };
            if !(200..300).contains(&response.status) {
                tracing::warn!(
                    status = response.status,
                    "Realtime current user refresh returned non-success"
                );
                return;
            }
            let snapshot = match serde_json::from_str::<Value>(&response.data) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    tracing::warn!("Realtime current user refresh json failed: {error}");
                    return;
                }
            };
            let Some(output) = runtime.current_user.apply_refreshed_snapshot(
                generation,
                snapshot,
                serde_json::Value::Object(overlay_patch),
                runtime.current_user_authority(),
            ) else {
                return;
            };
            runtime.apply_current_user_output(output);
        });
    }

    pub fn capture_current_user_refresh_expectation(
        &self,
    ) -> Option<RealtimeCurrentUserRefreshExpectation> {
        let active = self.active_current_user_context()?;
        let sequence = self.current_user.snapshot_sequence(active.generation)?;
        Some(RealtimeCurrentUserRefreshExpectation {
            generation: active.generation,
            session_generation: active.session_generation,
            sequence,
        })
    }

    pub fn apply_current_user_refreshed_snapshot_if_sequence(
        &self,
        expectation: RealtimeCurrentUserRefreshExpectation,
        snapshot: Value,
        response_authority_fields: &[&str],
    ) -> bool {
        if !self
            .active_current_user_context()
            .is_some_and(|active| self.current_user_context_matches(&active, &expectation))
        {
            return false;
        }
        let Some(output) = self.current_user.apply_refreshed_snapshot_if_sequence(
            expectation.generation,
            expectation.sequence,
            snapshot,
            Value::Null,
            response_authority_fields,
            self.current_user_authority(),
        ) else {
            return false;
        };
        self.apply_current_user_output(output);
        true
    }

    pub async fn refresh_current_user_now(self: &Arc<Self>, overlay_patch: Value) -> Result<bool> {
        enum RefreshFlight {
            Leader(watch::Sender<CurrentUserRefreshStatus>),
            Follower(watch::Receiver<CurrentUserRefreshStatus>),
        }

        let flight = {
            let mut slot = self
                .current_user_refresh_inflight
                .lock()
                .map_err(|error| Error::Custom(format!("current user refresh lock: {error}")))?;
            match slot.as_ref() {
                Some(rx) => RefreshFlight::Follower(rx.clone()),
                None => {
                    let (tx, rx) = watch::channel(None);
                    *slot = Some(rx);
                    RefreshFlight::Leader(tx)
                }
            }
        };
        match flight {
            RefreshFlight::Follower(mut rx) => loop {
                let settled = rx.borrow().clone();
                if let Some(result) = settled {
                    return result.map_err(Error::Custom);
                }
                if rx.changed().await.is_err() {
                    return Ok(false);
                }
            },
            RefreshFlight::Leader(tx) => {
                let result = self.refresh_current_user_once(overlay_patch).await;
                if let Ok(mut slot) = self.current_user_refresh_inflight.lock() {
                    *slot = None;
                }
                let _ = tx.send(Some(
                    result
                        .as_ref()
                        .map(|applied| *applied)
                        .map_err(|error| error.to_string()),
                ));
                result
            }
        }
    }

    async fn refresh_current_user_once(self: &Arc<Self>, overlay_patch: Value) -> Result<bool> {
        let Some(active) = self.active_current_user_context() else {
            return Ok(false);
        };
        let Some(sequence) = self.current_user.snapshot_sequence(active.generation) else {
            return Ok(false);
        };
        let response = self
            .deps
            .web
            .execute_api(
                current_user_get_input(active.session.endpoint.clone()),
                ApiScope::Vrchat,
                &self.deps.db,
            )
            .await?;
        if !(200..300).contains(&response.status) {
            return Err(Error::Custom(format!(
                "Current user refresh returned HTTP {}.",
                response.status
            )));
        }
        let snapshot = serde_json::from_str::<Value>(&response.data)
            .map_err(|error| Error::Custom(format!("current user refresh json: {error}")))?;
        let expectation = RealtimeCurrentUserRefreshExpectation {
            generation: active.generation,
            session_generation: active.session_generation,
            sequence,
        };
        if !self
            .active_current_user_context()
            .is_some_and(|current| self.current_user_context_matches(&current, &expectation))
        {
            return Ok(false);
        }
        let Some(output) = self.current_user.apply_refreshed_snapshot_if_sequence(
            expectation.generation,
            expectation.sequence,
            snapshot,
            overlay_patch,
            &[],
            self.current_user_authority(),
        ) else {
            return Ok(false);
        };
        self.apply_current_user_output(output);
        Ok(true)
    }

    pub(super) fn active_current_user_context(&self) -> Option<ActiveRealtimeContext> {
        let active = {
            let state = self.state.lock().ok()?;
            state.connection.active_context.clone()?
        };
        if !self
            .deps
            .session
            .is_realtime_generation_active(active.session_generation)
        {
            return None;
        }
        Some(active)
    }

    fn current_user_context_matches(
        &self,
        active: &ActiveRealtimeContext,
        expectation: &RealtimeCurrentUserRefreshExpectation,
    ) -> bool {
        active.generation == expectation.generation
            && active.session_generation == expectation.session_generation
    }

    pub(super) fn current_user_authority(&self) -> RealtimeCurrentUserAuthority {
        let local_game_context = self.deps.local_game_context.snapshot();
        match local_game_context {
            LocalGameContextSnapshot::Unavailable => RealtimeCurrentUserAuthority::Unavailable,
            LocalGameContextSnapshot::Available {
                is_game_running,
                location,
                destination,
                world_name,
                ..
            } => RealtimeCurrentUserAuthority::Available {
                is_game_running,
                game_log: Some(RealtimeCurrentUserGameLogContext {
                    location,
                    destination,
                    world_name,
                }),
            },
        }
    }

    pub(super) fn sync_current_user_game_running_state(
        &self,
        generation: u64,
        is_game_running: bool,
    ) {
        let Some(output) = self.current_user_game_running_output(generation, is_game_running)
        else {
            return;
        };
        self.apply_current_user_output(output);
    }

    pub(super) fn current_user_game_running_output(
        &self,
        generation: u64,
        is_game_running: bool,
    ) -> Option<RealtimeCurrentUserOutput> {
        let authority = self
            .current_user_authority()
            .with_game_running(is_game_running);
        self.current_user
            .apply_game_running_state(generation, authority)
    }

    pub(super) fn current_user_transport_finalization_output(
        &self,
        generation: u64,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.current_user
            .finalize_transport(generation, self.current_user_authority())
    }

    pub(super) fn current_user_transport_interruption_output(
        &self,
        generation: u64,
    ) -> Option<RealtimeCurrentUserOutput> {
        self.current_user
            .interrupt_transport(generation, self.current_user_authority())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::service::host::test_support::runtime_with_active_session;
    use serde_json::json;

    #[test]
    fn sync_accepts_a_snapshot_for_the_current_auth_scope_generation() -> Result<()> {
        let (_dir, test_runtime, session) =
            runtime_with_active_session("current-user-auth-scope")?;
        let runtime = test_runtime.runtime();
        let auth_scope_generation = test_runtime.auth_scope().snapshot().generation;
        let generation = runtime
            .state
            .lock()
            .unwrap()
            .connection
            .active_context
            .as_ref()
            .expect("active realtime context")
            .generation;
        runtime.current_user.set_snapshot(
            session.user_id.clone(),
            generation,
            json!({ "id": session.user_id, "displayName": "Current User" }),
        );

        let accepted = runtime.sync_current_user_snapshot(
            session.clone(),
            auth_scope_generation,
            None,
            json!({ "id": session.user_id, "displayName": "Updated User" }),
            Value::Null,
        )?;

        assert!(accepted);
        assert_eq!(
            runtime.current_user_snapshot().unwrap()["displayName"],
            "Updated User"
        );
        Ok(())
    }

    #[test]
    fn sync_rejects_a_snapshot_from_a_previous_auth_scope_generation() -> Result<()> {
        let (_dir, test_runtime, session) =
            runtime_with_active_session("stale-current-user-auth-scope")?;
        let runtime = test_runtime.runtime();
        let stale_auth_scope_generation = test_runtime.auth_scope().snapshot().generation;
        test_runtime.auth_scope().set("", "");
        let replacement_scope = test_runtime
            .auth_scope()
            .set(&session.user_id, &session.endpoint);
        {
            let mut state = runtime.state.lock().unwrap();
            let active = state
                .connection
                .active_context
                .as_mut()
                .expect("active realtime context");
            active.auth_scope_generation = replacement_scope.generation;
            runtime.current_user.set_snapshot(
                session.user_id.clone(),
                active.generation,
                json!({ "id": session.user_id, "displayName": "Current User" }),
            );
        }

        let accepted = runtime.sync_current_user_snapshot(
            session.clone(),
            stale_auth_scope_generation,
            None,
            json!({ "id": session.user_id, "displayName": "Stale User" }),
            Value::Null,
        )?;

        assert!(!accepted);
        assert_eq!(
            runtime.current_user_snapshot().unwrap()["displayName"],
            "Current User"
        );
        Ok(())
    }
}
