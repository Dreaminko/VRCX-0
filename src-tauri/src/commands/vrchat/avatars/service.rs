#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application_core::vrchat_api::avatars::{
    avatar_delete_input, avatar_file_get_input, avatar_gallery_get_input,
    avatar_impostor_create_input, avatar_impostor_delete_input, avatar_list_by_user_get_input,
    avatar_moderation_delete_input, avatar_moderation_send_input, avatar_moderations_get_input,
    avatar_save_input, avatar_select_fallback_input, avatar_select_input, avatar_styles_get_input,
    AvatarListByUserGetInput,
};
use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_application_core::vrchat_api::{VrchatApiRequest, VrchatApiResponse, VrchatScope};

use super::types::{
    VrchatAvatarFileInput, VrchatAvatarIdInput, VrchatAvatarListByUserInput,
    VrchatAvatarModerationInput, VrchatAvatarSaveInput,
};

fn avatar_mutation_deps<'a>(
    state: &'a State<'_, AppState>,
) -> Result<vrcx_0_application::AvatarRemoteMutationDeps<'a>, AppError> {
    Ok(vrcx_0_application::AvatarRemoteMutationDeps {
        db: &state.db,
        web: &state.web,
        diagnostics: &state.runtime_context.diagnostics,
        sync: &state.runtime_context.sync,
        realtime: &state.realtime_runtime,
        avatar_cache: &state.runtime_context.avatar_cache,
        mutation: vrcx_0_application::AuthenticatedMutationContext::capture(
            &state.runtime_context.auth_scope,
            &state.runtime_context.remote_mutations,
            "Avatar mutation",
        )?,
    })
}

async fn execute_avatar_api(
    state: State<'_, AppState>,
    command: &str,
    detail: impl Into<String>,
    input: VrchatApiRequest,
) -> Result<VrchatApiResponse, AppError> {
    super::super::execute::execute_vrchat_api(state, command, detail, input, VrchatScope::Vrchat)
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_gallery_get(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let (avatar_id, request) =
        avatar_gallery_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.avatar_id)?;
    execute_avatar_api(
        state,
        "app__vrchat_avatar_gallery_get",
        format!("Getting avatar gallery for {avatar_id}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_list_by_user_get(
    state: State<'_, AppState>,
    input: VrchatAvatarListByUserInput,
) -> Result<VrchatApiResponse, AppError> {
    let (display_user, request) = avatar_list_by_user_get_input(AvatarListByUserGetInput {
        endpoint: VRCHAT_API_DEFAULT_ENDPOINT.into(),
        user_id: input.user_id,
        user: input.user,
        n: input.n,
        offset: input.offset,
        sort: input.sort,
        order: input.order,
        release_status: input.release_status,
    })?;
    execute_avatar_api(
        state,
        "app__vrchat_avatar_list_by_user_get",
        format!("Getting avatars for {display_user}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_styles_get(
    state: State<'_, AppState>,
) -> Result<VrchatApiResponse, AppError> {
    execute_avatar_api(
        state,
        "app__vrchat_avatar_styles_get",
        "Getting avatar styles.",
        avatar_styles_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into()),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_moderations_get(
    state: State<'_, AppState>,
) -> Result<VrchatApiResponse, AppError> {
    execute_avatar_api(
        state,
        "app__vrchat_avatar_moderations_get",
        "Getting avatar moderations.",
        avatar_moderations_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into()),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_file_get(
    state: State<'_, AppState>,
    input: VrchatAvatarFileInput,
) -> Result<VrchatApiResponse, AppError> {
    let (file_id, request) =
        avatar_file_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.file_id)?;
    execute_avatar_api(
        state,
        "app__vrchat_avatar_file_get",
        format!("Getting file {file_id}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_select(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<vrcx_0_application::AvatarSelectionMutationOutcome, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) =
        avatar_select_input(deps.mutation.scope().endpoint.clone(), input.avatar_id)?;
    Ok(vrcx_0_application::select_avatar(
        &deps,
        "app__vrchat_avatar_select",
        format!("Selecting avatar {avatar_id}."),
        request,
        vrcx_0_application_realtime::CURRENT_USER_AVATAR_RESPONSE_AUTHORITY_FIELDS,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_select_fallback(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<vrcx_0_application::AvatarSelectionMutationOutcome, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) =
        avatar_select_fallback_input(deps.mutation.scope().endpoint.clone(), input.avatar_id)?;
    Ok(vrcx_0_application::select_avatar(
        &deps,
        "app__vrchat_avatar_select_fallback",
        format!("Selecting fallback avatar {avatar_id}."),
        request,
        vrcx_0_application_realtime::CURRENT_USER_FALLBACK_AVATAR_RESPONSE_AUTHORITY_FIELDS,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_save(
    state: State<'_, AppState>,
    input: VrchatAvatarSaveInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) = avatar_save_input(
        deps.mutation.scope().endpoint.clone(),
        input.avatar_id,
        input.params,
    )?;
    Ok(vrcx_0_application::save_avatar(
        &deps,
        "app__vrchat_avatar_save",
        format!("Saving avatar {avatar_id}."),
        request,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_delete(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) =
        avatar_delete_input(deps.mutation.scope().endpoint.clone(), input.avatar_id)?;
    Ok(vrcx_0_application::delete_avatar(
        &deps,
        avatar_id.clone(),
        "app__vrchat_avatar_delete",
        format!("Deleting avatar {avatar_id}."),
        request,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_impostor_create(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) =
        avatar_impostor_create_input(deps.mutation.scope().endpoint.clone(), input.avatar_id)?;
    Ok(vrcx_0_application::execute_avatar_remote_mutation(
        &deps,
        "app__vrchat_avatar_impostor_create",
        format!("Creating avatar impostor for {avatar_id}."),
        request,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_impostor_delete(
    state: State<'_, AppState>,
    input: VrchatAvatarIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, request) =
        avatar_impostor_delete_input(deps.mutation.scope().endpoint.clone(), input.avatar_id)?;
    Ok(vrcx_0_application::execute_avatar_remote_mutation(
        &deps,
        "app__vrchat_avatar_impostor_delete",
        format!("Deleting avatar impostor for {avatar_id}."),
        request,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_moderation_send(
    state: State<'_, AppState>,
    input: VrchatAvatarModerationInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, type_name, request) = avatar_moderation_send_input(
        deps.mutation.scope().endpoint.clone(),
        input.avatar_id,
        input.type_name,
    )?;
    Ok(vrcx_0_application::execute_avatar_remote_mutation(
        &deps,
        "app__vrchat_avatar_moderation_send",
        format!("Sending avatar moderation {type_name} for {avatar_id}."),
        request,
    )
    .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_avatar_moderation_delete(
    state: State<'_, AppState>,
    input: VrchatAvatarModerationInput,
) -> Result<VrchatApiResponse, AppError> {
    let deps = avatar_mutation_deps(&state)?;
    let (avatar_id, type_name, request) = avatar_moderation_delete_input(
        deps.mutation.scope().endpoint.clone(),
        input.avatar_id,
        input.type_name,
    )?;
    Ok(vrcx_0_application::execute_avatar_remote_mutation(
        &deps,
        "app__vrchat_avatar_moderation_delete",
        format!("Deleting avatar moderation {type_name} for {avatar_id}."),
        request,
    )
    .await?)
}
