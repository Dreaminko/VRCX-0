#![allow(non_snake_case)]

use serde_json::Value;
use tauri::State;
use vrcx_0_application_core::FriendProfileLoadStatusPayload;

use crate::error::AppError;
use crate::state::AppState;

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserRefreshOutcome {
    pub applied: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn app__current_user_refresh(
    state: State<'_, AppState>,
) -> Result<CurrentUserRefreshOutcome, AppError> {
    let applied = state
        .realtime_runtime
        .refresh_current_user_now(Value::Null)
        .await?;
    Ok(CurrentUserRefreshOutcome { applied })
}

#[tauri::command]
#[specta::specta]
pub fn app__ingest_user_facts(
    state: State<'_, AppState>,
    entries: Vec<Value>,
) -> Result<(), AppError> {
    state.realtime_runtime.ingest_user_facts(entries);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn app__friend_profile_load_start(
    state: State<'_, AppState>,
) -> Result<FriendProfileLoadStatusPayload, AppError> {
    Ok(state.realtime_runtime.start_friend_profile_bulk_load()?)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_profile_load_cancel(
    state: State<'_, AppState>,
) -> Result<FriendProfileLoadStatusPayload, AppError> {
    Ok(state.realtime_runtime.cancel_friend_profile_bulk_load()?)
}
