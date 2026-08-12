#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application_core::vrchat_api::media::{
    asset_upload_input, avatar_gallery_image_upload_input, file_delete_input, files_get_input,
    inventory_bundle_consume_input, inventory_item_equip_input, inventory_item_update_input,
    inventory_items_get_input, inventory_slot_unequip_input, inventory_template_get_input,
    print_delete_input, print_get_input, print_upload_input, prints_get_input, reward_redeem_input,
    sticker_upload_input, tagged_image_upload_input, user_inventory_item_get_input,
};
use vrcx_0_application_core::RuntimeOperationStatus;
use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_application::{
    self as media_upload, collect_inventory_items, AuthenticatedMutationContext,
    InventoryItemsCollectDeps, InventoryItemsCollectInput, InventoryItemsCollectOutput,
    LegacyEntityImageKind, LegacyEntityImageUploadInput, LegacyMediaUploadDeps, PrintFavoriteState,
};
use vrcx_0_application_core::vrchat_api::{VrchatApiRequest, VrchatApiResponse, VrchatScope};

use super::types::{
    VrchatMediaAssetUploadInput, VrchatMediaAvatarGalleryImageUploadInput, VrchatMediaFileIdInput,
    VrchatMediaImageUploadInput, VrchatMediaInventoryItemInput, VrchatMediaInventoryTemplateInput,
    VrchatMediaLegacyImageUploadInput, VrchatMediaParamsInput, VrchatMediaPrintIdInput,
    VrchatMediaPrintUploadInput, VrchatMediaPrintsInput, VrchatMediaProfileDecorationEquipInput,
    VrchatMediaProfileDecorationUnequipInput, VrchatMediaRewardRedeemInput,
    VrchatMediaUserInventoryItemInput, VrchatPrintFavoriteSetInput,
};

fn require_profile_decoration_slot(equip_slot: String) -> Result<String, AppError> {
    let equip_slot = equip_slot.trim();
    match equip_slot {
        "iconFrame" | "profileEffect" | "nameplateEffect" => Ok(equip_slot.to_string()),
        _ => Err(AppError::Custom(
            "Unsupported profile decoration equip slot.".into(),
        )),
    }
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

fn prepare_media_upload_request(input: VrchatApiRequest) -> Result<VrchatApiRequest, AppError> {
    Ok(media_upload::prepare_media_upload_request(input)?)
}

async fn run_legacy_entity_image_upload(
    state: State<'_, AppState>,
    input: VrchatMediaLegacyImageUploadInput,
    kind: LegacyEntityImageKind,
    command: &str,
) -> Result<VrchatApiResponse, AppError> {
    let diagnostics = state.runtime_context.diagnostics.clone();
    let mutation = AuthenticatedMutationContext::capture(
        &state.runtime_context.auth_scope,
        &state.runtime_context.remote_mutations,
        "Legacy media mutation",
    )?;
    diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        format!("Uploading legacy {} image.", kind.label()),
    );
    let result = media_upload::upload_legacy_entity_image(
        LegacyMediaUploadDeps {
            db: state.db.as_ref(),
            web: state.web.as_ref(),
            mutation,
        },
        LegacyEntityImageUploadInput {
            entity_id: input.entity_id,
            image_url: input.image_url,
            base64_file: input.base64_file,
            file_size_in_bytes: input.file_size_in_bytes,
        },
        kind,
    )
    .await;
    match &result {
        Ok(response) => {
            diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!("status={}", response.status),
            );
        }
        Err(error) => {
            diagnostics.record_command(command, RuntimeOperationStatus::Error, error.to_string())
        }
    }
    Ok(result?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_files_get(
    state: State<'_, AppState>,
    input: VrchatMediaParamsInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_files_get",
        "Getting media files.",
        files_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.params),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_file_delete(
    state: State<'_, AppState>,
    input: VrchatMediaFileIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let file_id = input.file_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_file_delete",
        format!("Deleting media file {file_id}."),
        file_delete_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.file_id)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_gallery_image_upload(
    state: State<'_, AppState>,
    input: VrchatMediaImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_gallery_image_upload",
        "Uploading gallery image.",
        prepare_media_upload_request(tagged_image_upload_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.image_data,
            "gallery",
            false,
        )?)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_avatar_gallery_image_upload(
    state: State<'_, AppState>,
    input: VrchatMediaAvatarGalleryImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_avatar_gallery_image_upload",
        "Uploading avatar gallery image.",
        prepare_media_upload_request(avatar_gallery_image_upload_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.image_data,
            input.avatar_id,
        )?)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_vrc_plus_icon_upload(
    state: State<'_, AppState>,
    input: VrchatMediaImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_vrc_plus_icon_upload",
        "Uploading VRC+ icon.",
        prepare_media_upload_request(tagged_image_upload_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.image_data,
            "icon",
            true,
        )?)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_emoji_upload(
    state: State<'_, AppState>,
    input: VrchatMediaImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_emoji_upload",
        "Uploading emoji.",
        prepare_media_upload_request(
            vrcx_0_application_core::vrchat_api::media::image_upload_input(
                VRCHAT_API_DEFAULT_ENDPOINT.into(),
                "file/image",
                input.image_data,
                input.params,
                true,
            )?,
        )?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_sticker_upload(
    state: State<'_, AppState>,
    input: VrchatMediaImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_sticker_upload",
        "Uploading sticker.",
        prepare_media_upload_request(sticker_upload_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.image_data,
        )?)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_print_upload(
    state: State<'_, AppState>,
    input: VrchatMediaPrintUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_print_upload",
        "Uploading print.",
        prepare_media_upload_request(print_upload_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.image_data,
            input.crop_white_border,
            input.params,
        )?)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_asset_upload(
    state: State<'_, AppState>,
    input: VrchatMediaAssetUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    let (asset_kind, request) = asset_upload_input(
        VRCHAT_API_DEFAULT_ENDPOINT.into(),
        input.asset_kind,
        input.image_data,
        input.crop_white_border,
        input.params,
    )?;
    let request = prepare_media_upload_request(request)?;

    execute_media_api(
        state,
        "app__vrchat_media_asset_upload",
        format!("Uploading media asset {asset_kind}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_prints_get(
    state: State<'_, AppState>,
    input: VrchatMediaPrintsInput,
) -> Result<VrchatApiResponse, AppError> {
    let user_id = input.user_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_prints_get",
        format!("Getting prints for user {user_id}."),
        prints_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.user_id, input.n)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_print_get(
    state: State<'_, AppState>,
    input: VrchatMediaPrintIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let print_id = input.print_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_print_get",
        format!("Getting print {print_id}."),
        print_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.print_id)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_print_delete(
    state: State<'_, AppState>,
    input: VrchatMediaPrintIdInput,
) -> Result<VrchatApiResponse, AppError> {
    let print_id = input.print_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_print_delete",
        format!("Deleting print {print_id}."),
        print_delete_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.print_id)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_prints_favorites_list(
    state: State<'_, AppState>,
) -> Result<PrintFavoriteState, AppError> {
    Ok(vrcx_0_application::favorite_state(state.db.as_ref())?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_prints_favorite_set(
    state: State<'_, AppState>,
    input: VrchatPrintFavoriteSetInput,
) -> Result<PrintFavoriteState, AppError> {
    Ok(vrcx_0_application::set_print_favorite(
        state.db.as_ref(),
        &input.print_id,
        input.favorite,
    )?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_inventory_items_get(
    state: State<'_, AppState>,
    input: VrchatMediaParamsInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_inventory_items_get",
        "Getting inventory items.",
        inventory_items_get_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.params),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_inventory_items_collect(
    state: State<'_, AppState>,
    input: InventoryItemsCollectInput,
) -> Result<InventoryItemsCollectOutput, AppError> {
    let expected_scope =
        crate::commands::application::scope::require_active_scope(&state, "Inventory collect")?;
    let deps = InventoryItemsCollectDeps {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    };
    Ok(collect_inventory_items(&deps, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_inventory_template_get(
    state: State<'_, AppState>,
    input: VrchatMediaInventoryTemplateInput,
) -> Result<VrchatApiResponse, AppError> {
    let inventory_template_id = input.inventory_template_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_inventory_template_get",
        format!("Getting inventory template {inventory_template_id}."),
        inventory_template_get_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.inventory_template_id,
        )?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_profile_decoration_equip(
    state: State<'_, AppState>,
    input: VrchatMediaProfileDecorationEquipInput,
) -> Result<VrchatApiResponse, AppError> {
    let equip_slot = require_profile_decoration_slot(input.equip_slot)?;
    let inventory_id = input.inventory_id.clone();
    let request = inventory_item_equip_input(
        VRCHAT_API_DEFAULT_ENDPOINT.into(),
        input.inventory_id,
        equip_slot,
    )?;
    execute_media_api(
        state,
        "app__vrchat_media_profile_decoration_equip",
        format!("Equipping profile decoration {inventory_id}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_profile_decoration_unequip(
    state: State<'_, AppState>,
    input: VrchatMediaProfileDecorationUnequipInput,
) -> Result<VrchatApiResponse, AppError> {
    let equip_slot = require_profile_decoration_slot(input.equip_slot)?;
    let request =
        inventory_slot_unequip_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), equip_slot.clone())?;
    execute_media_api(
        state,
        "app__vrchat_media_profile_decoration_unequip",
        format!("Unequipping profile decoration slot {equip_slot}."),
        request,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_user_inventory_item_get(
    state: State<'_, AppState>,
    input: VrchatMediaUserInventoryItemInput,
) -> Result<VrchatApiResponse, AppError> {
    let inventory_id = input.inventory_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_user_inventory_item_get",
        format!("Getting inventory item {inventory_id}."),
        user_inventory_item_get_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.user_id,
            input.inventory_id,
        )?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_inventory_item_update(
    state: State<'_, AppState>,
    input: VrchatMediaInventoryItemInput,
) -> Result<VrchatApiResponse, AppError> {
    let inventory_id = input.inventory_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_inventory_item_update",
        format!("Updating inventory item {inventory_id}."),
        inventory_item_update_input(
            VRCHAT_API_DEFAULT_ENDPOINT.into(),
            input.inventory_id,
            input.params,
        )?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_inventory_bundle_consume(
    state: State<'_, AppState>,
    input: VrchatMediaInventoryItemInput,
) -> Result<VrchatApiResponse, AppError> {
    let inventory_id = input.inventory_id.clone();
    execute_media_api(
        state,
        "app__vrchat_media_inventory_bundle_consume",
        format!("Consuming inventory bundle {inventory_id}."),
        inventory_bundle_consume_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.inventory_id)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_reward_redeem(
    state: State<'_, AppState>,
    input: VrchatMediaRewardRedeemInput,
) -> Result<VrchatApiResponse, AppError> {
    execute_media_api(
        state,
        "app__vrchat_media_reward_redeem",
        "Redeeming reward.",
        reward_redeem_input(VRCHAT_API_DEFAULT_ENDPOINT.into(), input.code)?,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_avatar_image_upload_legacy(
    state: State<'_, AppState>,
    input: VrchatMediaLegacyImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    run_legacy_entity_image_upload(
        state,
        input,
        LegacyEntityImageKind::Avatar,
        "app__vrchat_media_avatar_image_upload_legacy",
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn app__vrchat_media_world_image_upload_legacy(
    state: State<'_, AppState>,
    input: VrchatMediaLegacyImageUploadInput,
) -> Result<VrchatApiResponse, AppError> {
    run_legacy_entity_image_upload(
        state,
        input,
        LegacyEntityImageKind::World,
        "app__vrchat_media_world_image_upload_legacy",
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::require_profile_decoration_slot;

    #[test]
    fn profile_decoration_mutation_accepts_only_supported_slots() {
        for slot in ["iconFrame", "profileEffect", "nameplateEffect"] {
            assert_eq!(
                require_profile_decoration_slot(format!(" {slot} ")).unwrap(),
                slot
            );
        }
        assert!(require_profile_decoration_slot("drone".into()).is_err());
        assert!(require_profile_decoration_slot(" ".into()).is_err());
    }
}
