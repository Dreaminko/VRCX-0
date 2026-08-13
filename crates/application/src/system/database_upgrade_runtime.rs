use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use vrcx_0_application_core::RuntimeOperationStatus;

use vrcx_0_persistence::legacy_migration::{
    prepare_legacy_migration, LegacyMigrationPaths, LegacyMigrationProgress,
};
use vrcx_0_persistence::legacy_vrcx::LegacyVrcxSource;
use vrcx_0_persistence::{DatabaseService, VRCX0_SCHEMA_VERSION};

use super::database_upgrade::{
    database_upgrade_preflight, run_database_upgrade_with_progress, DatabaseUpgradePreflight,
    DatabaseUpgradePreflightStatus, DatabaseUpgradeProgress, DatabaseUpgradeRunResult,
    DatabaseUpgradeRunStatus, DatabaseUpgradeStage,
};
use crate::{Error, RuntimeBackgroundJobs, RuntimeDiagnostics};

const COMMAND: &str = "app__database_upgrade_run";
const JOB: &str = "databaseUpgrade";

#[derive(Clone)]
pub struct DatabaseUpgradeRuntime {
    db: Arc<DatabaseService>,
    shared: Arc<DatabaseUpgradeRuntimeShared>,
    diagnostics: RuntimeDiagnostics,
    background_jobs: RuntimeBackgroundJobs,
}

struct DatabaseUpgradeRuntimeShared {
    state: Mutex<DatabaseUpgradeRuntimeState>,
    progress: Mutex<DatabaseUpgradeProgress>,
    legacy_prepare: Mutex<()>,
    changed: Condvar,
}

enum DatabaseUpgradeRuntimeState {
    Idle,
    Running { from_version: i64, to_version: i64 },
    Finished(Box<DatabaseUpgradeRunResult>),
}

impl DatabaseUpgradeRuntime {
    pub fn new(
        db: Arc<DatabaseService>,
        diagnostics: RuntimeDiagnostics,
        background_jobs: RuntimeBackgroundJobs,
    ) -> Self {
        Self {
            db,
            shared: Arc::new(DatabaseUpgradeRuntimeShared {
                state: Mutex::new(DatabaseUpgradeRuntimeState::Idle),
                progress: Mutex::new(DatabaseUpgradeProgress::indeterminate(
                    DatabaseUpgradeStage::Preflight,
                )),
                legacy_prepare: Mutex::new(()),
                changed: Condvar::new(),
            }),
            diagnostics,
            background_jobs,
        }
    }

    pub fn preflight(&self) -> Result<DatabaseUpgradePreflight, Error> {
        let state = self.lock_state();
        match &*state {
            DatabaseUpgradeRuntimeState::Idle => database_upgrade_preflight(&self.db),
            DatabaseUpgradeRuntimeState::Running {
                from_version,
                to_version,
            } => Ok(DatabaseUpgradePreflight {
                status: DatabaseUpgradePreflightStatus::Running,
                from_version: *from_version,
                to_version: *to_version,
                stage: Some(self.progress().stage),
                result: None,
                failed_upgrade: None,
            }),
            DatabaseUpgradeRuntimeState::Finished(result) => Ok(DatabaseUpgradePreflight {
                status: DatabaseUpgradePreflightStatus::Finished,
                from_version: result.from_version,
                to_version: result.to_version,
                stage: result.failed_stage,
                result: Some(result.as_ref().clone()),
                failed_upgrade: result.failed_upgrade.clone(),
            }),
        }
    }

    pub fn run(&self) -> DatabaseUpgradeRunResult {
        self.run_with(|db, on_progress| run_database_upgrade_with_progress(db, on_progress))
    }

    pub fn progress(&self) -> DatabaseUpgradeProgress {
        self.shared
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn prepare_legacy_migration(
        &self,
        paths: &LegacyMigrationPaths,
        source: &LegacyVrcxSource,
    ) -> Result<(), Error> {
        let _prepare_guard = self
            .shared
            .legacy_prepare
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.set_progress(DatabaseUpgradeProgress::indeterminate(
            DatabaseUpgradeStage::PrepareLegacySnapshot,
        ));
        prepare_legacy_migration(paths, source, |progress| {
            let progress = match progress {
                LegacyMigrationProgress::DatabaseCopy {
                    completed_pages,
                    total_pages,
                } => DatabaseUpgradeProgress::determinate(
                    DatabaseUpgradeStage::PrepareLegacySnapshot,
                    completed_pages,
                    total_pages,
                ),
                LegacyMigrationProgress::Configuration => DatabaseUpgradeProgress::indeterminate(
                    DatabaseUpgradeStage::PrepareLegacyConfiguration,
                ),
                LegacyMigrationProgress::Finalizing => DatabaseUpgradeProgress::indeterminate(
                    DatabaseUpgradeStage::FinalizeLegacyMigration,
                ),
            };
            self.set_progress(progress);
        })?;
        Ok(())
    }

    pub fn retry(&self) -> Result<DatabaseUpgradeRunResult, Error> {
        {
            let mut state = self.lock_state();
            match &*state {
                DatabaseUpgradeRuntimeState::Running { .. } => {
                    drop(state);
                    return Ok(self.run());
                }
                DatabaseUpgradeRuntimeState::Finished(result)
                    if matches!(
                        result.status,
                        DatabaseUpgradeRunStatus::Current
                            | DatabaseUpgradeRunStatus::Upgraded
                            | DatabaseUpgradeRunStatus::NewerSchema
                    ) =>
                {
                    return Ok(result.as_ref().clone());
                }
                DatabaseUpgradeRuntimeState::Idle | DatabaseUpgradeRuntimeState::Finished(_) => {}
            }

            self.db.discard_failed_upgrade()?;
            *state = DatabaseUpgradeRuntimeState::Idle;
        }

        Ok(self.run())
    }

    fn fresh_database_available(&self, state: &DatabaseUpgradeRuntimeState) -> Result<bool, Error> {
        match state {
            DatabaseUpgradeRuntimeState::Idle => Ok(matches!(
                database_upgrade_preflight(&self.db)?.status,
                DatabaseUpgradePreflightStatus::Blocked
                    | DatabaseUpgradePreflightStatus::NewerSchema
            )),
            DatabaseUpgradeRuntimeState::Running { .. } => Ok(false),
            DatabaseUpgradeRuntimeState::Finished(result) => Ok(matches!(
                result.status,
                DatabaseUpgradeRunStatus::Blocked
                    | DatabaseUpgradeRunStatus::NewerSchema
                    | DatabaseUpgradeRunStatus::Failed
            )),
        }
    }

    pub fn start_fresh_database(&self) -> Result<PathBuf, Error> {
        let mut state = self.lock_state();
        if !self.fresh_database_available(&state)? {
            return Err(Error::Custom(
                "A fresh database is only available after an upgrade failure or for an unsupported newer schema."
                    .into(),
            ));
        }

        let recovery_dir = self.db.archive_main_database_and_create_fresh_database()?;
        *state = DatabaseUpgradeRuntimeState::Idle;
        Ok(recovery_dir)
    }

    fn run_with(
        &self,
        execute: impl FnOnce(
            &DatabaseService,
            &mut dyn FnMut(DatabaseUpgradeProgress),
        ) -> DatabaseUpgradeRunResult,
    ) -> DatabaseUpgradeRunResult {
        loop {
            let mut state = self.lock_state();
            match &*state {
                DatabaseUpgradeRuntimeState::Idle => {
                    let (from_version, to_version) = database_upgrade_preflight(&self.db)
                        .map(|preflight| (preflight.from_version, preflight.to_version))
                        .unwrap_or((0, VRCX0_SCHEMA_VERSION));
                    *state = DatabaseUpgradeRuntimeState::Running {
                        from_version,
                        to_version,
                    };
                    self.set_progress(DatabaseUpgradeProgress::indeterminate(
                        DatabaseUpgradeStage::Preflight,
                    ));
                    break;
                }
                DatabaseUpgradeRuntimeState::Running { .. } => {
                    state = self
                        .shared
                        .changed
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    drop(state);
                }
                DatabaseUpgradeRuntimeState::Finished(result) => {
                    return result.as_ref().clone();
                }
            }
        }

        self.record_running();
        let mut on_progress = |progress: DatabaseUpgradeProgress| {
            self.set_progress(progress);
        };
        let result = execute(&self.db, &mut on_progress);

        let mut state = self.lock_state();
        *state = DatabaseUpgradeRuntimeState::Finished(Box::new(result.clone()));
        self.shared.changed.notify_all();
        drop(state);
        self.record_result(&result);
        result
    }

    fn set_progress(&self, progress: DatabaseUpgradeProgress) {
        *self
            .shared
            .progress
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = progress;
    }

    fn record_running(&self) {
        self.diagnostics.record_command(
            COMMAND,
            RuntimeOperationStatus::Running,
            "Running database upgrade orchestration.",
        );
        self.background_jobs.register_job(
            JOB,
            "rust-runtime",
            None,
            RuntimeOperationStatus::Running,
            "Running database upgrade orchestration.",
        );
    }

    fn record_result(&self, result: &DatabaseUpgradeRunResult) {
        match result.status {
            DatabaseUpgradeRunStatus::Current | DatabaseUpgradeRunStatus::Upgraded => {
                self.diagnostics.record_command(
                    COMMAND,
                    RuntimeOperationStatus::Ok,
                    format!(
                        "status={:?}, from={}, to={}",
                        result.status, result.from_version, result.to_version
                    ),
                );
                self.background_jobs.mark_completed(
                    JOB,
                    format!(
                        "Database upgrade orchestration finished with status {:?}.",
                        result.status
                    ),
                );
            }
            _ => {
                let detail = result.error.clone().unwrap_or_else(|| {
                    format!("Database upgrade stopped with status {:?}.", result.status)
                });
                self.diagnostics.record_command(
                    COMMAND,
                    RuntimeOperationStatus::Error,
                    detail.clone(),
                );
                self.background_jobs.mark_failed(JOB, detail);
            }
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, DatabaseUpgradeRuntimeState> {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;

    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-database-upgrade-runtime-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn success_result() -> DatabaseUpgradeRunResult {
        DatabaseUpgradeRunResult {
            status: DatabaseUpgradeRunStatus::Upgraded,
            from_version: 17,
            to_version: VRCX0_SCHEMA_VERSION,
            failed_stage: None,
            error: None,
            failed_upgrade: None,
            repair_warning: None,
        }
    }

    #[test]
    fn rebuilt_frontend_observes_and_joins_the_active_run() {
        let dir = TestDir::new();
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap());
        let background_jobs = RuntimeBackgroundJobs::new();
        let runtime =
            DatabaseUpgradeRuntime::new(db, RuntimeDiagnostics::new(), background_jobs.clone());
        let leader_runtime = runtime.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let leader = std::thread::spawn(move || {
            leader_runtime.run_with(|_, on_progress| {
                on_progress(DatabaseUpgradeProgress::indeterminate(
                    DatabaseUpgradeStage::Optimize,
                ));
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                success_result()
            })
        });
        started_rx.recv().unwrap();

        let preflight = runtime.preflight().unwrap();
        assert_eq!(preflight.status, DatabaseUpgradePreflightStatus::Running);
        assert_eq!(preflight.stage, Some(DatabaseUpgradeStage::Optimize));

        let waiter_runtime = runtime.clone();
        let waiter = std::thread::spawn(move || waiter_runtime.run());
        release_tx.send(()).unwrap();

        assert_eq!(
            leader.join().unwrap().status,
            DatabaseUpgradeRunStatus::Upgraded
        );
        assert_eq!(
            waiter.join().unwrap().status,
            DatabaseUpgradeRunStatus::Upgraded
        );
        let finished = runtime.preflight().unwrap();
        assert_eq!(finished.status, DatabaseUpgradePreflightStatus::Finished);
        assert_eq!(
            finished.result.unwrap().status,
            DatabaseUpgradeRunStatus::Upgraded
        );
        let job = background_jobs
            .snapshot()
            .into_iter()
            .find(|job| job.name == JOB)
            .expect("database upgrade background job");
        assert_eq!(job.status, RuntimeOperationStatus::Idle);
        assert!(job.last_finished_at.is_some());
        assert!(job.last_detail.contains("Upgraded"));
    }

    #[test]
    fn failed_run_can_be_explicitly_retried_in_the_same_process() {
        let dir = TestDir::new();
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap());
        let runtime = DatabaseUpgradeRuntime::new(
            db,
            RuntimeDiagnostics::new(),
            RuntimeBackgroundJobs::new(),
        );
        let failed = DatabaseUpgradeRunResult {
            status: DatabaseUpgradeRunStatus::Failed,
            from_version: 0,
            to_version: VRCX0_SCHEMA_VERSION,
            failed_stage: Some(DatabaseUpgradeStage::Optimize),
            error: Some("injected failure".into()),
            failed_upgrade: None,
            repair_warning: None,
        };
        *runtime.lock_state() = DatabaseUpgradeRuntimeState::Finished(Box::new(failed));

        let retried = runtime.retry().unwrap();

        assert_eq!(retried.status, DatabaseUpgradeRunStatus::Upgraded);
        assert_eq!(
            runtime.preflight().unwrap().result.unwrap().status,
            DatabaseUpgradeRunStatus::Upgraded
        );
    }

    #[test]
    fn failed_run_can_archive_the_old_database_and_reset_runtime_state() {
        let dir = TestDir::new();
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap());
        let runtime = DatabaseUpgradeRuntime::new(
            db,
            RuntimeDiagnostics::new(),
            RuntimeBackgroundJobs::new(),
        );
        *runtime.lock_state() =
            DatabaseUpgradeRuntimeState::Finished(Box::new(DatabaseUpgradeRunResult {
                status: DatabaseUpgradeRunStatus::Failed,
                from_version: 17,
                to_version: VRCX0_SCHEMA_VERSION,
                failed_stage: Some(DatabaseUpgradeStage::Optimize),
                error: Some("injected failure".into()),
                failed_upgrade: None,
                repair_warning: None,
            }));

        let recovery_dir = runtime.start_fresh_database().unwrap();

        assert!(recovery_dir.join("VRCX-0.sqlite3").is_file());
        assert_eq!(
            runtime.preflight().unwrap().status,
            DatabaseUpgradePreflightStatus::UpgradeRequired
        );
    }
}
