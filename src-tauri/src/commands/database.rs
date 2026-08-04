#![allow(non_snake_case)]

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_application::{
    DatabaseUpgradePreflight, DatabaseUpgradePreflightStatus, DatabaseUpgradeProgress,
    DatabaseUpgradeRunResult, DatabaseUpgradeRunStatus,
};

const ERROR_LOG_FILE: &str = "error-log.txt";
const ANONYMOUS_USAGE_TELEMETRY_CONFIG_KEY: &str = "anonymousUsageTelemetry";
const FAILURE_TELEMETRY_FLUSH_TIMEOUT: Duration = Duration::from_secs(1);

pub(crate) async fn flush_pending_upgrade_failure_telemetry(state: &AppState) {
    let telemetry = state.desktop.telemetry.clone();
    if tokio::time::timeout(
        FAILURE_TELEMETRY_FLUSH_TIMEOUT,
        telemetry.flush_pending_rust_errors(),
    )
    .await
    .is_err()
    {
        tracing::debug!("database upgrade failure telemetry flush timed out");
    }
}

#[tauri::command]
#[specta::specta]
pub async fn app__database_upgrade_preflight(
    state: State<'_, AppState>,
) -> Result<DatabaseUpgradePreflight, AppError> {
    let runtime = state.database_upgrade.clone();
    let preflight = tauri::async_runtime::spawn_blocking(move || runtime.preflight())
        .await
        .map_err(|error| {
            AppError::Custom(format!("database upgrade preflight task failed: {error}"))
        })?
        .map_err(AppError::from)?;
    if preflight.status == DatabaseUpgradePreflightStatus::Blocked {
        tracing::error!(
            from_version = preflight.from_version,
            to_version = preflight.to_version,
            error = %preflight
                .failed_upgrade
                .as_ref()
                .and_then(|failure| failure.reason.as_deref())
                .unwrap_or("previous database upgrade did not finish"),
            "database upgrade remains blocked by a previous failure"
        );
        flush_pending_upgrade_failure_telemetry(&state).await;
    }
    Ok(preflight)
}

#[tauri::command]
#[specta::specta]
pub async fn app__database_upgrade_run(
    state: State<'_, AppState>,
) -> Result<DatabaseUpgradeRunResult, AppError> {
    let runtime = state.database_upgrade.clone();
    let result = tauri::async_runtime::spawn_blocking(move || runtime.run())
        .await
        .map_err(|error| AppError::Custom(format!("database upgrade task failed: {error}")))?;
    if result.status == DatabaseUpgradeRunStatus::Failed {
        flush_pending_upgrade_failure_telemetry(&state).await;
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn app__database_upgrade_progress(state: State<'_, AppState>) -> DatabaseUpgradeProgress {
    state.database_upgrade.progress()
}

#[tauri::command]
#[specta::specta]
pub async fn app__database_upgrade_retry(
    state: State<'_, AppState>,
) -> Result<DatabaseUpgradeRunResult, AppError> {
    let runtime = state.database_upgrade.clone();
    let result = tauri::async_runtime::spawn_blocking(move || runtime.retry())
        .await
        .map_err(|error| AppError::Custom(format!("database upgrade retry task failed: {error}")))?
        .map_err(AppError::from)?;
    if result.status == DatabaseUpgradeRunStatus::Failed {
        flush_pending_upgrade_failure_telemetry(&state).await;
    }
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn app__database_upgrade_failure_log_path(state: State<'_, AppState>) -> String {
    state
        .paths
        .app_data
        .join(ERROR_LOG_FILE)
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
#[specta::specta]
pub async fn app__database_upgrade_start_fresh(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let anonymous_usage_telemetry = state
        .runtime_context
        .config()
        .get_bool(ANONYMOUS_USAGE_TELEMETRY_CONFIG_KEY, true)
        .unwrap_or(true);
    let runtime = state.database_upgrade.clone();
    super::host::window::stop_runtime_services(&app_handle);
    let recovery_result =
        match tauri::async_runtime::spawn_blocking(move || runtime.start_fresh_database()).await {
            Ok(result) => result.map_err(AppError::from),
            Err(error) => Err(AppError::Custom(format!(
                "database fresh-start task failed: {error}"
            ))),
        };
    match recovery_result {
        Ok(recovery_dir) => {
            if !anonymous_usage_telemetry {
                if let Err(error) = state
                    .runtime_context
                    .config()
                    .set_bool(ANONYMOUS_USAGE_TELEMETRY_CONFIG_KEY, false)
                {
                    tracing::error!(
                        error = %error,
                        "failed to preserve the disabled telemetry preference in the fresh database"
                    );
                }
            }
            tracing::info!(
                recovery_dir = %recovery_dir.display(),
                "archived the previous database before starting fresh"
            );
            app_handle.request_restart();
            Ok(recovery_dir.to_string_lossy().into_owned())
        }
        Err(error) => {
            tracing::error!(error = %error, "failed to start with a fresh database");
            app_handle.request_restart();
            Err(error)
        }
    }
}
