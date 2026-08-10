#![allow(non_snake_case)]

use serde_json::Value;
use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

use vrcx_0_core::json::RawJson;
use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;
use vrcx_0_persistence::avatars::{
    AvatarCacheOutput, AvatarTagInput, AvatarTagOutput, AvatarTagsPatchInput, AvatarTimeSpentOutput,
};

#[derive(Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvatarGetInput {
    pub avatar_id: String,
    #[serde(default)]
    pub full: bool,
    #[serde(default)]
    pub fresh: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn app__avatar_get(
    state: State<'_, AppState>,
    input: AvatarGetInput,
) -> Result<Option<RawJson>, AppError> {
    let auth_scope = state.runtime_context.auth_scope.snapshot();
    let endpoint = if auth_scope.endpoint.is_empty() {
        VRCHAT_API_DEFAULT_ENDPOINT
    } else {
        auth_scope.endpoint.as_str()
    };
    state
        .runtime_context
        .avatar_cache
        .resolve(
            state.web.as_ref(),
            &auth_scope.current_user_id,
            endpoint,
            &input.avatar_id,
            input.full,
            input.fresh,
        )
        .await
        .map(|avatar| avatar.map(|value| RawJson::from(value.as_ref().clone())))
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_find_by_image_url(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<Option<RawJson>, AppError> {
    let auth_scope = state.runtime_context.auth_scope.snapshot();
    let endpoint = if auth_scope.endpoint.is_empty() {
        VRCHAT_API_DEFAULT_ENDPOINT
    } else {
        auth_scope.endpoint.as_str()
    };
    state
        .runtime_context
        .avatar_cache
        .find_by_image_url(&auth_scope.current_user_id, endpoint, &image_url)
        .map(|avatar| avatar.map(|value| RawJson::from(value.as_ref().clone())))
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_history_clear(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<(), AppError> {
    vrcx_0_persistence::avatars::avatar_history_clear(state.db.as_ref(), user_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_history_list(
    state: State<'_, AppState>,
    user_id: String,
    limit: i64,
) -> Result<Vec<AvatarCacheOutput>, AppError> {
    vrcx_0_persistence::avatars::avatar_history_list(state.db.as_ref(), user_id, limit)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tag_add(
    state: State<'_, AppState>,
    avatar_id: String,
    tag: Value,
    color: Value,
) -> Result<i64, AppError> {
    vrcx_0_persistence::avatars::avatar_tag_add(state.db.as_ref(), avatar_id, tag, color)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tag_remove(
    state: State<'_, AppState>,
    avatar_id: String,
    tag: Value,
) -> Result<i64, AppError> {
    vrcx_0_persistence::avatars::avatar_tag_remove(state.db.as_ref(), avatar_id, tag)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tag_update_color(
    state: State<'_, AppState>,
    avatar_id: String,
    tag: Value,
    color: Value,
) -> Result<i64, AppError> {
    vrcx_0_persistence::avatars::avatar_tag_update_color(state.db.as_ref(), avatar_id, tag, color)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_distinct(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    vrcx_0_persistence::avatars::avatar_tags_distinct(state.db.as_ref()).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_get(
    state: State<'_, AppState>,
    avatar_id: String,
) -> Result<Vec<AvatarTagOutput>, AppError> {
    vrcx_0_persistence::avatars::avatar_tags_get(state.db.as_ref(), avatar_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_list(state: State<'_, AppState>) -> Result<Vec<AvatarTagOutput>, AppError> {
    vrcx_0_persistence::avatars::avatar_tags_list(state.db.as_ref()).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_patch(
    state: State<'_, AppState>,
    avatar_id: String,
    patch: AvatarTagsPatchInput,
) -> Result<(), AppError> {
    vrcx_0_persistence::avatars::avatar_tags_patch(state.db.as_ref(), avatar_id, patch)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_remove_all(
    state: State<'_, AppState>,
    avatar_id: String,
) -> Result<i64, AppError> {
    vrcx_0_persistence::avatars::avatar_tags_remove_all(state.db.as_ref(), avatar_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_tags_replace(
    state: State<'_, AppState>,
    avatar_id: String,
    entries: Vec<AvatarTagInput>,
) -> Result<(), AppError> {
    vrcx_0_persistence::avatars::avatar_tags_replace(state.db.as_ref(), avatar_id, entries)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_time_spent_add(
    state: State<'_, AppState>,
    user_id: String,
    avatar_id: String,
    time_spent: i64,
) -> Result<(), AppError> {
    vrcx_0_persistence::avatars::avatar_time_spent_add(
        state.db.as_ref(),
        user_id,
        avatar_id,
        time_spent,
    )
    .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_time_spent_get(
    state: State<'_, AppState>,
    user_id: String,
    avatar_id: String,
) -> Result<AvatarTimeSpentOutput, AppError> {
    vrcx_0_persistence::avatars::avatar_time_spent_get(state.db.as_ref(), user_id, avatar_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub fn app__avatar_time_spent_list(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<AvatarTimeSpentOutput>, AppError> {
    vrcx_0_persistence::avatars::avatar_time_spent_list(state.db.as_ref(), user_id)
        .map_err(AppError::from)
}
