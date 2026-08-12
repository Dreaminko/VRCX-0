#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    FavoriteBulkRemoveInput, FavoriteBulkRemoveResult, FavoriteTransferSelectionInput,
    FavoriteTransferSelectionResult,
};

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn app__favorites_transfer_selection(
    state: State<'_, AppState>,
    input: FavoriteTransferSelectionInput,
) -> Result<FavoriteTransferSelectionResult, AppError> {
    state
        .runtime_context
        .favorite_mutations
        .transfer_selection(input)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn app__favorites_remove_selection(
    state: State<'_, AppState>,
    input: FavoriteBulkRemoveInput,
) -> Result<FavoriteBulkRemoveResult, AppError> {
    state
        .runtime_context
        .favorite_mutations
        .remove_selection(input)
        .await
        .map_err(AppError::from)
}
