use std::sync::Arc;

use vrcx_0_application::{FavoriteMutationCoordinator, MutualGraphFetchRuntime};
use vrcx_0_application_core::{RuntimeAuthScope, TaskSupervisor, WebClient};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_persistence::config::ConfigRepository;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_runtime_host::RuntimeHostState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpCaller {
    Assistant,
    ExternalServer,
}

#[derive(Clone)]
pub struct McpRuntime {
    pub(crate) db: Arc<DatabaseService>,
    pub(crate) web: Arc<WebClient>,
    pub(crate) realtime_runtime: Arc<RealtimeHostRuntime>,
    pub(crate) auth_scope: RuntimeAuthScope,
    pub(crate) config: ConfigRepository,
    pub(crate) mutual_graph_fetch: MutualGraphFetchRuntime,
    pub(crate) favorite_mutations: FavoriteMutationCoordinator,
    pub(crate) tasks: TaskSupervisor,
    pub(crate) caller: McpCaller,
}

impl McpRuntime {
    pub fn from_host(state: &RuntimeHostState, caller: McpCaller) -> Self {
        Self {
            db: Arc::clone(&state.db),
            web: Arc::clone(&state.web),
            realtime_runtime: Arc::clone(&state.realtime_runtime),
            auth_scope: state.runtime_context.auth_scope.clone(),
            config: state.runtime_context.config.clone(),
            mutual_graph_fetch: state.runtime_context.mutual_graph_fetch.clone(),
            favorite_mutations: state.runtime_context.favorite_mutations.clone(),
            tasks: state.runtime_context.tasks.clone(),
            caller,
        }
    }

    pub(crate) fn vrchat_writes_allowed(&self) -> bool {
        match self.caller {
            McpCaller::Assistant => true,
            McpCaller::ExternalServer => self
                .config
                .get_bool(crate::config::MCP_ALLOW_VRCHAT_WRITES_CONFIG_KEY, false)
                .unwrap_or(false),
        }
    }

    pub(crate) fn current_user_id(&self) -> Option<String> {
        let from_auth = self.auth_scope.snapshot().current_user_id;
        current_user_id_from_sources(&from_auth, None)
    }

    pub(crate) fn current_endpoint(&self) -> String {
        self.realtime_runtime
            .friend_snapshot()
            .map(|snapshot| snapshot.endpoint)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_default()
    }
}

fn current_user_id_from_sources(
    auth_scope_user_id: &str,
    _realtime_user_id: Option<&str>,
) -> Option<String> {
    let auth_scope_user_id = auth_scope_user_id.trim();
    if !auth_scope_user_id.is_empty() {
        return Some(auth_scope_user_id.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{current_user_id_from_sources, McpCaller};

    #[test]
    fn current_user_owner_prefers_auth_scope_over_realtime_snapshot() {
        let owner = current_user_id_from_sources(" usr_auth ", Some("usr_ws"));

        assert_eq!(owner.as_deref(), Some("usr_auth"));
    }

    #[test]
    fn current_user_owner_does_not_fall_back_to_realtime_when_auth_scope_is_empty() {
        let owner = current_user_id_from_sources(" ", Some(" usr_ws "));

        assert_eq!(owner, None);
    }

    #[test]
    fn current_user_owner_stays_empty_when_all_sources_are_empty() {
        assert_eq!(current_user_id_from_sources("", None), None);
        assert_eq!(current_user_id_from_sources(" ", Some(" ")), None);
    }

    #[test]
    fn caller_policy_keeps_external_and_assistant_write_authority_separate() {
        let (_dir, mut runtime) =
            crate::test_support::test_runtime("mcp-caller", "usr_test").expect("test runtime");

        assert!(!runtime.vrchat_writes_allowed());
        runtime
            .config
            .set_bool(crate::config::MCP_ALLOW_VRCHAT_WRITES_CONFIG_KEY, true)
            .unwrap();
        assert!(runtime.vrchat_writes_allowed());

        runtime.caller = McpCaller::Assistant;
        runtime
            .config
            .set_bool(crate::config::MCP_ALLOW_VRCHAT_WRITES_CONFIG_KEY, false)
            .unwrap();
        assert!(runtime.vrchat_writes_allowed());
    }
}
