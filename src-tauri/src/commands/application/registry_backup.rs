#![allow(non_snake_case)]

use std::path::PathBuf;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use vrcx_0_application_game::{
    RegistryBackupMaintenanceMode, RegistryBackupMaintenanceResult, RegistryBackupSnapshot,
};
use vrcx_0_host_desktop::host_capabilities::{require_host_capability, HostCapability};
use vrcx_0_host_desktop::{shell_actions, vrchat_registry};

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_list(
    state: State<'_, AppState>,
) -> Result<Vec<RegistryBackupSnapshot>, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    Ok(state.registry_backup_list()?)
}

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_create(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<RegistryBackupSnapshot>, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    Ok(state.registry_backup_create(&name)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_restore(
    state: State<'_, AppState>,
    key: String,
) -> Result<RegistryBackupSnapshot, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    Ok(state.registry_backup_restore(&key)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_delete(
    state: State<'_, AppState>,
    key: String,
) -> Result<Vec<RegistryBackupSnapshot>, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    Ok(state.registry_backup_delete(&key)?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__registry_backup_export_to_file(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    key: String,
) -> Result<String, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    let backup = state
        .registry_backup_list()?
        .into_iter()
        .find(|backup| backup.key == key)
        .ok_or_else(|| AppError::Custom("Registry backup not found.".into()))?;
    let json = state.registry_backup_export_json(&key)?;
    let backup_name = if backup.name.trim().is_empty() {
        "VRChat Registry Backup"
    } else {
        backup.name.trim()
    };
    let file_path = crate::commands::host::dialog::save_file(
        app_handle
            .dialog()
            .file()
            .set_file_name(format!("{backup_name}.json"))
            .add_filter("JSON Files", &["json"]),
    )
    .await;
    let Some(file_path) = file_path else {
        return Ok(String::new());
    };
    let path = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        other => PathBuf::from(other.to_string()),
    };
    shell_actions::write_string_file(&path, &json)?;
    state.desktop.host_file_access.register_path(&path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn app__registry_backup_import_from_file(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<bool, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    let file_path = crate::commands::host::dialog::pick_file(
        app_handle
            .dialog()
            .file()
            .add_filter("JSON Files", &["json"]),
    )
    .await;
    let Some(file_path) = file_path else {
        return Ok(false);
    };
    let path = match file_path {
        tauri_plugin_dialog::FilePath::Path(path) => path,
        other => PathBuf::from(other.to_string()),
    };
    state.desktop.host_file_access.register_path(&path);
    let json = vrchat_registry::read_reg_json_file(&path.to_string_lossy())?;
    state.registry_backup_import_json(&json)?;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_maintenance_run(
    state: State<'_, AppState>,
    reason: String,
) -> Result<RegistryBackupMaintenanceResult, AppError> {
    require_host_capability(HostCapability::RegistryPrefs)?;
    Ok(
        state
            .registry_backup_maintenance_run(&reason, RegistryBackupMaintenanceMode::Foreground)?,
    )
}

#[tauri::command]
#[specta::specta]
pub fn app__registry_backup_restore_prompt_acknowledge(
    state: State<'_, AppState>,
    backup_date: String,
) -> Result<String, AppError> {
    Ok(
        vrcx_0_application_game::registry_backup_restore_prompt_acknowledge(
            state.db.as_ref(),
            &backup_date,
        )?,
    )
}
