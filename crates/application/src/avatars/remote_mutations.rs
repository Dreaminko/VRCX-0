use std::{sync::Arc, time::Duration};

use serde::Serialize;
use vrcx_0_application_core::{
    vrchat_api::{execute_api_command, VrchatApiRequest, VrchatApiResponse, VrchatScope},
    AvatarCache, RuntimeDiagnostics, RuntimeSyncEngine, WebClient,
};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_persistence::DatabaseService;

use crate::{AuthenticatedMutationContext, Result};

const AVATAR_REMOTE_MUTATION_INTERVAL: Duration = Duration::from_millis(250);

pub struct AvatarRemoteMutationDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub diagnostics: &'a RuntimeDiagnostics,
    pub sync: &'a RuntimeSyncEngine,
    pub realtime: &'a Arc<RealtimeHostRuntime>,
    pub avatar_cache: &'a Arc<AvatarCache>,
    pub mutation: AuthenticatedMutationContext<'a>,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvatarSelectionMutationOutcome {
    pub applied: bool,
    pub response: VrchatApiResponse,
}

pub async fn execute_avatar_remote_mutation(
    deps: &AvatarRemoteMutationDeps<'_>,
    command: &str,
    detail: String,
    mut request: VrchatApiRequest,
) -> Result<VrchatApiResponse> {
    deps.mutation.apply_scope_to_request(&mut request);
    deps.mutation
        .run_after_wait(AVATAR_REMOTE_MUTATION_INTERVAL, || async move {
            execute_api_command(
                deps.web,
                deps.db,
                deps.diagnostics,
                deps.sync,
                (command, detail),
                request,
                VrchatScope::Vrchat,
            )
            .await
        })
        .await
}

pub async fn select_avatar(
    deps: &AvatarRemoteMutationDeps<'_>,
    command: &str,
    detail: String,
    mut request: VrchatApiRequest,
    response_authority_fields: &[&str],
) -> Result<AvatarSelectionMutationOutcome> {
    deps.mutation.apply_scope_to_request(&mut request);
    let (expectation, response) = deps
        .mutation
        .run_after_wait(AVATAR_REMOTE_MUTATION_INTERVAL, || async move {
            let expectation = deps.realtime.capture_current_user_refresh_expectation();
            let response = execute_api_command(
                deps.web,
                deps.db,
                deps.diagnostics,
                deps.sync,
                (command, detail),
                request,
                VrchatScope::Vrchat,
            )
            .await?;
            Ok((expectation, response))
        })
        .await?;
    let mut applied = false;
    if let Some(expectation) = expectation {
        if (200..300).contains(&response.status) {
            if let Ok(snapshot) = serde_json::from_str::<serde_json::Value>(&response.data) {
                applied = deps
                    .realtime
                    .apply_current_user_refreshed_snapshot_if_sequence(
                        expectation,
                        snapshot,
                        response_authority_fields,
                    );
            }
        }
    }
    Ok(AvatarSelectionMutationOutcome { applied, response })
}

pub async fn save_avatar(
    deps: &AvatarRemoteMutationDeps<'_>,
    command: &str,
    detail: String,
    request: VrchatApiRequest,
) -> Result<VrchatApiResponse> {
    let response = execute_avatar_remote_mutation(deps, command, detail, request).await?;
    if (200..300).contains(&response.status) {
        if let Ok(avatar) = serde_json::from_str::<serde_json::Value>(&response.data) {
            let scope = deps.mutation.scope();
            deps.avatar_cache
                .hydrate_from_payload(&scope.current_user_id, &scope.endpoint, avatar);
        }
    }
    Ok(response)
}

pub async fn delete_avatar(
    deps: &AvatarRemoteMutationDeps<'_>,
    avatar_id: String,
    command: &str,
    detail: String,
    request: VrchatApiRequest,
) -> Result<VrchatApiResponse> {
    let response = execute_avatar_remote_mutation(deps, command, detail, request).await?;
    if (200..300).contains(&response.status) {
        let scope = deps.mutation.scope();
        deps.avatar_cache
            .invalidate(&scope.current_user_id, &scope.endpoint, &avatar_id);
        if let Err(error) =
            vrcx_0_persistence::avatars::avatar_cache_remove(deps.db, avatar_id.clone())
        {
            tracing::warn!(avatar_id = %avatar_id, "Avatar cache cleanup failed: {error}");
        }
    }
    Ok(response)
}
