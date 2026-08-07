#![allow(non_snake_case)]

use serde::Deserialize;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_application_core::RuntimeOperationStatus;
use vrcx_0_runtime_host_desktop::AncillaryRuntimeSnapshot;

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeJobRecordInput {
    pub name: String,
    #[serde(default = "default_frontend_owner")]
    pub owner: String,
    #[serde(default)]
    pub cadence_seconds: Option<u64>,
    pub status: RuntimeOperationStatus,
    #[serde(default)]
    pub detail: String,
}

fn default_frontend_owner() -> String {
    "frontend".into()
}

#[tauri::command]
#[specta::specta]
pub async fn app__ancillary_runtime_snapshot_get(
    state: State<'_, AppState>,
) -> Result<AncillaryRuntimeSnapshot, AppError> {
    Ok(state.ancillary_runtime_snapshot().await)
}

#[tauri::command]
#[specta::specta]
pub async fn app__runtime_group_instances_refresh(
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.refresh_runtime_group_instances().await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn app__runtime_discord_reconcile_request(state: State<'_, AppState>) -> u64 {
    state.request_discord_reconcile()
}

#[tauri::command]
#[specta::specta]
pub fn app__runtime_background_job_record(
    state: State<'_, AppState>,
    input: RuntimeJobRecordInput,
) {
    let name = input.name.trim();
    if name.is_empty() {
        return;
    }

    let detail = input.detail.trim();
    state.runtime_context.background_jobs.register_job(
        name,
        input.owner.trim(),
        input.cadence_seconds,
        input.status,
        detail,
    );
    match input.status {
        RuntimeOperationStatus::Running => state
            .runtime_context
            .background_jobs
            .mark_running(name, detail),
        RuntimeOperationStatus::Completed | RuntimeOperationStatus::Idle => state
            .runtime_context
            .background_jobs
            .mark_completed(name, detail),
        RuntimeOperationStatus::Error => state
            .runtime_context
            .background_jobs
            .mark_failed(name, detail),
        status => state.runtime_context.background_jobs.register_job(
            name,
            input.owner.trim(),
            input.cadence_seconds,
            status,
            detail,
        ),
    }
}
