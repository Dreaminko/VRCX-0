#![allow(non_snake_case)]

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_persistence::friends::{
    FriendLogCurrentEntryInput, FriendLogCurrentOutput, FriendLogDeleteOptionsInput,
    FriendLogHistoryEntryInput, FriendLogHistoryOutput, FriendLogHistoryQueryInput,
    FriendLogMutationResult, FriendLogReplaceOptionsInput, FriendLogUpsertOptionsInput,
};

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_current_list(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<FriendLogCurrentOutput>, AppError> {
    vrcx_0_persistence::friends::friend_log_current_list(state.db.as_ref(), user_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_delete_current(
    state: State<'_, AppState>,
    user_id: String,
    target_user_id: String,
) -> Result<i64, AppError> {
    state
        .realtime_runtime
        .run_friend_log_current_mutation(|| {
            vrcx_0_persistence::friends::friend_log_delete_current(
                state.db.as_ref(),
                user_id,
                target_user_id,
            )
        })
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_delete_current_array(
    state: State<'_, AppState>,
    user_id: String,
    target_user_ids: Vec<String>,
    options: FriendLogDeleteOptionsInput,
) -> Result<FriendLogMutationResult, AppError> {
    state
        .realtime_runtime
        .run_friend_log_current_mutation(|| {
            vrcx_0_persistence::friends::friend_log_delete_current_array(
                state.db.as_ref(),
                user_id,
                target_user_ids,
                options,
            )
        })
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_history_add(
    state: State<'_, AppState>,
    user_id: String,
    entries: Vec<FriendLogHistoryEntryInput>,
) -> Result<i64, AppError> {
    vrcx_0_persistence::friends::friend_log_history_add(state.db.as_ref(), user_id, entries)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_history_delete(
    state: State<'_, AppState>,
    user_id: String,
    entry: FriendLogHistoryEntryInput,
) -> Result<i64, AppError> {
    vrcx_0_persistence::friends::friend_log_history_delete(state.db.as_ref(), user_id, entry)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_history_query(
    state: State<'_, AppState>,
    query: FriendLogHistoryQueryInput,
) -> Result<Vec<FriendLogHistoryOutput>, AppError> {
    vrcx_0_persistence::friends::friend_log_history_query(state.db.as_ref(), query)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_replace_current(
    state: State<'_, AppState>,
    user_id: String,
    entries: Vec<FriendLogCurrentEntryInput>,
    options: FriendLogReplaceOptionsInput,
) -> Result<FriendLogMutationResult, AppError> {
    state
        .realtime_runtime
        .run_friend_log_current_mutation(|| {
            vrcx_0_persistence::friends::friend_log_replace_current(
                state.db.as_ref(),
                user_id,
                entries,
                options,
            )
        })
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__friend_log_upsert_current(
    state: State<'_, AppState>,
    user_id: String,
    entry: FriendLogCurrentEntryInput,
    options: FriendLogUpsertOptionsInput,
) -> Result<FriendLogMutationResult, AppError> {
    state
        .realtime_runtime
        .run_friend_log_current_mutation(|| {
            vrcx_0_persistence::friends::friend_log_upsert_current(
                state.db.as_ref(),
                user_id,
                entry,
                options,
            )
        })
        .map_err(AppError::from)
}
