use std::sync::{Arc, Mutex};

use serde::Serialize;
use vrcx_0_vrchat_client::http_api::normalize_vrchat_api_endpoint;

use crate::ports::HostSessionRuntime;

#[derive(Clone, Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAuthScopeSnapshot {
    pub current_user_id: String,
    pub endpoint: String,
    pub generation: u64,
    pub active: bool,
}

impl RuntimeAuthScopeSnapshot {
    pub fn generation_matches(&self, expected: &Self) -> bool {
        self.active
            && self.generation == expected.generation
            && self.current_user_id == expected.current_user_id
            && self.endpoint == expected.endpoint
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeAuthScope {
    state: Arc<Mutex<RuntimeAuthScopeSnapshot>>,
}

impl RuntimeAuthScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &self,
        user_id: impl AsRef<str>,
        endpoint: impl AsRef<str>,
    ) -> RuntimeAuthScopeSnapshot {
        let mut state = self.lock_state();
        let current_user_id = normalize_text(user_id);
        let endpoint = normalize_endpoint(endpoint);
        let active = !current_user_id.is_empty();
        if state.current_user_id == current_user_id
            && state.endpoint == endpoint
            && state.active == active
        {
            return state.clone();
        }
        state.generation = state.generation.saturating_add(1);
        state.current_user_id = current_user_id;
        state.endpoint = endpoint;
        state.active = active;
        state.clone()
    }

    pub fn snapshot(&self) -> RuntimeAuthScopeSnapshot {
        self.lock_state().clone()
    }

    pub fn matches(&self, user_id: &str, endpoint: &str) -> bool {
        let state = self.lock_state();
        state.active
            && state.current_user_id == user_id.trim()
            && state.endpoint == normalize_endpoint(endpoint)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RuntimeAuthScopeSnapshot> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

pub fn auth_scope_matches(
    auth_scope: &RuntimeAuthScope,
    session: &HostSessionRuntime,
    user_id: &str,
    endpoint: &str,
) -> bool {
    if auth_scope.snapshot().active {
        return auth_scope.matches(user_id, endpoint);
    }

    let Some(context) = session.snapshot().realtime_context else {
        return true;
    };
    context.current_user_id == user_id
        && normalize_endpoint(&context.endpoint) == normalize_endpoint(endpoint)
}

fn normalize_text(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

fn normalize_endpoint(value: impl AsRef<str>) -> String {
    normalize_vrchat_api_endpoint(Some(value.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::{auth_scope_matches, RuntimeAuthScope};
    use crate::ports::{HostRealtimeSessionContext, HostSessionRuntime};

    #[test]
    fn tracks_active_auth_scope() {
        let scope = RuntimeAuthScope::new();
        assert!(!scope.snapshot().active);

        let snapshot = scope.set(" usr_current ", "https://api.example.test/api/1/");
        assert!(snapshot.active);
        assert_eq!(snapshot.current_user_id, "usr_current");
        assert_eq!(snapshot.endpoint, "https://api.example.test/api/1");
        assert!(scope.matches("usr_current", "https://api.example.test/api/1"));
        assert!(scope.matches("usr_current", "https://api.example.test/api/1/"));
        assert!(!scope.matches("usr_other", "https://api.example.test/api/1"));

        let unchanged = scope.set(" usr_current ", "https://api.example.test/api/1/");
        assert_eq!(unchanged.generation, snapshot.generation);

        let default_endpoint = scope.set("usr_current", "");
        assert_eq!(default_endpoint.endpoint, "https://api.vrchat.cloud/api/1");
        assert!(scope.matches("usr_current", ""));

        let cleared = scope.set("", "");
        assert!(!cleared.active);
        assert!(!scope.matches("usr_current", "https://api.example.test/api/1"));
    }

    #[test]
    fn bumps_generation_when_switching_to_a_different_user() {
        let scope = RuntimeAuthScope::new();

        let first = scope.set("usr_a", "");
        let switched = scope.set("usr_b", "");

        assert_eq!(switched.current_user_id, "usr_b");
        assert!(switched.generation > first.generation);
    }

    #[test]
    fn falls_back_to_realtime_context_when_scope_is_inactive() {
        let scope = RuntimeAuthScope::new();
        let session = HostSessionRuntime::new();

        assert!(auth_scope_matches(
            &scope,
            &session,
            "usr_current",
            "https://api.vrchat.cloud/api/1"
        ));

        session.set_realtime_context(HostRealtimeSessionContext::new(
            "usr_current".into(),
            String::new(),
            String::new(),
        ));

        assert!(auth_scope_matches(
            &scope,
            &session,
            "usr_current",
            "https://api.vrchat.cloud/api/1"
        ));
        assert!(!auth_scope_matches(
            &scope,
            &session,
            "usr_other",
            "https://api.vrchat.cloud/api/1"
        ));
    }

    #[test]
    fn realtime_context_fallback_normalizes_both_endpoints() {
        let scope = RuntimeAuthScope::new();
        let session = HostSessionRuntime::new();
        session.set_realtime_context(HostRealtimeSessionContext::new(
            "usr_current".into(),
            "https://api.vrchat.cloud/api/1".into(),
            String::new(),
        ));

        for endpoint in [
            "https://api.vrchat.cloud/api/1",
            "https://api.vrchat.cloud/api/1/",
            "  https://api.vrchat.cloud/api/1  ",
            "",
        ] {
            assert!(
                auth_scope_matches(&scope, &session, "usr_current", endpoint),
                "endpoint {endpoint:?} should match the realtime context"
            );
        }

        assert!(!auth_scope_matches(
            &scope,
            &session,
            "usr_current",
            "https://api.example.test/api/1"
        ));
    }

    #[test]
    fn active_scope_wins_over_realtime_context() {
        let scope = RuntimeAuthScope::new();
        let session = HostSessionRuntime::new();
        session.set_realtime_context(HostRealtimeSessionContext::new(
            "usr_stale".into(),
            "https://api.example.test/api/1".into(),
            String::new(),
        ));
        scope.set("usr_current", "https://api.example.test/api/1");

        assert!(auth_scope_matches(
            &scope,
            &session,
            "usr_current",
            "https://api.example.test/api/1"
        ));
        assert!(!auth_scope_matches(
            &scope,
            &session,
            "usr_stale",
            "https://api.example.test/api/1"
        ));
    }

    #[test]
    fn bumps_generation_and_deactivates_when_cleared() {
        let scope = RuntimeAuthScope::new();

        let active = scope.set("usr_a", "");
        let cleared = scope.set("", "");

        assert!(active.active);
        assert!(!cleared.active);
        assert!(cleared.generation > active.generation);
    }
}
