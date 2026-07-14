use std::fs::{self, File, OpenOptions};
use std::io::Write;

use rusqlite::Connection;

use crate::database::schema::VRCX0_SCHEMA_VERSION;
use crate::profile_backup::{
    create_backup_archive, ProfileBackupKind, ProfileBackupManifestMetadata,
    ProfileRestoreFailureCode, DATABASE_FILE_NAME, MANIFEST_FILE_NAME,
};

use super::super::validation::{
    copy_open_archive_with_budget, ensure_restore_archive_copy_budget, MAX_RESTORE_ARCHIVE_BYTES,
    RESTORE_FREE_SPACE_RESERVE_BYTES,
};
use super::common::{
    create_profile_backup, manifest_for_database, rejected_code, write_custom_archive,
    write_profile_database, TestDir,
};

#[test]
fn profile_backup_restore_hardening_rejects_invalid_entry_sets() {
    let dir = TestDir::new("entry-hardening");
    let db_path = dir.0.join("valid.sqlite3");
    write_profile_database(&db_path, VRCX0_SCHEMA_VERSION, "valid");
    let db_bytes = fs::read(db_path).unwrap();
    let manifest = serde_json::to_vec(&manifest_for_database(&db_bytes)).unwrap();
    let cases = [
        (
            "missing",
            vec![(DATABASE_FILE_NAME.into(), db_bytes.clone())],
            ProfileRestoreFailureCode::InvalidEntries,
        ),
        (
            "missing-database",
            vec![(MANIFEST_FILE_NAME.into(), manifest.clone())],
            ProfileRestoreFailureCode::InvalidEntries,
        ),
        (
            "extra",
            vec![
                (DATABASE_FILE_NAME.into(), db_bytes.clone()),
                (MANIFEST_FILE_NAME.into(), manifest.clone()),
                ("extra.txt".into(), Vec::new()),
            ],
            ProfileRestoreFailureCode::InvalidEntries,
        ),
        (
            "directory",
            vec![
                (format!("{DATABASE_FILE_NAME}/"), Vec::new()),
                (MANIFEST_FILE_NAME.into(), manifest.clone()),
            ],
            ProfileRestoreFailureCode::InvalidEntries,
        ),
        (
            "traversal",
            vec![
                ("../VRCX-0.sqlite3".into(), db_bytes.clone()),
                (MANIFEST_FILE_NAME.into(), manifest.clone()),
            ],
            ProfileRestoreFailureCode::InvalidEntries,
        ),
    ];

    for (name, entries, expected) in cases {
        let archive = dir.0.join(format!("{name}.vrcx0backup"));
        write_custom_archive(&archive, entries);
        let app_data = dir.0.join(format!("app-{name}"));
        assert_eq!(
            rejected_code(&archive, &app_data, "1.2.3"),
            expected,
            "{name}"
        );
    }

    let duplicate_archive = dir.0.join("duplicate.vrcx0backup");
    write_custom_archive(
        &duplicate_archive,
        vec![
            (DATABASE_FILE_NAME.into(), db_bytes.clone()),
            ("manifest.jsonx".into(), manifest),
        ],
    );
    let mut duplicate_bytes = fs::read(&duplicate_archive).unwrap();
    let old_name = b"manifest.jsonx";
    for start in 0..=duplicate_bytes.len() - old_name.len() {
        if duplicate_bytes[start..start + old_name.len()] == *old_name {
            duplicate_bytes[start..start + old_name.len()]
                .copy_from_slice(DATABASE_FILE_NAME.as_bytes());
        }
    }
    fs::write(&duplicate_archive, duplicate_bytes).unwrap();
    assert_eq!(
        rejected_code(&duplicate_archive, &dir.0.join("app-duplicate"), "1.2.3"),
        ProfileRestoreFailureCode::InvalidEntries
    );
}

#[test]
fn profile_backup_restore_rejects_hash_size_and_compatibility_failures() {
    let dir = TestDir::new("validation-failures");
    let db_path = dir.0.join("valid.sqlite3");
    write_profile_database(&db_path, VRCX0_SCHEMA_VERSION, "valid");
    let db_bytes = fs::read(&db_path).unwrap();

    let mut wrong_hash = manifest_for_database(&db_bytes);
    wrong_hash.contents[0].sha256 = "00".repeat(32);
    let wrong_hash_archive = dir.0.join("wrong-hash.vrcx0backup");
    write_custom_archive(
        &wrong_hash_archive,
        vec![
            (DATABASE_FILE_NAME.into(), db_bytes.clone()),
            (
                MANIFEST_FILE_NAME.into(),
                serde_json::to_vec(&wrong_hash).unwrap(),
            ),
        ],
    );
    assert_eq!(
        rejected_code(&wrong_hash_archive, &dir.0.join("hash-app"), "1.2.3"),
        ProfileRestoreFailureCode::ContentHashMismatch
    );

    let mut wrong_size = manifest_for_database(&db_bytes);
    wrong_size.contents[0].bytes += 1;
    let wrong_size_archive = dir.0.join("wrong-size.vrcx0backup");
    write_custom_archive(
        &wrong_size_archive,
        vec![
            (DATABASE_FILE_NAME.into(), db_bytes.clone()),
            (
                MANIFEST_FILE_NAME.into(),
                serde_json::to_vec(&wrong_size).unwrap(),
            ),
        ],
    );
    assert_eq!(
        rejected_code(&wrong_size_archive, &dir.0.join("size-app"), "1.2.3"),
        ProfileRestoreFailureCode::ContentSizeMismatch
    );

    let newer_app = create_profile_backup(
        &dir.0,
        "newer-app",
        "2.0.0",
        VRCX0_SCHEMA_VERSION,
        VRCX0_SCHEMA_VERSION,
        "new",
    );
    assert_eq!(
        rejected_code(&newer_app, &dir.0.join("newer-app-data"), "1.9.9"),
        ProfileRestoreFailureCode::NewerAppVersion
    );

    let newer_database = create_profile_backup(
        &dir.0,
        "newer-database",
        "1.2.3",
        VRCX0_SCHEMA_VERSION + 1,
        VRCX0_SCHEMA_VERSION,
        "new",
    );
    assert_eq!(
        rejected_code(&newer_database, &dir.0.join("newer-db-data"), "1.2.3"),
        ProfileRestoreFailureCode::NewerDatabaseVersion
    );

    let mismatched_database = create_profile_backup(
        &dir.0,
        "mismatched-database",
        "1.2.3",
        VRCX0_SCHEMA_VERSION - 1,
        VRCX0_SCHEMA_VERSION,
        "new",
    );
    assert_eq!(
        rejected_code(
            &mismatched_database,
            &dir.0.join("mismatched-db-data"),
            "1.2.3"
        ),
        ProfileRestoreFailureCode::DatabaseVersionMismatch
    );
}

#[test]
fn profile_backup_restore_rejects_non_archives_and_non_profile_databases() {
    let dir = TestDir::new("invalid-inputs");
    let not_archive = dir.0.join("not-archive.vrcx0backup");
    fs::write(&not_archive, b"not zip").unwrap();
    assert_eq!(
        rejected_code(&not_archive, &dir.0.join("not-archive-app"), "1.2.3"),
        ProfileRestoreFailureCode::InvalidArchive
    );

    let plain_db = dir.0.join("plain.sqlite3");
    Connection::open(&plain_db)
        .unwrap()
        .execute("CREATE TABLE unrelated (value TEXT)", [])
        .unwrap();
    let plain_backup = dir.0.join("plain.vrcx0backup");
    create_backup_archive(
        &plain_db,
        &plain_backup,
        ProfileBackupManifestMetadata {
            app_version: "1.2.3".into(),
            db_version: VRCX0_SCHEMA_VERSION,
            created_at: "2026-07-14T07:30:00Z".into(),
            platform: "windows".into(),
            kind: ProfileBackupKind::Manual,
        },
    )
    .unwrap();
    assert_eq!(
        rejected_code(&plain_backup, &dir.0.join("plain-app"), "1.2.3"),
        ProfileRestoreFailureCode::NotProfileDatabase
    );
}

#[test]
fn profile_backup_restore_enforces_archive_and_extraction_resource_budgets() {
    assert_eq!(
        ensure_restore_archive_copy_budget(MAX_RESTORE_ARCHIVE_BYTES + 1, u64::MAX),
        Err(ProfileRestoreFailureCode::InvalidArchive)
    );
    assert_eq!(
        ensure_restore_archive_copy_budget(1, RESTORE_FREE_SPACE_RESERVE_BYTES),
        Err(ProfileRestoreFailureCode::Io)
    );
}

#[test]
fn profile_backup_restore_copy_detects_source_growth_and_truncation_from_open_handle() {
    let dir = TestDir::new("source-length-race");
    let source = dir.0.join("source.vrcx0backup");
    fs::write(&source, b"initial").unwrap();
    let mut input = File::open(&source).unwrap();
    let initial_len = input.metadata().unwrap().len();
    OpenOptions::new()
        .append(true)
        .open(&source)
        .unwrap()
        .write_all(b"extra")
        .unwrap();
    assert_eq!(
        copy_open_archive_with_budget(
            &mut input,
            initial_len,
            u64::MAX,
            &dir.0.join("grown-copy.vrcx0backup"),
        ),
        Err(ProfileRestoreFailureCode::InvalidArchive)
    );

    fs::write(&source, b"initial").unwrap();
    let mut input = File::open(&source).unwrap();
    let initial_len = input.metadata().unwrap().len();
    OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_len(3)
        .unwrap();
    assert_eq!(
        copy_open_archive_with_budget(
            &mut input,
            initial_len,
            u64::MAX,
            &dir.0.join("truncated-copy.vrcx0backup"),
        ),
        Err(ProfileRestoreFailureCode::InvalidArchive)
    );
}

#[test]
fn profile_backup_restore_rejects_non_regular_source() {
    let dir = TestDir::new("non-regular-source");
    assert_eq!(
        rejected_code(&dir.0, &dir.0.join("app"), "1.2.3"),
        ProfileRestoreFailureCode::InvalidArchive
    );
}

#[test]
fn profile_backup_restore_stream_rejects_unlisted_third_local_entry() {
    let dir = TestDir::new("orphan-local-entry");
    let db_path = dir.0.join("valid.sqlite3");
    write_profile_database(&db_path, VRCX0_SCHEMA_VERSION, "valid");
    let db_bytes = fs::read(db_path).unwrap();
    let manifest = serde_json::to_vec(&manifest_for_database(&db_bytes)).unwrap();
    let source = dir.0.join("orphan-local.vrcx0backup");
    write_custom_archive(
        &source,
        vec![
            (DATABASE_FILE_NAME.into(), db_bytes),
            (MANIFEST_FILE_NAME.into(), manifest),
            ("unlisted.txt".into(), b"orphan".to_vec()),
        ],
    );
    fs::write(&source, hide_last_central_entry(fs::read(&source).unwrap())).unwrap();

    assert_eq!(
        rejected_code(&source, &dir.0.join("app"), "1.2.3"),
        ProfileRestoreFailureCode::InvalidEntries
    );
}

fn hide_last_central_entry(mut archive: Vec<u8>) -> Vec<u8> {
    let eocd = archive
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap();
    let central_offset =
        u32::from_le_bytes(archive[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let mut retained_end = central_offset;
    for _ in 0..2 {
        let name_len = u16::from_le_bytes(
            archive[retained_end + 28..retained_end + 30]
                .try_into()
                .unwrap(),
        ) as usize;
        let extra_len = u16::from_le_bytes(
            archive[retained_end + 30..retained_end + 32]
                .try_into()
                .unwrap(),
        ) as usize;
        let comment_len = u16::from_le_bytes(
            archive[retained_end + 32..retained_end + 34]
                .try_into()
                .unwrap(),
        ) as usize;
        retained_end += 46 + name_len + extra_len + comment_len;
    }
    archive.drain(retained_end..eocd);
    let new_eocd = retained_end;
    archive[new_eocd + 8..new_eocd + 12].copy_from_slice(&2_u16.to_le_bytes().repeat(2));
    archive[new_eocd + 12..new_eocd + 16]
        .copy_from_slice(&((retained_end - central_offset) as u32).to_le_bytes());
    archive
}

#[cfg(unix)]
#[test]
fn profile_backup_restore_staging_files_are_private_on_unix() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TestDir::new("private-staging");
    let source = create_profile_backup(
        &dir.0,
        "private-staging-source",
        "1.2.3",
        VRCX0_SCHEMA_VERSION,
        VRCX0_SCHEMA_VERSION,
        "new",
    );
    let app_data = dir.0.join("private-staging-app");
    let validation = super::common::accepted_validation(&source, &app_data, "1.2.3");
    let staged = app_data
        .join(crate::profile_backup::RESTORE_PENDING_DIRECTORY)
        .join(DATABASE_FILE_NAME);

    assert_eq!(
        validation.staged_bytes,
        fs::metadata(&staged).unwrap().len()
    );
    assert_eq!(
        fs::metadata(staged).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
