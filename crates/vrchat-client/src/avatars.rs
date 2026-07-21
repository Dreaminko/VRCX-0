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
    empty_body: bool,
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
            empty_body.then(|| json!({})),
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

    const ENDPOINT: &str = "https://api.vrchat.cloud/api/1";

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
}
