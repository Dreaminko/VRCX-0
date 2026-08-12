use std::{sync::Arc, time::Duration};

use vrcx_0_application_core::{
    vrchat_api::{self, VrchatApiRequest, VrchatApiResponse, VrchatScope},
    FavoriteChange, FavoriteChangeScope, FavoritesChangedPayload, RuntimeDiagnostics,
    RuntimeSyncEngine, VrchatFavoriteType, WebClient,
};
use vrcx_0_application_realtime::RealtimeHostRuntime;
use vrcx_0_core::json::RawJson;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::parse_api_json;

use crate::{AuthenticatedMutationContext, Result};

const FAVORITE_REMOTE_MUTATION_INTERVAL: Duration = Duration::from_millis(250);

pub struct FavoriteRemoteMutationDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub diagnostics: &'a RuntimeDiagnostics,
    pub sync: &'a RuntimeSyncEngine,
    pub realtime: &'a Arc<RealtimeHostRuntime>,
    pub mutation: AuthenticatedMutationContext<'a>,
}

pub struct FavoriteRemoteAddInput {
    pub kind: VrchatFavoriteType,
    pub entity_id: String,
    pub tags: String,
}

fn prepare_remote_favorite_add(
    endpoint: String,
    input: FavoriteRemoteAddInput,
) -> Result<(FavoriteChangeScope, String, String, VrchatApiRequest)> {
    let notification_scope = input.kind.into();
    let (kind, entity_id, request) = vrchat_api::favorites::favorite_add_input(
        endpoint,
        input.kind.as_str().to_string(),
        input.entity_id,
        input.tags,
    )?;
    Ok((notification_scope, kind, entity_id, request))
}

pub struct FavoriteRemoteDeleteInput {
    pub object_id: String,
}

pub struct FavoriteRemoteGroupSaveInput {
    pub kind: String,
    pub group: String,
    pub display_name: Option<String>,
    pub visibility: Option<String>,
}

pub struct FavoriteRemoteGroupClearInput {
    pub kind: String,
    pub group: String,
}

fn should_notify_favorite_change(status: i32) -> bool {
    vrchat_api::classify_api_response(status).class == vrchat_api::ApiResponseClass::Ok
}

async fn execute_remote_favorite_mutation(
    deps: &FavoriteRemoteMutationDeps<'_>,
    command: &str,
    detail: String,
    mut request: VrchatApiRequest,
) -> Result<VrchatApiResponse> {
    deps.mutation.apply_scope_to_request(&mut request);
    deps.mutation
        .run_after_wait(FAVORITE_REMOTE_MUTATION_INTERVAL, || async move {
            vrchat_api::execute_api_command(
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

fn has_exact_remote_favorite_identity(favorite: &serde_json::Value) -> bool {
    ["id", "favoriteId"].iter().all(|field| {
        favorite
            .get(field)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn notify_favorite_change(
    deps: &FavoriteRemoteMutationDeps<'_>,
    kind: FavoriteChangeScope,
    changes: Vec<FavoriteChange>,
) {
    deps.realtime
        .notify_favorites_changed(FavoritesChangedPayload::from_changes(
            deps.mutation.scope(),
            kind,
            false,
            true,
            changes,
        ));
}

pub async fn add_remote_favorite(
    deps: &FavoriteRemoteMutationDeps<'_>,
    input: FavoriteRemoteAddInput,
) -> Result<VrchatApiResponse> {
    let (notification_scope, kind, entity_id, request) =
        prepare_remote_favorite_add(deps.mutation.scope().endpoint.clone(), input)?;
    let response = execute_remote_favorite_mutation(
        deps,
        "favorite.remote.add",
        format!("Adding {kind} favorite {entity_id}."),
        request,
    )
    .await?;
    if should_notify_favorite_change(response.status) {
        let favorite = parse_api_json(&response.data);
        let changes = if has_exact_remote_favorite_identity(&favorite) {
            vec![FavoriteChange::RemoteAdded {
                favorite: RawJson::from(favorite),
            }]
        } else {
            Vec::new()
        };
        notify_favorite_change(deps, notification_scope, changes);
    }
    Ok(response)
}

pub async fn delete_remote_favorite(
    deps: &FavoriteRemoteMutationDeps<'_>,
    input: FavoriteRemoteDeleteInput,
) -> Result<VrchatApiResponse> {
    let (object_id, request) = vrchat_api::favorites::favorite_delete_input(
        deps.mutation.scope().endpoint.clone(),
        input.object_id,
    )?;
    let response = execute_remote_favorite_mutation(
        deps,
        "favorite.remote.delete",
        format!("Deleting favorite for {object_id}."),
        request,
    )
    .await?;
    if should_notify_favorite_change(response.status) {
        notify_favorite_change(
            deps,
            FavoriteChangeScope::All,
            vec![FavoriteChange::RemoteRemoved { object_id }],
        );
    }
    Ok(response)
}

pub async fn save_remote_favorite_group(
    deps: &FavoriteRemoteMutationDeps<'_>,
    input: FavoriteRemoteGroupSaveInput,
) -> Result<VrchatApiResponse> {
    let notification_scope = FavoriteChangeScope::from_remote_type(&input.kind);
    let (group, request) = vrchat_api::favorites::favorite_group_save_input(
        deps.mutation.scope().endpoint.clone(),
        deps.mutation.scope().current_user_id.clone(),
        input.kind,
        input.group,
        input.display_name,
        input.visibility,
    )?;
    let response = execute_remote_favorite_mutation(
        deps,
        "favorite.remote.group.save",
        format!("Saving favorite group {group}."),
        request,
    )
    .await?;
    if should_notify_favorite_change(response.status) {
        notify_favorite_change(deps, notification_scope, Vec::new());
    }
    Ok(response)
}

pub async fn clear_remote_favorite_group(
    deps: &FavoriteRemoteMutationDeps<'_>,
    input: FavoriteRemoteGroupClearInput,
) -> Result<VrchatApiResponse> {
    let notification_scope = FavoriteChangeScope::from_remote_type(&input.kind);
    let (group, request) = vrchat_api::favorites::favorite_group_clear_input(
        deps.mutation.scope().endpoint.clone(),
        deps.mutation.scope().current_user_id.clone(),
        input.kind,
        input.group,
    )?;
    let response = execute_remote_favorite_mutation(
        deps,
        "favorite.remote.group.clear",
        format!("Clearing favorite group {group}."),
        request,
    )
    .await?;
    if should_notify_favorite_change(response.status) {
        notify_favorite_change(deps, notification_scope, Vec::new());
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use vrcx_0_application_core::{FavoriteChangeScope, VrchatFavoriteType};

    use super::{
        has_exact_remote_favorite_identity, prepare_remote_favorite_add,
        should_notify_favorite_change, FavoriteRemoteAddInput,
    };

    #[test]
    fn only_successful_http_policies_are_favorite_changes() {
        for (status, expected) in [
            (200, true),
            (204, true),
            (302, false),
            (401, false),
            (429, false),
            (500, false),
        ] {
            assert_eq!(
                should_notify_favorite_change(status),
                expected,
                "status {status}"
            );
        }
    }

    #[test]
    fn vrc_plus_world_add_keeps_remote_type_and_notifies_world_scope() {
        let (scope, kind, entity_id, request) = prepare_remote_favorite_add(
            "endpoint".into(),
            FavoriteRemoteAddInput {
                kind: VrchatFavoriteType::VrcPlusWorld,
                entity_id: "wrld_1".into(),
                tags: "worlds4".into(),
            },
        )
        .unwrap();

        assert_eq!(scope, FavoriteChangeScope::World);
        assert_eq!(kind, "vrcPlusWorld");
        assert_eq!(entity_id, "wrld_1");
        assert_eq!(request.path.as_deref(), Some("favorites"));
        assert_eq!(
            request.body.as_json(),
            Some(&json!({
                "type": "vrcPlusWorld",
                "favoriteId": "wrld_1",
                "tags": "worlds4",
            }))
        );
    }

    #[test]
    fn exact_remote_add_requires_both_favorite_identifiers() {
        assert!(has_exact_remote_favorite_identity(&json!({
            "id": "fvrt_1",
            "favoriteId": "wrld_1"
        })));
        for payload in [
            json!({}),
            json!({"id": "fvrt_1"}),
            json!({"favoriteId": "wrld_1"}),
            json!({"id": "", "favoriteId": "wrld_1"}),
            json!({"id": "fvrt_1", "favoriteId": null}),
        ] {
            assert!(!has_exact_remote_favorite_identity(&payload));
        }
    }
}
