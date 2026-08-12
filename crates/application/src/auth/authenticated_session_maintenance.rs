use chrono::Utc;
use vrcx_0_persistence::{maintenance::avatar_auto_cleanup_run, DatabaseService};

pub fn run_authenticated_session_maintenance(db: &DatabaseService, user_id: &str) {
    let user_id = user_id.trim();
    if user_id.is_empty() {
        tracing::warn!("authenticated session maintenance skipped without a user id");
        return;
    }
    if let Err(error) = avatar_auto_cleanup_run(db, user_id, Utc::now()) {
        tracing::warn!(user_id, error = %error, "avatar auto-cleanup failed");
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-auth-maintenance-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn cleanup_failure_does_not_escape_authenticated_session_maintenance() {
        let dir = TestDir::new();
        let db = DatabaseService::new(&dir.0.join("VRCX-0.sqlite3")).unwrap();
        let _frozen = db.freeze_for_migration().unwrap();

        run_authenticated_session_maintenance(&db, "usr_self");

        assert!(!db.is_main_mode());
    }
}
