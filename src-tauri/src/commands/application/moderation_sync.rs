#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    self as moderation_sync, ModerationSyncDeps, ModerationSyncMutationInput,
    ModerationSyncMutationOutput, ModerationSyncRefreshInput, ModerationSyncRefreshOutput,
};
use vrcx_0_application_core::RuntimeOperationStatus;

use crate::error::AppError;
use crate::state::AppState;

fn deps<'a>(state: &'a State<'_, AppState>) -> ModerationSyncDeps<'a> {
    ModerationSyncDeps {
        db: &state.db,
        web: &state.web,
        auth_scope: &state.runtime_context.auth_scope,
        remote_mutations: state.runtime_context.remote_mutations.as_ref(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn app__moderation_sync_refresh(
    state: State<'_, AppState>,
    input: ModerationSyncRefreshInput,
) -> Result<ModerationSyncRefreshOutput, AppError> {
    let command = "app__moderation_sync_refresh";
    let diagnostics = state.runtime_context.diagnostics.clone();
    let sync = state.runtime_context.sync.clone();
    diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        "Moderation snapshot refresh started.",
    );

    let result = moderation_sync::refresh_player_moderations(deps(&state), input).await;
    match &result {
        Ok(output) => {
            diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!(
                    "user={} remote={} local={}",
                    output.user_id, output.remote_count, output.local_count
                ),
            );
            sync.record(
                "moderation",
                RuntimeOperationStatus::Ready,
                format!(
                    "Moderation snapshot refreshed for {} with {} local rows.",
                    output.user_id, output.local_count
                ),
                0,
            );
        }
        Err(error) => {
            diagnostics.record_command(command, RuntimeOperationStatus::Error, error.to_string());
            sync.record_failure("moderation", error.to_string());
        }
    }

    Ok(result?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__moderation_sync_update(
    state: State<'_, AppState>,
    input: ModerationSyncMutationInput,
) -> Result<ModerationSyncMutationOutput, AppError> {
    let command = "app__moderation_sync_update";
    let diagnostics = state.runtime_context.diagnostics.clone();
    let sync = state.runtime_context.sync.clone();
    diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        "Moderation mutation started.",
    );

    let result = moderation_sync::update_player_moderation(deps(&state), input).await;
    match &result {
        Ok(output) => {
            diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!(
                    "target={} type={} enabled={}",
                    output.target_user_id, output.r#type, output.enabled
                ),
            );
            sync.record(
                "moderation",
                RuntimeOperationStatus::Ready,
                format!(
                    "Moderation {} {} for {}.",
                    output.r#type,
                    if output.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    output.target_user_id
                ),
                0,
            );
        }
        Err(error) => {
            diagnostics.record_command(command, RuntimeOperationStatus::Error, error.to_string());
            sync.record_failure("moderation", error.to_string());
        }
    }

    Ok(result?)
}
