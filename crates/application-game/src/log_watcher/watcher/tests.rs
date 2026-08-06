use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::log_watcher::{GameLogEvent, GameLogEventOrigin, GameLogEventSink};
use vrcx_0_core::log_watcher::GameLogEventKind;

use super::{update, LogWatcher};

#[derive(Default)]
struct RecordingSink {
    origins: Mutex<Vec<GameLogEventOrigin>>,
}

impl GameLogEventSink for RecordingSink {
    fn ingest_game_log_event(&self, _event: &GameLogEvent) -> crate::Result<()> {
        Ok(())
    }

    fn ingest_game_log_events_with_origin(
        &self,
        _events: &[GameLogEvent],
        origin: GameLogEventOrigin,
    ) -> crate::Result<()> {
        self.origins.lock().unwrap().push(origin);
        Ok(())
    }
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "vrcx-0-log-watcher-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn update_tracks_only_vrchat_output_logs_and_removes_deleted_contexts() {
    let dir = TestDir::new("scan");
    std::fs::write(dir.path().join("output_log_2026-08-02.txt"), []).unwrap();
    std::fs::write(dir.path().join("output_log_ignored.log"), []).unwrap();
    std::fs::write(dir.path().join("other.txt"), []).unwrap();
    let watcher = LogWatcher::new(None);
    let mut contexts = HashMap::new();
    let mut first_run = true;

    assert!(!update(
        &watcher.inner,
        dir.path(),
        &mut contexts,
        &mut first_run,
    ));
    assert!(!first_run);
    assert_eq!(contexts.len(), 1);
    assert!(contexts.contains_key("output_log_2026-08-02.txt"));

    std::fs::remove_file(dir.path().join("output_log_2026-08-02.txt")).unwrap();
    assert!(!update(
        &watcher.inner,
        dir.path(),
        &mut contexts,
        &mut first_run,
    ));
    assert!(contexts.is_empty());
}

#[test]
fn update_skips_files_older_than_the_requested_cutoff() {
    let dir = TestDir::new("cutoff");
    std::fs::write(dir.path().join("output_log_2026-08-02.txt"), []).unwrap();
    let watcher = LogWatcher::new(None);
    watcher.set_date_till("2999-01-01T00:00:00.000Z");
    let mut contexts = HashMap::new();
    let mut first_run = true;

    assert!(!update(
        &watcher.inner,
        dir.path(),
        &mut contexts,
        &mut first_run,
    ));
    assert!(contexts.is_empty());
    assert!(!first_run);
}

#[test]
fn update_handles_a_missing_directory_as_an_empty_completed_scan() {
    let dir = TestDir::new("missing");
    let missing = dir.path().join("not-created");
    let watcher = LogWatcher::new(None);
    let mut contexts = HashMap::new();
    let mut first_run = true;

    assert!(!update(
        &watcher.inner,
        &missing,
        &mut contexts,
        &mut first_run,
    ));
    assert!(contexts.is_empty());
    assert!(!first_run);
}

#[test]
fn initial_scan_can_limit_replay_to_latest_output_log() {
    let dir = TestDir::new("latest-only");
    std::fs::write(dir.path().join("output_log_first.txt"), []).unwrap();
    std::fs::write(dir.path().join("output_log_second.txt"), []).unwrap();
    let watcher = LogWatcher::new(None);
    watcher.set_initial_scan_latest_file_only(true);
    let mut contexts = HashMap::new();
    let mut first_run = true;

    update(&watcher.inner, dir.path(), &mut contexts, &mut first_run);

    assert_eq!(contexts.len(), 1);
    assert!(contexts.contains_key("output_log_second.txt"));
    assert!(!first_run);
}

#[test]
fn flush_labels_initial_and_live_batches() {
    let sink = Arc::new(RecordingSink::default());
    let watcher = LogWatcher::new(Some(sink.clone()));
    let event = GameLogEvent {
        file_name: "output_log_test.txt".into(),
        created_at: "2026-08-06T00:00:00.000Z".into(),
        kind: GameLogEventKind::DesktopMode,
    };

    watcher
        .inner
        .event_buffer
        .lock()
        .unwrap()
        .push(event.clone());
    crate::log_watcher::queue::flush_game_log_events(&watcher.inner, true);
    watcher.inner.event_buffer.lock().unwrap().push(event);
    crate::log_watcher::queue::flush_game_log_events(&watcher.inner, false);

    assert_eq!(
        *sink.origins.lock().unwrap(),
        vec![GameLogEventOrigin::InitialScan, GameLogEventOrigin::Live]
    );
}
