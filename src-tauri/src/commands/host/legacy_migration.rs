#![allow(non_snake_case)]

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_persistence::legacy_migration::LegacyMigrationPaths;
use vrcx_0_persistence::legacy_vrcx::{
    LegacyVrcxDiscovery, LegacyVrcxMigrationStatus, LegacyVrcxSource,
};

#[tauri::command]
#[specta::specta]
pub fn app__check_legacy_vrcx_available(state: State<'_, AppState>) -> bool {
    state.legacy_vrcx_available
}

#[tauri::command]
#[specta::specta]
pub fn app__get_legacy_vrcx_migration_status(
    state: State<'_, AppState>,
) -> LegacyVrcxMigrationStatus {
    state.legacy_vrcx_migration_status.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn app__get_legacy_vrcx_force_migration_status(
) -> Result<LegacyVrcxMigrationStatus, AppError> {
    Ok(discover_legacy_vrcx_source().await?.status)
}

async fn discover_legacy_vrcx_source() -> Result<LegacyVrcxDiscovery, AppError> {
    tauri::async_runtime::spawn_blocking(
        vrcx_0_persistence::legacy_vrcx::discover_supported_legacy_source,
    )
    .await
    .map_err(|error| AppError::Custom(format!("legacy VRCX discovery task failed: {error}")))
}

fn legacy_migration_unavailable_reason(status: &LegacyVrcxMigrationStatus) -> String {
    status
        .reason
        .clone()
        .unwrap_or_else(|| "Legacy VRCX migration is unavailable.".to_string())
}

async fn stage_legacy_migration(
    state: &AppState,
    paths: LegacyMigrationPaths,
    source: LegacyVrcxSource,
) -> Result<(), AppError> {
    let runtime = state.database_upgrade.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        vrcx_0_persistence::legacy_vrcx::validate_legacy_source(&source)
            .map_err(AppError::Custom)?;
        runtime
            .prepare_legacy_migration(&paths, &source)
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::Custom(format!("legacy migration task failed: {error}")))
    .and_then(|result| result);
    if let Err(error) = &result {
        tracing::error!(error = %error, "legacy VRCX snapshot preparation failed");
        super::super::database::flush_pending_upgrade_failure_telemetry(state).await;
    }
    result
}

#[tauri::command]
#[specta::specta]
pub async fn app__request_legacy_migration(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let Some(source) = state.legacy_vrcx_source.clone() else {
        return Err(AppError::Custom(legacy_migration_unavailable_reason(
            &state.legacy_vrcx_migration_status,
        )));
    };
    #[cfg(debug_assertions)]
    {
        tracing::warn!("app__request_legacy_migration: dev mode does not auto-restart or persist migration flag");
        let _ = (app_handle, state, source);
        Ok(false)
    }

    #[cfg(not(debug_assertions))]
    {
        let paths = LegacyMigrationPaths::from_app_data(state.paths.app_data.clone());
        stage_legacy_migration(&state, paths, source).await?;
        super::window::stop_runtime_services(&app_handle);
        app_handle.request_restart();
        Ok(true)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn app__request_legacy_vrcx_force_migration(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let discovery = discover_legacy_vrcx_source().await?;
    let Some(source) = discovery.importable_source else {
        return Err(AppError::Custom(legacy_migration_unavailable_reason(
            &discovery.status,
        )));
    };
    let paths = LegacyMigrationPaths::from_app_data(state.paths.app_data.clone());
    stage_legacy_migration(&state, paths, source).await?;

    #[cfg(debug_assertions)]
    {
        tracing::warn!(
            "app__request_legacy_vrcx_force_migration: dev mode wrote migration flag but did not auto-restart"
        );
        let _ = app_handle;
        Ok(false)
    }

    #[cfg(not(debug_assertions))]
    {
        super::window::stop_runtime_services(&app_handle);
        app_handle.request_restart();
        Ok(true)
    }
}
