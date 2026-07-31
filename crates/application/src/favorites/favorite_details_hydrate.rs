use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vrcx_0_core::json::RawJson;
use vrcx_0_persistence::{
    avatars::{avatar_cache_existing_ids, avatar_cache_upsert},
    cache_entities::CacheEntityInput,
    worlds::{world_cache_get_many, world_cache_upsert},
    DatabaseService,
};
use vrcx_0_vrchat_client::{
    favorites::{favorite_avatars_get_input, favorite_worlds_get_input},
    http_api::{normalize_text, ApiScope, HttpApiRequestInput},
};

use crate::{Error, Result, RuntimeAuthScope, RuntimeAuthScopeSnapshot, WebClient};

const FAVORITE_DETAILS_PAGE_SIZE: i64 = 300;
const FAVORITE_DETAILS_MAX_PAGES: usize = 50;

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
    pub avatar_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteDetailsHydrateOutput {
    pub details_by_id: HashMap<String, RawJson>,
    pub cached_count: u32,
    pub fetched_at: String,
}

pub struct FavoriteDetailsHydrateDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub auth_scope: &'a RuntimeAuthScope,
    pub expected_scope: RuntimeAuthScopeSnapshot,
}

pub async fn hydrate_favorite_details(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    input: FavoriteDetailsHydrateInput,
) -> Result<FavoriteDetailsHydrateOutput> {
    let entities = match input.kind {
        FavoriteDetailsHydrateKind::Avatar => {
            fetch_favorite_avatar_entities(deps, &input.avatar_tags).await?
        }
        FavoriteDetailsHydrateKind::World => fetch_favorite_world_entities(deps).await?,
    };
    let details_by_id = filter_details_by_id(entities, &input.favorite_ids);
    let cached_count = persist_details(deps.db, input.kind, &details_by_id);
    Ok(FavoriteDetailsHydrateOutput {
        details_by_id: details_by_id
            .into_iter()
            .map(|(id, entity)| (id, RawJson::from(entity)))
            .collect(),
        cached_count,
        fetched_at: Utc::now().to_rfc3339(),
    })
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

async fn execute_page(
    deps: &FavoriteDetailsHydrateDeps<'_>,
    request: HttpApiRequestInput,
    action: &str,
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
            action,
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

fn persist_details(
    db: &DatabaseService,
    kind: FavoriteDetailsHydrateKind,
    details_by_id: &HashMap<String, Value>,
) -> u32 {
    match kind {
        FavoriteDetailsHydrateKind::Avatar => persist_avatar_details(db, details_by_id),
        FavoriteDetailsHydrateKind::World => persist_world_details(db, details_by_id),
    }
}

fn persist_avatar_details(db: &DatabaseService, details_by_id: &HashMap<String, Value>) -> u32 {
    let insert_candidates = details_by_id
        .iter()
        .filter(|(_, entity)| {
            avatar_cache_write_decision(entity) == CacheWriteDecision::InsertIfMissing
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
        let decision = avatar_cache_write_decision(entity);
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
            world_cache_write_decision(entity) == CacheWriteDecision::InsertIfMissing
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
        match world_cache_write_decision(entity) {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CacheWriteDecision {
    Upsert,
    InsertIfMissing,
    Skip,
}

fn avatar_cache_write_decision(entity: &Value) -> CacheWriteDecision {
    if !has_complete_snapshot(entity) {
        return CacheWriteDecision::Skip;
    }
    if release_status(entity) == "public" {
        CacheWriteDecision::Upsert
    } else {
        CacheWriteDecision::InsertIfMissing
    }
}

fn world_cache_write_decision(entity: &Value) -> CacheWriteDecision {
    if !has_complete_snapshot(entity) {
        return CacheWriteDecision::Skip;
    }
    match release_status(entity).as_str() {
        "public" => CacheWriteDecision::Upsert,
        "private" => CacheWriteDecision::InsertIfMissing,
        _ => CacheWriteDecision::Skip,
    }
}

fn release_status(entity: &Value) -> String {
    field_text(entity, &["releaseStatus"]).trim().to_lowercase()
}

fn has_complete_snapshot(entity: &Value) -> bool {
    let name = field_text(entity, &["name"]);
    let image_url = {
        let thumbnail = field_text(entity, &["thumbnailImageUrl"]);
        let thumbnail = thumbnail.trim();
        if thumbnail.is_empty() {
            field_text(entity, &["imageUrl"]).trim().to_string()
        } else {
            thumbnail.to_string()
        }
    };
    !name.trim().is_empty() && !image_url.is_empty()
}

fn cache_entry_from_entity(entity: &Value, fallback_id: &str) -> CacheEntityInput {
    let id = entity_id(entity);
    let id = if id.is_empty() {
        normalize_text(fallback_id)
    } else {
        id
    };
    CacheEntityInput {
        id: Value::String(id),
        author_id: Value::String(entity_field_id(entity, "authorId")),
        author_name: Value::String(field_text(entity, &["authorName"])),
        created_at: Value::String(field_text(entity, &["created_at", "createdAt"])),
        description: Value::String(field_text(entity, &["description"])),
        image_url: Value::String(field_text(entity, &["imageUrl"])),
        name: Value::String(field_text(entity, &["name"])),
        release_status: Value::String(field_text(entity, &["releaseStatus"])),
        thumbnail_image_url: Value::String(field_text(entity, &["thumbnailImageUrl"])),
        updated_at: Value::String(field_text(entity, &["updated_at", "updatedAt"])),
        version: Value::Number(entity_version(entity).into()),
    }
}

fn entity_id(entity: &Value) -> String {
    entity_field_id(entity, "id")
}

fn entity_field_id(entity: &Value, key: &str) -> String {
    normalize_text(field_text(entity, &[key]))
}

fn field_text(entity: &Value, keys: &[&str]) -> String {
    for key in keys {
        match entity.get(*key) {
            Some(Value::String(text)) => return text.clone(),
            Some(Value::Null) | None => continue,
            Some(other) => return other.to_string(),
        }
    }
    String::new()
}

fn entity_version(entity: &Value) -> i64 {
    match entity.get("version") {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
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
            avatar_cache_write_decision(&complete("public")),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn avatar_decision_inserts_non_public_complete_snapshots_only_when_missing() {
        for status in ["private", "hidden", ""] {
            assert_eq!(
                avatar_cache_write_decision(&complete(status)),
                CacheWriteDecision::InsertIfMissing
            );
        }
    }

    #[test]
    fn avatar_decision_skips_incomplete_snapshots() {
        assert_eq!(
            avatar_cache_write_decision(&json!({ "id": "avtr_1", "releaseStatus": "public" })),
            CacheWriteDecision::Skip
        );
        assert_eq!(
            avatar_cache_write_decision(&json!({
                "id": "avtr_1",
                "name": "Broken Avatar",
                "releaseStatus": "public",
            })),
            CacheWriteDecision::Skip
        );
    }

    #[test]
    fn avatar_decision_normalizes_release_status_case_and_whitespace() {
        let mut entity = complete("  Public  ");
        assert_eq!(
            avatar_cache_write_decision(&entity),
            CacheWriteDecision::Upsert
        );
        entity["imageUrl"] = json!("https://example.test/image.png");
        entity["thumbnailImageUrl"] = json!("   ");
        assert_eq!(
            avatar_cache_write_decision(&entity),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn world_decision_upserts_public_complete_snapshots() {
        assert_eq!(
            world_cache_write_decision(&complete("public")),
            CacheWriteDecision::Upsert
        );
    }

    #[test]
    fn world_decision_inserts_private_complete_snapshots_only_when_missing() {
        assert_eq!(
            world_cache_write_decision(&complete("private")),
            CacheWriteDecision::InsertIfMissing
        );
    }

    #[test]
    fn world_decision_skips_other_release_statuses_unlike_avatars() {
        for status in ["hidden", "labs", ""] {
            assert_eq!(
                world_cache_write_decision(&complete(status)),
                CacheWriteDecision::Skip
            );
        }
    }

    #[test]
    fn world_decision_skips_incomplete_snapshots() {
        assert_eq!(
            world_cache_write_decision(&json!({
                "id": "wrld_1",
                "name": "World",
                "releaseStatus": "public",
            })),
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
