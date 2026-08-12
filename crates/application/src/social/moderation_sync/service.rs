use std::time::Duration;

use serde_json::Value;
use vrcx_0_core::time::now_iso;
use vrcx_0_persistence::local_moderation::{
    self, LocalModerationInput, LocalModerationOutput, RemoteModerationInput,
};
use vrcx_0_vrchat_client::http_api::{
    normalize_vrchat_api_endpoint, ApiJsonResponse, ApiScope, HttpApiRequestInput,
};
use vrcx_0_vrchat_client::moderation::{
    player_moderation_update_input, player_moderations_get_input,
};

use super::types::{
    ModerationSyncDeps, ModerationSyncMutationInput, ModerationSyncMutationOutput,
    ModerationSyncRefreshInput, ModerationSyncRefreshOutput, RemoteModerationRow,
};
use crate::{AuthenticatedMutationContext, Error, Result};

const MODERATION_REMOTE_MUTATION_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalPlayerModerationKind {
    Block,
    Mute,
}

impl LocalPlayerModerationKind {
    fn from_remote_type(value: &str) -> Option<Self> {
        match value {
            "block" => Some(Self::Block),
            "mute" => Some(Self::Mute),
            _ => None,
        }
    }
}

pub async fn refresh_player_moderations(
    deps: ModerationSyncDeps<'_>,
    input: ModerationSyncRefreshInput,
) -> Result<ModerationSyncRefreshOutput> {
    let user_id = normalize_text(input.user_id);
    if user_id.is_empty() {
        return Ok(ModerationSyncRefreshOutput {
            accepted: false,
            user_id,
            remote_count: 0,
            local_count: 0,
            rows: Vec::new(),
        });
    }

    let (remote_count, rows) = fetch_remote_moderations(&deps, &input.endpoint).await?;
    let accepted = should_write_refresh_snapshot(&deps, &user_id, &input.endpoint, &rows);
    let local_count = if accepted {
        let local_inputs: Vec<RemoteModerationInput> = rows
            .iter()
            .map(RemoteModerationRow::to_local_input)
            .collect();
        local_moderation::local_moderation_sync_snapshot(deps.db, user_id.clone(), local_inputs)?
            .len()
    } else {
        0
    };

    Ok(ModerationSyncRefreshOutput {
        accepted,
        user_id,
        remote_count,
        local_count,
        rows,
    })
}

pub async fn update_player_moderation(
    deps: ModerationSyncDeps<'_>,
    input: ModerationSyncMutationInput,
) -> Result<ModerationSyncMutationOutput> {
    let target_user_id = normalize_text(input.target_user_id);
    let target_display_name = input.target_display_name.clone();
    let r#type = normalize_text(input.r#type);
    if target_user_id.is_empty() || r#type.is_empty() {
        return Err(Error::Custom(
            "ModerationSyncUpdate requires targetUserId and type.".into(),
        ));
    }
    let mutation = AuthenticatedMutationContext::capture(
        deps.auth_scope,
        deps.remote_mutations,
        "Moderation mutation",
    )?;
    let owner_user_id = mutation.scope().current_user_id.clone();

    execute_vrchat_mutation(
        &deps,
        &mutation,
        player_moderation_update_input(
            mutation.scope().endpoint.clone(),
            input.enabled,
            target_user_id.clone(),
            r#type.clone(),
        ),
    )
    .await?;

    let local = if let Some(kind) = LocalPlayerModerationKind::from_remote_type(&r#type) {
        let existing = local_moderation::local_moderation_get(
            deps.db,
            owner_user_id.clone(),
            target_user_id.clone(),
        )?;
        let (block, mute) = resolve_local_moderation_state(existing.as_ref(), kind, input.enabled);
        let updated_at = now_iso();
        if block || mute {
            local_moderation::local_moderation_set(
                deps.db,
                owner_user_id.clone(),
                LocalModerationInput {
                    user_id: target_user_id.clone(),
                    updated_at: updated_at.clone(),
                    display_name: target_display_name.clone(),
                    block,
                    mute,
                },
            )?;
            Some(LocalModerationOutput {
                user_id: target_user_id.clone(),
                updated_at,
                display_name: target_display_name.clone(),
                block,
                mute,
            })
        } else {
            local_moderation::local_moderation_delete(
                deps.db,
                owner_user_id.clone(),
                target_user_id.clone(),
            )?;
            Some(LocalModerationOutput {
                user_id: target_user_id.clone(),
                updated_at,
                display_name: target_display_name.clone(),
                block: false,
                mute: false,
            })
        }
    } else {
        None
    };

    Ok(ModerationSyncMutationOutput {
        owner_user_id,
        target_user_id,
        r#type,
        enabled: input.enabled,
        local,
    })
}

fn normalize_text(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

fn normalize_scope_endpoint(value: &str) -> String {
    normalize_endpoint(value)
}

fn value_as_normalized_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => normalize_text(value),
        Some(Value::Null) | None => String::new(),
        Some(value) => normalize_text(value.to_string()),
    }
}

fn value_as_string_or_empty(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn normalize_endpoint(endpoint: &str) -> String {
    normalize_vrchat_api_endpoint(Some(endpoint))
}

async fn execute_vrchat_json_request(
    deps: &ModerationSyncDeps<'_>,
    request: HttpApiRequestInput,
) -> Result<Value> {
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db)
        .await?;

    let response = ApiJsonResponse::from(&response);
    if response.is_failure() {
        return Err(Error::Custom(response.error_message_with_http_status(
            "VRChat moderation request failed",
        )));
    }

    Ok(response.json)
}

async fn execute_vrchat_mutation(
    deps: &ModerationSyncDeps<'_>,
    mutation: &AuthenticatedMutationContext<'_>,
    mut request: HttpApiRequestInput,
) -> Result<Value> {
    mutation.apply_scope_to_request(&mut request);
    let response = mutation
        .run_after_wait(MODERATION_REMOTE_MUTATION_INTERVAL, || async {
            deps.web
                .execute_api(request, ApiScope::Vrchat, deps.db)
                .await
        })
        .await?;

    let response = ApiJsonResponse::from(&response);
    if response.is_failure() {
        return Err(Error::Custom(response.error_message_with_http_status(
            "VRChat moderation request failed",
        )));
    }

    Ok(response.json)
}

fn normalize_remote_moderation_row(row: &Value) -> Option<RemoteModerationRow> {
    let record = row.as_object()?;
    let id = value_as_normalized_text(record.get("id"));
    let r#type = value_as_normalized_text(record.get("type"));
    let source_user_id = value_as_normalized_text(record.get("sourceUserId"));
    let target_user_id = value_as_normalized_text(record.get("targetUserId"));

    if id.is_empty() || r#type.is_empty() || target_user_id.is_empty() {
        return None;
    }

    Some(RemoteModerationRow {
        id,
        r#type,
        source_user_id,
        source_display_name: value_as_string_or_empty(record.get("sourceDisplayName")),
        target_user_id,
        target_display_name: value_as_string_or_empty(record.get("targetDisplayName")),
        created: value_as_string_or_empty(record.get("created")),
    })
}

fn normalize_remote_moderation_rows(json: &Value) -> Vec<RemoteModerationRow> {
    json.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(normalize_remote_moderation_row)
                .collect()
        })
        .unwrap_or_default()
}

async fn fetch_remote_moderations(
    deps: &ModerationSyncDeps<'_>,
    endpoint: &str,
) -> Result<(usize, Vec<RemoteModerationRow>)> {
    let json = execute_vrchat_json_request(
        deps,
        player_moderations_get_input(normalize_endpoint(endpoint)),
    )
    .await?;
    let remote_count = json.as_array().map_or(0, Vec::len);
    Ok((remote_count, normalize_remote_moderation_rows(&json)))
}

fn rows_have_verified_owner(rows: &[RemoteModerationRow], user_id: &str) -> bool {
    !rows.is_empty()
        && rows
            .iter()
            .all(|row| !row.source_user_id.is_empty() && row.source_user_id == user_id)
}

fn runtime_auth_scope_scope_matches(
    deps: &ModerationSyncDeps<'_>,
    user_id: &str,
    endpoint: &str,
) -> bool {
    let snapshot = deps.session.snapshot();
    let Some(context) = snapshot.realtime_context else {
        return false;
    };

    context.current_user_id == user_id
        && normalize_scope_endpoint(&context.endpoint) == normalize_scope_endpoint(endpoint)
}

fn should_write_refresh_snapshot(
    deps: &ModerationSyncDeps<'_>,
    user_id: &str,
    endpoint: &str,
    rows: &[RemoteModerationRow],
) -> bool {
    let auth_scope = deps.auth_scope.snapshot();
    if auth_scope.active {
        return deps.auth_scope.matches(user_id, endpoint);
    }

    runtime_auth_scope_scope_matches(deps, user_id, endpoint)
        || rows_have_verified_owner(rows, user_id)
}

fn resolve_local_moderation_state(
    existing: Option<&LocalModerationOutput>,
    r#type: LocalPlayerModerationKind,
    enabled: bool,
) -> (bool, bool) {
    match r#type {
        LocalPlayerModerationKind::Block => (enabled, existing.is_some_and(|entry| entry.mute)),
        LocalPlayerModerationKind::Mute => (existing.is_some_and(|entry| entry.block), enabled),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn moderation_error_message_keeps_http_status() {
        let message = ApiJsonResponse::parse(500, r#"{"error":{"message":"Application error."}}"#)
            .error_message_with_http_status("VRChat moderation request failed");

        assert_eq!(message, "Application error. (HTTP 500)");
    }

    #[test]
    fn normalizes_only_complete_remote_moderation_rows() {
        let rows = normalize_remote_moderation_rows(&json!([
            {
                "id": " mod_1 ",
                "type": " block ",
                "targetUserId": " usr_target ",
                "targetDisplayName": "Target",
                "created": "2026-05-16T00:00:00.000Z"
            },
            {
                "id": "mod_2",
                "type": "mute",
                "targetDisplayName": "Missing target"
            },
            {
                "type": "block",
                "targetUserId": "usr_missing_id"
            }
        ]));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].r#type, "block");
        assert_eq!(rows[0].target_user_id, "usr_target");
        assert_eq!(rows[0].target_display_name, "Target");
        assert_eq!(rows[0].created, "2026-05-16T00:00:00.000Z");
    }

    #[test]
    fn verifies_refresh_owner_from_remote_rows() {
        let rows = vec![RemoteModerationRow {
            id: "mod_1".into(),
            r#type: "block".into(),
            source_user_id: "usr_current".into(),
            source_display_name: String::new(),
            target_user_id: "usr_target".into(),
            target_display_name: String::new(),
            created: String::new(),
        }];

        assert!(rows_have_verified_owner(&rows, "usr_current"));
        assert!(!rows_have_verified_owner(&rows, "usr_other"));
        assert!(!rows_have_verified_owner(&[], "usr_current"));
    }

    #[test]
    fn local_moderation_update_preserves_other_bit_when_not_supplied() {
        let existing = LocalModerationOutput {
            user_id: "usr_target".into(),
            updated_at: String::new(),
            display_name: String::new(),
            block: true,
            mute: true,
        };

        assert_eq!(
            resolve_local_moderation_state(
                Some(&existing),
                LocalPlayerModerationKind::Block,
                false,
            ),
            (false, true)
        );
        assert_eq!(
            resolve_local_moderation_state(Some(&existing), LocalPlayerModerationKind::Mute, false,),
            (true, false)
        );
        assert_eq!(
            resolve_local_moderation_state(Some(&existing), LocalPlayerModerationKind::Block, true,),
            (true, true)
        );
    }
}
