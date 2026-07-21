use std::collections::HashMap;

use serde_json::{json, Value};

use crate::http_api::{
    api_input, encode_path_segment, get_input, object_body, require_text, HttpApiError,
    HttpApiRequestInput,
};

pub fn world_get_input(
    endpoint: String,
    world_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let world_id = require_text(world_id, "VrchatWorldGet requires worldId.")?;
    Ok((
        world_id.clone(),
        get_input(
            endpoint,
            format!("worlds/{}", encode_path_segment(&world_id)),
            HashMap::new(),
        ),
    ))
}

pub fn world_list_by_user_get_input(
    endpoint: String,
    user_id: String,
    n: i64,
    offset: i64,
    sort: String,
    order: String,
    release_status: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatWorldListByUserGet requires userId.")?;
    Ok((
        user_id.clone(),
        get_input(
            endpoint,
            "worlds",
            HashMap::from([
                ("n".to_string(), json!(n)),
                ("offset".to_string(), json!(offset)),
                ("sort".to_string(), Value::String(sort)),
                ("order".to_string(), Value::String(order)),
                ("userId".to_string(), Value::String(user_id)),
                ("releaseStatus".to_string(), Value::String(release_status)),
            ]),
        ),
    ))
}

pub fn world_persistent_data_exists_input(
    endpoint: String,
    user_id: String,
    world_id: String,
) -> Result<(String, String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatWorldPersistentDataExists requires userId.")?;
    let world_id = require_text(
        world_id,
        "VrchatWorldPersistentDataExists requires worldId.",
    )?;
    Ok((
        user_id.clone(),
        world_id.clone(),
        get_input(
            endpoint,
            format!(
                "users/{}/{}/persist/exists",
                encode_path_segment(&user_id),
                encode_path_segment(&world_id)
            ),
            HashMap::new(),
        ),
    ))
}

pub fn world_save_input(
    endpoint: String,
    world_id: String,
    params: Option<Value>,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let world_id = require_text(world_id, "VrchatWorldSave requires worldId.")?;
    Ok((
        world_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("worlds/{}", encode_path_segment(&world_id)),
            Some(object_body(params)),
        ),
    ))
}

pub fn world_delete_input(
    endpoint: String,
    world_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let world_id = require_text(world_id, "VrchatWorldDelete requires worldId.")?;
    Ok((
        world_id.clone(),
        api_input(
            endpoint,
            "DELETE",
            format!("worlds/{}", encode_path_segment(&world_id)),
            None,
        ),
    ))
}

pub fn world_publish_input(
    endpoint: String,
    world_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let world_id = require_text(world_id, "VrchatWorldPublish requires worldId.")?;
    Ok((
        world_id.clone(),
        api_input(
            endpoint,
            "PUT",
            format!("worlds/{}/publish", encode_path_segment(&world_id)),
            Some(json!({ "worldId": world_id })),
        ),
    ))
}

pub fn world_unpublish_input(
    endpoint: String,
    world_id: String,
) -> Result<(String, HttpApiRequestInput), HttpApiError> {
    let world_id = require_text(world_id, "VrchatWorldUnpublish requires worldId.")?;
    Ok((
        world_id.clone(),
        api_input(
            endpoint,
            "DELETE",
            format!("worlds/{}/publish", encode_path_segment(&world_id)),
            None,
        ),
    ))
}

pub fn world_persistent_data_delete_input(
    endpoint: String,
    user_id: String,
    world_id: String,
) -> Result<(String, String, HttpApiRequestInput), HttpApiError> {
    let user_id = require_text(user_id, "VrchatWorldPersistentDataDelete requires userId.")?;
    let world_id = require_text(
        world_id,
        "VrchatWorldPersistentDataDelete requires worldId.",
    )?;
    Ok((
        user_id.clone(),
        world_id.clone(),
        api_input(
            endpoint,
            "DELETE",
            format!(
                "users/{}/{}/persist",
                encode_path_segment(&user_id),
                encode_path_segment(&world_id)
            ),
            None,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_api::HttpApiRequestBody;

    fn json_body(request: &HttpApiRequestInput) -> &Value {
        request.body.as_json().expect("expected JSON request body")
    }

    #[test]
    fn world_reads_trim_ids_and_preserve_list_filters() {
        let (world_id, world) = world_get_input("endpoint".into(), " wrld/雪 ".into()).unwrap();
        assert_eq!(world_id, "wrld/雪");
        assert_eq!(world.method.as_deref(), Some("GET"));
        assert_eq!(world.path.as_deref(), Some("worlds/wrld%2F%E9%9B%AA"));

        let (user_id, list) = world_list_by_user_get_input(
            "endpoint".into(),
            " usr/1 ".into(),
            50,
            100,
            "updated".into(),
            "descending".into(),
            "all".into(),
        )
        .unwrap();
        assert_eq!(user_id, "usr/1");
        assert_eq!(list.path.as_deref(), Some("worlds"));
        assert_eq!(
            list.query_params,
            Some(HashMap::from([
                ("n".into(), json!(50)),
                ("offset".into(), json!(100)),
                ("sort".into(), json!("updated")),
                ("order".into(), json!("descending")),
                ("userId".into(), json!("usr/1")),
                ("releaseStatus".into(), json!("all")),
            ]))
        );

        let (user_id, world_id, exists) = world_persistent_data_exists_input(
            "endpoint".into(),
            " usr/1 ".into(),
            " wrld/雪 ".into(),
        )
        .unwrap();
        assert_eq!((user_id.as_str(), world_id.as_str()), ("usr/1", "wrld/雪"));
        assert_eq!(exists.method.as_deref(), Some("GET"));
        assert_eq!(
            exists.path.as_deref(),
            Some("users/usr%2F1/wrld%2F%E9%9B%AA/persist/exists")
        );
    }

    #[test]
    fn world_mutations_match_the_vrcx_0_repository_contract() {
        let (_, save_default) =
            world_save_input("endpoint".into(), " wrld/1 ".into(), None).unwrap();
        assert_eq!(save_default.method.as_deref(), Some("PUT"));
        assert_eq!(save_default.path.as_deref(), Some("worlds/wrld%2F1"));
        assert_eq!(json_body(&save_default), &json!({}));

        let (_, save) = world_save_input(
            "endpoint".into(),
            " wrld/1 ".into(),
            Some(json!({ "name": "World" })),
        )
        .unwrap();
        assert_eq!(json_body(&save), &json!({ "name": "World" }));

        let (_, delete) = world_delete_input("endpoint".into(), " wrld/1 ".into()).unwrap();
        assert_eq!(delete.method.as_deref(), Some("DELETE"));
        assert_eq!(delete.path.as_deref(), Some("worlds/wrld%2F1"));
        assert_eq!(delete.body, HttpApiRequestBody::Empty);

        let (_, publish) = world_publish_input("endpoint".into(), " wrld/1 ".into()).unwrap();
        assert_eq!(publish.method.as_deref(), Some("PUT"));
        assert_eq!(publish.path.as_deref(), Some("worlds/wrld%2F1/publish"));
        assert_eq!(json_body(&publish), &json!({ "worldId": "wrld/1" }));

        let (_, unpublish) = world_unpublish_input("endpoint".into(), " wrld/1 ".into()).unwrap();
        assert_eq!(unpublish.method.as_deref(), Some("DELETE"));
        assert_eq!(unpublish.path.as_deref(), Some("worlds/wrld%2F1/publish"));
        assert_eq!(unpublish.body, HttpApiRequestBody::Empty);

        let (user_id, world_id, delete_data) = world_persistent_data_delete_input(
            "endpoint".into(),
            " usr/1 ".into(),
            " wrld/雪 ".into(),
        )
        .unwrap();
        assert_eq!((user_id.as_str(), world_id.as_str()), ("usr/1", "wrld/雪"));
        assert_eq!(delete_data.method.as_deref(), Some("DELETE"));
        assert_eq!(
            delete_data.path.as_deref(),
            Some("users/usr%2F1/wrld%2F%E9%9B%AA/persist")
        );
        assert_eq!(delete_data.body, HttpApiRequestBody::Empty);
    }

    #[test]
    fn world_requests_reject_blank_required_ids() {
        assert!(world_get_input("".into(), " ".into()).is_err());
        assert!(world_list_by_user_get_input(
            "".into(),
            " ".into(),
            1,
            0,
            "updated".into(),
            "descending".into(),
            "all".into(),
        )
        .is_err());
        assert!(world_persistent_data_exists_input("".into(), "user".into(), " ".into()).is_err());
        assert!(world_save_input("".into(), " ".into(), None).is_err());
        assert!(world_delete_input("".into(), " ".into()).is_err());
        assert!(world_publish_input("".into(), " ".into()).is_err());
        assert!(world_unpublish_input("".into(), " ".into()).is_err());
        assert!(world_persistent_data_delete_input("".into(), " ".into(), "world".into()).is_err());
    }
}
