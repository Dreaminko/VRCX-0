use std::time::Duration;
use vrcx_0_media::media_files;

use vrcx_0_vrchat_client::http_api::{
    normalize_text, ApiScope, HttpApiExecuteResponse, HttpApiRequestInput,
};
use vrcx_0_vrchat_client::media::{
    avatar_image_set_input, file_put_input, file_upload_finish_input, file_upload_stage_path,
    file_upload_start_input, file_version_create_input, normalize_media_endpoint,
    world_image_set_input, FileUploadStageKind,
};

use crate::{Error, Result};
use serde_json::{json, Map, Value};

use super::{LegacyEntityImageKind, LegacyEntityImageUploadInput, LegacyMediaUploadDeps};

const LEGACY_MEDIA_REMOTE_MUTATION_INTERVAL: Duration = Duration::from_millis(250);

struct LegacyEntityImageTarget {
    entity_label: &'static str,
    entity_output_key: &'static str,
}

fn legacy_entity_image_target(kind: LegacyEntityImageKind) -> LegacyEntityImageTarget {
    match kind {
        LegacyEntityImageKind::Avatar => LegacyEntityImageTarget {
            entity_label: "Avatar",
            entity_output_key: "avatar",
        },
        LegacyEntityImageKind::World => LegacyEntityImageTarget {
            entity_label: "World",
            entity_output_key: "world",
        },
    }
}

pub async fn upload_legacy_entity_image(
    deps: LegacyMediaUploadDeps<'_>,
    input: LegacyEntityImageUploadInput,
    kind: LegacyEntityImageKind,
) -> Result<HttpApiExecuteResponse> {
    let target = legacy_entity_image_target(kind);
    let LegacyEntityImageUploadInput {
        entity_id,
        image_url,
        base64_file,
        file_size_in_bytes,
    } = input;
    let entity_id = require_text(
        entity_id,
        &format!(
            "VrchatMediaLegacyImageUpload requires {} id.",
            target.entity_label
        ),
    )?;
    let endpoint = normalize_media_endpoint(&deps.mutation.scope().endpoint);
    let source_file_id = extract_file_id(&image_url);
    if source_file_id.is_empty() {
        return Err(Error::Custom(format!(
            "{} image upload requires an existing source image file id.",
            target.entity_label
        )));
    }
    if base64_file.trim().is_empty() {
        return Err(Error::Custom(format!(
            "{} image upload requires image data.",
            target.entity_label
        )));
    }

    let file_data = media_files::decode_file_base64(&base64_file)?;
    drop(base64_file);
    let file_md5 = media_files::md5_base64(&file_data);
    let file_size_in_bytes = file_size_in_bytes
        .filter(|value| *value > 0)
        .unwrap_or(file_data.len() as i64);
    let signature_file = media_files::sign_file(&file_data);
    let signature_md5 = media_files::md5_base64(&signature_file);
    let signature_size_in_bytes = signature_file.len() as i64;

    let upload = execute_media_json(
        &deps,
        file_version_create_input(
            endpoint.clone(),
            source_file_id.clone(),
            file_md5.clone(),
            file_size_in_bytes,
            signature_md5.clone(),
            signature_size_in_bytes,
        )?,
        &format!("{} image upload failed", target.entity_label),
    )
    .await?;
    let uploaded_file_id = json_field_string(&upload, "id");
    let file_version = latest_file_version(&upload).ok_or_else(|| {
        Error::Custom(format!(
            "{} image upload did not return a file version.",
            target.entity_label
        ))
    })?;
    if uploaded_file_id.is_empty() {
        return Err(Error::Custom(format!(
            "{} image upload did not return a file id.",
            target.entity_label
        )));
    }

    for (kind, file_data, file_mime, file_md5) in [
        (FileUploadStageKind::File, file_data, "image/png", file_md5),
        (
            FileUploadStageKind::Signature,
            signature_file,
            "application/x-rsync-signature",
            signature_md5,
        ),
    ] {
        let stage_path = file_upload_stage_path(uploaded_file_id.clone(), file_version, kind)?;
        let start = execute_media_json(
            &deps,
            file_upload_start_input(endpoint.clone(), stage_path.clone()),
            &format!("{} image upload failed", target.entity_label),
        )
        .await?;
        let upload_url = json_field_string(&start, "url");
        if upload_url.is_empty() {
            return Err(Error::Custom(format!(
                "{} image upload did not return a {kind} upload URL.",
                target.entity_label
            )));
        }
        execute_media_success(
            &deps,
            file_put_input(upload_url, file_data, file_mime.to_string(), file_md5),
            &format!("{} image file PUT failed", target.entity_label),
        )
        .await?;
        execute_media_json(
            &deps,
            file_upload_finish_input(endpoint.clone(), stage_path),
            &format!("{} image upload failed", target.entity_label),
        )
        .await?;
    }

    let next_image_url = format!("{endpoint}/file/{uploaded_file_id}/{file_version}/file");
    let entity_update_request = match kind {
        LegacyEntityImageKind::Avatar => {
            avatar_image_set_input(endpoint, entity_id.clone(), next_image_url.clone())?
        }
        LegacyEntityImageKind::World => {
            world_image_set_input(endpoint, entity_id.clone(), next_image_url.clone())?
        }
    };
    let entity = execute_media_json(
        &deps,
        entity_update_request,
        &format!("{} image change failed", target.entity_label),
    )
    .await?;
    if json_field_string(&entity, "imageUrl") != next_image_url {
        return Err(Error::Custom(format!(
            "{} image change failed.",
            target.entity_label
        )));
    }

    let mut payload = Map::new();
    payload.insert(target.entity_output_key.to_string(), entity);
    payload.insert("imageUrl".into(), Value::String(next_image_url));
    payload.insert("fileId".into(), Value::String(uploaded_file_id));
    payload.insert("fileVersion".into(), json!(file_version));
    let payload = Value::Object(payload);
    Ok(vrcx_0_vrchat_client::http_api::execute_response(
        200,
        payload.to_string(),
    ))
}

fn require_text(value: impl AsRef<str>, message: &str) -> Result<String> {
    let value = normalize_text(value);
    if value.is_empty() {
        return Err(Error::Custom(message.to_string()));
    }
    Ok(value)
}

async fn execute_media_json(
    deps: &LegacyMediaUploadDeps<'_>,
    input: HttpApiRequestInput,
    fallback_message: &str,
) -> Result<Value> {
    response_json(execute_media_request(deps, input).await?, fallback_message)
}

async fn execute_media_success(
    deps: &LegacyMediaUploadDeps<'_>,
    input: HttpApiRequestInput,
    fallback_message: &str,
) -> Result<()> {
    let response = execute_media_request(deps, input).await?;
    if response.status < 200 || response.status >= 300 {
        return Err(Error::Custom(format!(
            "{fallback_message} ({})",
            response.status
        )));
    }
    Ok(())
}

async fn execute_media_request(
    deps: &LegacyMediaUploadDeps<'_>,
    mut input: HttpApiRequestInput,
) -> Result<HttpApiExecuteResponse> {
    deps.mutation.apply_scope_to_request(&mut input);
    deps.mutation
        .run_after_wait(LEGACY_MEDIA_REMOTE_MUTATION_INTERVAL, || async move {
            deps.web
                .execute_api(input, ApiScope::VrchatMedia, deps.db)
                .await
        })
        .await
}

fn response_json(response: HttpApiExecuteResponse, fallback_message: &str) -> Result<Value> {
    let json = serde_json::from_str::<Value>(&response.data).unwrap_or(Value::Null);
    if response.status >= 400
        || json
            .as_object()
            .is_some_and(|object| object.contains_key("error"))
    {
        let message = json
            .as_object()
            .and_then(|object| object.get("error"))
            .and_then(|error| {
                error
                    .as_str()
                    .map(ToOwned::to_owned)
                    .or_else(|| serde_json::to_string(error).ok())
            })
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| format!("{fallback_message} ({})", response.status));
        return Err(Error::Custom(message));
    }
    Ok(json)
}

fn json_field_string(value: &Value, field: &str) -> String {
    value
        .as_object()
        .and_then(|object| object.get(field))
        .and_then(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .or_else(|| (!value.is_null()).then(|| value.to_string()))
        })
        .unwrap_or_default()
}

fn value_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()))
}

fn extract_file_id(value: &str) -> String {
    let Some(start) = value.find("file_") else {
        return String::new();
    };
    value[start..]
        .chars()
        .take_while(|character| {
            character.is_ascii_alphanumeric() || *character == '_' || *character == '-'
        })
        .collect()
}

fn latest_file_version(upload: &Value) -> Option<i64> {
    upload
        .as_object()
        .and_then(|object| object.get("versions"))
        .and_then(Value::as_array)
        .and_then(|versions| versions.last())
        .and_then(|version| version.as_object().and_then(|object| object.get("version")))
        .and_then(value_as_i64)
}
