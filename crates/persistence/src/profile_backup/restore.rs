mod artifacts;
mod filesystem;
mod journal;
mod validation;

pub use artifacts::{
    cleanup_profile_backup_artifacts, discard_staged_profile_restore, has_pending_profile_restore,
    take_last_profile_restore_result,
};
pub use journal::{
    consume_pending_profile_restore, request_staged_profile_restore, PendingProfileRestore,
};
pub use validation::{read_profile_database_version, validate_and_stage_profile_restore};

#[cfg(test)]
mod tests;
