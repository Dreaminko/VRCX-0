#![allow(non_snake_case)]

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;
use vrcx_0_persistence::worlds::WorldSummaryOutput;

#[tauri::command]
#[specta::specta]
pub async fn app__world_get(
    state: State<'_, AppState>,
    world_id: String,
) -> Result<Option<WorldSummaryOutput>, AppError> {
    let auth_scope = state.runtime_context.auth_scope.snapshot();
    let endpoint = if auth_scope.endpoint.is_empty() {
        VRCHAT_API_DEFAULT_ENDPOINT
    } else {
        auth_scope.endpoint.as_str()
    };
    Ok(state
        .runtime_context
        .world_cache
        .resolve_summary(state.web.as_ref(), endpoint, &world_id)
        .await)
}

#[tauri::command]
#[specta::specta]
pub fn app__world_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<WorldSummaryOutput>, AppError> {
    state
        .runtime_context
        .world_cache
        .search_summaries(&query, 16)
        .map_err(AppError::from)
}
