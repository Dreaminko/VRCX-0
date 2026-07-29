use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const LIFECYCLE_LOG_ENV: &str = "VRCX_0_REALTIME_LOG";
const DIRECTORY_NAME: &str = "diagnostics";
const FILE_NAME: &str = "realtime-lifecycle.jsonl";
const ROTATED_FILE_NAME: &str = "realtime-lifecycle.1.jsonl";
const MAX_BYTES: u64 = 8 * 1024 * 1024;

static ENABLED: OnceLock<bool> = OnceLock::new();
static WRITE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
static ANNOUNCED: OnceLock<()> = OnceLock::new();

fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var(LIFECYCLE_LOG_ENV)
            .map(|value| matches!(value.trim(), "1" | "true"))
            .unwrap_or(false)
    })
}

pub fn resolve_path(db_path: &Path) -> Option<PathBuf> {
    if !enabled() {
        return None;
    }

    let path = db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(DIRECTORY_NAME)
        .join(FILE_NAME);

    if ANNOUNCED.set(()).is_ok() {
        tracing::info!(path = %path.display(), "[Realtime] lifecycle trail enabled");
        record_at(&path, "trail_opened", serde_json::json!({}));
    }
    Some(path)
}

pub fn record(db_path: &Path, kind: &str, fields: serde_json::Value) {
    let Some(path) = resolve_path(db_path) else {
        return;
    };
    record_at(&path, kind, fields);
}

pub(crate) fn record_at(path: &Path, kind: &str, fields: serde_json::Value) {
    let mut line = serde_json::json!({
        "at": chrono::Utc::now().to_rfc3339(),
        "kind": kind,
    });
    if let (Some(line), Some(fields)) = (line.as_object_mut(), fields.as_object()) {
        for (key, value) in fields {
            if key == "at" || key == "kind" {
                continue;
            }
            line.insert(key.clone(), value.clone());
        }
    }
    if let Err(error) = write_line(path, &line.to_string()) {
        tracing::warn!(error = %error, path = %path.display(), "[Realtime] failed to write lifecycle trail");
    }
}

fn write_line(path: &Path, line: &str) -> std::io::Result<()> {
    let mutex = WRITE_MUTEX.get_or_init(|| Mutex::new(()));
    let _guard = mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    rotate_if_needed(path);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

fn rotate_if_needed(path: &Path) {
    let oversized = std::fs::metadata(path)
        .map(|metadata| metadata.len() >= MAX_BYTES)
        .unwrap_or(false);
    if !oversized {
        return;
    }
    let rotated = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(ROTATED_FILE_NAME);
    let _ = std::fs::remove_file(&rotated);
    let _ = std::fs::rename(path, rotated);
}

#[cfg(test)]
mod tests {
    use super::{record_at, ROTATED_FILE_NAME};

    fn read_last_line(path: &std::path::Path) -> String {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .next_back()
            .unwrap()
            .to_string()
    }

    #[test]
    fn caller_fields_never_shadow_the_trail_timestamp() {
        let dir = std::env::temp_dir().join(format!("vrcx-lifecycle-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("realtime-lifecycle.jsonl");

        record_at(
            &path,
            "supervisionEnded",
            serde_json::json!({ "at": "authExpired", "kind": "nope", "stage": "authExpired" }),
        );

        let line = read_last_line(&path);
        assert!(line.contains("\"kind\":\"supervisionEnded\""));
        assert!(line.contains("\"stage\":\"authExpired\""));
        assert!(!line.contains("\"at\":\"authExpired\""));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotates_once_the_trail_outgrows_the_cap() {
        let dir = std::env::temp_dir().join(format!("vrcx-lifecycle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("realtime-lifecycle.jsonl");

        std::fs::write(&path, vec![b'x'; super::MAX_BYTES as usize]).unwrap();
        record_at(&path, "connected", serde_json::json!({ "generation": 7 }));

        let rotated = std::fs::read_to_string(dir.join(ROTATED_FILE_NAME)).unwrap();
        let current = std::fs::read_to_string(&path).unwrap();
        assert_eq!(rotated.len(), super::MAX_BYTES as usize);
        assert!(current.contains("\"kind\":\"connected\""));
        assert!(current.contains("\"generation\":7"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
