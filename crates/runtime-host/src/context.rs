use std::sync::Arc;
use std::time::Duration;

use vrcx_0_application::HostSessionRuntime;
use vrcx_0_application::ImageCache;
use vrcx_0_application::LoginSessionRuntime;
use vrcx_0_application::MutualGraphFetchRuntime;
use vrcx_0_application::PrintCleanupQueue;
use vrcx_0_application::RuntimeAuthScope;
use vrcx_0_application::RuntimeBackgroundJobs;
use vrcx_0_application::RuntimeDiagnostics;
use vrcx_0_application::RuntimeEventBus;
use vrcx_0_application::RuntimeLifecycle;
use vrcx_0_application::RuntimeSyncEngine;
use vrcx_0_application::TaskSupervisor;
use vrcx_0_application::WebClient;
use vrcx_0_application::WorldCache;
use vrcx_0_persistence::config::ConfigRepository;
use vrcx_0_persistence::DatabaseService;

const WORLD_CACHE_WORKING_CAPACITY: u64 = 512;
const WORLD_CACHE_WORKING_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
pub struct RuntimeHostContext {
    pub db: Arc<DatabaseService>,
    pub web: Arc<WebClient>,
    pub image_cache: Arc<ImageCache>,
    pub event_bus: RuntimeEventBus,
    pub runtime: RuntimeLifecycle,
    pub background_jobs: RuntimeBackgroundJobs,
    pub sync: RuntimeSyncEngine,
    pub diagnostics: RuntimeDiagnostics,
    pub tasks: TaskSupervisor,
    pub session: HostSessionRuntime,
    pub auth_scope: RuntimeAuthScope,
    pub print_cleanup: PrintCleanupQueue,
    pub mutual_graph_fetch: MutualGraphFetchRuntime,
    pub login_session: LoginSessionRuntime,
    pub world_cache: Arc<WorldCache>,
    pub config: ConfigRepository,
}

impl RuntimeHostContext {
    pub fn new(
        db: Arc<DatabaseService>,
        web: Arc<WebClient>,
        image_cache: Arc<ImageCache>,
    ) -> Self {
        let config = ConfigRepository::new(Arc::clone(&db));
        let event_bus = RuntimeEventBus::new();
        let diagnostics = RuntimeDiagnostics::new();
        let tasks = TaskSupervisor::new();
        let session = HostSessionRuntime::new();
        let world_cache = Arc::new(WorldCache::new(
            Arc::clone(&db),
            WORLD_CACHE_WORKING_CAPACITY,
            WORLD_CACHE_WORKING_TTL,
        ));
        Self {
            db,
            web,
            image_cache,
            event_bus,
            runtime: RuntimeLifecycle::new(),
            background_jobs: RuntimeBackgroundJobs::new(),
            sync: RuntimeSyncEngine::new(),
            diagnostics,
            tasks,
            session,
            auth_scope: RuntimeAuthScope::new(),
            print_cleanup: PrintCleanupQueue::new(),
            mutual_graph_fetch: MutualGraphFetchRuntime::new(),
            login_session: LoginSessionRuntime::new(),
            world_cache,
            config,
        }
    }

    pub fn config(&self) -> &ConfigRepository {
        &self.config
    }
}
