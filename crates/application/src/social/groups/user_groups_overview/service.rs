use vrcx_0_application_core::RuntimeOperationStatus;
use std::collections::HashMap;

use serde_json::Value;
use vrcx_0_vrchat_client::http_api::{normalize_vrchat_api_endpoint, ApiJsonResponse};

use crate::{Error, Result};
use vrcx_0_application_core::vrchat_api::groups::{
    user_group_permissions_get_input, user_groups_get_input,
};
use vrcx_0_application_core::vrchat_api::VrchatApiRequest;
use vrcx_0_application_core::{HostSessionRuntime, RuntimeAuthScope};
use vrcx_0_core::json::scalar_text as value_as_string;

use super::super::permissions::{parse_permission_map, permissions_for_group};
use super::super::service::{execute_group_api_raw, GroupApiDeps};
use super::types::{UserGroupsOverviewGroup, UserGroupsOverviewInput, UserGroupsOverviewOutput};

#[derive(Clone)]
pub struct UserGroupsOverviewDeps {
    pub groups: GroupApiDeps,
    pub auth_scope: RuntimeAuthScope,
    pub session: HostSessionRuntime,
}

pub async fn get_user_groups_overview(
    deps: UserGroupsOverviewDeps,
    input: UserGroupsOverviewInput,
) -> Result<UserGroupsOverviewOutput> {
    let command = "app__user_groups_overview_get";
    deps.groups
        .diagnostics
        .record_command(command, RuntimeOperationStatus::Running, "User groups overview started.");
    let result = load_user_groups_overview(deps.clone(), input).await;
    match &result {
        Ok(output) => {
            deps.groups.diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!(
                    "user={} groups={} permissionsDegraded={}",
                    output.current_user_id,
                    output.groups.len(),
                    output.permissions_degraded
                ),
            );
            deps.groups.sync.record(
                "api",
                RuntimeOperationStatus::Ready,
                format!(
                    "User groups overview loaded for {}.",
                    output.current_user_id
                ),
                0,
            );
        }
        Err(error) => {
            deps.groups
                .diagnostics
                .record_command(command, RuntimeOperationStatus::Error, error.to_string());
            deps.groups.sync.record_failure("api", error.to_string());
        }
    }
    result
}

async fn load_user_groups_overview(
    deps: UserGroupsOverviewDeps,
    input: UserGroupsOverviewInput,
) -> Result<UserGroupsOverviewOutput> {
    let current_user_id = normalize_text(input.current_user_id);
    if current_user_id.is_empty() {
        return Err(Error::Custom(
            "User groups overview requires currentUserId.".into(),
        ));
    }
    let endpoint = normalize_endpoint(&input.endpoint);
    if !auth_scope_matches(&deps, &current_user_id, &endpoint) {
        return Ok(UserGroupsOverviewOutput {
            current_user_id,
            groups: Vec::new(),
            permissions_degraded: false,
        });
    }

    let group_rows = array_rows(
        &execute_vrchat_json_request(
            &deps,
            user_groups_get_input(endpoint.clone(), current_user_id.clone())?.1,
            "VRChat user groups overview groups request failed",
        )
        .await?,
    );

    let (permission_map, permissions_degraded) = match execute_vrchat_json_request(
        &deps,
        user_group_permissions_get_input(endpoint.clone(), current_user_id.clone())?.1,
        "VRChat user groups overview permissions request failed",
    )
    .await
    {
        Ok(json) => (parse_permission_map(&json), false),
        Err(_) => (HashMap::new(), true),
    };

    Ok(UserGroupsOverviewOutput {
        current_user_id,
        groups: build_overview_groups(&group_rows, &permission_map),
        permissions_degraded,
    })
}

fn build_overview_groups(
    group_rows: &[Value],
    permission_map: &HashMap<String, Vec<String>>,
) -> Vec<UserGroupsOverviewGroup> {
    let mut groups = group_rows
        .iter()
        .filter_map(|group| group_overview_from_value(group, permission_map))
        .collect::<Vec<_>>();
    groups.sort_by_key(|group| group.name.to_lowercase());
    groups
}

fn group_overview_from_value(
    group: &Value,
    permission_map: &HashMap<String, Vec<String>>,
) -> Option<UserGroupsOverviewGroup> {
    let group_id = object_string(group, &["groupId", "id"]);
    if group_id.is_empty() {
        return None;
    }
    let name = object_string(group, &["name", "displayName"]);
    let short_code = object_string(group, &["shortCode", "shortcode"]);
    let icon_url = object_string(
        group,
        &["iconUrl", "imageUrl", "thumbnailImageUrl", "bannerUrl"],
    );
    let member_count = group
        .as_object()
        .and_then(|object| object.get("memberCount"))
        .and_then(Value::as_i64);
    let permissions = permissions_for_group(group, permission_map, &group_id);

    Some(UserGroupsOverviewGroup {
        name: if name.is_empty() {
            group_id.clone()
        } else {
            name
        },
        group_id,
        short_code: (!short_code.is_empty()).then_some(short_code),
        icon_url: (!icon_url.is_empty()).then_some(icon_url),
        member_count,
        permissions,
    })
}

async fn execute_vrchat_json_request(
    deps: &UserGroupsOverviewDeps,
    request: VrchatApiRequest,
    fallback: &str,
) -> Result<Value> {
    let response = execute_vrchat_api(deps, request).await?;
    if response.is_failure() {
        return Err(Error::Custom(response.error_message_or(fallback)));
    }
    Ok(response.json)
}

async fn execute_vrchat_api(
    deps: &UserGroupsOverviewDeps,
    request: VrchatApiRequest,
) -> Result<ApiJsonResponse> {
    let response = execute_group_api_raw(&deps.groups, request).await?;
    Ok(ApiJsonResponse::from(&response))
}

fn object_string(value: &Value, keys: &[&str]) -> String {
    let Some(object) = value.as_object() else {
        return String::new();
    };
    for key in keys {
        let text = value_as_string(object.get(*key));
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

fn normalize_text(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_string()
}

fn normalize_endpoint(value: &str) -> String {
    normalize_vrchat_api_endpoint(Some(value))
}

fn auth_scope_matches(deps: &UserGroupsOverviewDeps, user_id: &str, endpoint: &str) -> bool {
    vrcx_0_application_core::auth_scope_matches(&deps.auth_scope, &deps.session, user_id, endpoint)
}

fn array_rows(value: &Value) -> Vec<Value> {
    if let Some(rows) = value.as_array() {
        return rows.clone();
    }
    value
        .as_object()
        .and_then(|object| object.get("results"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_rows_with_permission_map_override_and_my_member_fallback() {
        let group_rows = vec![
            json!({
                "id": "grp_1",
                "name": "Alpha",
                "shortCode": "ALPHA",
                "iconUrl": "https://example.com/a.png",
                "memberCount": 12,
                "myMember": { "permissions": ["group-members-remove"] }
            }),
            json!({
                "id": "grp_2",
                "name": "Beta",
                "myMember": { "permissions": ["group-bans-manage"] }
            }),
        ];
        let permission_map = parse_permission_map(&json!({ "grp_1": ["*"] }));

        let groups = build_overview_groups(&group_rows, &permission_map);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group_id, "grp_1");
        assert_eq!(groups[0].permissions, vec!["*".to_string()]);
        assert_eq!(groups[0].short_code.as_deref(), Some("ALPHA"));
        assert_eq!(groups[0].member_count, Some(12));
        assert_eq!(groups[1].group_id, "grp_2");
        assert_eq!(groups[1].permissions, vec!["group-bans-manage".to_string()]);
        assert_eq!(groups[1].short_code, None);
        assert_eq!(groups[1].member_count, None);
    }

    #[test]
    fn prefers_group_id_over_membership_record_id() {
        let group_rows = vec![json!({
            "id": "gmem_11111111-1111-1111-1111-111111111111",
            "groupId": "grp_1",
            "name": "Alpha"
        })];
        let permission_map = parse_permission_map(&json!({ "grp_1": ["group-bans-manage"] }));

        let groups = build_overview_groups(&group_rows, &permission_map);

        assert_eq!(groups[0].group_id, "grp_1");
        assert_eq!(groups[0].permissions, vec!["group-bans-manage".to_string()]);
    }

    #[test]
    fn skips_rows_without_a_group_id() {
        let group_rows = vec![json!({ "name": "No id" })];
        let groups = build_overview_groups(&group_rows, &HashMap::new());
        assert!(groups.is_empty());
    }

    #[test]
    fn falls_back_to_group_id_when_name_is_missing() {
        let group_rows = vec![json!({ "id": "grp_1" })];
        let groups = build_overview_groups(&group_rows, &HashMap::new());
        assert_eq!(groups[0].name, "grp_1");
        assert!(groups[0].permissions.is_empty());
    }
}
