#![allow(non_snake_case)]

use serde_json::Value;
use tauri::State;
use vrcx_0_application::{
    get_my_avatar_by_id, get_my_avatars, MyAvatarByIdInput, MyAvatarsDeps, MyAvatarsInput,
};
use vrcx_0_application_core::RuntimeAuthScopeSnapshot;

use crate::{error::AppError, state::AppState};

#[tauri::command]
#[specta::specta]
pub async fn app__my_avatars_get(
    state: State<'_, AppState>,
    input: MyAvatarsInput,
) -> Result<Vec<Value>, AppError> {
    let deps = my_avatars_deps(&state)?;
    Ok(get_my_avatars(&deps, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn app__my_avatar_by_id_get(
    state: State<'_, AppState>,
    input: MyAvatarByIdInput,
) -> Result<Option<Value>, AppError> {
    let deps = my_avatars_deps(&state)?;
    Ok(get_my_avatar_by_id(&deps, input).await?)
}

fn my_avatars_deps<'a>(state: &'a State<'_, AppState>) -> Result<MyAvatarsDeps<'a>, AppError> {
    let expected_scope = active_scope(state)?;
    Ok(MyAvatarsDeps {
        db: state.db.as_ref(),
        web: state.web.as_ref(),
        auth_scope: &state.runtime_context.auth_scope,
        expected_scope,
    })
}

fn active_scope(state: &AppState) -> Result<RuntimeAuthScopeSnapshot, AppError> {
    let scope = state.runtime_context.auth_scope.snapshot();
    if scope.active && !scope.current_user_id.trim().is_empty() {
        Ok(scope)
    } else {
        Err(vrcx_0_application_core::Error::Custom(
            "My avatars query requires an authenticated session.".into(),
        )
        .into())
    }
}
