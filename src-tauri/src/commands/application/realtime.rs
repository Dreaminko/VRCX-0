#![allow(non_snake_case)]

use std::collections::HashMap;

use serde_json::Value;
use tauri::State;
use vrcx_0_application::{
    FriendBaselineResult, FriendProfileLoadStatusPayload, RealtimeStopRequest,
    RealtimeTransportStartResult,
};
use vrcx_0_core::friends::FriendRecord;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__start_realtime_transport(
    state: State<'_, AppState>,
    user_id: String,
    endpoint: String,
    websocket: String,
    client_run_id: u64,
    current_user_snapshot: Value,
    friends_by_id: HashMap<String, FriendRecord>,
) -> Result<RealtimeTransportStartResult, AppError> {
    let result = state.realtime_runtime.start(
        user_id.clone(),
        endpoint.clone(),
        websocket.clone(),
        client_run_id,
        current_user_snapshot.clone(),
        friends_by_id,
    )?;
    state.sync_frontend_authenticated_session(user_id, endpoint, websocket, current_user_snapshot);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn app__sync_frontend_authenticated_session(
    state: State<'_, AppState>,
    user_id: String,
    endpoint: String,
    websocket: String,
    current_user_snapshot: Value,
) -> Result<(), AppError> {
    state.sync_frontend_authenticated_session(user_id, endpoint, websocket, current_user_snapshot);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn app__sync_realtime_friend_snapshot(
    state: State<'_, AppState>,
    user_id: String,
    endpoint: String,
    websocket: String,
    generation: Option<u64>,
    friends_by_id: HashMap<String, FriendRecord>,
) -> Result<FriendBaselineResult, AppError> {
    Ok(state.realtime_runtime.sync_friend_snapshot(
        user_id,
        endpoint,
        websocket,
        generation,
        friends_by_id,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn app__sync_realtime_current_user_snapshot(
    state: State<'_, AppState>,
    user_id: String,
    endpoint: String,
    websocket: String,
    generation: Option<u64>,
    snapshot: Value,
    overlay_patch: Value,
) -> Result<bool, AppError> {
    Ok(state.realtime_runtime.sync_current_user_snapshot(
        user_id,
        endpoint,
        websocket,
        generation,
        snapshot,
        overlay_patch,
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn app__expire_realtime_notification(
    state: State<'_, AppState>,
    user_id: String,
    notification_id: String,
) -> Result<(), AppError> {
    Ok(state
        .realtime_runtime
        .expire_notification(user_id, notification_id)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__stop_realtime_transport(
    state: State<'_, AppState>,
    user_id: Option<String>,
    endpoint: Option<String>,
    websocket: Option<String>,
    client_run_id: Option<u64>,
    generation: Option<u64>,
) {
    state.realtime_runtime.stop(RealtimeStopRequest {
        user_id,
        endpoint,
        websocket,
        client_run_id,
        generation,
    });
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
