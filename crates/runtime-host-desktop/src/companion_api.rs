use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use vrcx_0_application_core::{
    InstanceRosterSnapshot, RuntimeAuthScope, RuntimeAuthScopeSnapshot, RuntimeEventBus,
};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_companion_api::{
    CompanionApiConfigStore, CompanionApiController, CompanionApiError, CompanionApiInput,
    CompanionApiInputReceiver, CompanionApiStartFailedPayload, CompanionApiStatus, RoomMemberState,
    RoomState, DEFAULT_COMPANION_API_PORT,
};
use vrcx_0_persistence::config::ConfigRepository;
use vrcx_0_runtime_host::RuntimeHostContext;

pub(crate) struct DesktopCompanionApiConfigStore {
    config: ConfigRepository,
}

impl DesktopCompanionApiConfigStore {
    pub(crate) fn new(config: ConfigRepository) -> Self {
        Self { config }
    }
}

impl CompanionApiConfigStore for DesktopCompanionApiConfigStore {
    fn get_bool(&self, key: &str, default: bool) -> Result<bool, CompanionApiError> {
        self.config
            .get_bool(key, default)
            .map_err(|error| CompanionApiError::Config(error.to_string()))
    }

    fn get_string(&self, key: &str, default: &str) -> Result<String, CompanionApiError> {
        self.config
            .get_string(key, default)
            .map_err(|error| CompanionApiError::Config(error.to_string()))
    }

    fn set_bool(&self, key: &str, value: bool) -> Result<(), CompanionApiError> {
        self.config
            .set_bool(key, value)
            .map_err(|error| CompanionApiError::Config(error.to_string()))
    }

    fn set_string(&self, key: &str, value: &str) -> Result<(), CompanionApiError> {
        self.config
            .set_string(key, value)
            .map_err(|error| CompanionApiError::Config(error.to_string()))
    }
}

pub struct DesktopCompanionApiRuntime {
    controller: Arc<CompanionApiController>,
    auth_scope: RuntimeAuthScope,
    roster: Mutex<CompanionApiRosterState>,
    enrichment_sender: broadcast::Sender<CompanionApiEnrichmentRequest>,
}

#[derive(Default)]
struct CompanionApiRosterState {
    lifecycle_epoch: u64,
    game_running: bool,
    latest: Option<Arc<InstanceRosterSnapshot>>,
}

#[derive(Clone)]
pub(crate) struct CompanionApiEnrichmentRequest {
    lifecycle_epoch: u64,
    listener_generation: u64,
    auth_scope: RuntimeAuthScopeSnapshot,
    snapshot: Arc<InstanceRosterSnapshot>,
}

impl DesktopCompanionApiRuntime {
    pub(crate) fn new(
        controller: Arc<CompanionApiController>,
        auth_scope: RuntimeAuthScope,
    ) -> (Self, broadcast::Receiver<CompanionApiEnrichmentRequest>) {
        let (enrichment_sender, enrichment_receiver) = broadcast::channel(1);
        (
            Self {
                controller,
                auth_scope,
                roster: Mutex::new(CompanionApiRosterState::default()),
                enrichment_sender,
            },
            enrichment_receiver,
        )
    }

    pub async fn status(&self) -> Result<CompanionApiStatus, CompanionApiError> {
        self.controller.status().await
    }

    pub async fn set_enabled(
        &self,
        enabled: bool,
    ) -> Result<CompanionApiStatus, CompanionApiError> {
        let status = self.controller.set_enabled(enabled).await?;
        self.replay_latest_if_running().await;
        Ok(status)
    }

    pub async fn set_port(&self, port: u16) -> Result<CompanionApiStatus, CompanionApiError> {
        let status = self.controller.set_port(port).await?;
        self.replay_latest_if_running().await;
        Ok(status)
    }

    pub async fn set_allow_lan_connections(
        &self,
        enabled: bool,
    ) -> Result<CompanionApiStatus, CompanionApiError> {
        let status = self.controller.set_allow_lan_connections(enabled).await?;
        self.replay_latest_if_running().await;
        Ok(status)
    }

    pub async fn rotate_token(&self) -> Result<CompanionApiStatus, CompanionApiError> {
        self.controller.rotate_token().await
    }

    async fn observe_roster(
        &self,
        lifecycle_epoch: u64,
        snapshot: Arc<InstanceRosterSnapshot>,
    ) {
        {
            let mut roster = self
                .roster
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if !roster.game_running || roster.lifecycle_epoch != lifecycle_epoch {
                return;
            }
            roster.latest = Some(snapshot.clone());
        }
        self.enqueue_if_running(lifecycle_epoch, snapshot).await;
    }

    fn observe_game_running(&self, lifecycle_epoch: u64, running: bool) {
        let mut roster = self
            .roster
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if lifecycle_epoch < roster.lifecycle_epoch {
            return;
        }
        if lifecycle_epoch != roster.lifecycle_epoch || !running {
            roster.latest = None;
        }
        roster.lifecycle_epoch = lifecycle_epoch;
        roster.game_running = running;
    }

    async fn replay_latest_if_running(&self) {
        let (lifecycle_epoch, snapshot) = {
            let roster = self
                .roster
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            (roster.lifecycle_epoch, roster.latest.clone())
        };
        if let Some(snapshot) = snapshot {
            self.enqueue_if_running(lifecycle_epoch, snapshot).await;
        }
    }

    async fn enqueue_if_running(
        &self,
        lifecycle_epoch: u64,
        snapshot: Arc<InstanceRosterSnapshot>,
    ) {
        let Some(listener_generation) = self.controller.running_generation().await else {
            return;
        };
        let _ = self.enrichment_sender.send(CompanionApiEnrichmentRequest {
            lifecycle_epoch,
            listener_generation,
            auth_scope: self.auth_scope.snapshot(),
            snapshot,
        });
    }

    fn lifecycle_matches(&self, expected_epoch: u64) -> bool {
        let roster = self
            .roster
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        roster.game_running && roster.lifecycle_epoch == expected_epoch
    }
}

pub(crate) fn start_companion_api_input_task(
    context: Arc<RuntimeHostContext>,
    realtime_runtime: Arc<RealtimeHostRuntime>,
    runtime: Arc<DesktopCompanionApiRuntime>,
    mut receiver: CompanionApiInputReceiver,
    enrichment_receiver: broadcast::Receiver<CompanionApiEnrichmentRequest>,
) {
    let enrichment_context = Arc::clone(&context);
    let enrichment_realtime_runtime = Arc::clone(&realtime_runtime);
    let enrichment_runtime = Arc::clone(&runtime);
    context.tasks.spawn(run_companion_api_enrichment(
        enrichment_context,
        enrichment_realtime_runtime,
        enrichment_runtime,
        enrichment_receiver,
    ));
    let task_context = Arc::clone(&context);
    context.tasks.spawn(async move {
        if let Err(error) = runtime.controller.start_from_config().await {
            emit_start_failed(&task_context.event_bus, &runtime.controller, &error).await;
        }
        while let Some(input) = receiver.recv().await {
            match input {
                CompanionApiInput::GameRunning {
                    lifecycle_epoch,
                    running,
                } => {
                    runtime.observe_game_running(lifecycle_epoch, running);
                    match runtime.controller.set_game_running(running).await {
                        Ok(_) => runtime.replay_latest_if_running().await,
                        Err(error) => {
                            if running {
                                emit_start_failed(
                                    &task_context.event_bus,
                                    &runtime.controller,
                                    &error,
                                )
                                .await;
                            }
                        }
                    }
                }
                CompanionApiInput::Roster {
                    lifecycle_epoch,
                    snapshot,
                } => {
                    runtime.observe_roster(lifecycle_epoch, snapshot).await;
                }
            }
        }
    });
}

async fn run_companion_api_enrichment(
    context: Arc<RuntimeHostContext>,
    realtime_runtime: Arc<RealtimeHostRuntime>,
    runtime: Arc<DesktopCompanionApiRuntime>,
    mut receiver: broadcast::Receiver<CompanionApiEnrichmentRequest>,
) {
    loop {
        let request = match receiver.recv().await {
            Ok(request) => request,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        };
        if runtime.controller.running_generation().await != Some(request.listener_generation)
            || !auth_scope_matches(&runtime.auth_scope.snapshot(), &request.auth_scope)
            || !runtime.lifecycle_matches(request.lifecycle_epoch)
        {
            continue;
        }
        let db = Arc::clone(&context.db);
        let realtime_runtime = Arc::clone(&realtime_runtime);
        let owner_user_id = request.auth_scope.current_user_id.clone();
        let enrichment_auth_scope = request.auth_scope.clone();
        match tokio::task::spawn_blocking(move || {
            enrich_room_snapshot(
                db.as_ref(),
                &owner_user_id,
                &enrichment_auth_scope,
                &realtime_runtime,
                request.snapshot,
            )
        })
        .await
        {
            Ok(room)
                if auth_scope_matches(&runtime.auth_scope.snapshot(), &request.auth_scope)
                    && runtime.lifecycle_matches(request.lifecycle_epoch) =>
            {
                runtime
                    .controller
                    .publish_if_generation(request.listener_generation, room)
                    .await;
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(error = %error, "Companion API room enrichment task failed");
            }
        }
    }
}

fn enrich_room_snapshot(
    db: &vrcx_0_persistence::DatabaseService,
    owner_user_id: &str,
    auth_scope: &RuntimeAuthScopeSnapshot,
    realtime_runtime: &RealtimeHostRuntime,
    snapshot: Arc<InstanceRosterSnapshot>,
) -> Option<RoomState> {
    let world_id = vrcx_0_core::location::world_id_from_location(&snapshot.location);
    if world_id.is_empty() {
        return None;
    }
    let user_ids = snapshot
        .members
        .iter()
        .map(|member| member.user_id.clone())
        .collect::<Vec<_>>();
    let cached_profiles = realtime_runtime
        .cached_user_profiles(auth_scope, &user_ids)
        .into_iter()
        .map(|profile| (profile.user_id.clone(), profile))
        .collect::<HashMap<_, _>>();
    let remote_notes =
        vrcx_0_persistence::memos::memo_list_user_notes(db, owner_user_id.to_owned())
            .unwrap_or_else(|error| {
                tracing::debug!(error = %error, "Companion API user note enrichment failed");
                Vec::new()
            })
            .into_iter()
            .map(|note| (note.user_id, note.note))
            .collect::<HashMap<_, _>>();
    let members = snapshot
        .members
        .iter()
        .map(|member| {
            let profile = cached_profiles.get(&member.user_id);
            let local_memo = vrcx_0_persistence::memos::memo_get_user(db, member.user_id.clone())
                .unwrap_or_else(|error| {
                    tracing::debug!(
                        user_id = %member.user_id,
                        error = %error,
                        "Companion API local memo enrichment failed"
                    );
                    None
                })
                .map(|memo| memo.memo)
                .unwrap_or_default();
            let note = if local_memo.is_empty() {
                remote_notes
                    .get(&member.user_id)
                    .cloned()
                    .unwrap_or_default()
            } else {
                local_memo
            };
            RoomMemberState {
                is_self: !owner_user_id.is_empty() && member.user_id == owner_user_id,
                is_friend: profile.is_some_and(|profile| profile.is_friend),
                joined_at: member.joined_at_ms.and_then(|joined_at_ms| {
                    chrono::DateTime::from_timestamp_millis(joined_at_ms)
                        .map(vrcx_0_core::time::iso_millis)
                }),
                languages: profile
                    .map(|profile| profile.languages.clone())
                    .unwrap_or_default(),
                note,
                user_id: member.user_id.clone(),
                display_name: member.display_name.clone(),
            }
        })
        .collect();
    Some(RoomState {
        location: snapshot.location.clone(),
        world_id,
        world_name: snapshot.world_name.clone(),
        destination: snapshot.destination.clone(),
        entered_at: snapshot.entered_at.clone(),
        members,
    })
}

fn auth_scope_matches(
    current: &RuntimeAuthScopeSnapshot,
    expected: &RuntimeAuthScopeSnapshot,
) -> bool {
    current.generation == expected.generation
        && current.current_user_id == expected.current_user_id
        && current.endpoint == expected.endpoint
        && current.active == expected.active
}

async fn emit_start_failed(
    event_bus: &RuntimeEventBus,
    controller: &CompanionApiController,
    error: &CompanionApiError,
) {
    let fallback_port = controller
        .status()
        .await
        .map(|status| status.port)
        .unwrap_or(DEFAULT_COMPANION_API_PORT);
    event_bus.emit(CompanionApiStartFailedPayload::from_error(
        error,
        fallback_port,
    ));
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct MemoryConfig {
        values: Mutex<HashMap<String, String>>,
    }

    impl CompanionApiConfigStore for MemoryConfig {
        fn get_bool(&self, key: &str, default: bool) -> Result<bool, CompanionApiError> {
            Ok(self
                .values
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .get(key)
                .and_then(|value| value.parse().ok())
                .unwrap_or(default))
        }

        fn get_string(&self, key: &str, default: &str) -> Result<String, CompanionApiError> {
            Ok(self
                .values
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .get(key)
                .cloned()
                .unwrap_or_else(|| default.into()))
        }

        fn set_bool(&self, key: &str, value: bool) -> Result<(), CompanionApiError> {
            self.set_string(key, if value { "true" } else { "false" })
        }

        fn set_string(&self, key: &str, value: &str) -> Result<(), CompanionApiError> {
            self.values
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .insert(key.into(), value.into());
            Ok(())
        }
    }

    fn unused_port() -> u16 {
        std::net::TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[tokio::test]
    async fn enabling_in_an_existing_room_replays_the_latest_roster() {
        let config = Arc::new(MemoryConfig::default());
        config
            .set_string(
                vrcx_0_companion_api::COMPANION_API_PORT_CONFIG_KEY,
                &unused_port().to_string(),
            )
            .unwrap();
        let controller = Arc::new(CompanionApiController::new(config, "1.2.3".into()).unwrap());
        let auth_scope = RuntimeAuthScope::new();
        auth_scope.set("usr_self", "https://api.vrchat.cloud/api/1");
        let (runtime, mut receiver) =
            DesktopCompanionApiRuntime::new(Arc::clone(&controller), auth_scope);
        runtime.observe_game_running(1, true);
        controller.set_game_running(true).await.unwrap();
        runtime
            .observe_roster(
                1,
                Arc::new(InstanceRosterSnapshot {
                    location: "wrld_a:1".into(),
                    ..InstanceRosterSnapshot::default()
                }),
            )
            .await;
        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));

        runtime.set_enabled(true).await.unwrap();
        let request = receiver.recv().await.unwrap();
        assert_eq!(request.snapshot.location, "wrld_a:1");
        assert_eq!(
            controller.running_generation().await,
            Some(request.listener_generation)
        );
        runtime.set_enabled(false).await.unwrap();
    }

    #[tokio::test]
    async fn previous_lifecycle_roster_cannot_replace_the_current_cache() {
        let config = Arc::new(MemoryConfig::default());
        let controller = Arc::new(CompanionApiController::new(config, "1.2.3".into()).unwrap());
        let (runtime, mut receiver) =
            DesktopCompanionApiRuntime::new(controller, RuntimeAuthScope::new());
        runtime.observe_game_running(3, true);

        runtime
            .observe_roster(
                1,
                Arc::new(InstanceRosterSnapshot {
                    location: "wrld_old:1".into(),
                    ..InstanceRosterSnapshot::default()
                }),
            )
            .await;
        assert!(matches!(
            receiver.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));
        assert!(runtime
            .roster
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .latest
            .is_none());
    }

    #[test]
    fn auth_scope_match_rejects_account_and_generation_changes() {
        let expected = RuntimeAuthScopeSnapshot {
            current_user_id: "usr_a".into(),
            endpoint: "https://api.vrchat.cloud/api/1".into(),
            generation: 4,
            active: true,
        };
        assert!(auth_scope_matches(&expected, &expected));

        let mut changed = expected.clone();
        changed.current_user_id = "usr_b".into();
        changed.generation = 5;
        assert!(!auth_scope_matches(&changed, &expected));
    }
}
