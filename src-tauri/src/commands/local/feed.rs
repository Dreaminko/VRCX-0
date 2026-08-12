#![allow(non_snake_case)]

use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

use vrcx_0_persistence::feed::{
    FeedLatestQueryInput, FeedReadModelOutput, FeedRowOutput, FeedRowsQueryInput,
    FeedSearchQueryInput,
};

#[tauri::command]
#[specta::specta]
pub fn app__feed_persistence_set_disabled(
    state: State<'_, AppState>,
    disabled: bool,
) -> Result<(), AppError> {
    state
        .realtime_runtime
        .set_feed_persistence_disabled(disabled)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_feed_persistence_set_disabled(
    state: State<'_, AppState>,
    disabled: bool,
) -> Result<(), AppError> {
    state
        .realtime_runtime
        .set_avatar_feed_persistence_disabled(disabled)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__avatar_feed_history_cleanup(
    state: State<'_, AppState>,
    cutoff_date: Option<String>,
) -> Result<vrcx_0_application::AvatarFeedCleanupOutcome, AppError> {
    let db = state.db.clone();
    let user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    tauri::async_runtime::spawn_blocking(move || {
        vrcx_0_application::cleanup_avatar_feed_history(db.as_ref(), user_id, cutoff_date)
    })
    .await
    .map_err(|error| AppError::Custom(format!("avatar feed cleanup task: {error}")))?
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__feed_latest_query(
    state: State<'_, AppState>,
    query: FeedLatestQueryInput,
) -> Result<FeedReadModelOutput, AppError> {
    let realtime_runtime = state.realtime_runtime.clone();
    tauri::async_runtime::spawn_blocking(move || realtime_runtime.query_feed_latest(query))
        .await
        .map_err(|error| AppError::Custom(format!("feed latest query task: {error}")))?
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__feed_search_query(
    state: State<'_, AppState>,
    query: FeedSearchQueryInput,
) -> Result<Vec<FeedRowOutput>, AppError> {
    let realtime_runtime = state.realtime_runtime.clone();
    tauri::async_runtime::spawn_blocking(move || realtime_runtime.query_feed_search(query))
        .await
        .map_err(|error| AppError::Custom(format!("feed search query task: {error}")))?
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__feed_rows_query(
    state: State<'_, AppState>,
    query: FeedRowsQueryInput,
) -> Result<Vec<FeedRowOutput>, AppError> {
    let db = state.db.clone();
    tauri::async_runtime::spawn_blocking(move || {
        vrcx_0_persistence::feed::feed_rows_query(db.as_ref(), query)
    })
    .await
    .map_err(|error| AppError::Custom(format!("feed rows query task: {error}")))?
    .map_err(AppError::from)
}
