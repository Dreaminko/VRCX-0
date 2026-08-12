use std::sync::{Arc, Mutex};

use serde::Serialize;
use vrcx_0_vrchat_client::http_api::normalize_vrchat_api_endpoint;

#[derive(Clone, Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAuthScopeSnapshot {
    pub current_user_id: String,
    pub endpoint: String,
    pub generation: u64,
    pub active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeAuthIdentity {
    pub user_id: String,
    pub display_name: String,
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
    state: Arc<Mutex<RuntimeAuthScopeState>>,
}

#[derive(Clone, Debug, Default)]
struct RuntimeAuthScopeState {
    snapshot: RuntimeAuthScopeSnapshot,
    display_name: String,
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
        self.set_inner(user_id.as_ref(), None, endpoint.as_ref())
    }

    pub fn set_identity(
        &self,
        user_id: impl AsRef<str>,
        display_name: impl AsRef<str>,
        endpoint: impl AsRef<str>,
    ) -> RuntimeAuthScopeSnapshot {
        self.set_inner(
            user_id.as_ref(),
            Some(display_name.as_ref()),
            endpoint.as_ref(),
        )
    }

    fn set_inner(
        &self,
        user_id: &str,
        display_name: Option<&str>,
        endpoint: &str,
    ) -> RuntimeAuthScopeSnapshot {
        let mut state = self.lock_state();
        let current_user_id = normalize_text(user_id);
        let endpoint = normalize_endpoint(endpoint);
        let active = !current_user_id.is_empty();
        if state.snapshot.current_user_id == current_user_id
            && state.snapshot.endpoint == endpoint
            && state.snapshot.active == active
        {
            if let Some(display_name) = display_name {
                state.display_name = normalize_display_name(display_name, &current_user_id);
            }
            return state.snapshot.clone();
        }
        state.snapshot.generation = state.snapshot.generation.saturating_add(1);
        state.snapshot.current_user_id = current_user_id;
        state.snapshot.endpoint = endpoint;
        state.snapshot.active = active;
        state.display_name = if active {
            normalize_display_name(
                display_name.unwrap_or_default(),
                &state.snapshot.current_user_id,
            )
        } else {
            String::new()
        };
        state.snapshot.clone()
    }

    pub fn snapshot(&self) -> RuntimeAuthScopeSnapshot {
        self.lock_state().snapshot.clone()
    }

    pub fn identity(&self) -> RuntimeAuthIdentity {
        let state = self.lock_state();
        RuntimeAuthIdentity {
            user_id: state.snapshot.current_user_id.clone(),
            display_name: state.display_name.clone(),
        }
    }

    pub fn matches(&self, user_id: &str, endpoint: &str) -> bool {
        let state = self.lock_state();
        state.snapshot.active
            && state.snapshot.current_user_id == user_id.trim()
            && state.snapshot.endpoint == normalize_endpoint(endpoint)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, RuntimeAuthScopeState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }
}

fn normalize_text(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

fn normalize_endpoint(value: impl AsRef<str>) -> String {
    normalize_vrchat_api_endpoint(Some(value.as_ref()))
}

fn normalize_display_name(value: &str, user_id: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        user_id.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeAuthScope;

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
    fn tracks_display_name_without_changing_scope_generation() {
        let scope = RuntimeAuthScope::new();

        let first = scope.set_identity("usr_a", " Alice ", "");
        let renamed = scope.set_identity("usr_a", "Alice Two", "");

        assert_eq!(renamed.generation, first.generation);
        assert_eq!(
            scope.identity(),
            super::RuntimeAuthIdentity {
                user_id: "usr_a".into(),
                display_name: "Alice Two".into(),
            }
        );
    }

    #[test]
    fn identity_falls_back_to_user_id_and_clears_with_scope() {
        let scope = RuntimeAuthScope::new();

        scope.set("usr_a", "");
        assert_eq!(scope.identity().display_name, "usr_a");
        scope.set("", "");
        assert_eq!(scope.identity(), super::RuntimeAuthIdentity::default());
    }

    #[test]
    fn inactive_scope_never_authorizes_requests() {
        let scope = RuntimeAuthScope::new();

        assert!(!scope.matches("usr_current", "https://api.vrchat.cloud/api/1"));
        assert!(!scope.matches("usr_other", "https://api.vrchat.cloud/api/1"));
    }

    #[test]
    fn active_scope_normalizes_requested_endpoints() {
        let scope = RuntimeAuthScope::new();
        scope.set("usr_current", "https://api.vrchat.cloud/api/1");

        for endpoint in [
            "https://api.vrchat.cloud/api/1",
            "https://api.vrchat.cloud/api/1/",
            "  https://api.vrchat.cloud/api/1  ",
            "",
        ] {
            assert!(
                scope.matches("usr_current", endpoint),
                "endpoint {endpoint:?} should match the active auth scope"
            );
        }

        assert!(!scope.matches("usr_current", "https://api.example.test/api/1"));
    }

    #[test]
    fn active_scope_matches_only_its_current_user() {
        let scope = RuntimeAuthScope::new();
        scope.set("usr_current", "https://api.example.test/api/1");

        assert!(scope.matches("usr_current", "https://api.example.test/api/1"));
        assert!(!scope.matches("usr_stale", "https://api.example.test/api/1"));
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
