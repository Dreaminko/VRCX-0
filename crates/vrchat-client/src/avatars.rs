use std::collections::HashMap;

use serde_json::{json, Value};

use crate::http_api::{
    api_input, encode_path_segment, get_input, normalize_text, object_body, query_input,
    require_text, HttpApiError, HttpApiRequestInput,
};

pub fn avatar_get_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarGet requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        get_input(
            endpoint,
            format!("avatars/{}", encode_path_segment(&avatar_id)),
            HashMap::new(),
        ),
    ))
}

pub fn avatar_gallery_get_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarGalleryGet requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        get_input(
            endpoint,
            "files",
            HashMap::from([
                ("tag".to_string(), Value::String("avatargallery".into())),
                ("galleryId".to_string(), Value::String(avatar_id)),
                ("n".to_string(), json!(100)),
                ("offset".to_string(), json!(0)),
            ]),
        ),
    ))
}

pub struct AvatarListByUserGetInput {
    pub endpoint: String,
    pub user_id: String,
    pub user: String,
    pub n: i64,
    pub offset: i64,
    pub sort: String,
    pub order: String,
    pub release_status: String,
}

pub fn avatar_list_by_user_get_input(
    input: AvatarListByUserGetInput,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user = normalize_text(input.user);
    let user_id = normalize_text(input.user_id);
    if user.is_empty() && user_id.is_empty() {
        return Err(HttpApiError::Custom(
            "VrchatAvatarListByUserGet requires user or userId.".into(),
        ));
    }
    let mut params = HashMap::from([
        ("n".to_string(), json!(input.n)),
        ("offset".to_string(), json!(input.offset)),
        ("sort".to_string(), Value::String(input.sort)),
        ("order".to_string(), Value::String(input.order)),
        (
            "releaseStatus".to_string(),
            Value::String(input.release_status),
        ),
    ]);
    let display = if user.is_empty() {
        params.insert("userId".to_string(), Value::String(user_id.clone()));
        user_id
    } else {
        params.insert("user".to_string(), Value::String(user.clone()));
        user
    };
    Ok((display, get_input(input.endpoint, "avatars", params)))
}

pub fn avatar_styles_get_input(endpoint: String) -> HttpApiRequestInput {
    get_input(endpoint, "avatarStyles", HashMap::new())
}

pub fn avatar_moderations_get_input(endpoint: String) -> HttpApiRequestInput {
    get_input(endpoint, "auth/user/avatarmoderations", HashMap::new())
}

pub fn avatar_file_get_input(
    endpoint: String,
    file_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let file_id = require_text(file_id, "VrchatAvatarFileGet requires fileId.")?;
    Ok((
        file_id.clone(),
        get_input(
            endpoint,
            format!("file/{}", encode_path_segment(&file_id)),
            HashMap::new(),
        ),
    ))
}

pub fn avatar_select_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarSelect requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("avatars/{}/select", encode_path_segment(&avatar_id)),
            Some(json!({ "avatarId": avatar_id })),
        ),
    ))
}

pub fn avatar_select_fallback_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarSelectFallback requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("avatars/{}/selectfallback", encode_path_segment(&avatar_id)),
            Some(json!({ "avatarId": avatar_id })),
        ),
    ))
}

pub fn avatar_save_input(
    endpoint: String,
    avatar_id: String,
    params: Option<Value>,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarSave requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("avatars/{}", encode_path_segment(&avatar_id)),
            Some(object_body(params)),
        ),
    ))
}

pub fn avatar_delete_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarDelete requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "DELETE",
            format!("avatars/{}", encode_path_segment(&avatar_id)),
            None,
        ),
    ))
}

pub fn avatar_impostor_create_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarImpostorCreate requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "POST",
            format!(
                "avatars/{}/impostor/enqueue",
                encode_path_segment(&avatar_id)
            ),
            Some(json!({})),
        ),
    ))
}

pub fn avatar_impostor_delete_input(
    endpoint: String,
    avatar_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarImpostorDelete requires avatarId.")?;
    Ok((
        avatar_id.clone(),
        api_input(
            endpoint,
            "DELETE",
            format!("avatars/{}/impostor", encode_path_segment(&avatar_id)),
            None,
        ),
    ))
}

pub fn avatar_moderation_send_input(
    endpoint: String,
    avatar_id: String,
    type_name: String,
) -> Result<(String, String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarModerationSend requires avatarId.")?;
    let type_name = moderation_type(type_name);
    Ok((
        avatar_id.clone(),
        type_name.clone(),
        api_input(
            endpoint,
            "POST",
            "auth/user/avatarmoderations",
            Some(json!({
                "avatarModerationType": type_name,
                "targetAvatarId": avatar_id,
            })),
        ),
    ))
}

pub fn avatar_moderation_delete_input(
    endpoint: String,
    avatar_id: String,
    type_name: String,
) -> Result<(String, String, HttpApiRequestInput), HttpApiError> {
    let avatar_id = require_text(avatar_id, "VrchatAvatarModerationDelete requires avatarId.")?;
    let type_name = moderation_type(type_name);
    Ok((
        avatar_id.clone(),
        type_name.clone(),
        query_input(
            endpoint,
            "DELETE",
            "auth/user/avatarmoderations",
            HashMap::from([
                ("avatarModerationType".to_string(), Value::String(type_name)),
                ("targetAvatarId".to_string(), Value::String(avatar_id)),
            ]),
        ),
    ))
}

fn moderation_type(type_name: String) -> String {
    let type_name = normalize_text(type_name);
    if type_name.is_empty() {
        "block".to_string()
    } else {
        type_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_api::{build_web_execute_request, ApiScope};
    use url::Url;

    const ENDPOINT: &str = "https://api.vrchat.cloud/api/1";

    fn query_pairs(url: &str) -> HashMap<String, String> {
        Url::parse(url)
            .unwrap()
            .query_pairs()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    fn list_by_user_input(user_id: &str, user: &str) -> AvatarListByUserGetInput {
        AvatarListByUserGetInput {
            endpoint: ENDPOINT.into(),
            user_id: user_id.into(),
            user: user.into(),
            n: 60,
            offset: 0,
            sort: "updated".into(),
            order: "descending".into(),
            release_status: "all".into(),
        }
    }

    #[test]
    fn avatar_get_rejects_blank_avatar_id() {
        let error = avatar_get_input(ENDPOINT.into(), "  ".into()).unwrap_err();

        assert!(
            matches!(error, HttpApiError::Custom(message) if message == "VrchatAvatarGet requires avatarId.")
        );
    }

    #[test]
    fn avatar_list_by_user_prefers_user_over_user_id() {
        let (display, input) =
            avatar_list_by_user_get_input(list_by_user_input("usr_id_value", " display_name "))
                .unwrap();

        assert_eq!(display, "display_name");
        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        let params = query_pairs(&request.url);
        assert_eq!(params.get("user"), Some(&"display_name".to_string()));
        assert_eq!(params.get("userId"), None);
    }

    #[test]
    fn avatar_list_by_user_falls_back_to_user_id_when_user_blank() {
        let (display, input) =
            avatar_list_by_user_get_input(list_by_user_input(" usr_id_value ", "  ")).unwrap();

        assert_eq!(display, "usr_id_value");
        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        let params = query_pairs(&request.url);
        assert_eq!(params.get("userId"), Some(&"usr_id_value".to_string()));
        assert_eq!(params.get("user"), None);
    }

    #[test]
    fn avatar_list_by_user_requires_user_or_user_id() {
        let error = avatar_list_by_user_get_input(list_by_user_input("  ", "  ")).unwrap_err();

        assert!(matches!(
            error,
            HttpApiError::Custom(message)
                if message == "VrchatAvatarListByUserGet requires user or userId."
        ));
    }

    #[test]
    fn avatar_moderation_send_defaults_type_to_block_when_blank() {
        let (avatar_id, type_name, input) =
            avatar_moderation_send_input(ENDPOINT.into(), " avtr_test ".into(), "  ".into())
                .unwrap();

        assert_eq!(avatar_id, "avtr_test");
        assert_eq!(type_name, "block");
        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        assert_eq!(
            request.body.as_deref(),
            Some(r#"{"avatarModerationType":"block","targetAvatarId":"avtr_test"}"#)
        );
    }

    #[test]
    fn avatar_moderation_send_preserves_given_type() {
        let (_, type_name, input) =
            avatar_moderation_send_input(ENDPOINT.into(), "avtr_test".into(), " hide ".into())
                .unwrap();

        assert_eq!(type_name, "hide");
        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        assert_eq!(
            request.body.as_deref(),
            Some(r#"{"avatarModerationType":"hide","targetAvatarId":"avtr_test"}"#)
        );
    }

    #[test]
    fn avatar_moderation_delete_defaults_type_to_block_and_uses_query_params() {
        let (avatar_id, type_name, input) =
            avatar_moderation_delete_input(ENDPOINT.into(), " avtr_test ".into(), "  ".into())
                .unwrap();

        assert_eq!(avatar_id, "avtr_test");
        assert_eq!(type_name, "block");
        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        assert_eq!(request.method, "DELETE");
        let params = query_pairs(&request.url);
        assert_eq!(
            params.get("avatarModerationType"),
            Some(&"block".to_string())
        );
        assert_eq!(params.get("targetAvatarId"), Some(&"avtr_test".to_string()));
        assert!(request.body.is_none());
    }

    #[test]
    fn avatar_save_defaults_body_to_empty_object_for_non_object_params() {
        let input = avatar_save_input(
            ENDPOINT.into(),
            "avtr_test".into(),
            Some(json!("not an object")),
        )
        .unwrap()
        .1;

        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        assert_eq!(request.body.as_deref(), Some("{}"));
    }

    #[test]
    fn avatar_save_uses_provided_object_params_as_body() {
        let input = avatar_save_input(
            ENDPOINT.into(),
            "avtr_test".into(),
            Some(json!({ "name": "New Name" })),
        )
        .unwrap()
        .1;

        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();
        assert_eq!(request.body.as_deref(), Some(r#"{"name":"New Name"}"#));
    }

    #[test]
    fn avatar_selection_sends_avatar_id_as_json() {
        let cases = [
            (
                "select",
                avatar_select_input(ENDPOINT.into(), " avtr_test ".into())
                    .unwrap()
                    .1,
            ),
            (
                "selectfallback",
                avatar_select_fallback_input(ENDPOINT.into(), " avtr_test ".into())
                    .unwrap()
                    .1,
            ),
        ];

        for (path, input) in cases {
            let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();

            assert_eq!(request.method, "PUT");
            assert_eq!(
                request.url,
                format!("{ENDPOINT}/avatars/avtr%5Ftest/{path}")
            );
            assert_eq!(request.body.as_deref(), Some(r#"{"avatarId":"avtr_test"}"#));
            assert!(request.headers.contains(&(
                "Content-Type".into(),
                "application/json;charset=utf-8".into()
            )));
        }
    }

    #[test]
    fn avatar_impostor_enqueue_sends_the_legacy_empty_json_body() {
        let input = avatar_impostor_create_input(ENDPOINT.into(), " avtr_test ".into())
            .unwrap()
            .1;

        let request = build_web_execute_request(input, ApiScope::Vrchat).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(
            request.url,
            format!("{ENDPOINT}/avatars/avtr%5Ftest/impostor/enqueue")
        );
        assert_eq!(request.body.as_deref(), Some("{}"));
        assert!(request.headers.contains(&(
            "Content-Type".into(),
            "application/json;charset=utf-8".into()
        )));
    }
}
