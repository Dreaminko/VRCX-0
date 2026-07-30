use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{json, Value};
use vrcx_0_persistence::avatars::{avatar_tags_list, avatar_time_spent_list};
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::{
    avatars::{avatar_list_by_user_get_input, AvatarListByUserGetInput},
    http_api::{ApiScope, HttpApiRequestInput},
};

use crate::{Error, Result, RuntimeAuthScope, RuntimeAuthScopeSnapshot, WebClient};

const MY_AVATARS_PAGE_SIZE: i64 = 50;
const MY_AVATARS_MAX_OFFSET: i64 = 5_000;

pub struct MyAvatarsDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub auth_scope: &'a RuntimeAuthScope,
    pub expected_scope: RuntimeAuthScopeSnapshot,
}

#[derive(Clone, Debug, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MyAvatarsInput {
    #[serde(default)]
    pub current_avatar_id: String,
    #[serde(default)]
    pub previous_avatar_swap_time: f64,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MyAvatarByIdInput {
    pub avatar_id: String,
}

pub async fn get_my_avatars(deps: &MyAvatarsDeps<'_>, input: MyAvatarsInput) -> Result<Vec<Value>> {
    let avatars = fetch_my_avatar_pages(deps, None).await?;
    let tags_by_avatar = collect_tags_by_avatar(deps.db)?;
    let time_spent_by_avatar: HashMap<String, i64> =
        avatar_time_spent_list(deps.db, deps.expected_scope.current_user_id.clone())?
            .into_iter()
            .map(|row| (row.avatar_id, row.time_spent))
            .collect();

    let current_avatar_id = input.current_avatar_id.trim().to_string();
    let swap_delta = live_swap_delta_ms(input.previous_avatar_swap_time);

    Ok(avatars
        .into_iter()
        .map(|mut avatar| {
            let avatar_id = record_id(&avatar);
            let mut time_spent = time_spent_by_avatar.get(&avatar_id).copied().unwrap_or(0);
            if !current_avatar_id.is_empty() && avatar_id == current_avatar_id {
                time_spent += swap_delta;
            }
            if let Some(object) = avatar.as_object_mut() {
                object.insert(
                    "$tags".into(),
                    Value::Array(tags_by_avatar.get(&avatar_id).cloned().unwrap_or_default()),
                );
                object.insert("$timeSpent".into(), json!(time_spent));
            }
            avatar
        })
        .collect())
}

pub async fn get_my_avatar_by_id(
    deps: &MyAvatarsDeps<'_>,
    input: MyAvatarByIdInput,
) -> Result<Option<Value>> {
    let avatar_id = input.avatar_id.trim().to_string();
    if avatar_id.is_empty() {
        return Err(Error::Custom(
            "My avatar lookup requires an avatar id.".into(),
        ));
    }
    let matches = fetch_my_avatar_pages(deps, Some(&avatar_id)).await?;
    Ok(matches.into_iter().next())
}

async fn fetch_my_avatar_pages(
    deps: &MyAvatarsDeps<'_>,
    target_avatar_id: Option<&str>,
) -> Result<Vec<Value>> {
    let mut avatars = Vec::new();
    let mut offset = 0;

    while offset <= MY_AVATARS_MAX_OFFSET {
        let (_, request) = avatar_list_by_user_get_input(AvatarListByUserGetInput {
            endpoint: deps.expected_scope.endpoint.clone(),
            user_id: String::new(),
            user: "me".into(),
            n: MY_AVATARS_PAGE_SIZE,
            offset,
            sort: "updated".into(),
            order: "descending".into(),
            release_status: "all".into(),
        })?;
        let page = execute_json_array(deps, request).await?;
        let page_len = page.len() as i64;

        if let Some(target) = target_avatar_id {
            if let Some(found) = page.iter().find(|avatar| record_id(avatar) == target) {
                return Ok(vec![found.clone()]);
            }
        } else {
            avatars.extend(page);
        }

        if page_len < MY_AVATARS_PAGE_SIZE {
            break;
        }
        offset += MY_AVATARS_PAGE_SIZE;
    }

    Ok(avatars)
}

async fn execute_json_array(
    deps: &MyAvatarsDeps<'_>,
    request: HttpApiRequestInput,
) -> Result<Vec<Value>> {
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db)
        .await?;
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
    let payload = serde_json::from_str::<Value>(&response.data)
        .unwrap_or_else(|_| Value::String(response.data.clone()));
    if response.status >= 400 || payload.get("error").is_some() {
        return Err(Error::Custom(response_error_message(
            &payload,
            response.status,
        )));
    }
    Ok(payload.as_array().cloned().unwrap_or_default())
}

fn response_error_message(payload: &Value, status: i32) -> String {
    let detail = payload
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("status {status}"));
    format!("My avatars request failed: {detail}")
}

fn collect_tags_by_avatar(db: &DatabaseService) -> Result<HashMap<String, Vec<Value>>> {
    let mut tags_by_avatar: HashMap<String, Vec<Value>> = HashMap::new();
    for row in avatar_tags_list(db)? {
        let tag = row.tag.trim().to_string();
        if tag.is_empty() {
            continue;
        }
        let color = if row.color.is_string() {
            row.color
        } else {
            Value::Null
        };
        tags_by_avatar
            .entry(row.avatar_id.trim().to_string())
            .or_default()
            .push(json!({ "tag": tag, "color": color }));
    }
    Ok(tags_by_avatar)
}

fn record_id(record: &Value) -> String {
    record
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn live_swap_delta_ms(previous_avatar_swap_time: f64) -> i64 {
    if !previous_avatar_swap_time.is_finite() || previous_avatar_swap_time <= 0.0 {
        return 0;
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as f64)
        .unwrap_or(0.0);
    let delta = now_ms - previous_avatar_swap_time;
    if delta > 0.0 {
        delta as i64
    } else {
        0
    }
}

fn ensure_scope_matches(
    current: &RuntimeAuthScopeSnapshot,
    expected: &RuntimeAuthScopeSnapshot,
) -> Result<()> {
    if current.active
        && current.generation == expected.generation
        && current.current_user_id == expected.current_user_id
        && current.endpoint == expected.endpoint
    {
        Ok(())
    } else {
        Err(Error::Custom(
            "My avatars authentication scope changed.".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_swap_delta_ignores_invalid_swap_times() {
        assert_eq!(live_swap_delta_ms(0.0), 0);
        assert_eq!(live_swap_delta_ms(-5.0), 0);
        assert_eq!(live_swap_delta_ms(f64::NAN), 0);
        assert_eq!(live_swap_delta_ms(f64::INFINITY), 0);
    }

    #[test]
    fn live_swap_delta_counts_elapsed_wall_clock() {
        let one_minute_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as f64
            - 60_000.0;
        let delta = live_swap_delta_ms(one_minute_ago);
        assert!(delta >= 60_000 && delta < 120_000);
    }

    #[test]
    fn record_id_trims_and_defaults() {
        assert_eq!(record_id(&json!({ "id": " avtr_1 " })), "avtr_1");
        assert_eq!(record_id(&json!({ "id": 7 })), "");
        assert_eq!(record_id(&json!("not-an-object")), "");
    }
}
