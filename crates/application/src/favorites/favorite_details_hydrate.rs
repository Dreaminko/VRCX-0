use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use futures_util::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;
use vrcx_0_core::json::RawJson;
use vrcx_0_persistence::{
    avatars::{avatar_cache_existing_ids, avatar_cache_upsert},
    worlds::{world_cache_get_many, world_cache_upsert},
    DatabaseService,
};
use vrcx_0_vrchat_client::{
    favorites::{favorite_avatars_get_input, favorite_worlds_get_input},
    http_api::{normalize_text, ApiScope, HttpApiRequestInput},
    worlds::world_get_input,
};

use crate::{Error, Result, RuntimeAuthScope, RuntimeAuthScopeSnapshot, WebClient};

use super::cache_policy::{
    cache_entry_from_entity, cache_write_decision, entity_id, release_status, CacheWriteDecision,
    FavoriteCacheKind,
};

const FAVORITE_DETAILS_PAGE_SIZE: i64 = 300;
const FAVORITE_DETAILS_MAX_PAGES: usize = 50;
const FAVORITE_DETAILS_PROBE_CONCURRENCY: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum FavoriteDetailsHydrateKind {
    Avatar,
    World,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteDetailsHydrateInput {
    pub kind: FavoriteDetailsHydrateKind,
    #[serde(default)]
    pub favorite_ids: Vec<String>,
    #[serde(default)]
    pub requested_ids: Vec<String>,
    #[serde(default)]
    pub avatar_tags: Vec<String>,
    #[serde(default)]
    pub refresh_key: String,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteDetailsHydrateOutput {
    pub details_by_id: HashMap<String, RawJson>,
    pub availability_by_id: HashMap<String, String>,
    pub cached_count: u32,
    pub fetched_at: String,
}

struct FavoriteDetailsHydrateDeps<'a> {
    db: &'a DatabaseService,
    web: &'a WebClient,
    auth_scope: &'a RuntimeAuthScope,
    expected_scope: RuntimeAuthScopeSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FavoriteWorldDetailsSnapshotKey {
    current_user_id: String,
    endpoint: String,
    generation: u64,
    favorite_ids: Vec<String>,
    refresh_key: String,
}

#[derive(Clone, Debug)]
struct FavoriteWorldDetailsSnapshot {
    key: FavoriteWorldDetailsSnapshotKey,
    details_by_id: HashMap<String, Value>,
    availability_by_id: HashMap<String, String>,
    fetched_at: String,
}

struct FavoriteDetailsRuntimeInner {
    db: Arc<DatabaseService>,
    web: Arc<WebClient>,
    auth_scope: RuntimeAuthScope,
    world_snapshot: Mutex<Option<Arc<FavoriteWorldDetailsSnapshot>>>,
    world_sync_gate: AsyncMutex<()>,
}

#[derive(Clone)]
pub struct FavoriteDetailsRuntime {
    inner: Arc<FavoriteDetailsRuntimeInner>,
}

impl FavoriteDetailsRuntime {
    pub fn new(
        db: Arc<DatabaseService>,
        web: Arc<WebClient>,
        auth_scope: RuntimeAuthScope,
    ) -> Self {
        Self {
            inner: Arc::new(FavoriteDetailsRuntimeInner {
                db,
                web,
                auth_scope,
                world_snapshot: Mutex::new(None),
                world_sync_gate: AsyncMutex::new(()),
            }),
        }
    }

    pub async fn hydrate(
        &self,
        input: FavoriteDetailsHydrateInput,
        expected_scope: RuntimeAuthScopeSnapshot,
    ) -> Result<FavoriteDetailsHydrateOutput> {
        match input.kind {
            FavoriteDetailsHydrateKind::Avatar => self.hydrate_avatar(input, expected_scope).await,
            FavoriteDetailsHydrateKind::World => {
                let requested_ids = input.requested_ids.clone();
                let (snapshot, cached_count) = self.world_snapshot(input, expected_scope).await?;
                Ok(project_world_snapshot(
                    &snapshot,
                    &requested_ids,
                    cached_count,
                ))
            }
        }
    }

    async fn hydrate_avatar(
        &self,
        input: FavoriteDetailsHydrateInput,
        expected_scope: RuntimeAuthScopeSnapshot,
    ) -> Result<FavoriteDetailsHydrateOutput> {
        let deps = FavoriteDetailsHydrateDeps {
            db: self.inner.db.as_ref(),
            web: self.inner.web.as_ref(),
            auth_scope: &self.inner.auth_scope,
            expected_scope,
        };
        let entities = fetch_favorite_avatar_entities(&deps, &input.avatar_tags).await?;
        let details_by_id = filter_details_by_id(entities, &input.favorite_ids);
        let cached_count = persist_avatar_details(deps.db, &details_by_id);
        Ok(project_details(
            &details_by_id,
            &HashMap::new(),
            &input.requested_ids,
            cached_count,
            Utc::now().to_rfc3339(),
        ))
    }

    async fn world_snapshot(
        &self,
        input: FavoriteDetailsHydrateInput,
        expected_scope: RuntimeAuthScopeSnapshot,
    ) -> Result<(Arc<FavoriteWorldDetailsSnapshot>, u32)> {
        let key = world_snapshot_key(&expected_scope, &input.favorite_ids, &input.refresh_key);
        if let Some(snapshot) = self.cached_world_snapshot(&key) {
            return Ok((snapshot, 0));
        }

        let _guard = self.inner.world_sync_gate.lock().await;
        if let Some(snapshot) = self.cached_world_snapshot(&key) {
            return Ok((snapshot, 0));
        }

        let deps = FavoriteDetailsHydrateDeps {
            db: self.inner.db.as_ref(),
            web: self.inner.web.as_ref(),
            auth_scope: &self.inner.auth_scope,
            expected_scope,
        };
        let entities = fetch_favorite_world_entities(&deps).await?;
        let mut details_by_id = filter_details_by_id(entities, &key.favorite_ids);
        let availability_by_id =
            probe_missing_world_details(&deps, &key.favorite_ids, &mut details_by_id).await?;
        let cached_count = persist_world_details(deps.db, &details_by_id);
        ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
        let snapshot = Arc::new(FavoriteWorldDetailsSnapshot {
            key,
            details_by_id,
            availability_by_id,
            fetched_at: Utc::now().to_rfc3339(),
        });
        *self
            .inner
            .world_snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone());
        Ok((snapshot, cached_count))
    }

    fn cached_world_snapshot(
        &self,
        key: &FavoriteWorldDetailsSnapshotKey,
    ) -> Option<Arc<FavoriteWorldDetailsSnapshot>> {
        self.inner
            .world_snapshot
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .filter(|snapshot| snapshot.key == *key)
            .cloned()
    }
}

fn world_snapshot_key(
    expected_scope: &RuntimeAuthScopeSnapshot,
    favorite_ids: &[String],
    refresh_key: &str,
) -> FavoriteWorldDetailsSnapshotKey {
    let mut favorite_ids = normalize_ids(favorite_ids);
    favorite_ids.sort_unstable();
    FavoriteWorldDetailsSnapshotKey {
        current_user_id: expected_scope.current_user_id.clone(),
        endpoint: expected_scope.endpoint.clone(),
        generation: expected_scope.generation,
        favorite_ids,
        refresh_key: refresh_key.trim().to_string(),
    }
}

fn project_world_snapshot(
    snapshot: &FavoriteWorldDetailsSnapshot,
    requested_ids: &[String],
    cached_count: u32,
) -> FavoriteDetailsHydrateOutput {
    project_details(
        &snapshot.details_by_id,
        &snapshot.availability_by_id,
        requested_ids,
        cached_count,
        snapshot.fetched_at.clone(),
    )
}

fn project_details(
    details_by_id: &HashMap<String, Value>,
    availability_by_id: &HashMap<String, String>,
    requested_ids: &[String],
    cached_count: u32,
    fetched_at: String,
) -> FavoriteDetailsHydrateOutput {
    let requested = normalize_ids(requested_ids)
        .into_iter()
        .collect::<HashSet<_>>();
    FavoriteDetailsHydrateOutput {
        details_by_id: details_by_id
            .iter()
            .filter(|(id, _)| requested.contains(*id))
            .map(|(id, entity)| (id.clone(), RawJson::from(entity.clone())))
            .collect(),
        availability_by_id: availability_by_id
            .iter()
            .filter(|(id, _)| requested.contains(*id))
            .map(|(id, availability)| (id.clone(), availability.clone()))
            .collect(),
        cached_count,
        fetched_at,
    }
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    ids.iter()
        .map(normalize_text)
        .filter(|id| !id.is_empty())
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

async fn probe_missing_world_details(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    favorite_ids: &[String],
    details_by_id: &mut HashMap<String, Value>,
) -> Result<HashMap<String, String>> {
    let mut availability_by_id = HashMap::new();
    let mut probes = stream::iter(
        missing_world_ids(favorite_ids, details_by_id)
            .into_iter()
            .map(|id| async move {
                let outcome = probe_world(deps, &id).await;
                (id, outcome)
            }),
    )
    .buffer_unordered(FAVORITE_DETAILS_PROBE_CONCURRENCY);
    while let Some((id, outcome)) = probes.next().await {
        match outcome? {
            WorldProbeOutcome::Deleted => {
                availability_by_id.insert(id, "deleted".to_string());
            }
            WorldProbeOutcome::Available(entity, availability) => {
                availability_by_id.insert(id.clone(), availability);
                details_by_id.insert(id, entity);
            }
            WorldProbeOutcome::Failed => {}
        }
    }
    Ok(availability_by_id)
}

async fn probe_world(deps: &FavoriteDetailsHydrateDeps<'_>, id: &str) -> Result<WorldProbeOutcome> {
    let request = match world_get_input(deps.expected_scope.endpoint.clone(), id.to_string()) {
        Ok((_, request)) => request,
        Err(error) => {
            tracing::warn!("failed to build world availability probe for {id}: {error}");
            return Ok(WorldProbeOutcome::Failed);
        }
    };
    match execute_json(deps, request).await {
        Ok((status, payload)) => Ok(classify_world_probe(status, payload)),
        Err(error) => {
            ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
            tracing::warn!("world availability probe failed for {id}: {error}");
            Ok(WorldProbeOutcome::Failed)
        }
    }
}

fn missing_world_ids(
    favorite_ids: &[String],
    details_by_id: &HashMap<String, Value>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    favorite_ids
        .iter()
        .map(normalize_text)
        .filter(|id| !id.is_empty())
        .filter(|id| seen.insert(id.clone()))
        .filter(|id| {
            details_by_id
                .get(id)
                .is_none_or(|entity| !has_displayable_detail(entity))
        })
        .collect()
}

fn has_displayable_detail(entity: &Value) -> bool {
    let display_fields = [
        "name",
        "authorName",
        "thumbnailImageUrl",
        "imageUrl",
        "description",
        "releaseStatus",
    ];
    if display_fields.iter().any(
        |field| matches!(entity.get(*field), Some(Value::String(text)) if !text.trim().is_empty()),
    ) {
        return true;
    }
    matches!(entity.get("tags"), Some(Value::Array(tags)) if !tags.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WorldProbeOutcome {
    Available(Value, String),
    Deleted,
    Failed,
}

fn classify_world_probe(status: i32, payload: Value) -> WorldProbeOutcome {
    if status == 404 {
        return WorldProbeOutcome::Deleted;
    }
    if status >= 400 || payload.get("error").is_some() {
        return WorldProbeOutcome::Failed;
    }
    let availability = if release_status(&payload) == "public" {
        "public"
    } else {
        "private"
    };
    WorldProbeOutcome::Available(payload, availability.to_string())
}

async fn fetch_favorite_world_entities(
    deps: &FavoriteDetailsHydrateDeps<'_>,
) -> Result<Vec<Value>> {
    let mut entities = Vec::new();
    let mut offset = 0_i64;
    for _ in 0..FAVORITE_DETAILS_MAX_PAGES {
        let request = favorite_worlds_get_input(
            deps.expected_scope.endpoint.clone(),
            FAVORITE_DETAILS_PAGE_SIZE,
            offset,
            String::new(),
            String::new(),
            String::new(),
        );
        let rows = execute_page(deps, request, "favorite world detail sync").await?;
        let page_len = rows.len();
        entities.extend(rows);
        if page_len < FAVORITE_DETAILS_PAGE_SIZE as usize {
            break;
        }
        offset += FAVORITE_DETAILS_PAGE_SIZE;
    }
    Ok(entities)
}

async fn fetch_favorite_avatar_entities(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    avatar_tags: &[String],
) -> Result<Vec<Value>> {
    let tags = normalize_avatar_tags(avatar_tags);
    let mut entities = Vec::new();
    let mut seen_ids = HashSet::new();
    for tag in tags {
        let mut offset = 0_i64;
        for _ in 0..FAVORITE_DETAILS_MAX_PAGES {
            let request = favorite_avatars_get_input(
                deps.expected_scope.endpoint.clone(),
                FAVORITE_DETAILS_PAGE_SIZE,
                offset,
                tag.clone(),
            );
            let rows = execute_page(deps, request, "favorite avatar detail sync").await?;
            let page_len = rows.len();
            merge_avatar_rows(rows, &mut seen_ids, &mut entities);
            if page_len < FAVORITE_DETAILS_PAGE_SIZE as usize {
                break;
            }
            offset += FAVORITE_DETAILS_PAGE_SIZE;
        }
    }
    Ok(entities)
}

async fn execute_json(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    request: HttpApiRequestInput,
) -> Result<(i32, Value)> {
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
    let response = deps
        .web
        .execute_api(request, ApiScope::Vrchat, deps.db)
        .await?;
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
    let payload = serde_json::from_str::<Value>(&response.data)
        .unwrap_or_else(|_| Value::String(response.data.clone()));
    Ok((response.status, payload))
}

async fn execute_page(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    request: HttpApiRequestInput,
    action: &str,
) -> Result<Vec<Value>> {
    let (status, payload) = execute_json(deps, request).await?;
    if status >= 400 || payload.get("error").is_some() {
        return Err(Error::Custom(response_error_message(
            &payload, status, action,
        )));
    }
    Ok(payload.as_array().cloned().unwrap_or_default())
}

fn normalize_avatar_tags(avatar_tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let tags = avatar_tags
        .iter()
        .map(normalize_text)
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.clone()))
        .collect::<Vec<_>>();
    if tags.is_empty() {
        vec![String::new()]
    } else {
        tags
    }
}

fn merge_avatar_rows(rows: Vec<Value>, seen_ids: &mut HashSet<String>, entities: &mut Vec<Value>) {
    for row in rows {
        let id = entity_id(&row);
        if id.is_empty() || !seen_ids.insert(id) {
            continue;
        }
        entities.push(row);
    }
}

fn filter_details_by_id(entities: Vec<Value>, favorite_ids: &[String]) -> HashMap<String, Value> {
    let wanted = favorite_ids
        .iter()
        .map(normalize_text)
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>();
    let mut details_by_id = HashMap::new();
    for entity in entities {
        let id = entity_id(&entity);
        if id.is_empty() {
            continue;
        }
        if !wanted.is_empty() && !wanted.contains(&id) {
            continue;
        }
        details_by_id.insert(id, entity);
    }
    details_by_id
}

fn persist_avatar_details(db: &DatabaseService, details_by_id: &HashMap<String, Value>) -> u32 {
    let insert_candidates = details_by_id
        .iter()
        .filter(|(_, entity)| {
            cache_write_decision(FavoriteCacheKind::Avatar, entity)
                == CacheWriteDecision::InsertIfMissing
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let existing_ids: HashSet<String> = if insert_candidates.is_empty() {
        HashSet::new()
    } else {
        match avatar_cache_existing_ids(db, &insert_candidates) {
            Ok(ids) => ids.into_iter().collect(),
            Err(error) => {
                tracing::warn!("failed to read favorite avatar cache: {error}");
                return 0;
            }
        }
    };

    let mut cached_count = 0;
    for (id, entity) in details_by_id {
        let decision = cache_write_decision(FavoriteCacheKind::Avatar, entity);
        if decision == CacheWriteDecision::Skip {
            continue;
        }
        if decision == CacheWriteDecision::InsertIfMissing && existing_ids.contains(id) {
            continue;
        }
        match avatar_cache_upsert(db, cache_entry_from_entity(entity, id)) {
            Ok(_) => cached_count += 1,
            Err(error) => {
                tracing::warn!("failed to cache favorite avatar details for {id}: {error}");
            }
        }
    }
    cached_count
}

fn persist_world_details(db: &DatabaseService, details_by_id: &HashMap<String, Value>) -> u32 {
    let insert_candidates = details_by_id
        .iter()
        .filter(|(_, entity)| {
            cache_write_decision(FavoriteCacheKind::World, entity)
                == CacheWriteDecision::InsertIfMissing
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let existing_ids = if insert_candidates.is_empty() {
        HashSet::new()
    } else {
        match world_cache_get_many(db, &insert_candidates) {
            Ok(rows) => rows.into_iter().map(|row| row.id).collect(),
            Err(error) => {
                tracing::warn!("failed to read favorite world cache: {error}");
                return 0;
            }
        }
    };

    let mut cached_count = 0;
    for (id, entity) in details_by_id {
        match cache_write_decision(FavoriteCacheKind::World, entity) {
            CacheWriteDecision::Skip => continue,
            CacheWriteDecision::InsertIfMissing if existing_ids.contains(id) => continue,
            CacheWriteDecision::InsertIfMissing | CacheWriteDecision::Upsert => {}
        }
        match world_cache_upsert(db, cache_entry_from_entity(entity, id)) {
            Ok(_) => cached_count += 1,
            Err(error) => {
                tracing::warn!("failed to cache favorite world details for {id}: {error}");
            }
        }
    }
    cached_count
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
            "Favorite detail hydrate authentication scope changed.".into(),
        ))
    }
}

fn response_error_message(payload: &Value, status: i32, action: &str) -> String {
    payload
        .get("error")
        .and_then(Value::as_object)
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| payload.get("message").and_then(Value::as_str))
        .map(str::to_string)
        .unwrap_or_else(|| format!("VRChat {action} failed with HTTP {status}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn complete(release_status: &str) -> Value {
        json!({
            "id": "avtr_1",
            "name": "Entity",
            "releaseStatus": release_status,
            "thumbnailImageUrl": "https://example.test/thumb.png",
        })
    }

    #[test]
    fn avatar_decision_upserts_public_complete_snapshots() {
        assert_eq!(
            cache_write_decision(FavoriteCacheKind::Avatar, &complete("public")),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn avatar_decision_inserts_non_public_complete_snapshots_only_when_missing() {
        for status in ["private", "hidden", ""] {
            assert_eq!(
                cache_write_decision(FavoriteCacheKind::Avatar, &complete(status)),
                CacheWriteDecision::InsertIfMissing
            );
        }
    }

    #[test]
    fn avatar_decision_skips_incomplete_snapshots() {
        assert_eq!(
            cache_write_decision(
                FavoriteCacheKind::Avatar,
                &json!({ "id": "avtr_1", "releaseStatus": "public" }),
            ),
            CacheWriteDecision::Skip
        );
        assert_eq!(
            cache_write_decision(
                FavoriteCacheKind::Avatar,
                &json!({
                    "id": "avtr_1",
                    "name": "Broken Avatar",
                    "releaseStatus": "public",
                })
            ),
            CacheWriteDecision::Skip
        );
    }

    #[test]
    fn avatar_decision_normalizes_release_status_case_and_whitespace() {
        let mut entity = complete("  Public  ");
        assert_eq!(
            cache_write_decision(FavoriteCacheKind::Avatar, &entity),
            CacheWriteDecision::Upsert
        );
        entity["imageUrl"] = json!("https://example.test/image.png");
        entity["thumbnailImageUrl"] = json!("   ");
        assert_eq!(
            cache_write_decision(FavoriteCacheKind::Avatar, &entity),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn world_decision_upserts_public_complete_snapshots() {
        assert_eq!(
            cache_write_decision(FavoriteCacheKind::World, &complete("public")),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn world_decision_inserts_private_complete_snapshots_only_when_missing() {
        assert_eq!(
            cache_write_decision(FavoriteCacheKind::World, &complete("private")),
            CacheWriteDecision::InsertIfMissing
        );
    }

    #[test]
    fn world_decision_skips_other_release_statuses_unlike_avatars() {
        for status in ["hidden", "labs", ""] {
            assert_eq!(
                cache_write_decision(FavoriteCacheKind::World, &complete(status)),
                CacheWriteDecision::Skip
            );
        }
    }

    #[test]
    fn world_decision_skips_incomplete_snapshots() {
        assert_eq!(
            cache_write_decision(
                FavoriteCacheKind::World,
                &json!({
                    "id": "wrld_1",
                    "name": "World",
                    "releaseStatus": "public",
                })
            ),
            CacheWriteDecision::Skip
        );
    }

    #[test]
    fn filter_keeps_only_requested_favorite_ids() {
        let entities = vec![
            json!({ "id": "wrld_1", "name": "One" }),
            json!({ "id": " wrld_2 ", "name": "Two" }),
            json!({ "id": "wrld_3", "name": "Three" }),
            json!({ "name": "No id" }),
        ];

        let details = filter_details_by_id(entities, &["wrld_2".into(), " wrld_3 ".into()]);

        assert_eq!(details.len(), 2);
        assert!(details.contains_key("wrld_2"));
        assert!(details.contains_key("wrld_3"));
    }

    #[test]
    fn filter_keeps_everything_when_favorite_ids_are_empty() {
        let entities = vec![
            json!({ "id": "wrld_1" }),
            json!({ "id": "wrld_2" }),
            json!({ "name": "No id" }),
        ];

        let details = filter_details_by_id(entities, &[]);

        assert_eq!(details.len(), 2);
    }

    #[test]
    fn merge_avatar_rows_deduplicates_across_tag_pages() {
        let mut seen_ids = HashSet::new();
        let mut entities = Vec::new();

        merge_avatar_rows(
            vec![
                json!({ "id": "avtr_1", "name": "First" }),
                json!({ "id": "avtr_2" }),
            ],
            &mut seen_ids,
            &mut entities,
        );
        merge_avatar_rows(
            vec![
                json!({ "id": " avtr_1 ", "name": "Duplicate" }),
                json!({ "id": "" }),
                json!({ "id": "avtr_3" }),
            ],
            &mut seen_ids,
            &mut entities,
        );

        let ids = entities.iter().map(entity_id).collect::<Vec<_>>();
        assert_eq!(ids, vec!["avtr_1", "avtr_2", "avtr_3"]);
        assert_eq!(entities[0]["name"], json!("First"));
    }

    #[test]
    fn normalize_avatar_tags_deduplicates_and_falls_back_to_single_untagged_round() {
        assert_eq!(
            normalize_avatar_tags(&[" one ".into(), "one".into(), "two".into(), "  ".into()]),
            vec!["one".to_string(), "two".to_string()]
        );
        assert_eq!(normalize_avatar_tags(&[]), vec![String::new()]);
        assert_eq!(normalize_avatar_tags(&["  ".into()]), vec![String::new()]);
    }

    #[test]
    fn missing_world_ids_returns_favorites_without_displayable_details() {
        let details_by_id = HashMap::from([
            ("wrld_named".to_string(), json!({ "name": "Named" })),
            ("wrld_tagged".to_string(), json!({ "tags": ["tag"] })),
            ("wrld_blank".to_string(), json!({ "name": "   " })),
            ("wrld_empty".to_string(), json!({})),
        ]);

        let missing = missing_world_ids(
            &[
                " wrld_named ".to_string(),
                "wrld_tagged".to_string(),
                "wrld_blank".to_string(),
                "wrld_empty".to_string(),
                "wrld_absent".to_string(),
                "wrld_absent".to_string(),
                "  ".to_string(),
            ],
            &details_by_id,
        );

        assert_eq!(missing, vec!["wrld_blank", "wrld_empty", "wrld_absent"]);
    }

    #[test]
    fn world_probe_marks_http_404_as_deleted() {
        assert_eq!(
            classify_world_probe(404, json!({ "error": { "message": "not found" } })),
            WorldProbeOutcome::Deleted
        );
    }

    #[test]
    fn world_probe_failures_do_not_produce_availability() {
        assert_eq!(
            classify_world_probe(500, json!({ "message": "boom" })),
            WorldProbeOutcome::Failed
        );
        assert_eq!(
            classify_world_probe(200, json!({ "error": { "message": "soft error" } })),
            WorldProbeOutcome::Failed
        );
    }

    #[test]
    fn world_probe_classifies_release_status_into_public_or_private() {
        let world = json!({ "id": "wrld_1", "name": "World", "releaseStatus": "Public" });
        assert_eq!(
            classify_world_probe(200, world.clone()),
            WorldProbeOutcome::Available(world, "public".to_string())
        );

        for status in ["private", "hidden", ""] {
            let world = json!({ "id": "wrld_1", "releaseStatus": status });
            assert_eq!(
                classify_world_probe(200, world.clone()),
                WorldProbeOutcome::Available(world, "private".to_string())
            );
        }
    }

    #[test]
    fn world_snapshot_key_normalizes_membership_order_and_refresh_scope() {
        let scope = RuntimeAuthScopeSnapshot {
            current_user_id: "usr_current".into(),
            endpoint: "https://api.example.test/api/1".into(),
            generation: 7,
            active: true,
        };

        let left = world_snapshot_key(
            &scope,
            &[" wrld_2 ".into(), "wrld_1".into(), "wrld_2".into()],
            " baseline:1 ",
        );
        let right = world_snapshot_key(&scope, &["wrld_1".into(), "wrld_2".into()], "baseline:1");

        assert_eq!(left, right);
        assert_eq!(left.favorite_ids, vec!["wrld_1", "wrld_2"]);
    }

    #[test]
    fn world_snapshot_projects_only_requested_details_and_availability() {
        let snapshot = FavoriteWorldDetailsSnapshot {
            key: FavoriteWorldDetailsSnapshotKey {
                current_user_id: "usr_current".into(),
                endpoint: "endpoint".into(),
                generation: 1,
                favorite_ids: vec!["wrld_1".into(), "wrld_2".into()],
                refresh_key: "refresh".into(),
            },
            details_by_id: HashMap::from([
                ("wrld_1".into(), json!({ "id": "wrld_1", "name": "One" })),
                ("wrld_2".into(), json!({ "id": "wrld_2", "name": "Two" })),
            ]),
            availability_by_id: HashMap::from([
                ("wrld_1".into(), "public".into()),
                ("wrld_2".into(), "private".into()),
                ("wrld_deleted".into(), "deleted".into()),
            ]),
            fetched_at: "2026-08-11T00:00:00Z".into(),
        };

        let output =
            project_world_snapshot(&snapshot, &[" wrld_2 ".into(), "wrld_deleted".into()], 2);

        assert_eq!(output.details_by_id.len(), 1);
        assert!(output.details_by_id.contains_key("wrld_2"));
        assert_eq!(
            output.availability_by_id,
            HashMap::from([
                ("wrld_2".into(), "private".into()),
                ("wrld_deleted".into(), "deleted".into()),
            ])
        );
        assert_eq!(output.cached_count, 2);
        assert_eq!(output.fetched_at, "2026-08-11T00:00:00Z");
    }

    #[test]
    fn cache_entry_maps_snake_and_camel_timestamps_with_version_fallback() {
        let entity = json!({
            "id": "avtr_1",
            "authorId": " usr_author ",
            "authorName": "Author",
            "createdAt": "2026-06-01T00:00:00.000Z",
            "updated_at": "2026-06-02T00:00:00.000Z",
            "description": "Desc",
            "imageUrl": "https://example.test/image.png",
            "name": "Entity",
            "releaseStatus": "public",
            "thumbnailImageUrl": "https://example.test/thumb.png",
            "version": 7,
        });

        let entry = cache_entry_from_entity(&entity, "avtr_fallback");

        assert_eq!(entry.id, json!("avtr_1"));
        assert_eq!(entry.author_id, json!("usr_author"));
        assert_eq!(entry.created_at, json!("2026-06-01T00:00:00.000Z"));
        assert_eq!(entry.updated_at, json!("2026-06-02T00:00:00.000Z"));
        assert_eq!(entry.version, json!(7));

        let sparse = json!({ "name": "Fallback", "version": "not-a-number" });
        let entry = cache_entry_from_entity(&sparse, " avtr_fallback ");
        assert_eq!(entry.id, json!("avtr_fallback"));
        assert_eq!(entry.version, json!(0));
    }
}
