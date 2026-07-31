#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    hydrate_favorite_details, mark_notifications_seen_batch, run_avatar_content_tags_batch,
    run_group_leave_batch, run_group_visibility_batch, sync_notifications,
    AvatarContentTagsBatchInput, BatchMutationResult, FavoriteDetailsHydrateDeps,
    FavoriteDetailsHydrateInput, FavoriteDetailsHydrateOutput, FavoriteImportStartInput,
    FavoriteImportStatus, GroupLeaveBatchInput, GroupVisibilityBatchInput,
    NotificationMarkSeenBatchInput, NotificationMarkSeenBatchResult, NotificationSyncDeps,
    NotificationSyncOutcome, VrchatBatchMutationActions, VrchatNotificationMarkSeenActions,
};
use vrcx_0_application_core::RuntimeAuthScopeSnapshot;

use crate::{error::AppError, state::AppState};

#[tauri::command]
#[specta::specta]
pub fn app__favorite_import_start(
    state: State<'_, AppState>,
    input: FavoriteImportStartInput,
) -> Result<FavoriteImportStatus, AppError> {
    Ok(state.favorite_import.start(input)?)
}

#[tauri::command]
#[specta::specta]
pub fn app__favorite_import_status(state: State<'_, AppState>) -> FavoriteImportStatus {
    state.favorite_import.status()
}

#[tauri::command]
#[specta::specta]
pub fn app__favorite_import_cancel(state: State<'_, AppState>) -> FavoriteImportStatus {
    state.favorite_import.cancel()
}

#[tauri::command]
#[specta::specta]
pub async fn app__favorite_details_hydrate(
    state: State<'_, AppState>,
    input: FavoriteDetailsHydrateInput,
) -> Result<FavoriteDetailsHydrateOutput, AppError> {
    let expected_scope = active_scope(&state)?;
    let deps = FavoriteDetailsHydrateDeps {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(hydrate_favorite_details(&deps, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__avatar_content_tags_batch(
    state: State<'_, AppState>,
    input: AvatarContentTagsBatchInput,
) -> Result<BatchMutationResult, AppError> {
    let expected_scope = active_scope(&state)?;
    let actions = VrchatBatchMutationActions {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(run_avatar_content_tags_batch(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__group_visibility_batch(
    state: State<'_, AppState>,
    input: GroupVisibilityBatchInput,
) -> Result<BatchMutationResult, AppError> {
    let expected_scope = active_scope(&state)?;
    let actions = VrchatBatchMutationActions {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(run_group_visibility_batch(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__group_leave_batch(
    state: State<'_, AppState>,
    input: GroupLeaveBatchInput,
) -> Result<BatchMutationResult, AppError> {
    let expected_scope = active_scope(&state)?;
    let actions = VrchatBatchMutationActions {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(run_group_leave_batch(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_mark_seen_batch(
    state: State<'_, AppState>,
    input: NotificationMarkSeenBatchInput,
) -> Result<NotificationMarkSeenBatchResult, AppError> {
    let expected_scope = active_scope(&state)?;
    let actions = VrchatNotificationMarkSeenActions {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(mark_notifications_seen_batch(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_sync(
    state: State<'_, AppState>,
) -> Result<NotificationSyncOutcome, AppError> {
    let expected_scope = active_scope(&state)?;
    let deps = NotificationSyncDeps {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(sync_notifications(&deps).await?)
}

pub(crate) fn require_active_scope(
    state: &AppState,
    requirement: &str,
) -> Result<RuntimeAuthScopeSnapshot, AppError> {
    let scope = state.runtime_context.auth_scope.snapshot();
    if scope.active && !scope.current_user_id.trim().is_empty() {
        Ok(scope)
    } else {
        Err(vrcx_0_application_core::Error::Custom(format!(
            "{requirement} requires an authenticated session."
        ))
        .into())
    }
}

fn active_scope(state: &AppState) -> Result<RuntimeAuthScopeSnapshot, AppError> {
    require_active_scope(state, "Batch action")
}
