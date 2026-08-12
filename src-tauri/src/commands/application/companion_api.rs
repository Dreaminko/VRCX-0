#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_companion_api::CompanionApiStatus;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn app__companion_api_status(
    state: State<'_, AppState>,
) -> Result<CompanionApiStatus, AppError> {
    state
        .runtime
        .companion_api()
        .status()
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__companion_api_set_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<CompanionApiStatus, AppError> {
    state
        .runtime
        .companion_api()
        .set_enabled(enabled)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__companion_api_set_port(
    state: State<'_, AppState>,
    port: u16,
) -> Result<CompanionApiStatus, AppError> {
    state
        .runtime
        .companion_api()
        .set_port(port)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__companion_api_set_allow_lan_connections(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<CompanionApiStatus, AppError> {
    state
        .runtime
        .companion_api()
        .set_allow_lan_connections(enabled)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__companion_api_rotate_token(
    state: State<'_, AppState>,
) -> Result<CompanionApiStatus, AppError> {
    state
        .runtime
        .companion_api()
        .rotate_token()
        .await
        .map_err(AppError::from)
}
