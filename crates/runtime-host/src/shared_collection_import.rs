use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
#[cfg(test)]
use std::{future::Future, pin::Pin};

use chrono::Utc;
#[cfg(test)]
use vrcx_0_application::PreparedSharedCollectionImport;
use vrcx_0_application::{
    prepare_shared_collection_import, run_shared_collection_import, FavoriteMutationCoordinator,
    SharedCollectionImportProgress, SharedCollectionImportResult, SharedCollectionImportStartInput,
    SharedCollectionImportState, SharedCollectionImportStatus, VrchatSharedCollectionImportActions,
};
use vrcx_0_application_core::{
    RuntimeAuthScope, RuntimeAuthScopeSnapshot, RuntimeEventBus, TaskSupervisor, WebClient,
    WorldCache,
};
use vrcx_0_persistence::DatabaseService;

use crate::{Error, Result};

#[cfg(test)]
type TestImportRunner = Arc<
    dyn Fn(
            PreparedSharedCollectionImport,
            Arc<AtomicBool>,
        ) -> Pin<
            Box<
                dyn Future<Output = vrcx_0_application_core::Result<SharedCollectionImportResult>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

#[derive(Clone)]
pub struct SharedCollectionImportRuntime {
    shared: Arc<SharedCollectionImportRuntimeShared>,
    db: Arc<DatabaseService>,
    web: Arc<WebClient>,
    world_cache: Arc<WorldCache>,
    event_bus: RuntimeEventBus,
    tasks: TaskSupervisor,
    auth_scope: RuntimeAuthScope,
    favorite_mutations: FavoriteMutationCoordinator,
    #[cfg(test)]
    test_runner: Option<TestImportRunner>,
}

struct SharedCollectionImportRuntimeShared {
    state: Mutex<SharedCollectionImportRuntimeInner>,
    generation: AtomicU64,
}

#[derive(Default)]
struct SharedCollectionImportRuntimeInner {
    status: SharedCollectionImportStatus,
    cancel: Option<Arc<AtomicBool>>,
    auth_generation: u64,
}

impl SharedCollectionImportRuntime {
    pub fn new(
        db: Arc<DatabaseService>,
        web: Arc<WebClient>,
        world_cache: Arc<WorldCache>,
        event_bus: RuntimeEventBus,
        tasks: TaskSupervisor,
        auth_scope: RuntimeAuthScope,
        favorite_mutations: FavoriteMutationCoordinator,
    ) -> Self {
        Self {
            shared: Arc::new(SharedCollectionImportRuntimeShared {
                state: Mutex::new(SharedCollectionImportRuntimeInner::default()),
                generation: AtomicU64::new(0),
            }),
            db,
            web,
            world_cache,
            event_bus,
            tasks,
            auth_scope,
            favorite_mutations,
            #[cfg(test)]
            test_runner: None,
        }
    }

    #[cfg(test)]
    fn with_test_runner(mut self, test_runner: TestImportRunner) -> Self {
        self.test_runner = Some(test_runner);
        self
    }

    pub fn status(&self) -> SharedCollectionImportStatus {
        self.lock_inner().status.clone()
    }

    pub fn start(
        &self,
        input: SharedCollectionImportStartInput,
    ) -> Result<SharedCollectionImportStatus> {
        let prepared = prepare_shared_collection_import(input)?;
        let scope = self.auth_scope.snapshot();
        if !scope.active || scope.current_user_id.trim().is_empty() {
            return Err(Error::Custom(
                "Shared collection import requires an authenticated session.".into(),
            ));
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let status = {
            let mut inner = self.lock_inner();
            if is_active_status(inner.status.status) {
                return Err(Error::Custom(
                    "Another shared collection import is already active.".into(),
                ));
            }
            let generation = self.shared.generation.fetch_add(1, Ordering::AcqRel) + 1;
            let status = SharedCollectionImportStatus {
                run_id: format!("shared-{}-{generation}", Utc::now().timestamp_millis()),
                status: SharedCollectionImportState::Running,
                total: prepared.world_ids.len(),
                processed: 0,
                imported: 0,
                failed: 0,
                group_name: prepared.group_name.clone(),
                started_at: Some(Utc::now().to_rfc3339()),
                finished_at: None,
                last_error: None,
            };
            inner.status = status.clone();
            inner.cancel = Some(Arc::clone(&cancel));
            inner.auth_generation = scope.generation;
            status
        };
        self.emit_status(status.clone());

        let runtime = self.clone();
        let run_id = status.run_id.clone();
        self.tasks.spawn_cancellable(move |stop_token| async move {
            #[cfg(test)]
            if let Some(test_runner) = runtime.test_runner.clone() {
                let result = test_runner(prepared, Arc::clone(&cancel)).await;
                runtime.finish(&run_id, &scope, result);
                return;
            }
            let actions = VrchatSharedCollectionImportActions {
                db: runtime.db.as_ref(),
                web: runtime.web.as_ref(),
                world_cache: runtime.world_cache.as_ref(),
                endpoint: &scope.endpoint,
            };
            let cancel_for_check = Arc::clone(&cancel);
            let auth_scope_for_check = runtime.auth_scope.clone();
            let scope_for_check = scope.clone();
            let runtime_for_progress = runtime.clone();
            let run_id_for_progress = run_id.clone();
            let result = run_shared_collection_import(
                &actions,
                prepared,
                move || {
                    cancel_for_check.load(Ordering::Acquire)
                        || stop_token.is_stop_requested()
                        || !auth_scope_for_check
                            .snapshot()
                            .generation_matches(&scope_for_check)
                },
                move |progress| {
                    runtime_for_progress.apply_progress(&run_id_for_progress, progress);
                },
            )
            .await;
            runtime.finish(&run_id, &scope, result);
        });

        Ok(status)
    }

    pub fn cancel(&self) -> SharedCollectionImportStatus {
        let status = {
            let mut inner = self.lock_inner();
            if !is_active_status(inner.status.status) {
                return inner.status.clone();
            }
            if let Some(cancel) = &inner.cancel {
                cancel.store(true, Ordering::Release);
            }
            inner.status.status = SharedCollectionImportState::Cancelling;
            inner.status.clone()
        };
        self.emit_status(status.clone());
        status
    }

    pub fn cancel_if_scope_mismatch(&self) -> SharedCollectionImportStatus {
        let scope = self.auth_scope.snapshot();
        let status = {
            let mut inner = self.lock_inner();
            if !mark_cancelling_if_scope_mismatch(&mut inner, &scope) {
                return inner.status.clone();
            }
            inner.status.clone()
        };
        self.emit_status(status.clone());
        status
    }

    fn apply_progress(&self, run_id: &str, progress: SharedCollectionImportProgress) {
        let status = {
            let mut inner = self.lock_inner();
            if inner.status.run_id != run_id || !is_active_status(inner.status.status) {
                return;
            }
            inner.status.processed = progress.processed;
            inner.status.imported = progress.imported;
            inner.status.failed = progress.failed;
            inner.status.last_error = progress.last_error;
            inner.status.clone()
        };
        self.emit_status(status);
    }

    fn finish(
        &self,
        run_id: &str,
        scope: &RuntimeAuthScopeSnapshot,
        result: vrcx_0_application_core::Result<SharedCollectionImportResult>,
    ) {
        let terminal = {
            let inner = self.lock_inner();
            let Some(terminal) = prepare_terminal_result(&inner, run_id, result) else {
                return;
            };
            terminal
        };
        self.favorite_mutations
            .complete_shared_collection_import(scope, terminal.status.imported);
        let status = {
            let mut inner = self.lock_inner();
            if !commit_terminal_status(&mut inner, run_id, terminal.status) {
                return;
            }
            inner.status.clone()
        };
        self.emit_status(status);
    }

    fn emit_status(&self, status: SharedCollectionImportStatus) {
        self.event_bus.emit(status);
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, SharedCollectionImportRuntimeInner> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn is_active_status(status: SharedCollectionImportState) -> bool {
    matches!(
        status,
        SharedCollectionImportState::Running | SharedCollectionImportState::Cancelling
    )
}

fn mark_cancelling_if_scope_mismatch(
    inner: &mut SharedCollectionImportRuntimeInner,
    scope: &RuntimeAuthScopeSnapshot,
) -> bool {
    if !is_active_status(inner.status.status) || inner.auth_generation == scope.generation {
        return false;
    }
    if let Some(cancel) = &inner.cancel {
        cancel.store(true, Ordering::Release);
    }
    inner.status.status = SharedCollectionImportState::Cancelling;
    true
}

fn prepare_terminal_result(
    inner: &SharedCollectionImportRuntimeInner,
    run_id: &str,
    result: vrcx_0_application_core::Result<SharedCollectionImportResult>,
) -> Option<AppliedSharedCollectionImportTerminal> {
    if inner.status.run_id != run_id || !is_active_status(inner.status.status) {
        return None;
    }
    let mut status = inner.status.clone();
    match result {
        Ok(result) => {
            status.processed = result.processed;
            status.imported = result.imported;
            status.failed = result.failed;
            status.last_error = result.last_error;
            status.status = if result.cancelled {
                SharedCollectionImportState::Cancelled
            } else {
                SharedCollectionImportState::Completed
            };
        }
        Err(error) => {
            status.status = SharedCollectionImportState::Error;
            status.last_error = Some(error.to_string());
        }
    }
    status.finished_at = Some(Utc::now().to_rfc3339());
    Some(AppliedSharedCollectionImportTerminal { status })
}

fn commit_terminal_status(
    inner: &mut SharedCollectionImportRuntimeInner,
    run_id: &str,
    status: SharedCollectionImportStatus,
) -> bool {
    if inner.status.run_id != run_id || !is_active_status(inner.status.status) {
        return false;
    }
    inner.status = status;
    inner.cancel = None;
    true
}

struct AppliedSharedCollectionImportTerminal {
    status: SharedCollectionImportStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::{path::PathBuf, sync::Condvar, time::Duration};
    use vrcx_0_application_core::RuntimeEventSink;
    use vrcx_0_persistence::storage::StorageService;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx0-shared-import-contract-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn running_inner() -> SharedCollectionImportRuntimeInner {
        SharedCollectionImportRuntimeInner {
            status: SharedCollectionImportStatus {
                run_id: "run-1".into(),
                status: SharedCollectionImportState::Running,
                total: 2,
                ..Default::default()
            },
            cancel: Some(Arc::new(AtomicBool::new(false))),
            auth_generation: 1,
        }
    }

    #[test]
    fn auth_scope_switch_marks_active_run_cancelling() {
        let mut inner = running_inner();
        let scope = RuntimeAuthScopeSnapshot {
            generation: 2,
            active: true,
            ..Default::default()
        };

        assert!(mark_cancelling_if_scope_mismatch(&mut inner, &scope));
        assert_eq!(inner.status.status, SharedCollectionImportState::Cancelling);
        assert!(inner.cancel.as_ref().unwrap().load(Ordering::Acquire));
    }

    #[test]
    fn same_auth_scope_hydration_keeps_active_run_running() {
        let auth_scope = RuntimeAuthScope::new();
        let first = auth_scope.set("usr_self", "https://api.vrchat.cloud/api/1");
        let hydrated = auth_scope.set(" usr_self ", "https://api.vrchat.cloud/api/1/");
        let mut inner = running_inner();
        inner.auth_generation = first.generation;

        assert_eq!(hydrated.generation, first.generation);
        assert!(!mark_cancelling_if_scope_mismatch(&mut inner, &hydrated));
        assert_eq!(inner.status.status, SharedCollectionImportState::Running);
        assert!(!inner.cancel.as_ref().unwrap().load(Ordering::Acquire));
    }

    #[test]
    fn status_snapshot_retains_running_progress_for_hydration() {
        let mut inner = running_inner();
        inner.status.processed = 1;
        inner.status.imported = 1;

        let hydrated = inner.status.clone();

        assert_eq!(hydrated.run_id, "run-1");
        assert_eq!(hydrated.processed, 1);
        assert_eq!(hydrated.imported, 1);
        assert_eq!(hydrated.total, 2);
    }

    #[test]
    fn cancelled_terminal_with_imports_is_prepared_once() {
        let mut inner = running_inner();
        let result = SharedCollectionImportResult {
            total: 2,
            processed: 1,
            imported: 1,
            cancelled: true,
            ..Default::default()
        };

        let terminal = prepare_terminal_result(&inner, "run-1", Ok(result.clone()));
        assert!(commit_terminal_status(
            &mut inner,
            "run-1",
            terminal.as_ref().unwrap().status.clone()
        ));
        let duplicate = prepare_terminal_result(&inner, "run-1", Ok(result));

        assert_eq!(
            terminal.as_ref().unwrap().status.status,
            SharedCollectionImportState::Cancelled
        );
        assert_eq!(terminal.unwrap().status.imported, 1);
        assert!(duplicate.is_none());
    }

    #[derive(Clone, Default)]
    struct BlockingFavoritesSink {
        state: Arc<(Mutex<BlockingFavoritesSinkState>, Condvar)>,
    }

    #[derive(Default)]
    struct BlockingFavoritesSinkState {
        entered: bool,
        released: bool,
    }

    impl BlockingFavoritesSink {
        fn wait_until_entered(&self) {
            let (state, changed) = self.state.as_ref();
            let deadline = std::time::Instant::now() + Duration::from_secs(2);
            let mut state = state.lock().unwrap();
            while !state.entered {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                assert!(!remaining.is_zero(), "favoritesChanged was not emitted");
                let (next, timeout) = changed.wait_timeout(state, remaining).unwrap();
                state = next;
                assert!(
                    !timeout.timed_out() || state.entered,
                    "favoritesChanged was not emitted"
                );
            }
        }

        fn release(&self) {
            let (state, changed) = self.state.as_ref();
            let mut state = state.lock().unwrap();
            state.released = true;
            changed.notify_all();
        }
    }

    impl RuntimeEventSink for BlockingFavoritesSink {
        fn emit(&self, event: &str, _payload: Value) {
            if event != "favoritesChanged" {
                return;
            }
            let (state, changed) = self.state.as_ref();
            let mut state = state.lock().unwrap();
            state.entered = true;
            changed.notify_all();
            while !state.released {
                state = changed.wait(state).unwrap();
            }
        }
    }

    #[test]
    fn terminal_status_is_committed_after_favorites_changed_delivery() {
        let dir = TestDir::new("lifecycle");
        let db = Arc::new(DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap());
        let storage = StorageService::new(&dir.0.join("storage.json")).unwrap();
        let web = Arc::new(
            WebClient::new(
                &storage,
                db.as_ref(),
                "wss://pipeline.vrchat.cloud".into(),
                "2.2.0",
            )
            .unwrap(),
        );
        let world_cache = Arc::new(WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60)));
        let event_bus = RuntimeEventBus::new();
        let blocking_sink = BlockingFavoritesSink::default();
        event_bus.set_sink(blocking_sink.clone());
        let tasks = TaskSupervisor::new();
        let auth_scope = RuntimeAuthScope::new();
        auth_scope.set("usr_current", "https://api.vrchat.cloud/api/1");
        let remote_mutations = Arc::new(vrcx_0_application::RemoteMutationGate::default());
        let favorite_mutations = FavoriteMutationCoordinator::new(
            Arc::clone(&db),
            Arc::clone(&web),
            vrcx_0_application_core::RuntimeDiagnostics::new(),
            vrcx_0_application_core::RuntimeSyncEngine::new(),
            event_bus.clone(),
            auth_scope.clone(),
            remote_mutations,
        );
        let runtime = SharedCollectionImportRuntime::new(
            db,
            web,
            world_cache,
            event_bus.clone(),
            tasks.clone(),
            auth_scope,
            favorite_mutations,
        )
        .with_test_runner(Arc::new(|prepared, cancel| {
            Box::pin(async move {
                while !cancel.load(Ordering::Acquire) {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Ok(SharedCollectionImportResult {
                    total: prepared.world_ids.len(),
                    processed: 1,
                    imported: 1,
                    cancelled: true,
                    ..Default::default()
                })
            })
        }));
        let input = SharedCollectionImportStartInput {
            world_ids: vec!["wrld_11111111-1111-1111-1111-111111111111".into()],
            group_name: "Imported worlds".into(),
        };

        let running = runtime.start(input.clone()).unwrap();
        assert_eq!(running.status, SharedCollectionImportState::Running);
        assert!(runtime
            .start(input)
            .unwrap_err()
            .to_string()
            .contains("already active"));

        let cancelling = runtime.cancel();
        assert_eq!(cancelling.status, SharedCollectionImportState::Cancelling);
        blocking_sink.wait_until_entered();
        assert_eq!(
            runtime.status().status,
            SharedCollectionImportState::Cancelling
        );
        blocking_sink.release();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while is_active_status(runtime.status().status) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }

        let terminal = runtime.status();
        assert_eq!(terminal.status, SharedCollectionImportState::Cancelled);
        assert_eq!(terminal.imported, 1);
        let mut events = Vec::new();
        let event_deadline = std::time::Instant::now() + Duration::from_secs(2);
        while events.len() < 4 && std::time::Instant::now() < event_deadline {
            events.extend(event_bus.take_events_for_test());
            if events.len() < 4 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert_eq!(
            events
                .iter()
                .map(|event| event.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "sharedCollectionImportStatus",
                "sharedCollectionImportStatus",
                "favoritesChanged",
                "sharedCollectionImportStatus"
            ]
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.name == "favoritesChanged")
                .count(),
            1
        );
        tasks.stop_all();
    }
}
