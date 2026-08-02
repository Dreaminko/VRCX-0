#![allow(non_snake_case)]

use tauri::State;
use vrcx_0_application_core::vrchat_api::{self, VrchatApiRequest, VrchatApiResponse, VrchatScope};
use vrcx_0_core::vrchat_endpoints::VRCHAT_API_DEFAULT_ENDPOINT;

use crate::error::AppError;
use crate::state::AppState;
use vrcx_0_application_core::RuntimeAuthScope;

pub fn require_auth_scope(
    auth_scope: &RuntimeAuthScope,
    expected_user_id: &str,
    stale_detail: &str,
) -> Result<(), AppError> {
    if auth_scope.matches(expected_user_id, VRCHAT_API_DEFAULT_ENDPOINT) {
        return Ok(());
    }
    Err(AppError::Custom(stale_detail.into()))
}

pub async fn execute_vrchat_api(
    state: State<'_, AppState>,
    command: &str,
    detail: impl Into<String>,
    input: VrchatApiRequest,
    scope: VrchatScope,
) -> Result<VrchatApiResponse, AppError> {
    vrchat_api::execute_api_command(
        state.web.as_ref(),
        state.db.as_ref(),
        &state.runtime_context.diagnostics,
        &state.runtime_context.sync,
        (command, detail),
        input,
        scope,
    )
    .await
    .map_err(AppError::from)
}
