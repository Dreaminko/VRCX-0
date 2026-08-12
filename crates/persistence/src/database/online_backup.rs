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

pub(crate) fn backup_connection_to_path(
    source: &Connection,
    destination_path: &Path,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), Error> {
    let result = (|| {
        let source_snapshot = source.unchecked_transaction().map_err(Error::sqlite)?;
        source_snapshot
            .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |_| Ok(()))
            .map_err(Error::sqlite)?;
        let mut destination = Connection::open(destination_path).map_err(Error::sqlite)?;
        let backup = Backup::new(&source_snapshot, &mut destination).map_err(Error::sqlite)?;
        let mut last_progress = None;
        let mut last_progress_at = Instant::now();

        loop {
            let step = backup.step(PAGES_PER_STEP).map_err(Error::sqlite)?;
            let progress = backup.progress();
            let total_pages = progress.pagecount.max(0) as u64;
            let remaining_pages = progress.remaining.max(0) as u64;
            let completed_pages = total_pages.saturating_sub(remaining_pages);
            let current_progress = (completed_pages, total_pages);
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
        drop(source_snapshot);
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
    fn wal_writes_do_not_restart_or_change_the_pinned_snapshot() {
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

        backup_connection_to_path(&source, &destination_path, |_, _| {
            if external_writes < 12 {
                external_writes += 1;
                writer
                    .execute("INSERT INTO backup_growth VALUES (zeroblob(1048576))", [])
                    .unwrap();
            }
        })
        .unwrap();

        let destination = Connection::open(&destination_path).unwrap();
        let destination_rows = destination
            .query_row("SELECT COUNT(*) FROM backup_growth", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap();
        let source_rows = source
            .query_row("SELECT COUNT(*) FROM backup_growth", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap();

        assert_eq!(external_writes, 12);
        assert_eq!(destination_rows, 1);
        assert_eq!(source_rows, 13);
    }
}
