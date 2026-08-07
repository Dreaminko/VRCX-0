#![allow(non_snake_case)]

use tauri::State;

use crate::commands::host::paths::{app__system_culture, app__system_language};
use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_host_desktop::host_capabilities::{current_host_capabilities, HostCapabilities};
use vrcx_0_persistence::config::{config_list_values, ConfigReadEntry};

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StartupBootstrapSnapshot {
    pub host_capabilities: HostCapabilities,
    pub config_entries: Vec<ConfigReadEntry>,
    pub system_language: String,
    pub system_culture: String,
}

#[tauri::command]
#[specta::specta]
pub fn app__startup_bootstrap_snapshot_get(
    state: State<'_, AppState>,
) -> Result<StartupBootstrapSnapshot, AppError> {
    let config_entries = config_list_values(state.db.as_ref()).map_err(AppError::from)?;
    Ok(StartupBootstrapSnapshot {
        host_capabilities: current_host_capabilities(),
        config_entries,
        system_language: app__system_language(),
        system_culture: app__system_culture(),
    })
}
