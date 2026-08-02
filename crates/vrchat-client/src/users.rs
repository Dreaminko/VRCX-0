use std::collections::HashMap;

use serde_json::{json, Value};

use crate::http_api::{
    api_input, encode_path_segment, get_input, object_body, require_text, HttpApiError,
    HttpApiRequestInput,
};

pub fn user_get_input(
    endpoint: String,
    user_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatUserGet requires userId.")?;
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("users/{}", encode_path_segment(&user_id)),
            HashMap::new(),
        ),
    ))
}

pub fn profile_get_input(
    endpoint: String,
    user_id: String,
    as_self: bool,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatProfileGet requires userId.")?;
    let params = HashMap::from([
        ("asSelf".to_string(), json!(as_self)),
        ("withGroupsAndWorlds".to_string(), json!(true)),
    ]);
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("profile/{}", encode_path_segment(&user_id)),
            params,
        ),
    ))
}

pub fn user_mutual_counts_get_input(
    endpoint: String,
    user_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatUserMutualCountsGet requires userId.")?;
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("users/{}/mutuals", encode_path_segment(&user_id)),
            HashMap::new(),
        ),
    ))
}

pub fn user_groups_get_input(
    endpoint: String,
    user_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatUserGroupsGet requires userId.")?;
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("users/{}/groups", encode_path_segment(&user_id)),
            HashMap::new(),
        ),
    ))
}

pub fn user_represented_group_get_input(
    endpoint: String,
    user_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatUserRepresentedGroupGet requires userId.")?;
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("users/{}/groups/represented", encode_path_segment(&user_id)),
            HashMap::new(),
        ),
    ))
}

pub fn user_mutual_friends_get_input(
    endpoint: String,
    user_id: String,
    n: i64,
    offset: i64,
    include_user_id_param: bool,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatUserMutualFriendsGet requires userId.")?;
    let mut params = HashMap::from([
        ("n".to_string(), json!(n)),
        ("offset".to_string(), json!(offset)),
    ]);
    if include_user_id_param {
        params.insert("userId".to_string(), Value::String(user_id.clone()));
    }
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            format!("users/{}/mutuals/friends", encode_path_segment(&user_id)),
            params,
        ),
    ))
}

pub fn current_user_update_input(
    endpoint: String,
    user_id: String,
    params: Option<Value>,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatCurrentUserUpdate requires userId.")?;
    Ok((
        user_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("users/{}", encode_path_segment(&user_id)),
            Some(object_body(params)),
        ),
    ))
}

pub fn current_user_badge_update_input(
    endpoint: String,
    user_id: String,
    badge_id: String,
    hidden: bool,
    showcased: bool,
) -> Result<(String, String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatCurrentUserBadgeUpdate requires userId.")?;
    let badge_id = require_text(badge_id, "VrchatCurrentUserBadgeUpdate requires badgeId.")?;
    Ok((
        user_id.clone(),
        badge_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!(
                "users/{}/badges/{}",
                encode_path_segment(&user_id),
                encode_path_segment(&badge_id)
            ),
            Some(json!({
                "userId": user_id,
                "badgeId": badge_id,
                "hidden": hidden,
                "showcased": showcased,
            })),
        ),
    ))
}

pub fn current_user_tags_add_input(
    endpoint: String,
    user_id: String,
    tags: Vec<String>,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatCurrentUserTagsAdd requires userId.")?;
    Ok((
        user_id.clone(),
        api_input(
            endpoint,
            "POST",
            format!("users/{}/addTags", encode_path_segment(&user_id)),
            Some(json!({ "tags": tags })),
        ),
    ))
}

pub fn current_user_tags_remove_input(
    endpoint: String,
    user_id: String,
    tags: Vec<String>,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatCurrentUserTagsRemove requires userId.")?;
    Ok((
        user_id.clone(),
        api_input(
            endpoint,
            "POST",
            format!("users/{}/removeTags", encode_path_segment(&user_id)),
            Some(json!({ "tags": tags })),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_body(request: &HttpApiRequestInput) -> &Value {
        request.body.as_json().expect("expected JSON request body")
    }

    #[test]
    fn user_reads_trim_and_encode_ids() {
        let cases = [
            (
                "profile",
                user_get_input("endpoint".into(), " usr/雪 ".into())
                    .unwrap()
                    .1,
                "users/usr%2F%E9%9B%AA",
            ),
            (
                "mutual counts",
                user_mutual_counts_get_input("endpoint".into(), " usr/雪 ".into())
                    .unwrap()
                    .1,
                "users/usr%2F%E9%9B%AA/mutuals",
            ),
            (
                "groups",
                user_groups_get_input("endpoint".into(), " usr/雪 ".into())
                    .unwrap()
                    .1,
                "users/usr%2F%E9%9B%AA/groups",
            ),
            (
                "represented group",
                user_represented_group_get_input("endpoint".into(), " usr/雪 ".into())
                    .unwrap()
                    .1,
                "users/usr%2F%E9%9B%AA/groups/represented",
            ),
        ];

        for (name, request, expected_path) in cases {
            assert_eq!(request.method.as_deref(), Some("GET"), "{name}");
            assert_eq!(request.path.as_deref(), Some(expected_path), "{name}");
            assert_eq!(request.query_params, Some(HashMap::new()), "{name}");
        }
    }

    #[test]
    fn profile_reads_encode_ids_and_always_send_the_self_and_expansion_queries() {
        for as_self in [false, true] {
            let (user_id, profile) =
                profile_get_input("endpoint".into(), " usr/雪 ".into(), as_self).unwrap();
            assert_eq!(user_id, "usr/雪");
            assert_eq!(profile.method.as_deref(), Some("GET"));
            assert_eq!(profile.path.as_deref(), Some("profile/usr%2F%E9%9B%AA"));
            assert_eq!(
                profile.query_params,
                Some(HashMap::from([
                    ("asSelf".to_string(), json!(as_self)),
                    ("withGroupsAndWorlds".to_string(), json!(true)),
                ]))
            );
        }
    }

    #[test]
    fn mutual_friends_controls_the_legacy_user_id_query_parameter() {
        for (include_user_id_param, expected_user_id) in [
            (false, None),
            (true, Some(Value::String("usr/test".into()))),
        ] {
            let (user_id, request) = user_mutual_friends_get_input(
                "endpoint".into(),
                " usr/test ".into(),
                100,
                200,
                include_user_id_param,
            )
            .unwrap();
            let params = request.query_params.unwrap();

            assert_eq!(user_id, "usr/test");
            assert_eq!(
                request.path.as_deref(),
                Some("users/usr%2Ftest/mutuals/friends")
            );
            assert_eq!(params.get("n"), Some(&json!(100)));
            assert_eq!(params.get("offset"), Some(&json!(200)));
            assert_eq!(params.get("userId").cloned(), expected_user_id);
        }
    }

    #[test]
    fn current_user_mutations_build_paths_and_json_bodies() {
        let (_, update_default) =
            current_user_update_input("endpoint".into(), " usr/1 ".into(), None).unwrap();
        assert_eq!(update_default.method.as_deref(), Some("PUT"));
        assert_eq!(update_default.path.as_deref(), Some("users/usr%2F1"));
        assert_eq!(json_body(&update_default), &json!({}));

        let (_, update) = current_user_update_input(
            "endpoint".into(),
            " usr/1 ".into(),
            Some(json!({ "status": "ask me" })),
        )
        .unwrap();
        assert_eq!(json_body(&update), &json!({ "status": "ask me" }));

        let (user_id, badge_id, badge) = current_user_badge_update_input(
            "endpoint".into(),
            " usr/1 ".into(),
            " bdg 雪 ".into(),
            true,
            false,
        )
        .unwrap();
        assert_eq!((user_id.as_str(), badge_id.as_str()), ("usr/1", "bdg 雪"));
        assert_eq!(badge.method.as_deref(), Some("PUT"));
        assert_eq!(
            badge.path.as_deref(),
            Some("users/usr%2F1/badges/bdg%20%E9%9B%AA")
        );
        assert_eq!(
            json_body(&badge),
            &json!({
                "userId": "usr/1",
                "badgeId": "bdg 雪",
                "hidden": true,
                "showcased": false,
            })
        );

        for (name, request, suffix) in [
            (
                "add",
                current_user_tags_add_input(
                    "endpoint".into(),
                    " usr/1 ".into(),
                    vec!["system_1".into(), "system_2".into()],
                )
                .unwrap()
                .1,
                "addTags",
            ),
            (
                "remove",
                current_user_tags_remove_input(
                    "endpoint".into(),
                    " usr/1 ".into(),
                    vec!["system_1".into(), "system_2".into()],
                )
                .unwrap()
                .1,
                "removeTags",
            ),
        ] {
            assert_eq!(request.method.as_deref(), Some("POST"), "{name}");
            assert_eq!(
                request.path.as_deref(),
                Some(format!("users/usr%2F1/{suffix}").as_str()),
                "{name}"
            );
            assert_eq!(
                json_body(&request),
                &json!({ "tags": ["system_1", "system_2"] }),
                "{name}"
            );
        }
    }

    #[test]
    fn user_requests_reject_blank_required_ids() {
        assert!(user_get_input("".into(), " ".into()).is_err());
        assert!(profile_get_input("".into(), " ".into(), false).is_err());
        assert!(user_mutual_counts_get_input("".into(), " ".into()).is_err());
        assert!(user_groups_get_input("".into(), " ".into()).is_err());
        assert!(user_represented_group_get_input("".into(), " ".into()).is_err());
        assert!(user_mutual_friends_get_input("".into(), " ".into(), 1, 0, false).is_err());
        assert!(current_user_update_input("".into(), " ".into(), None).is_err());
        assert!(current_user_badge_update_input(
            "".into(),
            "user".into(),
            " ".into(),
            false,
            false,
        )
        .is_err());
        assert!(current_user_tags_add_input("".into(), " ".into(), vec![]).is_err());
        assert!(current_user_tags_remove_input("".into(), " ".into(), vec![]).is_err());
    }
}
