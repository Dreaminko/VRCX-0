use std::path::Path;

use vrcx_0_persistence::profile_backup::{
    cleanup_profile_backup_artifacts, discard_staged_profile_restore, has_pending_profile_restore,
    request_staged_profile_restore, take_last_profile_restore_result,
    validate_and_stage_profile_restore,
};

use crate::Result;

use super::{OperationGuard, ProfileBackupRuntime};
use crate::profile_backup::{
    ProfileRestoreFailure, ProfileRestoreFailureCode, ProfileRestoreResult,
    ProfileRestoreValidationOutcome,
};

impl ProfileBackupRuntime {
    pub fn validate_restore(&self, source: &Path) -> ProfileRestoreValidationOutcome {
        let Some(_guard) = OperationGuard::try_acquire(&self.inner.operation_running) else {
            return restore_rejected(ProfileRestoreFailureCode::OperationBusy, None);
        };
        if !self.inner.db.is_main_mode() {
            return restore_rejected(ProfileRestoreFailureCode::OperationBusy, None);
        }
        if has_pending_profile_restore(&self.inner.app_data) {
            return restore_rejected(ProfileRestoreFailureCode::PendingRestore, None);
        }
        match validate_and_stage_profile_restore(
            source,
            &self.inner.app_data,
            &self.inner.app_version,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::warn!(error = %error, "failed to validate profile restore archive");
                restore_rejected(
                    ProfileRestoreFailureCode::Io,
                    Some(source.to_string_lossy().into_owned()),
                )
            }
        }
    }

    pub fn request_restore(
        &self,
        source: &Path,
        expected_sha256: &str,
    ) -> ProfileRestoreValidationOutcome {
        let Some(_guard) = OperationGuard::try_acquire(&self.inner.operation_running) else {
            return restore_rejected(ProfileRestoreFailureCode::OperationBusy, None);
        };
        if !self.inner.db.is_main_mode() {
            return restore_rejected(ProfileRestoreFailureCode::OperationBusy, None);
        }
        if has_pending_profile_restore(&self.inner.app_data) {
            return restore_rejected(ProfileRestoreFailureCode::PendingRestore, None);
        }
        let outcome = match validate_and_stage_profile_restore(
            source,
            &self.inner.app_data,
            &self.inner.app_version,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::warn!(error = %error, "failed to revalidate profile restore archive");
                return restore_rejected(
                    ProfileRestoreFailureCode::Io,
                    Some(source.to_string_lossy().into_owned()),
                );
            }
        };
        let Some(validation) = outcome.validation.as_ref() else {
            return outcome;
        };
        if validation.staged_sha256 != expected_sha256 {
            let _ = discard_staged_profile_restore(&self.inner.app_data);
            return restore_rejected(
                ProfileRestoreFailureCode::SourceFileChanged,
                Some(source.to_string_lossy().into_owned()),
            );
        }
        if let Err(error) = request_staged_profile_restore(&self.inner.app_data, validation) {
            tracing::warn!(error = %error, "failed to persist profile restore request");
            let _ = discard_staged_profile_restore(&self.inner.app_data);
            return restore_rejected(
                ProfileRestoreFailureCode::Io,
                Some(source.to_string_lossy().into_owned()),
            );
        }
        outcome
    }

    pub fn take_last_restore_result(&self) -> Result<Option<ProfileRestoreResult>> {
        Ok(take_last_profile_restore_result(&self.inner.app_data)?)
    }

    pub fn cleanup_startup_artifacts(&self) -> Result<()> {
        Ok(cleanup_profile_backup_artifacts(&self.inner.app_data)?)
    }
}

fn restore_rejected(
    code: ProfileRestoreFailureCode,
    path: Option<String>,
) -> ProfileRestoreValidationOutcome {
    ProfileRestoreValidationOutcome {
        validation: None,
        failure: Some(ProfileRestoreFailure { code, path }),
    }
}
