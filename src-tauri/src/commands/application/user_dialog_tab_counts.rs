#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application::{
    get_user_dialog_tab_counts, UserDialogTabCountsDeps, UserDialogTabCountsInput,
    UserDialogTabCountsOutput,
};

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn app__user_dialog_tab_counts_get(
    state: State<'_, AppState>,
    input: UserDialogTabCountsInput,
) -> Result<UserDialogTabCountsOutput, AppError> {
    Ok(get_user_dialog_tab_counts(
        &state.user_dialog_tab_counts,
        UserDialogTabCountsDeps {
            db: state.db.clone(),
            web: state.web.clone(),
            auth_scope: state.runtime_context.auth_scope.clone(),
        },
        input,
    )
    .await?)
}
