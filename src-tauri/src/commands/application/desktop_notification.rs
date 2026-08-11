#![allow(non_snake_case)]

use tauri::State;

use crate::desktop_notification_activation::DesktopNotificationActivation;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn app__take_pending_desktop_notification_activation(
    state: State<'_, AppState>,
) -> Option<DesktopNotificationActivation> {
    let auth_scope = state.runtime_context.auth_scope.snapshot();
    if !auth_scope.active {
        return None;
    }
    state
        .pending_desktop_notification_activations
        .take_for_owner(&auth_scope.current_user_id)
}
