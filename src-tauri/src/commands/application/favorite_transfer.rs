#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    remove_favorites_selection, transfer_favorite_selection, FavoriteBulkRemoveDeps,
    FavoriteBulkRemoveInput, FavoriteBulkRemoveResult, FavoriteTransferDeps,
    FavoriteTransferSelectionInput, FavoriteTransferSelectionResult,
};
use vrcx_0_application_core::{FavoritesChangedPayload, RuntimeOperationStatus};

use crate::error::AppError;
use crate::state::AppState;

fn record_bulk_remove_outcome(
    state: &State<'_, AppState>,
    command: &str,
    kind: vrcx_0_application_core::FavoriteChangeScope,
    result: &vrcx_0_application_core::Result<FavoriteBulkRemoveResult>,
) {
    let diagnostics = &state.runtime_context.diagnostics;
    let sync = &state.runtime_context.sync;
    match result {
        Ok(output) => {
            diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!("succeeded={}, failed={}", output.succeeded, output.failed),
            );
            sync.record(
                "favorite",
                RuntimeOperationStatus::Ready,
                format!(
                    "Removed {} favorite item(s); {} failed.",
                    output.succeeded, output.failed
                ),
                0,
            );
            state
                .realtime_runtime
                .notify_favorites_changed(FavoritesChangedPayload {
                    kind,
                    local: output.local_changed,
                    remote: output.remote_changed,
                });
        }
        Err(error) => {
            diagnostics.record_command(command, RuntimeOperationStatus::Error, error.to_string());
            sync.record_failure("favorite", error.to_string());
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn app__favorites_transfer_selection(
    state: State<'_, AppState>,
    input: FavoriteTransferSelectionInput,
) -> Result<FavoriteTransferSelectionResult, AppError> {
    let command = "app__favorites_transfer_selection";
    let item_count = input
        .batches
        .iter()
        .map(|batch| batch.items.len())
        .sum::<usize>();
    let kind = input
        .batches
        .first()
        .map(|batch| batch.kind.into())
        .unwrap_or(vrcx_0_application_core::FavoriteChangeScope::All);
    let diagnostics = state.runtime_context.diagnostics.clone();
    let sync = state.runtime_context.sync.clone();
    diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        format!("Transferring {item_count} favorite item(s)."),
    );
    let owner_user_id = state.runtime_context.auth_scope.snapshot().current_user_id;
    let result = transfer_favorite_selection(
        FavoriteTransferDeps {
            db: state.db.as_ref(),
            owner_user_id: &owner_user_id,
            web: state.web.as_ref(),
            diagnostics: &diagnostics,
            sync: &sync,
        },
        input,
    )
    .await;

    match &result {
        Ok(output) => {
            diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!("succeeded={}, failed={}", output.succeeded, output.failed),
            );
            sync.record(
                "favorite",
                RuntimeOperationStatus::Ready,
                format!(
                    "Transferred {} favorite item(s); {} failed.",
                    output.succeeded, output.failed
                ),
                0,
            );
            state
                .realtime_runtime
                .notify_favorites_changed(FavoritesChangedPayload {
                    kind,
                    local: output.local_changed,
                    remote: output.remote_changed,
                });
        }
        Err(error) => {
            diagnostics.record_command(command, RuntimeOperationStatus::Error, error.to_string());
            sync.record_failure("favorite", error.to_string());
        }
    }

    Ok(result?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__favorites_remove_selection(
    state: State<'_, AppState>,
    input: FavoriteBulkRemoveInput,
) -> Result<FavoriteBulkRemoveResult, AppError> {
    let command = "app__favorites_remove_selection";
    let target_count = input.items.len();
    let kind = input.kind.into();
    let diagnostics = state.runtime_context.diagnostics.clone();
    diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        format!("Removing {target_count} favorite item(s)."),
    );
    let expected_scope = super::scope::require_active_scope(&state, "Favorite bulk remove")?;
    let result = remove_favorites_selection(
        &FavoriteBulkRemoveDeps {
            db: state.db.as_ref(),
            web: state.web.as_ref(),
            auth_scope: &state.runtime_context.auth_scope,
            expected_scope,
            remote_mutation_gate: &state.remote_mutations,
        },
        input,
    )
    .await;

    record_bulk_remove_outcome(&state, command, kind, &result);

    Ok(result?)
}
