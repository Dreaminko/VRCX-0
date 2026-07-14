mod archive;
mod fsutil;
mod restore;
mod types;

pub use archive::{
    commit_file_without_overwrite, create_backup_archive, create_backup_archive_with_progress,
    select_auto_backups_for_removal,
};
pub use restore::{
    cleanup_profile_backup_artifacts, consume_pending_profile_restore,
    discard_staged_profile_restore, has_pending_profile_restore, read_profile_database_version,
    request_staged_profile_restore, take_last_profile_restore_result,
    validate_and_stage_profile_restore, PendingProfileRestore,
};
pub(crate) use types::MAX_PROFILE_DATABASE_BYTES;
pub use types::{
    ProfileBackupContent, ProfileBackupKind, ProfileBackupManifest, ProfileBackupManifestMetadata,
    ProfileRestoreAppVersionCheck, ProfileRestoreArchiveCheck, ProfileRestoreDataDisposition,
    ProfileRestoreDatabaseCheck, ProfileRestoreDatabaseVersionCheck, ProfileRestoreFailure,
    ProfileRestoreFailureCode, ProfileRestoreManifestSummary, ProfileRestoreResult,
    ProfileRestoreResultStatus, ProfileRestoreValidation, ProfileRestoreValidationOutcome,
    BACKUP_STAGING_DIRECTORY, DATABASE_FILE_NAME, MANIFEST_FILE_NAME, RESTORE_JOURNAL_FILE_NAME,
    RESTORE_PENDING_DIRECTORY, RESTORE_RESULT_FILE_NAME, RESTORE_ROLLBACK_DIRECTORY,
};
