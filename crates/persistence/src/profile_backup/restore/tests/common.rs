use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::database::schema::VRCX0_SCHEMA_VERSION;
use crate::profile_backup::{
    create_backup_archive, ProfileBackupContent, ProfileBackupKind, ProfileBackupManifest,
    ProfileBackupManifestMetadata, ProfileRestoreFailureCode, ProfileRestoreValidation,
    DATABASE_FILE_NAME,
};

use super::super::super::archive::sha256_hex;
use super::super::{request_staged_profile_restore, validate_and_stage_profile_restore};

pub(super) struct TestDir(pub(super) PathBuf);

impl TestDir {
    pub(super) fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vrcx-0-profile-restore-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn write_profile_database(path: &Path, version: i64, value: &str) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE configs (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         CREATE TABLE restore_value (value TEXT NOT NULL);",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO configs (key, value) VALUES ('config:vrcx_0_databaseversion', ?1)",
        [version.to_string()],
    )
    .unwrap();
    conn.execute("INSERT INTO restore_value (value) VALUES (?1)", [value])
        .unwrap();
}

pub(super) fn read_restore_value(path: &Path) -> String {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .query_row("SELECT value FROM restore_value", [], |row| row.get(0))
        .unwrap()
}

pub(super) fn create_profile_backup(
    root: &Path,
    name: &str,
    app_version: &str,
    manifest_db_version: i64,
    database_version: i64,
    value: &str,
) -> PathBuf {
    let snapshot = root.join(format!("{name}.sqlite3"));
    let archive = root.join(format!("{name}.vrcx0backup"));
    write_profile_database(&snapshot, database_version, value);
    create_backup_archive(
        &snapshot,
        &archive,
        ProfileBackupManifestMetadata {
            app_version: app_version.into(),
            db_version: manifest_db_version,
            created_at: "2026-07-14T07:30:00Z".into(),
            platform: "windows".into(),
            kind: ProfileBackupKind::Manual,
        },
    )
    .unwrap();
    archive
}

pub(super) fn accepted_validation(
    source: &Path,
    app_data: &Path,
    current_app_version: &str,
) -> ProfileRestoreValidation {
    let outcome =
        validate_and_stage_profile_restore(source, app_data, current_app_version).unwrap();
    assert!(outcome.failure.is_none());
    outcome.validation.unwrap()
}

pub(super) fn rejected_code(
    source: &Path,
    app_data: &Path,
    current_app_version: &str,
) -> ProfileRestoreFailureCode {
    validate_and_stage_profile_restore(source, app_data, current_app_version)
        .unwrap()
        .failure
        .unwrap()
        .code
}

pub(super) fn manifest_for_database(bytes: &[u8]) -> ProfileBackupManifest {
    ProfileBackupManifest {
        manifest_version: 1,
        app_version: "1.2.3".into(),
        db_version: VRCX0_SCHEMA_VERSION,
        created_at: "2026-07-14T07:30:00Z".into(),
        platform: "windows".into(),
        kind: ProfileBackupKind::Manual,
        contents: vec![ProfileBackupContent {
            path: DATABASE_FILE_NAME.into(),
            sha256: sha256_hex(Sha256::digest(bytes)),
            bytes: bytes.len() as u64,
        }],
    }
}

pub(super) fn write_custom_archive(path: &Path, entries: Vec<(String, Vec<u8>)>) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().sync_all().unwrap();
}

pub(super) fn prepare_restore(
    dir: &TestDir,
    name: &str,
) -> (PathBuf, PathBuf, ProfileRestoreValidation) {
    let app_data = dir.0.join(format!("app-{name}"));
    fs::create_dir_all(&app_data).unwrap();
    let db_path = app_data.join(DATABASE_FILE_NAME);
    write_profile_database(&db_path, VRCX0_SCHEMA_VERSION, "old");
    let source = create_profile_backup(
        &dir.0,
        &format!("source-{name}"),
        "1.2.3",
        VRCX0_SCHEMA_VERSION,
        VRCX0_SCHEMA_VERSION,
        "new",
    );
    let validation = accepted_validation(&source, &app_data, "1.2.3");
    request_staged_profile_restore(&app_data, &validation).unwrap();
    (app_data, db_path, validation)
}
