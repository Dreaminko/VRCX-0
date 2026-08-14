use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::de::{IgnoredAny, SeqAccess, Visitor};
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;
use vrcx_0_application_core::vrchat_api::avatars::{
    avatar_list_by_user_get_input, AvatarListByUserGetInput,
};
use vrcx_0_application_core::vrchat_api::favorites::{
    favorite_groups_get_input, favorite_worlds_get_input,
};
use vrcx_0_application_core::vrchat_api::groups::user_groups_get_input;
use vrcx_0_application_core::vrchat_api::users::user_mutual_counts_get_input;
use vrcx_0_application_core::vrchat_api::worlds::world_list_by_user_get_input;
use vrcx_0_application_core::{RuntimeAuthScope, RuntimeAuthScopeSnapshot, WebClient};
use vrcx_0_integrations::external_api::{self, ExternalApiScope, ExternalHttpRequestInput};
use vrcx_0_persistence::{config, DatabaseService};
use vrcx_0_vrchat_client::http_api::{ApiJsonResponse, ApiScope, HttpApiRequestInput};

use crate::{Error, Result};

const MAX_PROFILE_PAGES: usize = 50;
const TAB_COUNTS_CACHE_CAPACITY: u64 = 32;
const TAB_COUNTS_CACHE_TTL: Duration = Duration::from_secs(10 * 60);
const WORLD_PAGE_SIZE: usize = 100;
const WORLD_MAX_OFFSET: i64 = ((MAX_PROFILE_PAGES - 1) * WORLD_PAGE_SIZE) as i64;
const FAVORITE_GROUP_PAGE_SIZE: usize = 50;
const FAVORITE_WORLD_PAGE_SIZE: usize = 300;
const FAVORITE_WORLD_MAX_OFFSET: i64 = ((MAX_PROFILE_PAGES - 1) * FAVORITE_WORLD_PAGE_SIZE) as i64;
const MY_AVATAR_PAGE_SIZE: usize = 50;
const MY_AVATAR_MAX_OFFSET: i64 = 5_000;
const DEFAULT_AVATAR_PROVIDER: &str = "https://api.avtrdb.com/v3/avatar/search/vrcx";
const AVATAR_PROVIDER_ENABLED_KEY: &str = "avatarRemoteDatabase";
const AVATAR_PROVIDER_LIST_KEY: &str = "VRCX_avatarRemoteDatabaseProviderList";
const AVATAR_PROVIDER_SELECTED_KEY: &str = "VRCX_avatarRemoteDatabaseProvider";
const VRCX_ID_KEY: &str = "id";

#[derive(Clone)]
pub struct UserDialogTabCountsDeps {
    pub db: Arc<DatabaseService>,
    pub web: Arc<WebClient>,
    pub auth_scope: RuntimeAuthScope,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct UserDialogTabCountsCacheKey {
    auth_generation: u64,
    current_user_id: String,
    endpoint: String,
    target_user_id: String,
    avatar_release_status: String,
    avatar_provider: String,
    include_mutual_friends: bool,
}

#[derive(Clone)]
pub struct UserDialogTabCountsRuntime {
    cache: Cache<UserDialogTabCountsCacheKey, Arc<UserDialogTabCountsOutput>>,
}

impl UserDialogTabCountsRuntime {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(TAB_COUNTS_CACHE_CAPACITY)
                .time_to_live(TAB_COUNTS_CACHE_TTL)
                .build(),
        }
    }

    async fn resolve<F, Fut>(
        &self,
        key: UserDialogTabCountsCacheKey,
        force: bool,
        include_mutual_friends: bool,
        load: F,
    ) -> Result<UserDialogTabCountsOutput>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<UserDialogTabCountsOutput>>,
    {
        if force {
            self.cache.invalidate(&key).await;
        }
        let counts = self
            .cache
            .try_get_with(key.clone(), async move { load().await.map(Arc::new) })
            .await
            .map_err(|error| Error::Custom(error.to_string()))?;
        if !counts.is_complete(include_mutual_friends) {
            self.cache.invalidate(&key).await;
        }
        Ok((*counts).clone())
    }
}

impl Default for UserDialogTabCountsRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn get_user_dialog_tab_counts(
    runtime: &UserDialogTabCountsRuntime,
    deps: UserDialogTabCountsDeps,
    input: UserDialogTabCountsInput,
) -> Result<UserDialogTabCountsOutput> {
    let scope = require_active_scope(&deps.auth_scope)?;
    let target_user_id = input.user_id.trim().to_string();
    if target_user_id.is_empty() {
        return Err(Error::Custom(
            "User dialog tab counts require a user id.".into(),
        ));
    }
    let avatar_release_status = match input.avatar_release_status.trim() {
        "" => "all".to_string(),
        value => value.to_string(),
    };
    let avatar_provider = if target_user_id == scope.current_user_id {
        Ok(None)
    } else {
        selected_avatar_provider(deps.db.as_ref())
    };
    let avatar_provider_key = match &avatar_provider {
        Ok(Some(provider)) => provider.clone(),
        Ok(None) => "disabled".into(),
        Err(_) => "config-error".into(),
    };
    let key = UserDialogTabCountsCacheKey {
        auth_generation: scope.generation,
        current_user_id: scope.current_user_id.clone(),
        endpoint: scope.endpoint.clone(),
        target_user_id: target_user_id.clone(),
        avatar_release_status: avatar_release_status.clone(),
        avatar_provider: avatar_provider_key,
        include_mutual_friends: input.include_mutual_friends,
    };
    runtime
        .resolve(
            key,
            input.force,
            input.include_mutual_friends,
            move || async move {
                load_user_dialog_tab_counts(
                    deps,
                    scope,
                    target_user_id,
                    avatar_release_status,
                    avatar_provider,
                    input.include_mutual_friends,
                )
                .await
            },
        )
        .await
}

async fn load_user_dialog_tab_counts(
    deps: UserDialogTabCountsDeps,
    scope: RuntimeAuthScopeSnapshot,
    target_user_id: String,
    avatar_release_status: String,
    avatar_provider: Result<Option<String>>,
    include_mutual_friends: bool,
) -> Result<UserDialogTabCountsOutput> {
    let mutual_friends = if include_mutual_friends {
        Some(count_mutual_friends(&deps, &scope, &target_user_id).await)
    } else {
        None
    };
    let groups = count_groups(&deps, &scope, &target_user_id).await;
    let worlds = count_worlds(&deps, &scope, &target_user_id).await;
    let favorite_worlds = count_favorite_worlds(&deps, &scope, &target_user_id).await;
    let avatars = count_avatars(
        &deps,
        &scope,
        &target_user_id,
        &avatar_release_status,
        avatar_provider,
    )
    .await;
    Ok(counts_from_results(
        mutual_friends,
        groups,
        worlds,
        favorite_worlds,
        avatars,
    ))
}

async fn count_mutual_friends(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
) -> Result<usize> {
    let (_, request) = user_mutual_counts_get_input(scope.endpoint.clone(), target_user_id.into())?;
    let payload = execute_vrchat_payload(deps, scope, request, "mutual friends").await?;
    let value = serde_json::from_str::<Value>(&payload)?;
    value
        .get("friends")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| Error::Custom("Mutual friend count response is invalid.".into()))
}

async fn count_groups(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
) -> Result<usize> {
    let (_, request) = user_groups_get_input(scope.endpoint.clone(), target_user_id.into())?;
    let payload = execute_vrchat_payload(deps, scope, request, "groups").await?;
    json_array_len(&payload)
}

async fn count_worlds(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
) -> Result<usize> {
    let release_status = if target_user_id == scope.current_user_id {
        "all"
    } else {
        "public"
    };
    count_payload_pages_bounded(
        WORLD_PAGE_SIZE,
        WORLD_MAX_OFFSET,
        |offset| async move {
            let (_, request) = world_list_by_user_get_input(
                scope.endpoint.clone(),
                target_user_id.into(),
                WORLD_PAGE_SIZE as i64,
                offset,
                "updated".into(),
                "descending".into(),
                release_status.into(),
            )?;
            execute_vrchat_payload(deps, scope, request, "worlds").await
        },
        count_all_rows,
    )
    .await
}

async fn count_favorite_worlds(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
) -> Result<usize> {
    let group_names = collect_world_favorite_group_names(deps, scope, target_user_id).await?;
    let mut count = 0;
    for group_name in group_names {
        let result = count_payload_pages_bounded(
            FAVORITE_WORLD_PAGE_SIZE,
            FAVORITE_WORLD_MAX_OFFSET,
            |offset| {
                let group_name = group_name.clone();
                async move {
                    let request = favorite_worlds_get_input(
                        scope.endpoint.clone(),
                        FAVORITE_WORLD_PAGE_SIZE as i64,
                        offset,
                        target_user_id.into(),
                        target_user_id.into(),
                        group_name,
                    );
                    execute_vrchat_payload(deps, scope, request, "favorite worlds").await
                }
            },
            count_all_rows,
        )
        .await;
        match result {
            Ok(group_count) => count += group_count,
            Err(error) => tracing::debug!(%error, "favorite world count source failed"),
        }
    }
    Ok(count)
}

async fn collect_world_favorite_group_names(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
) -> Result<Vec<String>> {
    let mut group_names = Vec::new();
    for page in 0..MAX_PROFILE_PAGES {
        let request = favorite_groups_get_input(
            scope.endpoint.clone(),
            FAVORITE_GROUP_PAGE_SIZE as i64,
            (page * FAVORITE_GROUP_PAGE_SIZE) as i64,
            target_user_id.into(),
        );
        let payload = execute_vrchat_payload(deps, scope, request, "favorite groups").await?;
        let page_len = json_array_len(&payload)?;
        group_names.extend(world_favorite_group_names(&payload)?);
        if page_len < FAVORITE_GROUP_PAGE_SIZE || page + 1 == MAX_PROFILE_PAGES {
            return Ok(group_names);
        }
    }
    Ok(group_names)
}

async fn count_avatars(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    target_user_id: &str,
    release_status: &str,
    avatar_provider: Result<Option<String>>,
) -> Result<usize> {
    if target_user_id == scope.current_user_id {
        return count_payload_pages_bounded(
            MY_AVATAR_PAGE_SIZE,
            MY_AVATAR_MAX_OFFSET,
            |offset| async move {
                let (_, request) = avatar_list_by_user_get_input(AvatarListByUserGetInput {
                    endpoint: scope.endpoint.clone(),
                    user_id: String::new(),
                    user: "me".into(),
                    n: MY_AVATAR_PAGE_SIZE as i64,
                    offset,
                    sort: "updated".into(),
                    order: "descending".into(),
                    release_status: "all".into(),
                })?;
                execute_vrchat_payload(deps, scope, request, "my avatars").await
            },
            |payload| {
                let page_len = json_array_len(payload)?;
                Ok((page_len, count_my_avatars(payload, release_status)?))
            },
        )
        .await;
    }

    let Some(provider) = avatar_provider? else {
        return Ok(0);
    };
    let request = external_avatar_search_request(deps.db.as_ref(), &provider, target_user_id)?;
    let request = external_api::build_web_execute_request(request, ExternalApiScope::AvatarSearch)
        .map_err(|error| Error::Custom(error.to_string()))?;
    let (status, payload) = deps.web.execute_external(request).await?;
    if status != 200 {
        return Err(Error::Custom(format!(
            "Avatar search count request failed with status {status}."
        )));
    }
    count_target_avatars(&payload, target_user_id)
}

async fn execute_vrchat_payload(
    deps: &UserDialogTabCountsDeps,
    scope: &RuntimeAuthScopeSnapshot,
    request: HttpApiRequestInput,
    source: &str,
) -> Result<String> {
    ensure_scope_matches(&deps.auth_scope, scope)?;
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db.as_ref())
        .await?;
    ensure_scope_matches(&deps.auth_scope, scope)?;
    if response.status >= 400 || response.data.trim_start().starts_with('{') {
        let parsed = ApiJsonResponse::parse(response.status, &response.data);
        if parsed.is_failure() {
            return Err(Error::Custom(format!(
                "User dialog {source} count request failed: {}",
                parsed.error_message_or("VRChat API request failed")
            )));
        }
    }
    Ok(response.data)
}

fn require_active_scope(auth_scope: &RuntimeAuthScope) -> Result<RuntimeAuthScopeSnapshot> {
    crate::scope_gate::require_active_scope(auth_scope, "User dialog tab counts")
}

fn ensure_scope_matches(
    auth_scope: &RuntimeAuthScope,
    expected: &RuntimeAuthScopeSnapshot,
) -> Result<()> {
    crate::scope_gate::ensure_scope_matches(auth_scope, expected, "User dialog tab counts")
}

fn selected_avatar_provider(db: &DatabaseService) -> Result<Option<String>> {
    if !config::get_bool(db, AVATAR_PROVIDER_ENABLED_KEY, true)? {
        return Ok(None);
    }
    let configured = config::get_json(
        db,
        AVATAR_PROVIDER_LIST_KEY,
        serde_json::json!([DEFAULT_AVATAR_PROVIDER]),
    )?;
    let mut provider_values = match configured {
        Value::Array(values) => values,
        _ => vec![Value::String(DEFAULT_AVATAR_PROVIDER.into())],
    };
    let selected = config::get_string(db, AVATAR_PROVIDER_SELECTED_KEY, "")?
        .trim()
        .to_string();
    if !selected.is_empty()
        && !provider_values
            .iter()
            .any(|value| value.as_str() == Some(selected.as_str()))
    {
        provider_values.push(Value::String(selected.clone()));
    }

    let mut seen = HashSet::new();
    let providers = provider_values
        .into_iter()
        .filter_map(|value| value.as_str().and_then(normalize_avatar_provider))
        .filter(|provider| seen.insert(provider.clone()))
        .collect::<Vec<_>>();
    let selected = normalize_avatar_provider(&selected)
        .filter(|provider| providers.contains(provider))
        .or_else(|| providers.first().cloned());
    Ok(selected)
}

fn normalize_avatar_provider(value: &str) -> Option<String> {
    match value.trim() {
        "" | "https://avtr.just-h.party/vrcx_search.php" => None,
        "https://api.avtrdb.com/v1/avatar/search/vrcx"
        | "https://api.avtrdb.com/v2/avatar/search/vrcx" => Some(DEFAULT_AVATAR_PROVIDER.into()),
        value => Some(value.to_string()),
    }
}

fn external_avatar_search_request(
    db: &DatabaseService,
    provider: &str,
    target_user_id: &str,
) -> Result<ExternalHttpRequestInput> {
    let mut url = Url::parse(provider)
        .map_err(|error| Error::Custom(format!("Invalid avatar provider URL: {error}")))?;
    let retained_query = url
        .query_pairs()
        .filter(|(key, _)| key != "search" && key != "n")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.extend_pairs(retained_query);
        query.append_pair("search", target_user_id);
        query.append_pair("n", "5000");
    }

    let mut vrcx_id = config::get_string(db, VRCX_ID_KEY, "")?.trim().to_string();
    if vrcx_id.is_empty() {
        vrcx_id = Uuid::new_v4().to_string();
        config::set_string(db, VRCX_ID_KEY, &vrcx_id)?;
    }
    Ok(external_api::avatar_search_get_input(
        url.as_str(),
        &vrcx_id,
    ))
}

fn json_array_len(payload: &str) -> Result<usize> {
    struct ArrayLenVisitor;

    impl<'de> Visitor<'de> for ArrayLenVisitor {
        type Value = usize;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON array")
        }

        fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut count = 0;
            while sequence.next_element::<IgnoredAny>()?.is_some() {
                count += 1;
            }
            Ok(count)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(payload);
    Ok(deserializer.deserialize_seq(ArrayLenVisitor)?)
}

fn count_all_rows(payload: &str) -> Result<(usize, usize)> {
    let page_len = json_array_len(payload)?;
    Ok((page_len, page_len))
}

#[derive(Clone, Debug, Default, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserDialogTabCountsInput {
    pub user_id: String,
    #[serde(default)]
    pub avatar_release_status: String,
    #[serde(default)]
    pub include_mutual_friends: bool,
    #[serde(default)]
    pub force: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserDialogTabCountsOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutual_friends: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worlds: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_worlds: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatars: Option<usize>,
}

impl UserDialogTabCountsOutput {
    fn is_complete(&self, include_mutual_friends: bool) -> bool {
        (!include_mutual_friends || self.mutual_friends.is_some())
            && self.groups.is_some()
            && self.worlds.is_some()
            && self.favorite_worlds.is_some()
            && self.avatars.is_some()
    }
}

fn counts_from_results(
    mutual_friends: Option<Result<usize>>,
    groups: Result<usize>,
    worlds: Result<usize>,
    favorite_worlds: Result<usize>,
    avatars: Result<usize>,
) -> UserDialogTabCountsOutput {
    UserDialogTabCountsOutput {
        mutual_friends: mutual_friends.and_then(|result| resolved_count("mutual friends", result)),
        groups: resolved_count("groups", groups),
        worlds: resolved_count("worlds", worlds),
        favorite_worlds: resolved_count("favorite worlds", favorite_worlds),
        avatars: resolved_count("avatars", avatars),
    }
}

fn resolved_count(source: &str, result: Result<usize>) -> Option<usize> {
    match result {
        Ok(count) => Some(count),
        Err(error) => {
            tracing::debug!(%error, source, "user dialog tab count source failed");
            None
        }
    }
}

#[derive(Deserialize)]
struct FavoriteGroupCountRow {
    #[serde(default)]
    name: Value,
    #[serde(default, rename = "type")]
    kind: Value,
}

fn world_favorite_group_names(payload: &str) -> Result<Vec<String>> {
    let rows = serde_json::from_str::<Vec<FavoriteGroupCountRow>>(payload)?;
    Ok(rows
        .into_iter()
        .filter(|row| row.kind.as_str() == Some("world"))
        .filter_map(|row| {
            row.name
                .as_str()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .collect())
}

#[derive(Deserialize)]
struct AvatarSearchCountRow {
    #[serde(
        default,
        alias = "Id",
        alias = "_id",
        alias = "avatarId",
        alias = "AvatarId"
    )]
    id: Value,
    #[serde(default, rename = "authorId", alias = "AuthorId", alias = "author_id")]
    author_id: Value,
}

fn count_target_avatars(payload: &str, user_id: &str) -> Result<usize> {
    struct TargetAvatarCountVisitor<'a> {
        user_id: &'a str,
    }

    impl<'de> Visitor<'de> for TargetAvatarCountVisitor<'_> {
        type Value = usize;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON avatar array")
        }

        fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut avatar_ids = HashSet::new();
            while let Some(row) = sequence.next_element::<AvatarSearchCountRow>()? {
                let author_id = row.author_id.as_str().map(str::trim).unwrap_or_default();
                if author_id != self.user_id {
                    continue;
                }
                let avatar_id = row.id.as_str().map(str::trim).unwrap_or_default();
                if !avatar_id.is_empty() {
                    avatar_ids.insert(avatar_id.to_string());
                }
            }
            Ok(avatar_ids.len())
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(payload);
    Ok(deserializer.deserialize_seq(TargetAvatarCountVisitor { user_id })?)
}

#[derive(Deserialize)]
struct MyAvatarCountRow {
    #[serde(default, rename = "releaseStatus")]
    release_status: Value,
}

fn count_my_avatars(payload: &str, release_status: &str) -> Result<usize> {
    let rows = serde_json::from_str::<Vec<MyAvatarCountRow>>(payload)?;
    if release_status == "all" {
        return Ok(rows.len());
    }
    Ok(rows
        .iter()
        .filter(|row| row.release_status.as_str() == Some(release_status))
        .count())
}

async fn count_payload_pages_bounded<F, Fut, C>(
    page_size: usize,
    max_offset: i64,
    fetch_page: F,
    count_page: C,
) -> Result<usize>
where
    F: FnMut(i64) -> Fut,
    Fut: Future<Output = Result<String>>,
    C: Fn(&str) -> Result<(usize, usize)>,
{
    let mut fetch_page = fetch_page;
    let mut count = 0;
    let mut offset = 0;
    loop {
        let payload = fetch_page(offset).await?;
        let (page_len, selected_count) = count_page(&payload)?;
        count += selected_count;
        if page_len < page_size || offset >= max_offset {
            return Ok(count);
        }
        offset += page_size as i64;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn keeps_successful_counts_when_one_source_fails() {
        let counts = counts_from_results(
            Some(Ok(7)),
            Ok(12),
            Err(Error::Custom("worlds failed".into())),
            Ok(34),
            Ok(56),
        );

        assert_eq!(
            counts,
            UserDialogTabCountsOutput {
                mutual_friends: Some(7),
                groups: Some(12),
                worlds: None,
                favorite_worlds: Some(34),
                avatars: Some(56),
            }
        );
    }

    #[test]
    fn favorite_group_parser_only_keeps_named_world_groups() {
        let payload = serde_json::json!([
            { "name": "worlds1", "type": "world" },
            { "name": "avatars1", "type": "avatar" },
            { "name": "  worlds2  ", "type": "world" },
            { "name": "", "type": "world" }
        ])
        .to_string();

        assert_eq!(
            world_favorite_group_names(&payload).unwrap(),
            vec!["worlds1".to_string(), "worlds2".to_string()]
        );
    }

    #[test]
    fn avatar_count_deduplicates_ids_and_keeps_only_the_target_author() {
        let payload = serde_json::json!([
            { "id": "avtr_1", "authorId": "usr_target" },
            { "Id": "avtr_1", "AuthorId": "usr_target" },
            { "avatarId": "avtr_2", "author_id": "usr_target" },
            { "id": "avtr_3", "authorId": "usr_other" }
        ])
        .to_string();

        assert_eq!(count_target_avatars(&payload, "usr_target").unwrap(), 2);
    }

    #[test]
    fn my_avatar_count_matches_the_selected_release_status() {
        let payload = serde_json::json!([
            { "id": "avtr_public", "releaseStatus": "public" },
            { "id": "avtr_private", "releaseStatus": "private" },
            { "id": "avtr_public_2", "releaseStatus": "public" }
        ])
        .to_string();

        assert_eq!(count_my_avatars(&payload, "all").unwrap(), 3);
        assert_eq!(count_my_avatars(&payload, "public").unwrap(), 2);
        assert_eq!(count_my_avatars(&payload, "private").unwrap(), 1);
    }

    #[tokio::test]
    async fn page_counter_stops_on_a_short_page_and_uses_monotonic_offsets() {
        let payloads = Arc::new(Mutex::new(VecDeque::from([
            Ok(serde_json::json!([{ "id": 1 }, { "id": 2 }]).to_string()),
            Ok(serde_json::json!([{ "id": 3 }]).to_string()),
        ])));
        let offsets = Arc::new(Mutex::new(Vec::new()));

        let count = count_payload_pages_bounded(
            2,
            100,
            {
                let payloads = Arc::clone(&payloads);
                let offsets = Arc::clone(&offsets);
                move |offset| {
                    offsets.lock().unwrap().push(offset);
                    let payload = payloads.lock().unwrap().pop_front().unwrap();
                    std::future::ready(payload)
                }
            },
            count_all_rows,
        )
        .await
        .unwrap();

        assert_eq!(count, 3);
        assert_eq!(*offsets.lock().unwrap(), vec![0, 2]);
    }

    #[tokio::test]
    async fn bounded_page_counter_includes_the_page_at_the_maximum_offset() {
        let offsets = Arc::new(Mutex::new(Vec::new()));

        let count = count_payload_pages_bounded(
            2,
            4,
            {
                let offsets = Arc::clone(&offsets);
                move |offset| {
                    offsets.lock().unwrap().push(offset);
                    std::future::ready(Ok(
                        serde_json::json!([{ "id": offset }, { "id": offset + 1 }]).to_string(),
                    ))
                }
            },
            count_all_rows,
        )
        .await
        .unwrap();

        assert_eq!(count, 6);
        assert_eq!(*offsets.lock().unwrap(), vec![0, 2, 4]);
    }

    #[tokio::test]
    async fn runtime_reuses_complete_counts_until_force_refresh() {
        let runtime = UserDialogTabCountsRuntime::new();
        let key = UserDialogTabCountsCacheKey {
            auth_generation: 7,
            current_user_id: "usr_self".into(),
            endpoint: "https://api.example.test".into(),
            target_user_id: "usr_target".into(),
            avatar_release_status: "public".into(),
            avatar_provider: "provider-a".into(),
            include_mutual_friends: true,
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let expected = UserDialogTabCountsOutput {
            mutual_friends: Some(5),
            groups: Some(1),
            worlds: Some(2),
            favorite_worlds: Some(3),
            avatars: Some(4),
        };

        for force in [false, false, true] {
            let calls = Arc::clone(&calls);
            let loaded_counts = expected.clone();
            assert_eq!(
                runtime
                    .resolve(key.clone(), force, true, move || async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        Ok(loaded_counts)
                    })
                    .await
                    .unwrap(),
                expected
            );
        }

        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
