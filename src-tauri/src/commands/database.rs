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

fn database_upgrade_failure_token(value: Option<&str>, fallback: &str) -> String {
    let value = value.unwrap_or(fallback);
    let token = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '+' | '-'))
        .take(64)
        .collect::<String>();
    if token.is_empty() {
        fallback.to_string()
    } else {
        token
    }
}

fn log_interrupted_database_upgrade(
    stage: Option<&str>,
    operation: Option<&str>,
    from_version: i64,
    to_version: i64,
    started_app_version: Option<&str>,
    reason: &str,
) {
    let stage = database_upgrade_failure_token(stage, "beforeFirstStage");
    let operation = database_upgrade_failure_token(operation, "unknown");
    let started_app_version = database_upgrade_failure_token(started_app_version, "unknown");
    tracing::error!(
        "database upgrade failure [status=interrupted stage={stage} operation={operation} sqliteCategory=none from={from_version} to={to_version} appVersion={started_app_version}]: {reason}"
    );
}

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
        let failure = preflight.failed_upgrade.as_ref();
        if failure.is_none_or(|failure| failure.failed_at.is_none()) {
            log_interrupted_database_upgrade(
                failure.and_then(|failure| failure.stage.as_deref()),
                failure.and_then(|failure| failure.operation.as_deref()),
                preflight.from_version,
                preflight.to_version,
                failure.and_then(|failure| failure.app_version.as_deref()),
                failure
                    .and_then(|failure| failure.reason.as_deref())
                    .unwrap_or("previous database upgrade did not finish"),
            );
        }
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
