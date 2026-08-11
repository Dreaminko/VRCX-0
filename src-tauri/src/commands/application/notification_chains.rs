#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    accept_request_invite_notification, dismiss_boop_notifications, hide_and_expire_notification,
    respond_and_expire_notification, send_boop_reply_notification,
    send_instance_invite_notification, send_invite_response_notification,
    NotificationActionOutcome, NotificationBoopDismissInput, NotificationBoopReplyInput,
    NotificationHideExpireInput, NotificationInstanceInviteInput, NotificationInviteResponseInput,
    NotificationRequestInviteAcceptInput, NotificationRespondInput, VrchatNotificationChainActions,
};
use vrcx_0_application_core::RuntimeAuthScopeSnapshot;

use crate::{error::AppError, state::AppState};

fn active_scope(state: &AppState) -> Result<RuntimeAuthScopeSnapshot, AppError> {
    super::scope::require_active_scope(state, "Notification action")
}

fn chain_actions(state: &AppState) -> Result<VrchatNotificationChainActions<'_>, AppError> {
    Ok(VrchatNotificationChainActions {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope: active_scope(state)?,
        event_bus: &state.runtime_context.event_bus,
        world_cache: state.runtime_context.world_cache.as_ref(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_hide_and_expire(
    state: State<'_, AppState>,
    input: NotificationHideExpireInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(hide_and_expire_notification(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_request_invite_accept(
    state: State<'_, AppState>,
    input: NotificationRequestInviteAcceptInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(accept_request_invite_notification(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_instance_invite_send(
    state: State<'_, AppState>,
    input: NotificationInstanceInviteInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(send_instance_invite_notification(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_invite_response_send(
    state: State<'_, AppState>,
    input: NotificationInviteResponseInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(send_invite_response_notification(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_boop_dismiss(
    state: State<'_, AppState>,
    input: NotificationBoopDismissInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(dismiss_boop_notifications(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_boop_reply(
    state: State<'_, AppState>,
    input: NotificationBoopReplyInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(send_boop_reply_notification(&actions, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__notification_respond_and_expire(
    state: State<'_, AppState>,
    input: NotificationRespondInput,
) -> Result<NotificationActionOutcome, AppError> {
    let actions = chain_actions(&state)?;
    Ok(respond_and_expire_notification(&actions, input).await?)
}
