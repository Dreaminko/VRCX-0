#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application as media_upload;
use vrcx_0_application_core::vrchat_api::notifications::{
    boop_send_input, request_invite_photo_input, request_invite_send_input,
};
use vrcx_0_application_core::vrchat_api::{VrchatApiRequest, VrchatApiResponse, VrchatScope};
use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;

use super::types::{
    VrchatBoopInput, VrchatNotificationPhotoSendInput, VrchatNotificationSendInput,
};
use crate::error::AppError;
use crate::state::AppState;

async fn execute_notification_api(
    state: State<'_, AppState>,
    command: &str,
    detail: impl Into<String>,
    input: VrchatApiRequest,
) -> Result<VrchatApiResponse, AppError> {
    super::super::execute::execute_vrchat_api(state, command, detail, input, VrchatScope::Vrchat)
        .await
}

async fn execute_media_api(
    state: State<'_, AppState>,
    command: &str,
    detail: impl Into<String>,
    input: VrchatApiRequest,
) -> Result<VrchatApiResponse, AppError> {
    super::super::execute::execute_vrchat_api(
        state,
        command,
        detail,
        input,
        VrchatScope::VrchatMedia,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_request_invite_send(
    state: State<'_, AppState>,
    input: VrchatNotificationSendInput,
) -> Result<VrchatApiResponse, AppError> {
    let (receiver_user_id, request) = request_invite_send_input(
        VRCHAT_API_DEFAULT_ENDPOINT.into(),
        input.receiver_user_id,
        input.params,
    )?;
    execute_notification_api(
        state,
        "app__vrchat_request_invite_send",
        format!("Sending invite request to {receiver_user_id}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_request_invite_photo_send(
    state: State<'_, AppState>,
    input: VrchatNotificationPhotoSendInput,
) -> Result<VrchatApiResponse, AppError> {
    let (receiver_user_id, request) = request_invite_photo_input(
        VRCHAT_API_DEFAULT_ENDPOINT.into(),
        input.receiver_user_id,
        input.params,
        input.image_data,
    )?;
    execute_media_api(
        state,
        "app__vrchat_request_invite_photo_send",
        format!("Sending invite request photo to {receiver_user_id}."),
        media_upload::prepare_media_upload_request(request)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_boop_send(
    state: State<'_, AppState>,
    input: VrchatBoopInput,
) -> Result<VrchatApiResponse, AppError> {
    let (user_id, request) = boop_send_input(
        VRCHAT_API_DEFAULT_ENDPOINT.into(),
        input.user_id,
        input.emoji_id,
    )?;
    execute_notification_api(
        state,
        "app__vrchat_boop_send",
        format!("Sending boop to {user_id}."),
        request,
    )
    .await
}
