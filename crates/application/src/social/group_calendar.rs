use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use vrcx_0_application_core::RuntimeOperationStatus;

use futures_util::{stream, StreamExt};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Semaphore;
use vrcx_0_application_core::vrchat_api::groups::profile_get_input;
use vrcx_0_application_core::vrchat_api::tools::{
    calendars_get_input, featured_calendars_get_input, following_calendars_get_input,
};
use vrcx_0_application_core::{
    RuntimeAuthScope, RuntimeAuthScopeSnapshot, RuntimeDiagnostics, RuntimeSyncEngine, WebClient,
};
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::{ApiJsonResponse, ApiScope, HttpApiRequestInput};

use crate::{Error, Result};

const CALENDAR_PAGE_SIZE: i64 = 100;
const CALENDAR_MAX_PAGES: usize = 50;
const GROUP_PROFILE_CONCURRENCY: usize = 4;
const GROUP_PROFILE_CACHE_CAPACITY: u64 = 128;
const GROUP_PROFILE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct GroupCalendarDeps {
    pub db: Arc<DatabaseService>,
    pub web: Arc<WebClient>,
    pub auth_scope: RuntimeAuthScope,
    pub diagnostics: RuntimeDiagnostics,
    pub sync: RuntimeSyncEngine,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupCalendarInput {
    pub date: String,
    #[serde(default)]
    pub include_featured: bool,
}

#[derive(Clone, Debug, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupCalendarSnapshot {
    pub events: Vec<Value>,
    pub following_event_ids: Vec<String>,
    pub group_names: HashMap<String, String>,
    pub group_profiles: HashMap<String, Value>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct GroupProfileCacheKey {
    auth_scope_generation: u64,
    group_id: String,
}

static GROUP_PROFILE_CACHE: OnceLock<Cache<GroupProfileCacheKey, Value>> = OnceLock::new();
static GROUP_PROFILE_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

pub async fn load_group_calendar(
    deps: GroupCalendarDeps,
    input: GroupCalendarInput,
) -> Result<GroupCalendarSnapshot> {
    let command = "app__group_calendar_snapshot_get";
    let scope = require_active_scope(&deps.auth_scope)?;
    let date = input.date.trim().to_string();
    if date.is_empty() {
        return Err(Error::Custom(
            "Group calendar snapshot requires a date.".into(),
        ));
    }
    deps.diagnostics.record_command(
        command,
        RuntimeOperationStatus::Running,
        format!("Loading calendar {date}."),
    );

    let result = load_group_calendar_inner(&deps, &scope, &date, input.include_featured).await;
    match &result {
        Ok(snapshot) => {
            deps.diagnostics.record_command(
                command,
                RuntimeOperationStatus::Ok,
                format!(
                    "events={}, following={}, groups={}",
                    snapshot.events.len(),
                    snapshot.following_event_ids.len(),
                    snapshot.group_names.len()
                ),
            );
            deps.sync.record(
                "groupCalendar",
                RuntimeOperationStatus::Ready,
                "Group calendar snapshot loaded.",
                0,
            );
        }
        Err(error) => {
            deps.diagnostics.record_command(
                command,
                RuntimeOperationStatus::Error,
                error.to_string(),
            );
            deps.sync.record_failure("groupCalendar", error.to_string());
        }
    }
    result
}

async fn load_group_calendar_inner(
    deps: &GroupCalendarDeps,
    scope: &RuntimeAuthScopeSnapshot,
    date: &str,
    include_featured: bool,
) -> Result<GroupCalendarSnapshot> {
    let calendars = collect_calendar_pages(deps, scope, date, CalendarKind::All);
    let following = collect_calendar_pages(deps, scope, date, CalendarKind::Following);
    let featured = async {
        if include_featured {
            collect_calendar_pages(deps, scope, date, CalendarKind::Featured).await
        } else {
            Ok(Vec::new())
        }
    };
    let (mut events, following, featured) = tokio::try_join!(calendars, following, featured)?;
    events.extend(featured);

    let following_event_ids = following.iter().filter_map(event_id).collect::<Vec<_>>();
    let all_rows = events.iter().chain(following.iter()).collect::<Vec<_>>();
    let group_ids = collect_group_ids(&all_rows);
    let embedded_profiles = embedded_group_profiles(&all_rows);
    let fetched_profiles = fetch_group_profiles(deps, scope, &group_ids).await;
    ensure_scope_matches(&deps.auth_scope, scope)?;

    let mut group_profiles = embedded_profiles;
    group_profiles.extend(fetched_profiles);
    let group_names = group_ids
        .into_iter()
        .map(|group_id| {
            let name = group_profiles
                .get(&group_id)
                .and_then(|group| group.get("name"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(&group_id)
                .to_string();
            (group_id, name)
        })
        .collect();

    Ok(GroupCalendarSnapshot {
        events,
        following_event_ids,
        group_names,
        group_profiles,
    })
}

#[derive(Clone, Copy)]
enum CalendarKind {
    All,
    Following,
    Featured,
}

struct CalendarPage {
    rows: Vec<Value>,
    has_next: Option<bool>,
}

async fn collect_calendar_pages(
    deps: &GroupCalendarDeps,
    scope: &RuntimeAuthScopeSnapshot,
    date: &str,
    kind: CalendarKind,
) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    for page_index in 0..=CALENDAR_MAX_PAGES {
        ensure_scope_matches(&deps.auth_scope, scope)?;
        let offset = (page_index as i64) * CALENDAR_PAGE_SIZE;
        let params = HashMap::from([
            ("n".into(), Value::from(CALENDAR_PAGE_SIZE)),
            ("offset".into(), Value::from(offset)),
            ("date".into(), Value::from(date)),
        ]);
        let request = match kind {
            CalendarKind::All => calendars_get_input(scope.endpoint.clone(), params),
            CalendarKind::Following => {
                following_calendars_get_input(scope.endpoint.clone(), params)
            }
            CalendarKind::Featured => featured_calendars_get_input(scope.endpoint.clone(), params),
        };
        let page = execute_page(deps, scope, request).await?;
        let count = page.rows.len();
        if page_index == CALENDAR_MAX_PAGES {
            if count == 0 {
                return Ok(rows);
            }
            return Err(Error::Custom(
                "Group calendar pagination exceeded the safety limit.".into(),
            ));
        }
        rows.extend(page.rows);
        if page.has_next == Some(false) || count < CALENDAR_PAGE_SIZE as usize {
            return Ok(rows);
        }
    }
    Ok(rows)
}

async fn execute_page(
    deps: &GroupCalendarDeps,
    scope: &RuntimeAuthScopeSnapshot,
    request: HttpApiRequestInput,
) -> Result<CalendarPage> {
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db.as_ref())
        .await?;
    ensure_scope_matches(&deps.auth_scope, scope)?;
    let response = ApiJsonResponse {
        status: response.status,
        json: serde_json::from_str::<Value>(&response.data)?,
    };
    if !(200..300).contains(&response.status) || response.has_error_field() {
        return Err(Error::Custom(format!(
            "Group calendar request failed: {}",
            response.error_message_or("VRChat API request failed")
        )));
    }
    Ok(page_from_payload(&response.json))
}

async fn fetch_group_profiles(
    deps: &GroupCalendarDeps,
    scope: &RuntimeAuthScopeSnapshot,
    group_ids: &[String],
) -> HashMap<String, Value> {
    let mut profiles = HashMap::new();
    let mut missing = Vec::new();
    for group_id in group_ids {
        if let Some(profile) = cached_group_profile(scope.generation, group_id).await {
            profiles.insert(group_id.clone(), profile);
        } else {
            missing.push(group_id.clone());
        }
    }
    let fetched = stream::iter(missing)
        .map(|group_id| fetch_group_profile(deps, scope, group_id))
        .buffer_unordered(GROUP_PROFILE_CONCURRENCY)
        .filter_map(|profile| async move { profile })
        .collect::<HashMap<_, _>>()
        .await;
    profiles.extend(fetched);
    profiles
}

async fn fetch_group_profile(
    deps: &GroupCalendarDeps,
    scope: &RuntimeAuthScopeSnapshot,
    group_id: String,
) -> Option<(String, Value)> {
    let key = GroupProfileCacheKey {
        auth_scope_generation: scope.generation,
        group_id: group_id.clone(),
    };
    let profile = cached_or_fetch_group_profile(group_profile_cache(), key, {
        let deps = deps.clone();
        let scope = scope.clone();
        let request_group_id = group_id.clone();
        async move {
            if !deps.auth_scope.snapshot().generation_matches(&scope) {
                return None;
            }
            let _permit = GROUP_PROFILE_SEMAPHORE
                .get_or_init(|| Semaphore::new(GROUP_PROFILE_CONCURRENCY))
                .acquire()
                .await
                .ok()?;
            if !deps.auth_scope.snapshot().generation_matches(&scope) {
                return None;
            }
            let (_, request) =
                profile_get_input(scope.endpoint.clone(), request_group_id, false).ok()?;
            let response = deps
                .web
                .execute_api(request, ApiScope::Vrchat, deps.db.as_ref())
                .await
                .ok()?;
            if !deps.auth_scope.snapshot().generation_matches(&scope) {
                return None;
            }
            let response = ApiJsonResponse {
                status: response.status,
                json: serde_json::from_str::<Value>(&response.data).ok()?,
            };
            if !(200..300).contains(&response.status)
                || response.has_error_field()
                || !response.json.is_object()
            {
                return None;
            }
            Some(response.json)
        }
    })
    .await?;
    Some((group_id, profile))
}

async fn cached_or_fetch_group_profile(
    cache: &Cache<GroupProfileCacheKey, Value>,
    key: GroupProfileCacheKey,
    request: impl Future<Output = Option<Value>>,
) -> Option<Value> {
    cache.optionally_get_with(key, request).await
}

fn group_profile_cache() -> &'static Cache<GroupProfileCacheKey, Value> {
    GROUP_PROFILE_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(GROUP_PROFILE_CACHE_CAPACITY)
            .time_to_live(GROUP_PROFILE_CACHE_TTL)
            .build()
    })
}

async fn cached_group_profile(auth_scope_generation: u64, group_id: &str) -> Option<Value> {
    group_profile_cache()
        .get(&GroupProfileCacheKey {
            auth_scope_generation,
            group_id: group_id.to_string(),
        })
        .await
}

#[cfg(test)]
fn rows_from_payload(payload: &Value) -> Vec<Value> {
    page_from_payload(payload).rows
}

fn page_from_payload(payload: &Value) -> CalendarPage {
    if let Some(rows) = payload.as_array() {
        return CalendarPage {
            rows: rows.clone(),
            has_next: None,
        };
    }
    let wrapped = payload.get("json");
    let rows = payload
        .get("results")
        .and_then(Value::as_array)
        .or_else(|| wrapped.and_then(Value::as_array))
        .or_else(|| wrapped?.get("results")?.as_array())
        .cloned()
        .unwrap_or_default();
    let has_next = payload
        .get("hasNext")
        .or_else(|| wrapped.and_then(|value| value.get("hasNext")))
        .and_then(Value::as_bool);
    CalendarPage { rows, has_next }
}

fn collect_group_ids(rows: &[&Value]) -> Vec<String> {
    let mut seen = HashSet::new();
    rows.iter()
        .filter_map(|row| group_id(row))
        .filter(|group_id| seen.insert(group_id.clone()))
        .collect()
}

fn embedded_group_profiles(rows: &[&Value]) -> HashMap<String, Value> {
    rows.iter()
        .filter_map(|row| {
            let group = row.get("group")?.as_object()?;
            let group_id = group
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|group_id| !group_id.is_empty())?;
            Some((group_id.to_string(), Value::Object(group.clone())))
        })
        .collect()
}

fn group_id(row: &Value) -> Option<String> {
    row.get("ownerId")
        .and_then(Value::as_str)
        .or_else(|| row.get("groupId").and_then(Value::as_str))
        .or_else(|| {
            row.get("group")
                .and_then(|group| group.get("id"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|group_id| !group_id.is_empty())
        .map(str::to_string)
}

fn event_id(row: &Value) -> Option<String> {
    row.get("eventId")
        .or_else(|| row.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|event_id| !event_id.is_empty())
        .map(str::to_string)
}

fn require_active_scope(auth_scope: &RuntimeAuthScope) -> Result<RuntimeAuthScopeSnapshot> {
    crate::scope_gate::require_active_scope(auth_scope, "Group calendar snapshot")
}

fn ensure_scope_matches(
    auth_scope: &RuntimeAuthScope,
    expected: &RuntimeAuthScopeSnapshot,
) -> Result<()> {
    crate::scope_gate::ensure_scope_matches(auth_scope, expected, "Group calendar")
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn group_profile_cache_coalesces_the_same_in_flight_request() {
        let cache = Cache::builder()
            .max_capacity(GROUP_PROFILE_CACHE_CAPACITY)
            .time_to_live(GROUP_PROFILE_CACHE_TTL)
            .build();
        let key = GroupProfileCacheKey {
            auth_scope_generation: u64::MAX,
            group_id: "grp_in_flight_test".into(),
        };
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let first_calls = Arc::clone(&factory_calls);
        let first = cached_or_fetch_group_profile(&cache, key.clone(), async move {
            first_calls.fetch_add(1, Ordering::Relaxed);
            tokio::task::yield_now().await;
            Some(json!({ "id": "grp_in_flight_test" }))
        });
        let second_calls = Arc::clone(&factory_calls);
        let second = cached_or_fetch_group_profile(&cache, key, async move {
            second_calls.fetch_add(1, Ordering::Relaxed);
            Some(json!({ "id": "unexpected" }))
        });
        let (first, second) = tokio::join!(first, second);

        assert_eq!(factory_calls.load(Ordering::Relaxed), 1);
        assert_eq!(first, second);
    }

    #[test]
    fn rows_support_array_and_wrapped_page_shapes() {
        assert_eq!(rows_from_payload(&json!([{"id": "evt_1"}])).len(), 1);
        assert_eq!(
            rows_from_payload(&json!({"results": [{"id": "evt_1"}]})).len(),
            1
        );
        let wrapped = page_from_payload(&json!({
            "json": {
                "results": [{"id": "evt_1"}],
                "hasNext": false
            }
        }));
        assert_eq!(wrapped.rows.len(), 1);
        assert_eq!(wrapped.has_next, Some(false));
        assert!(rows_from_payload(&json!({"results": null})).is_empty());
    }

    #[test]
    fn group_ids_are_deduplicated_across_event_shapes() {
        let rows = [
            json!({"ownerId": "grp_1"}),
            json!({"group": {"id": "grp_1", "name": "One"}}),
            json!({"group": {"id": "grp_2"}}),
        ];
        let refs = rows.iter().collect::<Vec<_>>();
        assert_eq!(collect_group_ids(&refs), vec!["grp_1", "grp_2"]);
    }
}
