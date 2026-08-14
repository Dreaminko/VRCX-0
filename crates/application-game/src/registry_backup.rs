use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use vrcx_0_persistence::config;
use vrcx_0_persistence::DatabaseService;

use crate::{Error, Result};
use vrcx_0_core::time::{iso_millis, now_iso};

const CONFIG_AUTO_BACKUP: &str = "vrcRegistryAutoBackup";
const CONFIG_ASK_RESTORE: &str = "vrcRegistryAskRestore";
const CONFIG_BACKUPS: &str = "VRChatRegistryBackups";
const CONFIG_LAST_BACKUP_DATE: &str = "VRChatRegistryLastBackupDate";
const CONFIG_LAST_RESTORE_CHECK: &str = "VRChatRegistryLastRestoreCheck";

const AUTO_BACKUP_NAME: &str = "Auto Backup";
const MANUAL_BACKUP_NAME: &str = "Manual Backup";
const AUTO_BACKUP_INTERVAL_DAYS: i64 = 3;
const AUTO_BACKUP_RETENTION_DAYS: i64 = 14;

pub trait RegistryBackupHostActions: Send + Sync {
    fn has_registry_folder(&self) -> Result<bool>;
    fn get_registry(&self) -> Result<Value>;
    fn set_registry_json(&self, json: &str) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryBackupMaintenanceMode {
    Foreground,
    Silent,
}

#[derive(Clone, Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RegistryBackupSnapshot {
    pub key: String,
    pub name: String,
    pub date: String,
    pub data: Value,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RegistryBackupMaintenanceResult {
    pub auto_backup_created: bool,
    pub restore_prompt_needed: bool,
    pub restore_prompt_backup_date: Option<String>,
    #[serde(skip)]
    #[specta(skip)]
    pub restore_prompt_check_deferred: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, specta::Type)]
struct StoredRegistryBackup {
    #[serde(default)]
    name: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    data: Value,
}

pub fn registry_backup_list(db: &DatabaseService) -> Result<Vec<RegistryBackupSnapshot>> {
    Ok(read_backups(db)?
        .into_iter()
        .enumerate()
        .map(|(index, backup)| normalize_backup(backup, index))
        .collect())
}

pub fn registry_backup_create(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    name: &str,
) -> Result<Vec<RegistryBackupSnapshot>> {
    create_backup(db, host, normalized_backup_name(name), Utc::now())?;
    registry_backup_list(db)
}

pub fn registry_backup_restore(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    key: &str,
) -> Result<RegistryBackupSnapshot> {
    let backups = read_backups(db)?;
    let Some((index, backup)) = backups
        .into_iter()
        .enumerate()
        .find(|(index, backup)| backup_key(backup, *index) == key)
    else {
        return Err(Error::Custom("Registry backup not found.".into()));
    };

    let json = registry_backup_data_to_json(&backup.data)?;
    validate_registry_json(&json)?;
    host.set_registry_json(&json)?;
    config::set_string(
        db,
        CONFIG_LAST_RESTORE_CHECK,
        &non_empty_or_now(&backup.date),
    )?;
    Ok(normalize_backup(backup, index))
}

pub fn registry_backup_delete(
    db: &DatabaseService,
    key: &str,
) -> Result<Vec<RegistryBackupSnapshot>> {
    let backups = read_backups(db)?;
    let mut removed = false;
    let next_backups = backups
        .into_iter()
        .enumerate()
        .filter_map(|(index, backup)| {
            if backup_key(&backup, index) == key {
                removed = true;
                None
            } else {
                Some(backup)
            }
        })
        .collect::<Vec<_>>();
    if !removed {
        return Err(Error::Custom("Registry backup not found.".into()));
    }
    write_backups(db, &next_backups)?;
    registry_backup_list(db)
}

pub fn registry_backup_export_json(db: &DatabaseService, key: &str) -> Result<String> {
    let backups = read_backups(db)?;
    let Some(backup) = backups
        .into_iter()
        .enumerate()
        .find_map(|(index, backup)| (backup_key(&backup, index) == key).then_some(backup))
    else {
        return Err(Error::Custom("Registry backup not found.".into()));
    };
    let json = registry_backup_data_to_json(&backup.data)?;
    let parsed = serde_json::from_str::<Value>(&json)?;
    serde_json::to_string_pretty(&parsed).map_err(Error::from)
}

pub fn registry_backup_import_json(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    json: &str,
) -> Result<()> {
    validate_registry_json(json)?;
    host.set_registry_json(json)?;
    config::set_string(db, CONFIG_LAST_RESTORE_CHECK, &now_iso())?;
    Ok(())
}

pub fn registry_backup_restore_prompt_acknowledge(
    db: &DatabaseService,
    backup_date: &str,
) -> Result<String> {
    config::set_string(db, CONFIG_LAST_RESTORE_CHECK, backup_date)?;
    Ok(backup_date.to_string())
}

pub fn registry_backup_maintenance_run(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    mode: RegistryBackupMaintenanceMode,
    reason: &str,
) -> Result<RegistryBackupMaintenanceResult> {
    let auto_backup_enabled = config::get_bool(db, CONFIG_AUTO_BACKUP, true)?;
    if !auto_backup_enabled {
        return maintenance_result(
            false,
            false,
            None,
            false,
            "Registry auto backup is disabled.",
        );
    }

    let mut backups = read_backups(db)?;
    let now = Utc::now();
    let pruned = prune_old_auto_backups(&mut backups, now);
    if pruned {
        write_backups(db, &backups)?;
    }

    let has_registry_folder = host.has_registry_folder()?;
    if !has_registry_folder {
        return maybe_restore_prompt(db, mode);
    }

    if recent_auto_backup_exists(db, now)? {
        let detail =
            format!("Registry backup maintenance skipped; recent backup exists ({reason}).");
        return maintenance_result(false, false, None, false, detail);
    }

    match create_backup_with_backups(db, host, AUTO_BACKUP_NAME.into(), now, &mut backups) {
        Ok(()) => {
            config::set_string(db, CONFIG_LAST_BACKUP_DATE, &iso_millis(now))?;
            let detail = format!("Registry auto backup created ({reason}).");
            maintenance_result(true, false, None, false, detail)
        }
        Err(Error::Custom(message))
            if message == "No VRChat registry data was found to back up." =>
        {
            maintenance_result(
                false,
                false,
                None,
                false,
                "Registry auto backup skipped; no registry data was found.",
            )
        }
        Err(error) => Err(error),
    }
}

pub fn registry_backup_foreground_followup(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
) -> Result<RegistryBackupMaintenanceResult> {
    if host.has_registry_folder()? {
        return registry_backup_maintenance_run(
            db,
            host,
            RegistryBackupMaintenanceMode::Foreground,
            "foreground-followup",
        );
    }
    maybe_restore_prompt(db, RegistryBackupMaintenanceMode::Foreground)
}

fn read_backups(db: &DatabaseService) -> Result<Vec<StoredRegistryBackup>> {
    let raw = config::get_json(db, CONFIG_BACKUPS, json!([]))?;
    Ok(match raw {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| serde_json::from_value::<StoredRegistryBackup>(item).ok())
            .collect(),
        Value::String(raw) => {
            serde_json::from_str::<Vec<StoredRegistryBackup>>(&raw).unwrap_or_default()
        }
        _ => Vec::new(),
    })
}

fn write_backups(db: &DatabaseService, backups: &[StoredRegistryBackup]) -> Result<()> {
    let value = serde_json::to_value(backups)?;
    config::set_json(db, CONFIG_BACKUPS, &value)?;
    Ok(())
}

fn create_backup(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    name: String,
    now: chrono::DateTime<Utc>,
) -> Result<()> {
    let mut backups = read_backups(db)?;
    create_backup_with_backups(db, host, name, now, &mut backups)
}

fn create_backup_with_backups(
    db: &DatabaseService,
    host: &dyn RegistryBackupHostActions,
    name: String,
    now: chrono::DateTime<Utc>,
    backups: &mut Vec<StoredRegistryBackup>,
) -> Result<()> {
    let data = host.get_registry()?;
    if data.as_object().is_none_or(|object| object.is_empty()) {
        return Err(Error::Custom(
            "No VRChat registry data was found to back up.".into(),
        ));
    }

    backups.push(StoredRegistryBackup {
        name,
        date: iso_millis(now),
        data,
    });
    write_backups(db, backups)?;
    Ok(())
}

fn prune_old_auto_backups(
    backups: &mut Vec<StoredRegistryBackup>,
    now: chrono::DateTime<Utc>,
) -> bool {
    let before = backups.len();
    let cutoff = now - Duration::days(AUTO_BACKUP_RETENTION_DAYS);
    backups.retain(|backup| {
        if backup.name != AUTO_BACKUP_NAME {
            return true;
        }
        parse_backup_date(&backup.date).is_some_and(|date| date >= cutoff)
    });
    backups.len() != before
}

fn recent_auto_backup_exists(db: &DatabaseService, now: chrono::DateTime<Utc>) -> Result<bool> {
    let last_backup_date = config::get_string(db, CONFIG_LAST_BACKUP_DATE, "")?;
    let Some(last_backup_date) = parse_backup_date(&last_backup_date) else {
        return Ok(false);
    };
    Ok(now - last_backup_date < Duration::days(AUTO_BACKUP_INTERVAL_DAYS))
}

fn maybe_restore_prompt(
    db: &DatabaseService,
    mode: RegistryBackupMaintenanceMode,
) -> Result<RegistryBackupMaintenanceResult> {
    if mode != RegistryBackupMaintenanceMode::Foreground {
        return maintenance_result(
            false,
            false,
            None,
            true,
            "Registry folder is missing; silent maintenance does not prompt.",
        );
    }
    if !config::get_bool(db, CONFIG_ASK_RESTORE, true)? {
        return maintenance_result(
            false,
            false,
            None,
            false,
            "Registry folder is missing; restore prompt is disabled.",
        );
    }
    let last_backup_date = config::get_string(db, CONFIG_LAST_BACKUP_DATE, "")?;
    let last_restore_check = config::get_string(db, CONFIG_LAST_RESTORE_CHECK, "")?;
    if last_backup_date.trim().is_empty() || last_restore_check == last_backup_date {
        return maintenance_result(
            false,
            false,
            None,
            false,
            "Registry folder is missing; no restore prompt is due.",
        );
    }
    maintenance_result(
        false,
        true,
        Some(last_backup_date),
        false,
        "Registry restore prompt is needed.",
    )
}

fn maintenance_result(
    auto_backup_created: bool,
    restore_prompt_needed: bool,
    restore_prompt_backup_date: Option<String>,
    restore_prompt_check_deferred: bool,
    detail: impl Into<String>,
) -> Result<RegistryBackupMaintenanceResult> {
    Ok(RegistryBackupMaintenanceResult {
        auto_backup_created,
        restore_prompt_needed,
        restore_prompt_backup_date,
        restore_prompt_check_deferred,
        detail: detail.into(),
    })
}

fn backup_key(backup: &StoredRegistryBackup, index: usize) -> String {
    let name = if backup.name.trim().is_empty() {
        "backup"
    } else {
        &backup.name
    };
    if backup.date.trim().is_empty() {
        format!("{index}-{name}")
    } else {
        format!("{}-{name}", backup.date)
    }
}

fn normalize_backup(backup: StoredRegistryBackup, index: usize) -> RegistryBackupSnapshot {
    let key = backup_key(&backup, index);
    let name = if backup.name.trim().is_empty() {
        "Backup".into()
    } else {
        backup.name
    };
    RegistryBackupSnapshot {
        key,
        name,
        date: backup.date,
        data: backup.data,
    }
}

fn registry_backup_data_to_json(data: &Value) -> Result<String> {
    if let Some(raw) = data.as_str() {
        validate_registry_json(raw)?;
        return Ok(raw.to_string());
    }
    serde_json::to_string(data).map_err(Error::from)
}

fn validate_registry_json(raw: &str) -> Result<()> {
    vrcx_0_core::vrchat_registry_policy::validate_registry_json(raw).map_err(Error::from)
}

fn normalized_backup_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        MANUAL_BACKUP_NAME.into()
    } else {
        name.into()
    }
}

fn non_empty_or_now(value: &str) -> String {
    if value.trim().is_empty() {
        now_iso()
    } else {
        value.to_string()
    }
}

fn parse_backup_date(value: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
                "vrcx-0-registry-backup-{name}-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn open_db(&self) -> DatabaseService {
            DatabaseService::new(&self.path.join("VRCX-0.sqlite3")).unwrap()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct StubHost {
        has_registry_folder: bool,
        registry: Value,
        get_registry_calls: AtomicUsize,
    }

    impl StubHost {
        fn with_registry(registry: Value) -> Self {
            Self {
                has_registry_folder: true,
                registry,
                get_registry_calls: AtomicUsize::new(0),
            }
        }

        fn without_registry_folder() -> Self {
            Self {
                has_registry_folder: false,
                registry: json!({}),
                get_registry_calls: AtomicUsize::new(0),
            }
        }
    }

    impl RegistryBackupHostActions for StubHost {
        fn has_registry_folder(&self) -> Result<bool> {
            Ok(self.has_registry_folder)
        }

        fn get_registry(&self) -> Result<Value> {
            self.get_registry_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.registry.clone())
        }

        fn set_registry_json(&self, _json: &str) -> Result<()> {
            Ok(())
        }
    }

    fn backup(name: &str, date: &str) -> StoredRegistryBackup {
        StoredRegistryBackup {
            name: name.into(),
            date: date.into(),
            data: json!({"key": "value"}),
        }
    }

    #[test]
    fn restore_prompt_acknowledgement_persists_the_shown_backup_date() {
        let dir = TestDir::new("ack");
        let db = dir.open_db();
        let backup_date = "2026-08-01T12:34:56.000Z";

        assert_eq!(
            registry_backup_restore_prompt_acknowledge(&db, backup_date).unwrap(),
            backup_date
        );
        assert_eq!(
            config::get_string(&db, CONFIG_LAST_RESTORE_CHECK, "").unwrap(),
            backup_date
        );
    }

    #[test]
    fn maintenance_run_skips_everything_when_auto_backup_disabled() {
        let dir = TestDir::new("disabled");
        let db = dir.open_db();
        config::set_bool(&db, CONFIG_AUTO_BACKUP, false).unwrap();
        let host = StubHost::with_registry(json!({"a": 1}));

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Foreground,
            "test",
        )
        .unwrap();

        assert!(!result.auto_backup_created);
        assert!(!result.restore_prompt_needed);
        assert_eq!(result.detail, "Registry auto backup is disabled.");
        assert_eq!(host.get_registry_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn maintenance_result_does_not_serialize_stored_backup_data() {
        let dir = TestDir::new("lightweight-result");
        let db = dir.open_db();
        config::set_bool(&db, CONFIG_AUTO_BACKUP, false).unwrap();
        write_backups(
            &db,
            &[StoredRegistryBackup {
                name: "Large Backup".into(),
                date: "2026-08-01T00:00:00.000Z".into(),
                data: json!({"uniqueLargePayloadMarker": [1, 2, 3]}),
            }],
        )
        .unwrap();

        let result = registry_backup_maintenance_run(
            &db,
            &StubHost::with_registry(json!({"a": 1})),
            RegistryBackupMaintenanceMode::Foreground,
            "test",
        )
        .unwrap();
        let serialized = serde_json::to_value(result).unwrap();

        assert!(serialized.get("backups").is_none());
        assert!(!serialized.to_string().contains("uniqueLargePayloadMarker"));
        assert_eq!(registry_backup_list(&db).unwrap().len(), 1);
    }

    #[test]
    fn maintenance_run_creates_auto_backup_when_registry_present_and_no_recent_backup() {
        let dir = TestDir::new("create");
        let db = dir.open_db();
        let host = StubHost::with_registry(json!({"a": 1}));

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Foreground,
            "startup",
        )
        .unwrap();

        assert!(result.auto_backup_created);
        assert!(!result.restore_prompt_needed);
        assert_eq!(result.detail, "Registry auto backup created (startup).");
        let backups = registry_backup_list(&db).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].name, AUTO_BACKUP_NAME);
        assert!(!config::get_string(&db, CONFIG_LAST_BACKUP_DATE, "")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn maintenance_run_skips_creation_when_recent_auto_backup_exists() {
        let dir = TestDir::new("recent");
        let db = dir.open_db();
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, &iso_millis(Utc::now())).unwrap();
        let host = StubHost::with_registry(json!({"a": 1}));

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Silent,
            "background-mode",
        )
        .unwrap();

        assert!(!result.auto_backup_created);
        assert!(!result.restore_prompt_needed);
        assert_eq!(
            result.detail,
            "Registry backup maintenance skipped; recent backup exists (background-mode)."
        );
        assert_eq!(host.get_registry_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn maintenance_run_prunes_expired_auto_backups_before_checking_recent_backup() {
        let dir = TestDir::new("prune");
        let db = dir.open_db();
        let now = Utc::now();
        let expired_date = iso_millis(now - Duration::days(AUTO_BACKUP_RETENTION_DAYS + 1));
        write_backups(&db, &[backup(AUTO_BACKUP_NAME, &expired_date)]).unwrap();
        let host = StubHost::with_registry(json!({"a": 1}));

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Foreground,
            "test",
        )
        .unwrap();

        assert!(result.auto_backup_created);
        let backups = registry_backup_list(&db).unwrap();
        assert_eq!(backups.len(), 1);
        assert_ne!(backups[0].date, expired_date);
    }

    #[test]
    fn maintenance_run_falls_back_to_restore_prompt_when_registry_folder_missing() {
        let dir = TestDir::new("missing-folder");
        let db = dir.open_db();
        let last_backup_date = "2026-08-01T00:00:00.000Z";
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, last_backup_date).unwrap();
        let host = StubHost::without_registry_folder();

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Foreground,
            "test",
        )
        .unwrap();

        assert!(!result.auto_backup_created);
        assert!(result.restore_prompt_needed);
        assert_eq!(
            result.restore_prompt_backup_date,
            Some(last_backup_date.to_string())
        );
        assert_eq!(host.get_registry_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn maintenance_run_reports_skip_when_registry_data_is_empty() {
        let dir = TestDir::new("empty-data");
        let db = dir.open_db();
        let host = StubHost::with_registry(json!({}));

        let result = registry_backup_maintenance_run(
            &db,
            &host,
            RegistryBackupMaintenanceMode::Foreground,
            "test",
        )
        .unwrap();

        assert!(!result.auto_backup_created);
        assert!(!result.restore_prompt_needed);
        assert_eq!(
            result.detail,
            "Registry auto backup skipped; no registry data was found."
        );
        assert!(config::get_string(&db, CONFIG_LAST_BACKUP_DATE, "")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn prune_old_auto_backups_removes_only_expired_auto_backups() {
        let now = Utc::now();
        let fresh_date = iso_millis(now - Duration::days(1));
        let expired_date = iso_millis(now - Duration::days(AUTO_BACKUP_RETENTION_DAYS + 1));
        let mut backups = vec![
            backup(AUTO_BACKUP_NAME, &fresh_date),
            backup(AUTO_BACKUP_NAME, &expired_date),
        ];

        let pruned = prune_old_auto_backups(&mut backups, now);

        assert!(pruned);
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].date, fresh_date);
    }

    #[test]
    fn prune_old_auto_backups_keeps_manual_backups_regardless_of_age() {
        let now = Utc::now();
        let expired_date = iso_millis(now - Duration::days(AUTO_BACKUP_RETENTION_DAYS + 1));
        let mut backups = vec![backup(MANUAL_BACKUP_NAME, &expired_date)];

        let pruned = prune_old_auto_backups(&mut backups, now);

        assert!(!pruned);
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn prune_old_auto_backups_removes_auto_backups_with_unparsable_dates() {
        let now = Utc::now();
        let mut backups = vec![backup(AUTO_BACKUP_NAME, "not-a-date")];

        let pruned = prune_old_auto_backups(&mut backups, now);

        assert!(pruned);
        assert!(backups.is_empty());
    }

    #[test]
    fn recent_auto_backup_exists_is_false_when_no_backup_date_recorded() {
        let dir = TestDir::new("recent-none");
        let db = dir.open_db();

        assert!(!recent_auto_backup_exists(&db, Utc::now()).unwrap());
    }

    #[test]
    fn recent_auto_backup_exists_is_true_within_interval() {
        let dir = TestDir::new("recent-within");
        let db = dir.open_db();
        let now = Utc::now();
        config::set_string(
            &db,
            CONFIG_LAST_BACKUP_DATE,
            &iso_millis(now - Duration::days(1)),
        )
        .unwrap();

        assert!(recent_auto_backup_exists(&db, now).unwrap());
    }

    #[test]
    fn recent_auto_backup_exists_is_false_outside_interval() {
        let dir = TestDir::new("recent-outside");
        let db = dir.open_db();
        let now = Utc::now();
        config::set_string(
            &db,
            CONFIG_LAST_BACKUP_DATE,
            &iso_millis(now - Duration::days(AUTO_BACKUP_INTERVAL_DAYS + 1)),
        )
        .unwrap();

        assert!(!recent_auto_backup_exists(&db, now).unwrap());
    }

    #[test]
    fn maybe_restore_prompt_is_silent_in_background_mode() {
        let dir = TestDir::new("prompt-silent");
        let db = dir.open_db();
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, "2026-08-01T00:00:00.000Z").unwrap();

        let result = maybe_restore_prompt(&db, RegistryBackupMaintenanceMode::Silent).unwrap();

        assert!(!result.restore_prompt_needed);
        assert!(result.restore_prompt_check_deferred);
        assert_eq!(
            result.detail,
            "Registry folder is missing; silent maintenance does not prompt."
        );
    }

    #[test]
    fn maybe_restore_prompt_is_disabled_by_config() {
        let dir = TestDir::new("prompt-disabled");
        let db = dir.open_db();
        config::set_bool(&db, CONFIG_ASK_RESTORE, false).unwrap();
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, "2026-08-01T00:00:00.000Z").unwrap();

        let result = maybe_restore_prompt(&db, RegistryBackupMaintenanceMode::Foreground).unwrap();

        assert!(!result.restore_prompt_needed);
        assert_eq!(
            result.detail,
            "Registry folder is missing; restore prompt is disabled."
        );
    }

    #[test]
    fn maybe_restore_prompt_skips_when_no_backup_date_recorded() {
        let dir = TestDir::new("prompt-no-date");
        let db = dir.open_db();

        let result = maybe_restore_prompt(&db, RegistryBackupMaintenanceMode::Foreground).unwrap();

        assert!(!result.restore_prompt_needed);
        assert_eq!(
            result.detail,
            "Registry folder is missing; no restore prompt is due."
        );
    }

    #[test]
    fn maybe_restore_prompt_skips_when_already_acknowledged() {
        let dir = TestDir::new("prompt-acked");
        let db = dir.open_db();
        let backup_date = "2026-08-01T00:00:00.000Z";
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, backup_date).unwrap();
        config::set_string(&db, CONFIG_LAST_RESTORE_CHECK, backup_date).unwrap();

        let result = maybe_restore_prompt(&db, RegistryBackupMaintenanceMode::Foreground).unwrap();

        assert!(!result.restore_prompt_needed);
        assert_eq!(
            result.detail,
            "Registry folder is missing; no restore prompt is due."
        );
    }

    #[test]
    fn maybe_restore_prompt_fires_when_new_backup_is_unacknowledged() {
        let dir = TestDir::new("prompt-due");
        let db = dir.open_db();
        let backup_date = "2026-08-01T00:00:00.000Z";
        config::set_string(&db, CONFIG_LAST_BACKUP_DATE, backup_date).unwrap();
        config::set_string(&db, CONFIG_LAST_RESTORE_CHECK, "2026-07-01T00:00:00.000Z").unwrap();

        let result = maybe_restore_prompt(&db, RegistryBackupMaintenanceMode::Foreground).unwrap();

        assert!(result.restore_prompt_needed);
        assert_eq!(
            result.restore_prompt_backup_date,
            Some(backup_date.to_string())
        );
        assert_eq!(result.detail, "Registry restore prompt is needed.");
    }

    #[test]
    fn normalize_backup_falls_back_to_index_and_default_name_for_empty_fields() {
        let cases = [
            (
                "Auto Backup",
                "2026-01-01T00:00:00.000Z",
                5,
                "2026-01-01T00:00:00.000Z-Auto Backup",
                "Auto Backup",
            ),
            (
                "",
                "2026-01-01T00:00:00.000Z",
                2,
                "2026-01-01T00:00:00.000Z-backup",
                "Backup",
            ),
            ("Manual Backup", "", 3, "3-Manual Backup", "Manual Backup"),
            ("", "", 7, "7-backup", "Backup"),
        ];

        for (name, date, index, expected_key, expected_name) in cases {
            let snapshot = normalize_backup(backup(name, date), index);

            assert_eq!(snapshot.key, expected_key);
            assert_eq!(snapshot.name, expected_name);
            assert_eq!(snapshot.date, date);
        }
    }
}
