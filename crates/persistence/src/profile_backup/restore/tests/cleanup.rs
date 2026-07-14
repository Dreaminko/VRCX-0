use std::fs;

use crate::profile_backup::{
    BACKUP_STAGING_DIRECTORY, RESTORE_PENDING_DIRECTORY, RESTORE_ROLLBACK_DIRECTORY,
};

use super::super::{cleanup_profile_backup_artifacts, has_pending_profile_restore};
use super::common::{prepare_restore, TestDir};

#[test]
fn profile_backup_cleanup_preserves_active_restore_and_limits_rollbacks() {
    let dir = TestDir::new("cleanup");
    let (app_data, _, _) = prepare_restore(&dir, "cleanup");
    let backup_staging = app_data.join(BACKUP_STAGING_DIRECTORY);
    fs::create_dir_all(&backup_staging).unwrap();
    fs::write(backup_staging.join("partial"), b"partial").unwrap();
    let rollback_root = app_data.join(RESTORE_ROLLBACK_DIRECTORY);
    for name in ["20260710-000000", "20260711-000000", "20260712-000000"] {
        fs::create_dir_all(rollback_root.join(name)).unwrap();
    }

    cleanup_profile_backup_artifacts(&app_data).unwrap();

    assert!(!backup_staging.exists());
    assert!(app_data.join(RESTORE_PENDING_DIRECTORY).exists());
    assert!(has_pending_profile_restore(&app_data));
    assert!(fs::read_dir(rollback_root).unwrap().count() <= 2);
}
