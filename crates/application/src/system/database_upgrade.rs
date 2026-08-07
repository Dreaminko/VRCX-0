use serde::Serialize;
use vrcx_0_persistence::maintenance::{
    database_maintenance_run, ensure_required_database_schema, DatabaseMaintenanceTask,
};
use vrcx_0_persistence::migration::{
    preview as preview_migrations, run as run_schema_migrations, NoopProgress, Preview,
    PreviewStatus,
};
use vrcx_0_persistence::migrations::migrations;
use vrcx_0_persistence::{
    prepare_vrcx0_schema_version, write_database_schema_versions, DatabaseService,
    DatabaseUpgradeStatus, VRCX0_SCHEMA_VERSION,
};

use crate::Error;

const LEGACY_SCHEMA_VERSION: i64 = 16;
const COPRESENCE_DURATION_REPAIR_KEY: &str = "copresenceDurationRepairV1Done";
const LEGACY_DATA_CLEANUP_TASKS: &[DatabaseMaintenanceTask] = &[
    DatabaseMaintenanceTask::CleanLegendFromFriendLog,
    DatabaseMaintenanceTask::FixGameLogTraveling,
    DatabaseMaintenanceTask::FixNegativeGPS,
    DatabaseMaintenanceTask::FixBrokenLeaveEntries,
    DatabaseMaintenanceTask::FixBrokenGroupInvites,
    DatabaseMaintenanceTask::FixBrokenNotifications,
    DatabaseMaintenanceTask::FixBrokenGroupChange,
    DatabaseMaintenanceTask::FixCancelFriendRequestTypo,
    DatabaseMaintenanceTask::FixBrokenGameLogDisplayNames,
];
const LEGACY_SCHEMA_MIGRATION_TASKS: &[DatabaseMaintenanceTask] = &[
    DatabaseMaintenanceTask::UpdateTableForGroupNames,
    DatabaseMaintenanceTask::AddFriendLogFriendNumber,
    DatabaseMaintenanceTask::UpdateTableForAvatarHistory,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseUpgradePreflightStatus {
    Current,
    UpgradeRequired,
    Running,
    Finished,
    Blocked,
    NewerSchema,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseUpgradePreflight {
    pub status: DatabaseUpgradePreflightStatus,
    pub from_version: i64,
    pub to_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<DatabaseUpgradeStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<DatabaseUpgradeRunResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_upgrade: Option<DatabaseUpgradeStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseUpgradeRunStatus {
    Current,
    Upgraded,
    Blocked,
    NewerSchema,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseUpgradeStage {
    Preflight,
    PrepareLegacySnapshot,
    PrepareLegacyConfiguration,
    FinalizeLegacyMigration,
    InitializeSchema,
    CreateWorkCopy,
    LegacySchemaMigration,
    LegacyPerformanceIndexes,
    GlobalPerformanceIndexes,
    NotificationPerformanceIndexes,
    SchemaMigrations,
    Optimize,
    WriteVersion,
    Commit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseUpgradeProgress {
    pub stage: DatabaseUpgradeStage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_units: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_units: Option<u64>,
}

impl DatabaseUpgradeProgress {
    pub(super) fn indeterminate(stage: DatabaseUpgradeStage) -> Self {
        Self {
            stage,
            completed_units: None,
            total_units: None,
        }
    }

    pub(super) fn determinate(
        stage: DatabaseUpgradeStage,
        completed_units: u64,
        total_units: u64,
    ) -> Self {
        Self {
            stage,
            completed_units: Some(completed_units),
            total_units: Some(total_units),
        }
    }
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseUpgradeRunResult {
    pub status: DatabaseUpgradeRunStatus,
    pub from_version: i64,
    pub to_version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_stage: Option<DatabaseUpgradeStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_upgrade: Option<DatabaseUpgradeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_warning: Option<String>,
}

struct UpgradeFailure {
    from_version: i64,
    stage: DatabaseUpgradeStage,
    error: Error,
    upgrade_started: bool,
}

impl UpgradeFailure {
    fn before_upgrade(
        from_version: i64,
        stage: DatabaseUpgradeStage,
        error: impl Into<Error>,
    ) -> Self {
        Self {
            from_version,
            stage,
            error: error.into(),
            upgrade_started: false,
        }
    }

    fn during_upgrade(
        from_version: i64,
        stage: DatabaseUpgradeStage,
        error: impl Into<Error>,
    ) -> Self {
        Self {
            from_version,
            stage,
            error: error.into(),
            upgrade_started: true,
        }
    }
}

pub fn database_upgrade_preflight(db: &DatabaseService) -> Result<DatabaseUpgradePreflight, Error> {
    if let Some(failed_upgrade) = db.get_failed_upgrade()? {
        return Ok(DatabaseUpgradePreflight {
            status: DatabaseUpgradePreflightStatus::Blocked,
            from_version: failed_upgrade.from_version,
            to_version: failed_upgrade.to_version,
            stage: None,
            result: None,
            failed_upgrade: Some(failed_upgrade),
        });
    }

    let schema_version = prepare_vrcx0_schema_version(db)?;
    let migrations = migration_preview(db)?;
    let (status, from_version, to_version) = if schema_version > VRCX0_SCHEMA_VERSION {
        (
            DatabaseUpgradePreflightStatus::NewerSchema,
            schema_version,
            VRCX0_SCHEMA_VERSION,
        )
    } else if migrations.status == PreviewStatus::NewerSchema {
        (
            DatabaseUpgradePreflightStatus::NewerSchema,
            migrations.current_version,
            migrations.target_version,
        )
    } else if schema_version < VRCX0_SCHEMA_VERSION || migrations.status == PreviewStatus::Pending {
        (
            DatabaseUpgradePreflightStatus::UpgradeRequired,
            migrations.current_version,
            migrations.target_version,
        )
    } else {
        (
            DatabaseUpgradePreflightStatus::Current,
            migrations.current_version,
            migrations.target_version,
        )
    };

    Ok(DatabaseUpgradePreflight {
        status,
        from_version,
        to_version,
        stage: None,
        result: None,
        failed_upgrade: None,
    })
}

fn migration_preview(db: &DatabaseService) -> Result<Preview, Error> {
    Ok(preview_migrations(db, &migrations())?)
}

fn target_migration_version() -> i64 {
    migrations()
        .last()
        .map(|migration| migration.version)
        .unwrap_or(0)
}

pub fn run_database_upgrade(db: &DatabaseService) -> DatabaseUpgradeRunResult {
    run_database_upgrade_with_progress(db, |_| {})
}

pub(super) fn run_database_upgrade_with_progress(
    db: &DatabaseService,
    mut on_progress: impl FnMut(DatabaseUpgradeProgress),
) -> DatabaseUpgradeRunResult {
    match run_database_upgrade_inner(db, &mut on_progress) {
        Ok(mut result) => {
            if matches!(
                result.status,
                DatabaseUpgradeRunStatus::Current | DatabaseUpgradeRunStatus::Upgraded
            ) {
                result.repair_warning = run_copresence_duration_repair_once(db).err();
            }
            result
        }
        Err(failure) => recover_failed_upgrade(db, failure),
    }
}

fn run_database_upgrade_inner(
    db: &DatabaseService,
    on_progress: &mut impl FnMut(DatabaseUpgradeProgress),
) -> Result<DatabaseUpgradeRunResult, UpgradeFailure> {
    report_stage(on_progress, DatabaseUpgradeStage::Preflight);
    let preflight = database_upgrade_preflight(db).map_err(|error| {
        let from_version = prepare_vrcx0_schema_version(db).unwrap_or(0);
        UpgradeFailure::before_upgrade(from_version, DatabaseUpgradeStage::Preflight, error)
    })?;
    let from_version = preflight.from_version;
    let to_version = preflight.to_version;
    let schema_version = prepare_vrcx0_schema_version(db).map_err(|error| {
        UpgradeFailure::before_upgrade(from_version, DatabaseUpgradeStage::Preflight, error)
    })?;

    match preflight.status {
        DatabaseUpgradePreflightStatus::Blocked => {
            let Some(failed_upgrade) = preflight.failed_upgrade else {
                return Err(UpgradeFailure::before_upgrade(
                    from_version,
                    DatabaseUpgradeStage::Preflight,
                    Error::Custom("Blocked database upgrade has no failure status.".into()),
                ));
            };
            return Ok(DatabaseUpgradeRunResult {
                status: DatabaseUpgradeRunStatus::Blocked,
                from_version: failed_upgrade.from_version,
                to_version: failed_upgrade.to_version,
                failed_stage: None,
                error: failed_upgrade.reason.clone(),
                failed_upgrade: Some(failed_upgrade),
                repair_warning: None,
            });
        }
        DatabaseUpgradePreflightStatus::NewerSchema => {
            return Ok(DatabaseUpgradeRunResult {
                status: DatabaseUpgradeRunStatus::NewerSchema,
                from_version,
                to_version,
                failed_stage: None,
                error: Some(format!(
                    "Database version {from_version} is newer than this application supports ({to_version})."
                )),
                failed_upgrade: None,
                repair_warning: None,
            });
        }
        DatabaseUpgradePreflightStatus::Current => {
            report_stage(on_progress, DatabaseUpgradeStage::InitializeSchema);
            ensure_required_database_schema(db).map_err(|error| {
                UpgradeFailure::before_upgrade(
                    from_version,
                    DatabaseUpgradeStage::InitializeSchema,
                    error,
                )
            })?;
            return Ok(success_result(
                DatabaseUpgradeRunStatus::Current,
                from_version,
            ));
        }
        DatabaseUpgradePreflightStatus::UpgradeRequired => {}
        DatabaseUpgradePreflightStatus::Running | DatabaseUpgradePreflightStatus::Finished => {
            return Err(UpgradeFailure::before_upgrade(
                from_version,
                DatabaseUpgradeStage::Preflight,
                Error::Custom(
                    "Runtime-only database upgrade state reached the static runner.".into(),
                ),
            ));
        }
    }

    report_stage(on_progress, DatabaseUpgradeStage::CreateWorkCopy);
    db.begin_upgrade_with_progress(schema_version, VRCX0_SCHEMA_VERSION, |completed, total| {
        on_progress(DatabaseUpgradeProgress::determinate(
            DatabaseUpgradeStage::CreateWorkCopy,
            completed,
            total,
        ));
    })
    .map_err(|error| {
        UpgradeFailure::before_upgrade(from_version, DatabaseUpgradeStage::CreateWorkCopy, error)
    })?;

    report_stage(on_progress, DatabaseUpgradeStage::InitializeSchema);
    ensure_required_database_schema(db).map_err(|error| {
        UpgradeFailure::during_upgrade(from_version, DatabaseUpgradeStage::InitializeSchema, error)
    })?;

    if schema_version < LEGACY_SCHEMA_VERSION {
        report_stage(on_progress, DatabaseUpgradeStage::LegacySchemaMigration);
        for &task in LEGACY_DATA_CLEANUP_TASKS {
            run_task(db, task).map_err(|error| {
                UpgradeFailure::during_upgrade(
                    from_version,
                    DatabaseUpgradeStage::LegacySchemaMigration,
                    error,
                )
            })?;
        }
        for &task in LEGACY_SCHEMA_MIGRATION_TASKS {
            run_task(db, task).map_err(|error| {
                UpgradeFailure::during_upgrade(
                    from_version,
                    DatabaseUpgradeStage::LegacySchemaMigration,
                    error,
                )
            })?;
        }
    }

    run_required_task(
        db,
        on_progress,
        from_version,
        DatabaseUpgradeStage::LegacyPerformanceIndexes,
        DatabaseMaintenanceTask::AddLegacyPerformanceIndexes,
    )?;
    run_required_task(
        db,
        on_progress,
        from_version,
        DatabaseUpgradeStage::GlobalPerformanceIndexes,
        DatabaseMaintenanceTask::AddV17GlobalPerformanceIndexes,
    )?;
    run_required_task(
        db,
        on_progress,
        from_version,
        DatabaseUpgradeStage::NotificationPerformanceIndexes,
        DatabaseMaintenanceTask::AddNotificationPerformanceIndexes,
    )?;

    report_stage(on_progress, DatabaseUpgradeStage::SchemaMigrations);
    run_schema_migrations(db, &migrations(), &NoopProgress).map_err(|error| {
        UpgradeFailure::during_upgrade(from_version, DatabaseUpgradeStage::SchemaMigrations, error)
    })?;

    run_optional_task(
        db,
        on_progress,
        DatabaseUpgradeStage::Optimize,
        DatabaseMaintenanceTask::Optimize,
    );
    report_stage(on_progress, DatabaseUpgradeStage::WriteVersion);
    write_database_schema_versions(db, VRCX0_SCHEMA_VERSION).map_err(|error| {
        UpgradeFailure::during_upgrade(from_version, DatabaseUpgradeStage::WriteVersion, error)
    })?;
    report_stage(on_progress, DatabaseUpgradeStage::Commit);
    db.commit_upgrade().map_err(|error| {
        UpgradeFailure::during_upgrade(from_version, DatabaseUpgradeStage::Commit, error)
    })?;

    Ok(success_result(
        DatabaseUpgradeRunStatus::Upgraded,
        from_version,
    ))
}

fn report_stage(
    on_progress: &mut impl FnMut(DatabaseUpgradeProgress),
    stage: DatabaseUpgradeStage,
) {
    on_progress(DatabaseUpgradeProgress::indeterminate(stage));
}

fn run_optional_task(
    db: &DatabaseService,
    on_progress: &mut impl FnMut(DatabaseUpgradeProgress),
    stage: DatabaseUpgradeStage,
    task: DatabaseMaintenanceTask,
) {
    report_stage(on_progress, stage);
    if let Err(error) = run_task(db, task) {
        tracing::warn!(?stage, error = %error, "optional database upgrade task failed");
    }
}

fn run_required_task(
    db: &DatabaseService,
    on_progress: &mut impl FnMut(DatabaseUpgradeProgress),
    from_version: i64,
    stage: DatabaseUpgradeStage,
    task: DatabaseMaintenanceTask,
) -> Result<(), UpgradeFailure> {
    report_stage(on_progress, stage);
    run_task(db, task).map_err(|error| UpgradeFailure::during_upgrade(from_version, stage, error))
}

fn run_task(db: &DatabaseService, task: DatabaseMaintenanceTask) -> Result<(), Error> {
    database_maintenance_run(db, task).map_err(Error::from)
}

fn success_result(status: DatabaseUpgradeRunStatus, from_version: i64) -> DatabaseUpgradeRunResult {
    DatabaseUpgradeRunResult {
        status,
        from_version,
        to_version: target_migration_version(),
        failed_stage: None,
        error: None,
        failed_upgrade: None,
        repair_warning: None,
    }
}

fn recover_failed_upgrade(
    db: &DatabaseService,
    failure: UpgradeFailure,
) -> DatabaseUpgradeRunResult {
    let mut error = failure.error.to_string();
    if failure.upgrade_started {
        if let Err(recovery_error) = db.fail_upgrade(error.clone()) {
            error = format!(
                "{error} Failed to preserve the database upgrade work copy: {recovery_error}"
            );
        }
    }

    let failed_upgrade = match db.get_failed_upgrade() {
        Ok(status) => status,
        Err(status_error) => {
            error = format!("{error} Failed to read database upgrade status: {status_error}");
            None
        }
    };

    DatabaseUpgradeRunResult {
        status: DatabaseUpgradeRunStatus::Failed,
        from_version: failure.from_version,
        to_version: target_migration_version(),
        failed_stage: Some(failure.stage),
        error: Some(error),
        failed_upgrade,
        repair_warning: None,
    }
}

fn run_copresence_duration_repair_once(db: &DatabaseService) -> Result<(), String> {
    let done = vrcx_0_persistence::config::get_string(db, COPRESENCE_DURATION_REPAIR_KEY, "")
        .map_err(|error| error.to_string())?;
    if done == "1" {
        return Ok(());
    }

    run_task(db, DatabaseMaintenanceTask::RepairZeroCopresenceDurations)
        .map_err(|error| error.to_string())?;
    vrcx_0_persistence::config::set_string(db, COPRESENCE_DURATION_REPAIR_KEY, "1")
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use vrcx_0_persistence::migration::migration_version;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("vrcx-0-{name}-{}-{nonce}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn database(&self) -> DatabaseService {
            DatabaseService::new(&self.path.join("VRCX-0.sqlite3")).unwrap()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn set_version(db: &DatabaseService, version: i64) {
        write_database_schema_versions(db, version).unwrap();
    }

    fn set_migration_version(db: &DatabaseService, version: i64) {
        rusqlite::Connection::open(db.db_path())
            .unwrap()
            .execute_batch(&format!("PRAGMA user_version = {version}"))
            .unwrap();
    }

    fn install_failing_repair_fixture(db: &DatabaseService) {
        database_maintenance_run(db, DatabaseMaintenanceTask::InitGlobalTables).unwrap();
        let conn = rusqlite::Connection::open(db.db_path()).unwrap();
        conn.execute_batch(
            "INSERT INTO gamelog_join_leave
                 (created_at, type, display_name, location, user_id, time)
             VALUES
                 ('2026-01-01T00:00:00Z', 'OnPlayerJoined', 'Test', 'wrld_test:1', 'usr_test', 0),
                 ('2026-01-01T00:01:00Z', 'OnPlayerLeft', 'Test', 'wrld_test:1', 'usr_test', 0);
             CREATE TRIGGER reject_copresence_repair
             BEFORE UPDATE OF time ON gamelog_join_leave
             BEGIN
                 SELECT RAISE(FAIL, 'repair should not run');
             END;",
        )
        .unwrap();
    }

    #[test]
    fn preflight_reports_current_upgrade_and_newer_spans() {
        use DatabaseUpgradePreflightStatus::{Current, NewerSchema, UpgradeRequired};

        let target = target_migration_version();
        let schema = VRCX0_SCHEMA_VERSION;
        for (schema_version, migration, expected, span) in [
            (0, target, UpgradeRequired, (target, target)),
            (16, target, UpgradeRequired, (target, target)),
            (schema, target, Current, (target, target)),
            (schema + 1, target, NewerSchema, (schema + 1, schema)),
            (schema, target + 1, NewerSchema, (target + 1, target)),
        ] {
            let dir = TestDir::new(&format!(
                "database-upgrade-preflight-{schema_version}-{migration}"
            ));
            let db = dir.database();
            if schema_version > 0 {
                set_version(&db, schema_version);
            }
            set_migration_version(&db, migration);

            let preflight = database_upgrade_preflight(&db).unwrap();

            assert_eq!(preflight.status, expected);
            assert_eq!((preflight.from_version, preflight.to_version), span);
        }
    }

    #[test]
    fn upgrades_every_supported_old_version_span_and_is_idempotent() {
        for version in [0, 15, 16, 17] {
            let dir = TestDir::new(&format!("database-upgrade-span-{version}"));
            let db = dir.database();
            if version > 0 {
                set_version(&db, version);
            }

            let upgraded = run_database_upgrade(&db);

            assert_eq!(upgraded.status, DatabaseUpgradeRunStatus::Upgraded);
            assert_eq!(upgraded.from_version, 0);
            assert_eq!(upgraded.to_version, target_migration_version());
            assert_eq!(migration_version(&db).unwrap(), target_migration_version());
            assert_eq!(
                prepare_vrcx0_schema_version(&db).unwrap(),
                VRCX0_SCHEMA_VERSION
            );
            assert_eq!(
                vrcx_0_persistence::config::get_string(&db, "databaseVersion", "0").unwrap(),
                VRCX0_SCHEMA_VERSION.to_string()
            );
            assert_eq!(
                vrcx_0_persistence::config::get_string(&db, COPRESENCE_DURATION_REPAIR_KEY, "")
                    .unwrap(),
                "1"
            );

            let repeated = run_database_upgrade(&db);
            assert_eq!(repeated.status, DatabaseUpgradeRunStatus::Current);
            assert_eq!(repeated.from_version, target_migration_version());
        }
    }

    #[test]
    fn reports_determinate_work_copy_progress_and_indeterminate_schema_stages() {
        let dir = TestDir::new("database-upgrade-progress");
        let db = dir.database();
        set_version(&db, 17);
        rusqlite::Connection::open(db.db_path())
            .unwrap()
            .execute_batch(
                "CREATE TABLE progress_fixture (payload BLOB);
                 INSERT INTO progress_fixture (payload) VALUES (zeroblob(2097152));",
            )
            .unwrap();
        let mut progress = Vec::new();

        let result = run_database_upgrade_with_progress(&db, |snapshot| progress.push(snapshot));

        assert_eq!(result.status, DatabaseUpgradeRunStatus::Upgraded);
        assert!(progress.iter().any(|snapshot| {
            snapshot.stage == DatabaseUpgradeStage::CreateWorkCopy
                && snapshot.total_units.is_some_and(|total| total > 0)
                && snapshot.completed_units == snapshot.total_units
        }));
        assert!(progress.iter().any(|snapshot| {
            snapshot.stage == DatabaseUpgradeStage::NotificationPerformanceIndexes
                && snapshot.total_units.is_none()
        }));
    }

    #[test]
    fn legacy_upgrade_repairs_rows_that_match_old_cleanup_rules() {
        let dir = TestDir::new("database-upgrade-repairs-legacy-rows");
        let db = dir.database();
        set_version(&db, 15);
        rusqlite::Connection::open(db.db_path())
            .unwrap()
            .execute_batch(
                "CREATE TABLE usr_test_friend_log_history (
                id INTEGER PRIMARY KEY,
                created_at TEXT,
                type TEXT,
                trust_level TEXT,
                previous_trust_level TEXT,
                user_id TEXT
            );
             INSERT INTO usr_test_friend_log_history
                (created_at, type, trust_level, previous_trust_level, user_id)
             VALUES
                ('2026-01-01T00:00:00Z', 'TrustLevel', 'Veteran User', 'Trusted User', 'usr_glitch'),
                ('2026-01-01T00:00:00Z', 'CancelFriendRequst', NULL, NULL, 'usr_typo');",
            )
            .unwrap();

        let result = run_database_upgrade(&db);

        assert_eq!(result.status, DatabaseUpgradeRunStatus::Upgraded);
        let conn = rusqlite::Connection::open(db.db_path()).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM usr_test_friend_log_history WHERE user_id = 'usr_glitch'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT type FROM usr_test_friend_log_history WHERE user_id = 'usr_typo'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "CancelFriendRequest"
        );
    }

    #[test]
    fn preserves_failed_work_copy_and_blocks_reentry() {
        let dir = TestDir::new("database-upgrade-failed-copy");
        let db = dir.database();
        set_version(&db, 17);
        vrcx_0_persistence::game_log::ensure_game_log_tables(&db).unwrap();
        let conn = rusqlite::Connection::open(db.db_path()).unwrap();
        conn.execute_batch(
            "DROP TABLE gamelog_location;
             CREATE TABLE gamelog_location (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        drop(conn);

        let failed = run_database_upgrade(&db);

        assert_eq!(failed.status, DatabaseUpgradeRunStatus::Failed);
        assert_eq!(
            failed.failed_stage,
            Some(DatabaseUpgradeStage::LegacyPerformanceIndexes)
        );
        let failed_upgrade = failed.failed_upgrade.expect("failed upgrade status");
        assert!(std::path::Path::new(&failed_upgrade.work_db_path).exists());
        assert!(db.is_main_mode());
        assert_eq!(prepare_vrcx0_schema_version(&db).unwrap(), 17);

        let blocked = run_database_upgrade(&db);
        assert_eq!(blocked.status, DatabaseUpgradeRunStatus::Blocked);
        assert_eq!(blocked.from_version, 17);
        assert_eq!(blocked.to_version, VRCX0_SCHEMA_VERSION);
    }

    #[test]
    fn refuses_to_modify_a_newer_schema() {
        let dir = TestDir::new("database-upgrade-newer-schema");
        let db = dir.database();
        set_version(&db, VRCX0_SCHEMA_VERSION + 1);

        let result = run_database_upgrade(&db);

        assert_eq!(result.status, DatabaseUpgradeRunStatus::NewerSchema);
        assert_eq!(
            prepare_vrcx0_schema_version(&db).unwrap(),
            VRCX0_SCHEMA_VERSION + 1
        );
        assert!(db.get_failed_upgrade().unwrap().is_none());
    }

    #[test]
    fn one_time_repair_uses_its_own_marker_and_retries_non_fatal_failures() {
        let skipped_dir = TestDir::new("database-upgrade-repair-skipped");
        let skipped_db = skipped_dir.database();
        set_version(&skipped_db, VRCX0_SCHEMA_VERSION);
        set_migration_version(&skipped_db, target_migration_version());
        install_failing_repair_fixture(&skipped_db);
        vrcx_0_persistence::config::set_string(&skipped_db, COPRESENCE_DURATION_REPAIR_KEY, "1")
            .unwrap();

        let skipped = run_database_upgrade(&skipped_db);

        assert_eq!(skipped.status, DatabaseUpgradeRunStatus::Current);
        assert!(skipped.repair_warning.is_none());

        let retry_dir = TestDir::new("database-upgrade-repair-retry");
        let retry_db = retry_dir.database();
        set_version(&retry_db, VRCX0_SCHEMA_VERSION);
        set_migration_version(&retry_db, target_migration_version());
        install_failing_repair_fixture(&retry_db);

        let retry = run_database_upgrade(&retry_db);

        assert_eq!(retry.status, DatabaseUpgradeRunStatus::Current);
        assert!(retry
            .repair_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("repair should not run")));
        assert_eq!(
            vrcx_0_persistence::config::get_string(&retry_db, COPRESENCE_DURATION_REPAIR_KEY, "")
                .unwrap(),
            ""
        );
    }
}
