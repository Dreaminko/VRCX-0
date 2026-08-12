use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use vrcx_0_application::{
    FavoriteMutationCoordinator, MutualGraphFetchRuntime, RemoteMutationGate,
};
use vrcx_0_application_core::{
    HostSessionRuntime, NoopPrintCleanupInputSink, RuntimeAuthScope, RuntimeDiagnostics,
    RuntimeEventBus, RuntimeSyncEngine, TaskSupervisor, UnavailableLocalGameContextSource,
    WebClient, WorldCache,
};
use vrcx_0_application_realtime::{RealtimeHostRuntime, RealtimeHostRuntimeDeps};
use vrcx_0_persistence::{
    config::ConfigRepository, game_log::ensure_game_log_tables, storage::StorageService,
    DatabaseService,
};

use crate::runtime::McpRuntime;

pub(crate) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("vrcx-0-mcp-{name}-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn test_runtime(
    name: &str,
    auth_scope_user_id: &str,
) -> Result<(TestDir, McpRuntime), Box<dyn std::error::Error>> {
    let (dir, runtime, _) = test_runtime_with_event_bus(name, auth_scope_user_id)?;
    Ok((dir, runtime))
}

pub(crate) fn test_runtime_with_event_bus(
    name: &str,
    auth_scope_user_id: &str,
) -> Result<(TestDir, McpRuntime, RuntimeEventBus), Box<dyn std::error::Error>> {
    let dir = TestDir::new(name);
    let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3"))?);
    ensure_game_log_tables(db.as_ref())?;
    let storage = StorageService::new(&dir.path.join("storage.json"))?;
    let web = Arc::new(WebClient::new(
        &storage,
        db.as_ref(),
        "wss://pipeline.vrchat.cloud".into(),
        env!("CARGO_PKG_VERSION"),
    )?);
    let auth_scope = RuntimeAuthScope::new();
    if !auth_scope_user_id.trim().is_empty() {
        auth_scope.set(auth_scope_user_id, "https://api.vrchat.cloud/api/1");
    }
    let event_bus = RuntimeEventBus::new();
    let sync = RuntimeSyncEngine::new();
    let diagnostics = RuntimeDiagnostics::new();
    let tasks = TaskSupervisor::new();
    let session = HostSessionRuntime::new();
    let world_cache = Arc::new(WorldCache::new(
        Arc::clone(&db),
        512,
        Duration::from_secs(30 * 60),
    ));
    let remote_mutations = Arc::new(RemoteMutationGate::default());
    let favorite_mutations = FavoriteMutationCoordinator::new(
        Arc::clone(&db),
        Arc::clone(&web),
        diagnostics,
        sync.clone(),
        event_bus.clone(),
        auth_scope.clone(),
        Arc::clone(&remote_mutations),
    );
    let realtime_runtime = Arc::new(RealtimeHostRuntime::new(RealtimeHostRuntimeDeps {
        db: Arc::clone(&db),
        web: Arc::clone(&web),
        event_bus: event_bus.clone(),
        backend_status: vrcx_0_application_core::BackendRuntimeStatusPublisher::new(
            vrcx_0_application_core::BackendRuntime::new(
                vrcx_0_application_core::RuntimeHostProfile::Desktop,
            ),
            event_bus.clone(),
        ),
        friend_projection_sink: vrcx_0_application_realtime::FriendProjectionSink::new(
            event_bus.clone(),
            None,
        ),
        sync,
        tasks: tasks.clone(),
        session,
        auth_scope: auth_scope.clone(),
        remote_mutations,
        local_game_context: Arc::new(UnavailableLocalGameContextSource),
        activity_sink: None,
        world_cache,
        print_cleanup: Arc::new(NoopPrintCleanupInputSink),
        friend_note_change_sink: None,
        current_user_snapshot_sink: None,
    }));
    let runtime = McpRuntime {
        db: Arc::clone(&db),
        web,
        realtime_runtime,
        auth_scope,
        config: ConfigRepository::new(db),
        mutual_graph_fetch: MutualGraphFetchRuntime::new(),
        favorite_mutations,
        tasks,
        caller: crate::runtime::McpCaller::ExternalServer,
    };
    Ok((dir, runtime, event_bus))
}
