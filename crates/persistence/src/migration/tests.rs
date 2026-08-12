use sea_query::{ColumnDef, Index, Table};

use super::*;
use crate::database::DatabaseService;
use crate::Error;

struct TestDir {
    path: std::path::PathBuf,
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn test_db(name: &str) -> (TestDir, DatabaseService) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("vrcx0-migration-{name}-{nonce}"));
    std::fs::create_dir_all(&path).unwrap();
    let db = DatabaseService::new(&path.join("test.sqlite3")).unwrap();
    (TestDir { path }, db)
}

fn exec(db: &DatabaseService, sql: &str) {
    db.execute_non_query(sql, &Default::default()).unwrap();
}

fn set_migration_version(db: &DatabaseService, version: i64) {
    exec(db, &format!("PRAGMA user_version = {version}"));
}

fn table_exists(db: &DatabaseService, name: &str) -> bool {
    db.execute(
        &format!("SELECT name FROM sqlite_master WHERE type='table' AND name='{name}'"),
        &Default::default(),
    )
    .map(|rows| !rows.is_empty())
    .unwrap_or(false)
}

fn create_marker_table(version: i64) -> Migration {
    Migration::new(version, "create marker").step(Step::ddl(
        Table::create()
            .table(sea_query::Alias::new(format!("marker_{version}")))
            .col(ColumnDef::new("id").integer().primary_key())
            .to_owned(),
    ))
}

fn per_user_index_migration() -> Migration {
    Migration::new(1, "per-user index").step(Step::per_user("feed_gps", |table| {
        vec![Index::create()
            .name(format!("{table}_user_idx"))
            .table(sea_query::Alias::new(table))
            .col("user_id")
            .to_string(sea_query::SqliteQueryBuilder)]
    }))
}

fn user_index_names(db: &DatabaseService) -> Vec<String> {
    db.execute(
        "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE '%_user_idx' ORDER BY name",
        &Default::default(),
    )
    .unwrap()
    .iter()
    .filter_map(|row| row.first())
    .filter_map(|value| value.as_str().map(str::to_owned))
    .collect()
}

#[test]
fn applies_pending_migrations_and_advances_version() {
    let (_dir, db) = test_db("apply");

    let report = run(
        &db,
        &[create_marker_table(1), create_marker_table(2)],
        &NoopProgress,
    )
    .unwrap();

    assert_eq!(report.from_version, 0);
    assert_eq!(report.to_version, 2);
    assert_eq!(report.applied, vec![1, 2]);
    assert!(table_exists(&db, "marker_1"));
    assert!(table_exists(&db, "marker_2"));
    assert_eq!(migration_version(&db).unwrap(), 2);
}

#[test]
fn skips_migrations_at_or_below_current_version() {
    let (_dir, db) = test_db("skip");
    set_migration_version(&db, 1);

    let report = run(
        &db,
        &[create_marker_table(1), create_marker_table(2)],
        &NoopProgress,
    )
    .unwrap();

    assert_eq!(report.applied, vec![2]);
    assert!(!table_exists(&db, "marker_1"));
    assert!(table_exists(&db, "marker_2"));
}

#[test]
fn rerunning_is_a_noop() {
    let (_dir, db) = test_db("noop");
    let migrations = || vec![create_marker_table(1)];

    run(&db, &migrations(), &NoopProgress).unwrap();
    let second = run(&db, &migrations(), &NoopProgress).unwrap();

    assert!(second.applied.is_empty());
    assert_eq!(second.to_version, 1);
}

#[test]
fn failed_step_rolls_back_and_keeps_version() {
    let (_dir, db) = test_db("rollback");

    let broken = create_marker_table(1).step(Step::raw("THIS IS NOT SQL"));

    let error = run(&db, &[broken], &NoopProgress).unwrap_err();

    assert!(matches!(error, Error::Database(_)));
    assert!(!table_exists(&db, "marker_1"));
    assert_eq!(migration_version(&db).unwrap(), 0);
}

#[test]
fn failed_verify_rolls_back_the_whole_version() {
    let (_dir, db) = test_db("verify");

    let migration = create_marker_table(1)
        .verify(|_tx, _target| Err(Error::Database("verification failed".into())));

    let error = run(&db, &[migration], &NoopProgress).unwrap_err();

    assert!(matches!(error, Error::Database(_)));
    assert!(!table_exists(&db, "marker_1"));
    assert_eq!(migration_version(&db).unwrap(), 0);
}

#[test]
fn stops_at_first_failure_and_keeps_earlier_versions() {
    let (_dir, db) = test_db("stop");

    let error = run(
        &db,
        &[
            create_marker_table(1),
            Migration::new(2, "broken").step(Step::raw("NOT SQL")),
            create_marker_table(3),
        ],
        &NoopProgress,
    )
    .unwrap_err();

    assert!(matches!(error, Error::Database(_)));
    assert!(table_exists(&db, "marker_1"));
    assert!(!table_exists(&db, "marker_3"));
    assert_eq!(migration_version(&db).unwrap(), 1);
}

#[test]
fn expands_per_user_tables() {
    let (_dir, db) = test_db("per-user");
    for prefix in ["usraaa", "usrbbb"] {
        exec(
            &db,
            &format!("CREATE TABLE {prefix}_feed_gps (id INTEGER PRIMARY KEY, user_id TEXT)"),
        );
    }
    exec(&db, "CREATE TABLE usrccc_feed_bio (id INTEGER PRIMARY KEY)");

    run(&db, &[per_user_index_migration()], &NoopProgress).unwrap();

    assert_eq!(
        user_index_names(&db),
        vec!["usraaa_feed_gps_user_idx", "usrbbb_feed_gps_user_idx"]
    );
}

#[test]
fn ignores_tables_whose_prefix_is_not_a_bare_user_id() {
    let (_dir, db) = test_db("per-user-boundary");
    for table in [
        "usraaa_feed_gps",
        "usraaa_extra_feed_gps",
        "usr_feed_gps",
        "notusr_feed_gps",
        "usrbbb_gps",
    ] {
        exec(
            &db,
            &format!("CREATE TABLE {table} (id INTEGER PRIMARY KEY, user_id TEXT)"),
        );
    }

    run(&db, &[per_user_index_migration()], &NoopProgress).unwrap();

    assert_eq!(user_index_names(&db), vec!["usraaa_feed_gps_user_idx"]);
}

#[test]
fn per_user_custom_receives_table_and_prefix() {
    let (_dir, db) = test_db("per-user-custom");
    exec(&db, "CREATE TABLE usrzzz_feed_gps (id INTEGER PRIMARY KEY)");

    run(
        &db,
        &[
            Migration::new(1, "custom").step(Step::per_user_custom("feed_gps", |tx, target| {
                let table = target.require_table()?;
                assert_eq!(table, "usrzzz_feed_gps");
                assert_eq!(target.user_prefix, Some("usrzzz"));
                tx.execute_non_query(&format!("INSERT INTO {table} (id) VALUES (1)"))?;
                Ok(())
            })),
        ],
        &NoopProgress,
    )
    .unwrap();

    let rows = db
        .execute("SELECT id FROM usrzzz_feed_gps", &Default::default())
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn runs_inside_an_upgrade_work_copy() {
    let (_dir, db) = test_db("work-copy");
    crate::write_database_schema_versions(&db, crate::VRCX0_SCHEMA_VERSION).unwrap();
    db.begin_upgrade_with_progress(
        crate::VRCX0_SCHEMA_VERSION,
        crate::VRCX0_SCHEMA_VERSION,
        None,
        None,
        None,
        |_, _| {},
    )
    .unwrap();

    run(&db, &[create_marker_table(1)], &NoopProgress).unwrap();

    assert!(table_exists(&db, "marker_1"));
    assert_eq!(migration_version(&db).unwrap(), 1);

    db.commit_upgrade().unwrap();

    assert!(table_exists(&db, "marker_1"));
    assert_eq!(migration_version(&db).unwrap(), 1);
}

#[test]
fn preview_reports_pending_without_touching_the_database() {
    let (_dir, db) = test_db("preview");

    let result = preview(&db, &[create_marker_table(1), create_marker_table(2)]).unwrap();

    assert_eq!(result.status, PreviewStatus::Pending);
    assert_eq!(result.current_version, 0);
    assert_eq!(result.target_version, 2);
    assert_eq!(
        result
            .pending
            .iter()
            .map(|entry| entry.version)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(!table_exists(&db, "marker_1"));
}

#[test]
fn preview_detects_newer_schema() {
    let (_dir, db) = test_db("newer");
    set_migration_version(&db, 30);

    let result = preview(&db, &[create_marker_table(1)]).unwrap();

    assert_eq!(result.status, PreviewStatus::NewerSchema);
    assert!(result.pending.is_empty());
}

#[test]
fn run_rejects_a_database_newer_than_the_list() {
    let (_dir, db) = test_db("newer-run");
    set_migration_version(&db, 30);

    let error = run(&db, &[create_marker_table(1)], &NoopProgress).unwrap_err();

    assert!(matches!(error, Error::Database(_)));
    assert_eq!(migration_version(&db).unwrap(), 30);
}

#[test]
fn rejects_migration_versions_below_one() {
    let (_dir, db) = test_db("below-one");

    for list in [
        vec![create_marker_table(0), create_marker_table(1)],
        vec![create_marker_table(-1)],
    ] {
        assert!(matches!(
            run(&db, &list, &NoopProgress).unwrap_err(),
            Error::Database(_)
        ));
        assert!(matches!(
            preview(&db, &list).unwrap_err(),
            Error::Database(_)
        ));
    }
    assert!(!table_exists(&db, "marker_1"));
}

#[test]
fn rejects_unordered_migration_list() {
    let (_dir, db) = test_db("unordered");

    let error = preview(&db, &[create_marker_table(2), create_marker_table(1)]).unwrap_err();

    assert!(matches!(error, Error::Database(_)));
}
