use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::{
    string_field, AuthenticatedRuntimeSession, AuthenticatedSessionProjection,
    AuthenticatedSessionSnapshot, BackgroundCapabilitySession, RuntimeGroupInstancesProjection,
    RuntimeHostState,
};

fn establish_authenticated_session_projection(
    current: &AuthenticatedSessionProjection,
    session: &AuthenticatedRuntimeSession,
    auth_scope_generation: u64,
) -> AuthenticatedSessionProjection {
    AuthenticatedSessionProjection {
        revision: current.revision.saturating_add(1),
        session: Some(AuthenticatedSessionSnapshot {
            auth_scope_generation,
            user_id: session.user_id.clone(),
            display_name: session.display_name.clone(),
            endpoint: session.endpoint.clone(),
            websocket: session.websocket.clone(),
            current_user_snapshot: Arc::new(session.current_user.clone()),
        }),
    }
}

fn clear_authenticated_session_projection(
    current: &AuthenticatedSessionProjection,
) -> AuthenticatedSessionProjection {
    AuthenticatedSessionProjection {
        revision: current.revision.saturating_add(1),
        session: None,
    }
}

impl RuntimeHostState {
    pub fn authenticated_session_projection(&self) -> AuthenticatedSessionProjection {
        self.authenticated_session_projection
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn clear_authenticated_session_projection(&self) {
        let cleared = {
            let mut current = self
                .authenticated_session_projection
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            current.session.clone().map(|previous| {
                *current = clear_authenticated_session_projection(&current);
                (previous, current.clone())
            })
        };
        if let Some((_, projection)) = &cleared {
            self.runtime_context.event_bus.emit(projection.clone());
        }
        self.authenticated_runtime.stop();
        self.runtime_context
            .overlay_activity()
            .clear_runtime_state();
        if let Some(extension) = &self.profile_extension {
            extension.clear_profile_session();
        }
        self.runtime_context.session.clear_realtime_context();
        if let Some((previous, _)) = cleared {
            self.runtime_context
                .event_bus
                .emit(RuntimeGroupInstancesProjection::cleared_session(
                    previous.user_id,
                    previous.endpoint,
                ));
        }
    }

    pub(super) fn establish_authenticated_session_projection(
        &self,
        session: &AuthenticatedRuntimeSession,
        auth_scope_generation: u64,
    ) {
        let published = {
            let mut current = self
                .authenticated_session_projection
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let next = establish_authenticated_session_projection(
                &current,
                session,
                auth_scope_generation,
            );
            let scope_changed = current
                .session
                .as_ref()
                .zip(next.session.as_ref())
                .map(|current| {
                    current.0.user_id != current.1.user_id
                        || current.0.endpoint != current.1.endpoint
                        || current.0.websocket != current.1.websocket
                })
                .unwrap_or(true);
            if scope_changed {
                self.runtime_context
                    .overlay_activity()
                    .clear_runtime_state();
                if let Some(extension) = &self.profile_extension {
                    extension.profile_session_scope_changed();
                }
            }
            *current = next;
            current.clone()
        };
        self.runtime_context.event_bus.emit(published);
    }
}

pub(super) fn replace_authenticated_session_user_if_session_matches(
    session_slot: &Arc<Mutex<AuthenticatedSessionProjection>>,
    expected: &BackgroundCapabilitySession,
    snapshot: Value,
) -> bool {
    let mut slot = session_slot
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if !session_slot_matches(Some(&slot), expected) {
        return false;
    }
    let Some(session) = slot.session.as_mut() else {
        return false;
    };
    session.current_user_snapshot = Arc::new(snapshot);
    if let Some(display_name) = string_field(&session.current_user_snapshot, "displayName")
        .or_else(|| string_field(&session.current_user_snapshot, "username"))
    {
        session.display_name = display_name;
    }
    slot.revision = slot.revision.saturating_add(1);
    true
}

pub(super) fn session_slot_matches(
    slot: Option<&AuthenticatedSessionProjection>,
    expected: &BackgroundCapabilitySession,
) -> bool {
    slot.and_then(|projection| projection.session.as_ref())
        .map(|current| {
            current.auth_scope_generation == expected.auth_scope_generation
                && current.user_id == expected.current_user_id
                && current.endpoint == expected.endpoint
                && current.websocket == expected.websocket
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod authenticated_session_projection_tests {
    use super::*;
    use serde_json::json;

    fn session() -> AuthenticatedRuntimeSession {
        AuthenticatedRuntimeSession::from_user(
            json!({
                "id": "usr_owner",
                "displayName": "Projected User",
                "username": "projected_user",
            }),
            "https://api.example.test/api/1/".into(),
            "wss://pipeline.example.test/".into(),
        )
    }

    #[test]
    fn establishing_projection_preserves_the_authenticated_session_contract() {
        let projection = establish_authenticated_session_projection(
            &AuthenticatedSessionProjection::default(),
            &session(),
            7,
        );

        assert_eq!(projection.revision, 1);
        let projected_session = projection.session.expect("authenticated session");
        assert_eq!(projected_session.auth_scope_generation, 7);
        assert_eq!(projected_session.user_id, "usr_owner");
        assert_eq!(projected_session.display_name, "Projected User");
        assert_eq!(projected_session.endpoint, "https://api.example.test/api/1");
        assert_eq!(projected_session.websocket, "wss://pipeline.example.test");
        assert_eq!(projected_session.current_user_snapshot["id"], "usr_owner");
    }

    #[test]
    fn cloning_a_projection_shares_the_current_user_snapshot() {
        let projection = establish_authenticated_session_projection(
            &AuthenticatedSessionProjection::default(),
            &session(),
            7,
        );
        let cloned = projection.clone();
        let first = projection.session.as_ref().unwrap();
        let second = cloned.session.as_ref().unwrap();

        assert!(Arc::ptr_eq(
            &first.current_user_snapshot,
            &second.current_user_snapshot,
        ));
        assert_eq!(
            serde_json::to_value(cloned).unwrap()["session"]["currentUserSnapshot"]["id"],
            "usr_owner"
        );
    }

    #[test]
    fn clearing_projection_advances_revision_and_removes_the_session() {
        let established = establish_authenticated_session_projection(
            &AuthenticatedSessionProjection::default(),
            &session(),
            7,
        );

        let cleared = clear_authenticated_session_projection(&established);

        assert_eq!(cleared.revision, 2);
        assert!(cleared.session.is_none());
    }

    #[test]
    fn current_user_replacement_advances_the_projection() {
        let established = establish_authenticated_session_projection(
            &AuthenticatedSessionProjection::default(),
            &session(),
            7,
        );
        let slot = Arc::new(Mutex::new(established));
        let expected = BackgroundCapabilitySession {
            auth_scope_generation: 7,
            current_user_id: "usr_owner".into(),
            endpoint: "https://api.example.test/api/1".into(),
            websocket: "wss://pipeline.example.test".into(),
            current_user_snapshot: Arc::new(Value::Null),
        };
        assert!(replace_authenticated_session_user_if_session_matches(
            &slot,
            &expected,
            json!({ "id": "usr_owner", "displayName": "Updated User" }),
        ));

        let projection = slot.lock().unwrap().clone();
        assert_eq!(projection.revision, 2);
        assert_eq!(projection.session.unwrap().display_name, "Updated User");
    }

    #[test]
    fn current_user_replacement_rejects_a_previous_login_generation() {
        let established = establish_authenticated_session_projection(
            &AuthenticatedSessionProjection::default(),
            &session(),
            8,
        );
        let slot = Arc::new(Mutex::new(established));
        let expected = BackgroundCapabilitySession {
            auth_scope_generation: 7,
            current_user_id: "usr_owner".into(),
            endpoint: "https://api.example.test/api/1".into(),
            websocket: "wss://pipeline.example.test".into(),
            current_user_snapshot: Arc::new(Value::Null),
        };
        assert!(!replace_authenticated_session_user_if_session_matches(
            &slot,
            &expected,
            json!({ "id": "usr_owner", "displayName": "Stale User" }),
        ));
        assert_eq!(slot.lock().unwrap().revision, 1);
    }
}
