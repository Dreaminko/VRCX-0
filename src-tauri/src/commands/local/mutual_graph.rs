#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    get_user_mutual_friends_list, refresh_mutual_graph_friend, MutualGraphFetchCancelInput,
    MutualGraphFetchStartInput, MutualGraphFetchStatus, MutualGraphFriendRefreshInput,
    MutualGraphFriendRefreshOutput, MutualGraphRequestDeps, UserMutualFriendsListInput,
};
use vrcx_0_core::json::RawJson;
use vrcx_0_persistence::mutual_graph::MutualGraphSnapshotOutput;

use crate::error::AppError;
use crate::state::AppState;

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

#[tauri::command]
#[specta::specta]
pub async fn app__mutual_graph_friend_refresh(
    state: State<'_, AppState>,
    input: MutualGraphFriendRefreshInput,
) -> Result<MutualGraphFriendRefreshOutput, AppError> {
    Ok(refresh_mutual_graph_friend(
        MutualGraphRequestDeps {
            db: state.db.as_ref(),
            web: state.web.as_ref(),
            auth_scope: &state.runtime_context.auth_scope,
        },
        input,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__user_mutual_friends_list_get(
    state: State<'_, AppState>,
    input: UserMutualFriendsListInput,
) -> Result<Vec<RawJson>, AppError> {
    Ok(get_user_mutual_friends_list(
        MutualGraphRequestDeps {
            db: state.db.as_ref(),
            web: state.web.as_ref(),
            auth_scope: &state.runtime_context.auth_scope,
        },
        input,
    )
    .await?)
}
