use std::fs::OpenOptions;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::Connection;

use crate::Error;

const PAGES_PER_STEP: i32 = 256;
const PAUSE_BETWEEN_STEPS: Duration = Duration::from_millis(5);
const MAX_STALL_DURATION: Duration = Duration::from_secs(30);
const MAX_BACKUP_RESTARTS: u32 = 8;

fn backup_progress_regressed(previous: (u64, u64), current: (u64, u64)) -> bool {
    u128::from(current.0) * u128::from(previous.1)
        < u128::from(previous.0) * u128::from(current.1)
}

pub(crate) fn backup_connection_to_path(
    source: &Connection,
    destination_path: &Path,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), Error> {
    let result = (|| {
        let mut destination = Connection::open(destination_path)
            .map_err(|error| Error::Database(error.to_string()))?;
        let backup = Backup::new(source, &mut destination)
            .map_err(|error| Error::Database(error.to_string()))?;
        let mut last_progress = None;
        let mut last_progress_at = Instant::now();
        let mut backup_restart_count = 0;

        loop {
            let step = backup
                .step(PAGES_PER_STEP)
                .map_err(|error| Error::Database(error.to_string()))?;
            let progress = backup.progress();
            let total_pages = progress.pagecount.max(0) as u64;
            let remaining_pages = progress.remaining.max(0) as u64;
            let completed_pages = total_pages.saturating_sub(remaining_pages);
            let current_progress = (completed_pages, total_pages);
            if last_progress
                .is_some_and(|previous| backup_progress_regressed(previous, current_progress))
            {
                backup_restart_count += 1;
                if backup_restart_count > MAX_BACKUP_RESTARTS {
                    return Err(Error::Database(format!(
                        "SQLite online backup restarted more than {MAX_BACKUP_RESTARTS} times because the source database kept changing."
                    )));
                }
            }
            if last_progress != Some(current_progress) {
                last_progress = Some(current_progress);
                last_progress_at = Instant::now();
            } else if last_progress_at.elapsed() >= MAX_STALL_DURATION {
                return Err(Error::Database(
                    "SQLite online backup made no progress for 30 seconds.".into(),
                ));
            }
            on_progress(completed_pages, total_pages);

            if step == StepResult::Done {
                break;
            }
            thread::sleep(PAUSE_BETWEEN_STEPS);
        }

        drop(backup);
        drop(destination);
        OpenOptions::new()
            .write(true)
            .open(destination_path)?
            .sync_all()?;
        if let Some(parent) = destination_path.parent() {
            crate::profile_backup::sync_directory_durable(parent)?;
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(destination_path);
        let _ = super::sidecar::remove_sidecars(destination_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-online-backup-restart-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn repeated_external_writes_abort_a_restarting_backup() {
        let dir = TestDir::new();
        let source_path = dir.path.join("source.sqlite3");
        let destination_path = dir.path.join("destination.sqlite3");
        let source = Connection::open(&source_path).unwrap();
        source
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE backup_payload (payload BLOB NOT NULL);
                 CREATE TABLE backup_growth (payload BLOB NOT NULL);
                 INSERT INTO backup_payload VALUES (zeroblob(16777216));
                 INSERT INTO backup_growth VALUES (zeroblob(1));",
            )
            .unwrap();
        let writer = Connection::open(&source_path).unwrap();
        let mut external_writes = 0_u32;

        let error = backup_connection_to_path(&source, &destination_path, |_, _| {
            external_writes += 1;
            assert!(
                external_writes <= MAX_BACKUP_RESTARTS + 1,
                "backup did not abort after repeated source restarts"
            );
            writer
                .execute(
                    "INSERT INTO backup_growth VALUES (zeroblob(1048576))",
                    [],
                )
                .unwrap();
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("source database kept changing"));
        assert_eq!(external_writes, MAX_BACKUP_RESTARTS + 1);
        assert!(!destination_path.exists());
    }
}
