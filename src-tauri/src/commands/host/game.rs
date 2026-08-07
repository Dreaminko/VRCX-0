#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_host_desktop::game_launch;
use vrcx_0_host_desktop::host_capabilities::{
    require_host_capability, require_host_capability_supported, HostCapability,
};

use crate::adapters::host_file_access::ensure_vrchat_launch_path_allowed;
use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__is_game_running(state: State<'_, AppState>) -> Result<bool, AppError> {
    require_host_capability(HostCapability::GameProcessMonitor)?;
    Ok(state.game.process_monitor.is_game_running())
}

#[tauri::command]
#[specta::specta]
pub fn app__set_game_client_runtime_state(state: State<'_, AppState>, current_location: String) {
    state
        .game
        .game_client_runtime
        .set_runtime_state(&current_location);
}

#[tauri::command]
#[specta::specta]
pub fn app__start_game(launch_arguments: String) -> Result<bool, AppError> {
    require_host_capability(HostCapability::GameLaunch)?;
    Ok(game_launch::start_game(&launch_arguments)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__start_game_from_path(
    state: State<'_, AppState>,
    path: String,
    launch_arguments: String,
) -> Result<bool, AppError> {
    require_host_capability_supported(HostCapability::GameLaunch)?;
    let path =
        ensure_vrchat_launch_path_allowed(&state.desktop.host_file_access, &state.paths, &path)?;
    Ok(game_launch::start_game_from_path(&path, &launch_arguments)?)
}
