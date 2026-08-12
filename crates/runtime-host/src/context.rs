use std::sync::{Arc, Mutex};
use std::time::Duration;

use vrcx_0_application::{
    FavoriteMutationCoordinator, LoginSessionRuntime, MutualGraphFetchRuntime, PrintCleanupQueue,
    RemoteMutationGate, VrcStatusService,
};
use vrcx_0_application_activity::{
    OverlayActivityDelivery, OverlayActivityRuntime, OverlayActivitySink, OverlayActivitySnapshot,
};
use vrcx_0_application_core::{
    AvatarCache, HostSessionRuntime, ImageCache, RuntimeAuthScope, RuntimeBackgroundJobs,
    RuntimeDiagnostics, RuntimeEventBus, RuntimeLifecycle, RuntimeSyncEngine, TaskSupervisor,
    WebClient, WorldCache,
};
use vrcx_0_persistence::config::ConfigRepository;
use vrcx_0_persistence::DatabaseService;

use crate::notification::{
    load_overlay_activity_filters, save_notification_activity_filters,
    save_overlay_activity_preference_filters, AuthWebhookEvent, AuthWebhookQueue,
    AuthWebhookQueueDeps, NotificationActivityFiltersSetInput, NotificationWebhookSink,
    NotificationWebhookSinkDeps, OverlayActivityPreferenceFilters, UserImageCache,
    WebhookDeliveryMonitor, WebhookDeliverySnapshot,
};

const AVATAR_CACHE_WORKING_CAPACITY: u64 = 256;
const AVATAR_CACHE_WORKING_TTL: Duration = Duration::from_secs(2 * 60);
const WORLD_CACHE_WORKING_CAPACITY: u64 = 256;
const WORLD_CACHE_WORKING_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone, Default)]
struct OverlayActivityFanoutSink {
    sinks: Arc<Mutex<Vec<Arc<dyn OverlayActivitySink>>>>,
}

impl OverlayActivityFanoutSink {
    fn add(&self, sink: Arc<dyn OverlayActivitySink>) {
        match self.sinks.lock() {
            Ok(mut sinks) => sinks.push(sink),
            Err(error) => tracing::warn!("failed to lock overlay activity sinks: {error}"),
        }
    }

    fn sinks(&self) -> Vec<Arc<dyn OverlayActivitySink>> {
        self.sinks
            .lock()
            .map(|sinks| sinks.clone())
            .unwrap_or_else(|error| {
                tracing::warn!("failed to lock overlay activity sinks: {error}");
                Vec::new()
            })
    }
}

impl OverlayActivitySink for OverlayActivityFanoutSink {
    fn emit_overlay_activity_snapshot(&self, snapshot: OverlayActivitySnapshot) {
        for sink in self.sinks() {
            sink.emit_overlay_activity_snapshot(snapshot.clone());
        }
    }

    fn emit_overlay_activity_delivery(&self, delivery: OverlayActivityDelivery) {
        for sink in self.sinks() {
            sink.emit_overlay_activity_delivery(delivery.clone());
        }
    }
}

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
    pub remote_mutations: Arc<RemoteMutationGate>,
    pub favorite_mutations: FavoriteMutationCoordinator,
    pub vrc_status: VrcStatusService,
    pub login_session: LoginSessionRuntime,
    pub avatar_cache: Arc<AvatarCache>,
    pub world_cache: Arc<WorldCache>,
    pub config: ConfigRepository,
    overlay_activity: OverlayActivityRuntime,
    overlay_activity_sinks: OverlayActivityFanoutSink,
    notification_user_image_cache: Arc<UserImageCache>,
    auth_webhook_queue: AuthWebhookQueue,
    webhook_delivery_monitor: WebhookDeliveryMonitor,
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
        let sync = RuntimeSyncEngine::new();
        let auth_scope = RuntimeAuthScope::new();
        let tasks = TaskSupervisor::new();
        let session = HostSessionRuntime::new();
        let avatar_cache = Arc::new(AvatarCache::new(
            Arc::clone(&db),
            AVATAR_CACHE_WORKING_CAPACITY,
            AVATAR_CACHE_WORKING_TTL,
        ));
        let world_cache = Arc::new(WorldCache::new(
            Arc::clone(&db),
            WORLD_CACHE_WORKING_CAPACITY,
            WORLD_CACHE_WORKING_TTL,
        ));
        let overlay_activity =
            OverlayActivityRuntime::with_filters(load_overlay_activity_filters(&config));
        let overlay_activity_sinks = OverlayActivityFanoutSink::default();
        let notification_user_image_cache = Arc::new(UserImageCache::new());
        let webhook_delivery_monitor = WebhookDeliveryMonitor::default();
        let auth_webhook_queue = AuthWebhookQueue::new(AuthWebhookQueueDeps {
            config: config.clone(),
            web: Arc::clone(&web),
            diagnostics: diagnostics.clone(),
            monitor: webhook_delivery_monitor.clone(),
            tasks: tasks.clone(),
        });
        let vrc_status = VrcStatusService::new(Arc::clone(&web), event_bus.clone());
        overlay_activity_sinks.add(Arc::new(NotificationWebhookSink::new(
            NotificationWebhookSinkDeps {
                session: session.clone(),
                config: config.clone(),
                db: Arc::clone(&db),
                web: Arc::clone(&web),
                world_cache: Arc::clone(&world_cache),
                user_image_cache: Arc::clone(&notification_user_image_cache),
                diagnostics: diagnostics.clone(),
                monitor: webhook_delivery_monitor.clone(),
                tasks: tasks.clone(),
            },
        )));
        overlay_activity.set_sink(overlay_activity_sinks.clone());
        let mutual_graph_fetch = MutualGraphFetchRuntime::with_event_bus(event_bus.clone());
        let remote_mutations = Arc::new(RemoteMutationGate::default());
        let favorite_mutations = FavoriteMutationCoordinator::new(
            Arc::clone(&db),
            Arc::clone(&web),
            diagnostics.clone(),
            sync.clone(),
            event_bus.clone(),
            auth_scope.clone(),
            Arc::clone(&remote_mutations),
        );
        Self {
            db,
            web,
            image_cache,
            event_bus,
            runtime: RuntimeLifecycle::new(),
            background_jobs: RuntimeBackgroundJobs::new(),
            sync,
            diagnostics,
            tasks,
            session,
            auth_scope,
            print_cleanup: PrintCleanupQueue::new(),
            mutual_graph_fetch,
            remote_mutations,
            favorite_mutations,
            vrc_status,
            login_session: LoginSessionRuntime::new(),
            avatar_cache,
            world_cache,
            config,
            overlay_activity,
            overlay_activity_sinks,
            notification_user_image_cache,
            auth_webhook_queue,
            webhook_delivery_monitor,
        }
    }

    pub fn config(&self) -> &ConfigRepository {
        &self.config
    }

    pub fn overlay_activity(&self) -> OverlayActivityRuntime {
        self.overlay_activity.clone()
    }

    pub fn add_overlay_activity_sink(&self, sink: Arc<dyn OverlayActivitySink>) {
        self.overlay_activity_sinks.add(sink);
    }

    pub fn notification_user_image_cache(&self) -> Arc<UserImageCache> {
        Arc::clone(&self.notification_user_image_cache)
    }

    pub fn enqueue_auth_webhook(&self, event: AuthWebhookEvent) {
        self.auth_webhook_queue.enqueue(event);
    }

    pub fn webhook_delivery_snapshot(&self) -> WebhookDeliverySnapshot {
        self.webhook_delivery_monitor.snapshot()
    }

    pub fn reload_overlay_activity_filters(&self) {
        self.overlay_activity
            .set_filters(load_overlay_activity_filters(&self.config));
    }

    pub fn set_overlay_activity_preference_filters(
        &self,
        filters: OverlayActivityPreferenceFilters,
    ) -> crate::Result<()> {
        save_overlay_activity_preference_filters(&self.config, filters)?;
        self.reload_overlay_activity_filters();
        Ok(())
    }

    pub fn set_notification_activity_filters(
        &self,
        input: NotificationActivityFiltersSetInput,
    ) -> crate::Result<()> {
        save_notification_activity_filters(&self.config, input)?;
        self.reload_overlay_activity_filters();
        Ok(())
    }
}
