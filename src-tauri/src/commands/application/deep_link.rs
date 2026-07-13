#![allow(non_snake_case)]

use tauri::{AppHandle, State};

#[cfg(windows)]
use tauri_plugin_deep_link::DeepLinkExt;

use crate::deep_link::DeepLinkAction;
use crate::error::AppError;
use crate::state::AppState;

#[cfg(windows)]
const APP_DEEP_LINK_SCHEME: &str = "vrcx-0";

#[tauri::command]
#[specta::specta]
pub fn app__drain_pending_deep_links(state: State<'_, AppState>) -> Vec<DeepLinkAction> {
    state.pending_deep_links.drain()
}

#[tauri::command]
#[specta::specta]
pub fn app__deep_link_registration_status(app: AppHandle) -> Result<Option<bool>, AppError> {
    deep_link_registration_status(&app)
}

fn deep_link_registration_status(app: &AppHandle) -> Result<Option<bool>, AppError> {
    #[cfg(windows)]
    {
        return app
            .deep_link()
            .is_registered(APP_DEEP_LINK_SCHEME)
            .map(Some)
            .map_err(|error| AppError::Custom(error.to_string()));
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        Ok(None)
    }
}

#[tauri::command]
#[specta::specta]
pub fn app__deep_link_registration_repair(app: AppHandle) -> Result<Option<bool>, AppError> {
    #[cfg(windows)]
    {
        app.deep_link()
            .register(APP_DEEP_LINK_SCHEME)
            .map_err(|error| AppError::Custom(error.to_string()))?;
    }
    deep_link_registration_status(&app)
}
