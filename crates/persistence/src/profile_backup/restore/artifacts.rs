use std::fs;
use std::path::Path;

use crate::Error;

use super::super::{
    ProfileRestoreResult, BACKUP_STAGING_DIRECTORY, RESTORE_JOURNAL_FILE_NAME,
    RESTORE_PENDING_DIRECTORY, RESTORE_RESULT_FILE_NAME,
};
use super::filesystem::{prune_rollback_directories, remove_directory_if_exists};
use super::journal::active_rollback_directory_name;

pub fn take_last_profile_restore_result(
    app_data: &Path,
) -> Result<Option<ProfileRestoreResult>, Error> {
    let result_path = app_data.join(RESTORE_RESULT_FILE_NAME);
    if !result_path.exists() {
        return Ok(None);
    }
    let result = serde_json::from_slice(&fs::read(&result_path)?)?;
    if let Err(error) = fs::remove_file(&result_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!("Failed to remove the profile restore result file: {error}");
        }
    }
    Ok(Some(result))
}

pub fn cleanup_profile_backup_artifacts(app_data: &Path) -> Result<(), Error> {
    remove_directory_if_exists(&app_data.join(BACKUP_STAGING_DIRECTORY))?;
    let journal_path = app_data.join(RESTORE_JOURNAL_FILE_NAME);
    if !journal_path.exists() {
        remove_directory_if_exists(&app_data.join(RESTORE_PENDING_DIRECTORY))?;
        return prune_rollback_directories(app_data, 2, None);
    }
    let Some(active) = active_rollback_directory_name(app_data) else {
        return Ok(());
    };
    prune_rollback_directories(app_data, 2, Some(active.as_str()))
}

pub fn discard_staged_profile_restore(app_data: &Path) -> Result<(), Error> {
    if has_pending_profile_restore(app_data) {
        return Err(Error::InvalidData(
            "Cannot discard a staged profile restore after it is requested.".into(),
        ));
    }
    remove_directory_if_exists(&app_data.join(RESTORE_PENDING_DIRECTORY))
}

pub fn has_pending_profile_restore(app_data: &Path) -> bool {
    app_data.join(RESTORE_JOURNAL_FILE_NAME).is_file()
}
