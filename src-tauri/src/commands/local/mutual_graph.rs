#![allow(non_snake_case)]

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_application::{
    MutualGraphFetchCancelInput, MutualGraphFetchStartInput, MutualGraphFetchStatus,
};
use vrcx_0_persistence::maintenance::UserTableContextOutput;
use vrcx_0_persistence::mutual_graph::{
    MutualGraphMetaInput, MutualGraphSnapshotEntryInput, MutualGraphSnapshotOutput,
};

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_friend_update(
    state: State<'_, AppState>,
    user_id: String,
    friend_id: String,
    mutual_ids: Vec<String>,
) -> Result<(), AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_friend_update(
        state.db.as_ref(),
        user_id,
        friend_id,
        mutual_ids,
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_meta_bulk_upsert(
    state: State<'_, AppState>,
    user_id: String,
    entries: Vec<MutualGraphMetaInput>,
) -> Result<(), AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_meta_bulk_upsert(
        state.db.as_ref(),
        user_id,
        entries,
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_meta_upsert(
    state: State<'_, AppState>,
    user_id: String,
    entry: MutualGraphMetaInput,
) -> Result<(), AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_meta_upsert(state.db.as_ref(), user_id, entry)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_snapshot_get(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<MutualGraphSnapshotOutput, AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_snapshot_get(state.db.as_ref(), user_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_snapshot_save(
    state: State<'_, AppState>,
    user_id: String,
    entries: Vec<MutualGraphSnapshotEntryInput>,
) -> Result<(), AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_snapshot_save(
        state.db.as_ref(),
        user_id,
        entries,
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_tables_ensure(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<UserTableContextOutput, AppError> {
    vrcx_0_persistence::mutual_graph::mutual_graph_tables_ensure(state.db.as_ref(), user_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_fetch_status_get(state: State<'_, AppState>) -> MutualGraphFetchStatus {
    state.runtime_context.mutual_graph_fetch.status()
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_fetch_cancel(
    state: State<'_, AppState>,
    input: MutualGraphFetchCancelInput,
) -> Result<MutualGraphFetchStatus, AppError> {
    state
        .runtime_context
        .mutual_graph_fetch
        .cancel(input)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__mutual_graph_fetch_start(
    state: State<'_, AppState>,
    input: MutualGraphFetchStartInput,
) -> Result<MutualGraphFetchStatus, AppError> {
    state
        .runtime_context
        .mutual_graph_fetch
        .start(
            input,
            state.db.clone(),
            state.web.clone(),
            state.runtime_context.auth_scope.clone(),
            state.runtime_context.tasks.clone(),
        )
        .map_err(AppError::from)
}
