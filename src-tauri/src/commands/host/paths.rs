#![allow(non_snake_case)]

use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::state::AppState;
use tauri::State;
use vrcx_0_application::{
    DataDirMigrationActionOutcome, DataDirMigrationMode, DataDirMigrationPlan,
    DataDirMigrationState, DataDirMigrationStatus,
};
use vrcx_0_host::app_paths::{self, app_data_paths_match, AppDataDirSource};
use vrcx_0_host_desktop::vrchat_paths;
use vrcx_0_persistence::data_dir_migration::{
    cleanup_manifest_size, data_dir_available_space, data_dir_migration_required_bytes,
    has_pending_data_dir_migration, inspect_data_dir_migration_target, DataDirCleanupPending,
    DataDirCleanupReport, DataDirMigrationResult, DataDirMigrationTargetState,
    DATA_DIR_MIGRATION_SPACE_MARGIN_BYTES,
};

use vrcx_0_host_desktop::host_capabilities::{require_host_capability, HostCapability};

#[derive(Clone, Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppDataDirState {
    pub current_dir: String,
    pub default_dir: String,
    pub persisted_dir: Option<String>,
    pub cli_dir: Option<String>,
    pub source: AppDataDirSource,
    pub cli_override: bool,
    pub pending_migration: bool,
    pub cleanup_pending: Option<DataDirCleanupPending>,
    pub migration_status: DataDirMigrationStatus,
}

#[tauri::command]
#[specta::specta]
pub fn app__system_culture() -> String {
    normalize_locale(sys_locale::get_locale().unwrap_or_else(|| "en-US".into()))
}

#[tauri::command]
#[specta::specta]
pub fn app__system_language() -> String {
    normalize_locale(sys_locale::get_locale().unwrap_or_else(|| "en".into()))
}

fn normalize_locale(locale: String) -> String {
    locale.replace('_', "-")
}

#[tauri::command]
#[specta::specta]
pub fn app__get_vrchat_app_data_location() -> Result<String, AppError> {
    require_host_capability(HostCapability::VrchatPathDiscovery)?;
    Ok(vrchat_paths::vrchat_app_data()
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
#[specta::specta]
pub fn app__get_vrchat_photos_location() -> Result<String, AppError> {
    require_host_capability(HostCapability::VrchatPathDiscovery)?;
    Ok(vrchat_paths::vrchat_photos_location())
}

#[tauri::command]
#[specta::specta]
pub fn app__get_ugc_photo_location(path: Option<String>) -> Result<String, AppError> {
    if path.as_deref().is_none_or(|p| p.is_empty()) {
        require_host_capability(HostCapability::VrchatPathDiscovery)?;
    }
    Ok(vrchat_paths::ugc_photo_location(path))
}

#[tauri::command]
#[specta::specta]
pub fn app__get_vrchat_cache_location() -> Result<String, AppError> {
    require_host_capability(HostCapability::VrchatPathDiscovery)?;
    Ok(vrchat_paths::vrchat_cache_location())
}

#[tauri::command]
#[specta::specta]
pub fn app__get_vrchat_screenshots_location() -> Result<String, AppError> {
    require_host_capability(HostCapability::ScreenshotCache)?;
    Ok(vrchat_paths::vrchat_screenshots_location())
}

#[tauri::command]
#[specta::specta]
pub fn app__get_app_data_dir_state(
    state: State<'_, AppState>,
) -> Result<AppDataDirState, AppError> {
    data_dir_state(&state)
}

#[tauri::command]
#[specta::specta]
pub fn app__validate_app_data_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<app_paths::AppDataDirValidation, AppError> {
    Ok(app_paths::validate_app_data_dir_selection(
        path,
        &state.runtime.app_data_dir.current_dir,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn app__set_app_data_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<AppDataDirState, AppError> {
    ensure_data_dir_settings_available(&state)?;
    app_paths::persist_app_data_dir(path, &state.runtime.app_data_dir.current_dir)?;
    data_dir_state(&state)
}

#[tauri::command]
#[specta::specta]
pub fn app__clear_app_data_dir(state: State<'_, AppState>) -> Result<AppDataDirState, AppError> {
    ensure_data_dir_settings_available(&state)?;
    app_paths::clear_persisted_app_data_dir()?;
    data_dir_state(&state)
}

#[tauri::command]
#[specta::specta]
pub fn app__plan_data_dir_migration(
    state: State<'_, AppState>,
    path: String,
) -> Result<DataDirMigrationPlan, AppError> {
    ensure_data_dir_settings_available(&state)?;
    plan_data_dir_migration(&state, &path)
}

#[tauri::command]
#[specta::specta]
pub async fn app__request_data_dir_migration(
    state: State<'_, AppState>,
    path: String,
    mode: DataDirMigrationMode,
) -> Result<DataDirMigrationActionOutcome, AppError> {
    ensure_data_dir_settings_available(&state)?;
    let plan = plan_data_dir_migration(&state, &path)?;
    let runtime = state.data_dir_migration.clone();
    let current_dir = state.runtime.app_data_dir.current_dir.clone();
    tauri::async_runtime::spawn_blocking(move || match mode {
        DataDirMigrationMode::Migrate => {
            if plan.available_bytes < plan.required_bytes {
                return Err(AppError::Custom(format!(
                    "Data directory migration requires {} bytes but only {} bytes are available.",
                    plan.required_bytes, plan.available_bytes
                )));
            }
            Ok(runtime.run_migration(
                PathBuf::from(plan.target_path),
                plan.target_state == DataDirMigrationTargetState::ExistingProfile,
            ))
        }
        DataDirMigrationMode::AdoptExisting => {
            if plan.target_state != DataDirMigrationTargetState::ExistingProfile {
                return Err(AppError::Custom(
                    "Only a directory containing a VRCX-0 profile can be adopted.".into(),
                ));
            }
            app_paths::persist_app_data_dir(&plan.target_path, &current_dir)?;
            Ok(accepted_pointer_change(&runtime, &plan.target_path))
        }
        DataDirMigrationMode::FreshStart => {
            if plan.target_state == DataDirMigrationTargetState::ExistingProfile {
                return Err(AppError::Custom(
                    "A directory containing a VRCX-0 profile cannot be used as a fresh start."
                        .into(),
                ));
            }
            app_paths::persist_app_data_dir(&plan.target_path, &current_dir)?;
            Ok(accepted_pointer_change(&runtime, &plan.target_path))
        }
    })
    .await
    .map_err(|error| AppError::Custom(format!("data directory migration task: {error}")))?
}

#[tauri::command]
#[specta::specta]
pub fn app__cancel_data_dir_migration(
    state: State<'_, AppState>,
) -> Result<DataDirMigrationActionOutcome, AppError> {
    Ok(state.data_dir_migration.request_cancel())
}

#[tauri::command]
#[specta::specta]
pub fn app__data_dir_migration_current_status(
    state: State<'_, AppState>,
) -> Result<DataDirMigrationStatus, AppError> {
    Ok(state.data_dir_migration.current_status())
}

#[tauri::command]
#[specta::specta]
pub async fn app__take_data_dir_migration_result(
    state: State<'_, AppState>,
) -> Result<Option<DataDirMigrationResult>, AppError> {
    let runtime = state.data_dir_migration.clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || runtime.take_last_result())
            .await
            .map_err(|error| AppError::Custom(format!("data directory result task: {error}")))??,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn app__cleanup_migrated_data_dir(
    state: State<'_, AppState>,
) -> Result<Option<DataDirCleanupReport>, AppError> {
    let runtime = state.data_dir_migration.clone();
    Ok(
        tauri::async_runtime::spawn_blocking(move || runtime.cleanup_migrated_data())
            .await
            .map_err(|error| AppError::Custom(format!("data directory cleanup task: {error}")))??,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn app__dismiss_data_dir_cleanup(state: State<'_, AppState>) -> Result<(), AppError> {
    let runtime = state.data_dir_migration.clone();
    tauri::async_runtime::spawn_blocking(move || runtime.dismiss_cleanup())
        .await
        .map_err(|error| {
            AppError::Custom(format!("data directory cleanup dismiss task: {error}"))
        })??;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn app__mark_data_dir_cleanup_prompted(
    state: State<'_, AppState>,
    prompted_at: String,
) -> Result<(), AppError> {
    let runtime = state.data_dir_migration.clone();
    tauri::async_runtime::spawn_blocking(move || runtime.mark_cleanup_prompted(prompted_at))
        .await
        .map_err(|error| {
            AppError::Custom(format!("data directory cleanup prompt task: {error}"))
        })??;
    Ok(())
}

fn data_dir_state(state: &AppState) -> Result<AppDataDirState, AppError> {
    let directory = app_paths::app_data_dir_state(&state.runtime.app_data_dir)?;
    let mut cleanup_pending = state.data_dir_migration.cleanup_pending()?;
    if let Some(pending) = cleanup_pending.as_mut() {
        if let Ok(bytes) = cleanup_manifest_size(Path::new(&pending.old_dir)) {
            pending.bytes = bytes;
        }
    }
    let pending_migration = has_pending_data_dir_migration(&state.runtime.app_data_dir.default_dir)
        || configured_data_dir_differs(&directory);
    Ok(AppDataDirState {
        current_dir: directory.current_dir,
        default_dir: directory.default_dir,
        persisted_dir: directory.persisted_dir,
        cli_dir: directory.cli_dir,
        source: directory.source,
        cli_override: directory.cli_override,
        pending_migration,
        cleanup_pending,
        migration_status: state.data_dir_migration.current_status(),
    })
}

fn plan_data_dir_migration(
    state: &AppState,
    path: impl AsRef<Path>,
) -> Result<DataDirMigrationPlan, AppError> {
    let validation =
        app_paths::validate_app_data_dir_selection(path, &state.runtime.app_data_dir.current_dir)?;
    let target_path = PathBuf::from(&validation.path);
    let required_bytes = data_dir_migration_required_bytes(&state.paths.app_data)?
        .checked_add(DATA_DIR_MIGRATION_SPACE_MARGIN_BYTES)
        .ok_or_else(|| AppError::Custom("Data directory migration size overflowed.".into()))?;
    Ok(DataDirMigrationPlan {
        target_path: validation.path,
        required_bytes,
        available_bytes: data_dir_available_space(&target_path)?,
        target_state: inspect_data_dir_migration_target(&target_path)?,
    })
}

fn accepted_pointer_change(
    runtime: &vrcx_0_application::DataDirMigrationRuntime,
    target_path: &str,
) -> DataDirMigrationActionOutcome {
    let mut status = runtime.current_status();
    status.revision = status.revision.saturating_add(1);
    status.state = DataDirMigrationState::Completed;
    status.target_dir = Some(target_path.into());
    DataDirMigrationActionOutcome {
        accepted: true,
        status,
        error: None,
    }
}

fn ensure_data_dir_settings_available(state: &AppState) -> Result<(), AppError> {
    if state.runtime.app_data_dir.source == AppDataDirSource::Cli {
        return Err(AppError::Custom(
            "Data directory settings are disabled while --data-dir is active.".into(),
        ));
    }
    let directory = app_paths::app_data_dir_state(&state.runtime.app_data_dir)?;
    if configured_data_dir_differs(&directory) {
        return Err(AppError::Custom(
            "Restart VRCX-0 before changing the data directory again.".into(),
        ));
    }
    Ok(())
}

fn configured_data_dir_differs(state: &app_paths::AppDataDirState) -> bool {
    let configured = state.persisted_dir.as_deref().unwrap_or(&state.default_dir);
    !app_data_paths_match(Path::new(configured), Path::new(&state.current_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_locale_separator() {
        assert_eq!(normalize_locale("en_US".into()), "en-US");
        assert_eq!(normalize_locale("zh-Hans_CN".into()), "zh-Hans-CN");
    }
}
