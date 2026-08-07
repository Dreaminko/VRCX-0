#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application_activity::{
    overlay_activity_type_definitions, OverlayActivityTypeDefinition,
};
use vrcx_0_runtime_host::notification::{
    NotificationActivityFiltersSetInput, OverlayActivityPreferenceFilters,
};

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__overlay_activity_definitions_get(
) -> Result<Vec<OverlayActivityTypeDefinition>, AppError> {
    Ok(overlay_activity_type_definitions())
}

#[tauri::command]
#[specta::specta]
pub fn app__overlay_activity_filters_set(
    state: State<'_, AppState>,
    filters: OverlayActivityPreferenceFilters,
) -> Result<(), AppError> {
    state
        .runtime_context
        .set_overlay_activity_preference_filters(filters)?;
    state.desktop.vr_overlay_runtime.reconcile_current();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn app__notification_activity_filters_set(
    state: State<'_, AppState>,
    input: NotificationActivityFiltersSetInput,
) -> Result<(), AppError> {
    state
        .runtime_context
        .set_notification_activity_filters(input)?;
    state.desktop.vr_overlay_runtime.reconcile_current();
    Ok(())
}
