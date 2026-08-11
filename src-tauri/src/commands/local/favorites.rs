#![allow(non_snake_case)]

use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_application::{FavoriteRow, LocalFavoriteSnapshot};
use vrcx_0_application_core::{FavoriteEntityKind, FavoritesChangedPayload};

#[tauri::command]
#[specta::specta]
pub fn app__favorite_list(
    state: State<'_, AppState>,
    kind: FavoriteEntityKind,
) -> Result<Vec<FavoriteRow>, AppError> {
    let owner_user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    vrcx_0_application::list_local_favorites(state.db.as_ref(), &owner_user_id, kind)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__favorite_local_snapshot(
    state: State<'_, AppState>,
    kind: FavoriteEntityKind,
) -> Result<LocalFavoriteSnapshot, AppError> {
    let owner_user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    vrcx_0_application::get_local_favorite_snapshot(state.db.as_ref(), &owner_user_id, kind)
        .map_err(AppError::from)
}

pub fn favorite_add(
    state: State<'_, AppState>,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64, AppError> {
    let owner_user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    let affected = vrcx_0_application::add_local_favorite(
        state.db.as_ref(),
        &owner_user_id,
        kind,
        entity_id,
        group_name,
    )
    .map_err(AppError::from)?;
    state
        .realtime_runtime
        .notify_favorites_changed(FavoritesChangedPayload {
            kind: kind.into(),
            local: true,
            remote: false,
        });
    Ok(affected)
}

pub fn favorite_remove(
    state: State<'_, AppState>,
    kind: FavoriteEntityKind,
    entity_id: String,
    group_name: String,
) -> Result<i64, AppError> {
    let owner_user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    let affected = vrcx_0_application::remove_local_favorite(
        state.db.as_ref(),
        &owner_user_id,
        kind,
        entity_id,
        group_name,
    )
    .map_err(AppError::from)?;
    state
        .realtime_runtime
        .notify_favorites_changed(FavoritesChangedPayload {
            kind: kind.into(),
            local: true,
            remote: false,
        });
    Ok(affected)
}
