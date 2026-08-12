#![allow(non_snake_case)]

use std::time::Duration;

use tauri::State;
use vrcx_0_application::{is_remote_mutation_request, AuthenticatedMutationContext};
use vrcx_0_application_core::vrchat_api::{self, VrchatApiRequest, VrchatApiResponse, VrchatScope};

use crate::error::AppError;
use crate::state::AppState;

const VRCHAT_REMOTE_MUTATION_INTERVAL: Duration = Duration::from_millis(250);

pub async fn execute_vrchat_api(
    state: State<'_, AppState>,
    command: &str,
    detail: impl Into<String>,
    input: VrchatApiRequest,
    scope: VrchatScope,
) -> Result<VrchatApiResponse, AppError> {
    let detail = detail.into();
    if is_remote_mutation_request(&input) {
        let mutation = AuthenticatedMutationContext::capture(
            &state.runtime_context.auth_scope,
            &state.runtime_context.remote_mutations,
            "VRChat mutation",
        )?;
        return execute_vrchat_mutation(state.inner(), &mutation, command, detail, input, scope)
            .await;
    }
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

pub async fn execute_vrchat_mutation(
    state: &AppState,
    mutation: &AuthenticatedMutationContext<'_>,
    command: &str,
    detail: impl Into<String>,
    mut input: VrchatApiRequest,
    scope: VrchatScope,
) -> Result<VrchatApiResponse, AppError> {
    mutation.apply_scope_to_request(&mut input);
    mutation
        .run_after_wait(VRCHAT_REMOTE_MUTATION_INTERVAL, || async move {
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
        })
        .await
        .map_err(AppError::from)
}
