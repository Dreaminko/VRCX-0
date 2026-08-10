use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use moka::sync::Cache;
use serde_json::Value;
use vrcx_0_persistence::cache_entities::CacheEntityInput;
use vrcx_0_persistence::worlds::{
    world_cache_get, world_cache_search, world_cache_upsert, WorldSummaryOutput,
};
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::{normalize_vrchat_api_endpoint, ApiScope};
use vrcx_0_vrchat_client::worlds::world_get_input;

use crate::web_client::WebClient;
use vrcx_0_core::location::is_meaningful_world_name;

const WORLD_RESOLVE_FETCH_TIMEOUT_MS: u64 = 5_000;
const WORLD_RESOLVE_FAILURE_TTL_MS: u64 = 60_000;

pub struct WorldCache {
    working: Cache<String, Arc<WorldSummaryOutput>>,
    db: Arc<DatabaseService>,
    inflight: Mutex<HashMap<WorldResolveKey, Weak<tokio::sync::Mutex<()>>>>,
    failures: Mutex<HashMap<WorldResolveKey, Instant>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct WorldResolveKey {
    endpoint: String,
    world_id: String,
}

impl WorldCache {
    pub fn new(db: Arc<DatabaseService>, capacity: u64, working_ttl: Duration) -> Self {
        let capacity = capacity.max(1);
        Self {
            working: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(working_ttl)
                .build(),
            db,
            inflight: Mutex::new(HashMap::new()),
            failures: Mutex::new(HashMap::new()),
        }
    }

    pub fn clear_working(&self) {
        self.working.invalidate_all();
    }

    pub fn get_name(&self, world_id: &str) -> Option<String> {
        let world_id = normalize_id(world_id);
        if world_id.is_empty() {
            return None;
        }
        self.working
            .get(&world_id)
            .map(|summary| summary.name.clone())
    }

    pub fn get_summary(&self, world_id: &str) -> crate::Result<Option<WorldSummaryOutput>> {
        let world_id = normalize_id(world_id);
        if world_id.is_empty() {
            return Ok(None);
        }
        if let Some(summary) = self
            .working
            .get(&world_id)
            .map(|summary| summary.as_ref().clone())
        {
            if is_meaningful_world_name(&summary.name) {
                return Ok(Some(summary));
            }
            self.working.invalidate(&world_id);
        }
        let Some(summary) = world_cache_get(self.db.as_ref(), world_id.clone())? else {
            return Ok(None);
        };
        if !is_meaningful_world_name(&summary.name) {
            return Ok(None);
        }
        self.working.insert(world_id, Arc::new(summary.clone()));
        Ok(Some(summary))
    }

    pub fn search_summaries(
        &self,
        query: &str,
        limit: i64,
    ) -> crate::Result<Vec<WorldSummaryOutput>> {
        let summaries = world_cache_search(self.db.as_ref(), query, limit)?
            .into_iter()
            .filter(|summary| is_meaningful_world_name(&summary.name))
            .collect::<Vec<_>>();
        for summary in &summaries {
            self.working
                .insert(summary.id.clone(), Arc::new(summary.clone()));
        }
        Ok(summaries)
    }

    pub fn hydrate_from_payload(&self, world_value: &Value) -> Option<String> {
        let world_id = world_id(world_value);
        if world_id.is_empty() {
            return None;
        }
        let name = world_name(world_value)?;
        let cached = Arc::new(world_summary(world_value, world_id.clone(), name.clone()));
        self.working.insert(world_id.clone(), cached);

        if is_persistable_world(world_value, &name) {
            let entry = CacheEntityInput {
                id: Value::String(world_id.clone()),
                author_id: value_or_null(world_value, "authorId"),
                author_name: value_or_null(world_value, "authorName"),
                created_at: value_or_null_with_fallback(world_value, "created_at", "createdAt"),
                description: value_or_null(world_value, "description"),
                image_url: value_or_null(world_value, "imageUrl"),
                name: Value::String(name.clone()),
                release_status: value_or_null(world_value, "releaseStatus"),
                thumbnail_image_url: value_or_null(world_value, "thumbnailImageUrl"),
                updated_at: value_or_null_with_fallback(world_value, "updated_at", "updatedAt"),
                version: value_or_null(world_value, "version"),
            };
            if let Err(error) = world_cache_upsert(self.db.as_ref(), entry) {
                tracing::warn!(world_id = %world_id, "WorldCache upsert failed: {error}");
            }
        }
        Some(name)
    }

    pub async fn resolve_name(
        &self,
        web: &WebClient,
        endpoint: &str,
        world_id: &str,
    ) -> Option<String> {
        if let Some(name) = self.get_name(world_id) {
            return Some(name);
        }
        self.resolve_summary(web, endpoint, world_id)
            .await
            .map(|summary| summary.name)
    }

    pub async fn resolve_summary(
        &self,
        web: &WebClient,
        endpoint: &str,
        world_id: &str,
    ) -> Option<WorldSummaryOutput> {
        let world_id = normalize_id(world_id);
        if world_id.is_empty() {
            return None;
        }
        if let Some(summary) = self.get_summary(&world_id).ok().flatten() {
            return Some(summary);
        }
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return None;
        }
        let key = resolve_key(endpoint, &world_id);
        if self.recently_failed(&key) {
            return None;
        }
        let inflight = self.inflight_lock(&key);
        let _guard = inflight.lock().await;
        if let Some(summary) = self.get_summary(&world_id).ok().flatten() {
            return Some(summary);
        }
        if self.recently_failed(&key) {
            return None;
        }
        match self
            .fetch_world_summary(web, &key.endpoint, &key.world_id)
            .await
        {
            Some(summary) => {
                self.clear_failure(&key);
                Some(summary)
            }
            None => {
                self.record_failure(&key);
                None
            }
        }
    }

    async fn fetch_world_summary(
        &self,
        web: &WebClient,
        endpoint: &str,
        world_id: &str,
    ) -> Option<WorldSummaryOutput> {
        let (_, request) = world_get_input(endpoint.to_string(), world_id.to_string()).ok()?;
        let response = tokio::time::timeout(
            Duration::from_millis(WORLD_RESOLVE_FETCH_TIMEOUT_MS),
            web.execute_api(request, ApiScope::Vrchat, self.db.as_ref()),
        )
        .await
        .ok()?
        .ok()?;
        if !(200..=299).contains(&response.status) {
            return None;
        }
        let world = serde_json::from_str::<Value>(&response.data).ok()?;
        self.hydrate_from_payload(&world)?;
        self.get_summary(world_id).ok().flatten()
    }

    fn recently_failed(&self, key: &WorldResolveKey) -> bool {
        self.failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(key)
            .is_some_and(|at| at.elapsed() < Duration::from_millis(WORLD_RESOLVE_FAILURE_TTL_MS))
    }

    fn record_failure(&self, key: &WorldResolveKey) {
        let mut map = self
            .failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        map.retain(|_, at| at.elapsed() < Duration::from_millis(WORLD_RESOLVE_FAILURE_TTL_MS));
        map.insert(key.clone(), Instant::now());
    }

    fn clear_failure(&self, key: &WorldResolveKey) {
        self.failures
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(key);
    }

    fn inflight_lock(&self, key: &WorldResolveKey) -> Arc<tokio::sync::Mutex<()>> {
        let mut map = self
            .inflight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = map.get(key).and_then(Weak::upgrade) {
            return existing;
        }
        map.retain(|_, weak| weak.strong_count() > 0);
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        map.insert(key.clone(), Arc::downgrade(&lock));
        lock
    }
}

fn world_summary(value: &Value, id: String, name: String) -> WorldSummaryOutput {
    WorldSummaryOutput {
        id,
        author_id: text_field(value, "authorId"),
        author_name: text_field(value, "authorName"),
        created_at: text_field_with_fallback(value, "created_at", "createdAt"),
        description: text_field(value, "description"),
        image_url: text_field(value, "imageUrl"),
        name,
        release_status: text_field(value, "releaseStatus"),
        thumbnail_image_url: text_field(value, "thumbnailImageUrl"),
        updated_at: text_field_with_fallback(value, "updated_at", "updatedAt"),
        version: value
            .get("version")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
    }
}

fn text_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn text_field_with_fallback(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .or_else(|| value.get(fallback))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn normalize_id(value: &str) -> String {
    value.trim().to_string()
}

fn world_id(value: &Value) -> String {
    value
        .get("id")
        .or_else(|| value.get("worldId"))
        .and_then(Value::as_str)
        .map(normalize_id)
        .unwrap_or_default()
}

fn world_name(value: &Value) -> Option<String> {
    value
        .get("name")
        .or_else(|| value.get("worldName"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| is_meaningful_world_name(name))
        .map(ToString::to_string)
}

fn value_or_null(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or(Value::Null)
}

fn value_or_null_with_fallback(value: &Value, key: &str, fallback: &str) -> Value {
    value
        .get(key)
        .or_else(|| value.get(fallback))
        .cloned()
        .unwrap_or(Value::Null)
}

fn resolve_key(endpoint: &str, world_id: &str) -> WorldResolveKey {
    WorldResolveKey {
        endpoint: normalize_vrchat_api_endpoint(Some(endpoint)),
        world_id: world_id.to_string(),
    }
}

fn is_persistable_world(value: &Value, name: &str) -> bool {
    let release_status = value
        .get("releaseStatus")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let image_url = value
        .get("imageUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let thumbnail_image_url = value
        .get("thumbnailImageUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    release_status == "public"
        && is_meaningful_world_name(name)
        && (!image_url.is_empty() || !thumbnail_image_url.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use serde_json::json;
    use vrcx_0_persistence::cache_entities::CacheEntityInput;
    use vrcx_0_persistence::worlds::{world_cache_get, world_cache_remove, world_cache_upsert};

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("vrcx-0-world-cache-{name}-{nonce}"));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn test_db(name: &str) -> (TestDir, Arc<DatabaseService>) {
        let dir = TestDir::new(name);
        let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap());
        (dir, db)
    }

    fn world_entry(id: &str, name: &str, updated_at: &str) -> CacheEntityInput {
        CacheEntityInput {
            id: json!(id),
            author_id: json!(null),
            author_name: json!(null),
            created_at: json!("2026-01-01T00:00:00.000Z"),
            description: json!(null),
            image_url: json!("image.png"),
            name: json!(name),
            release_status: json!("public"),
            thumbnail_image_url: json!("thumb.png"),
            updated_at: json!(updated_at),
            version: json!(1),
        }
    }

    #[test]
    fn hydrate_from_payload_caches_name_only_and_persists_summary() {
        let (_dir, db) = test_db("hydrate-name-only");
        let cache = WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        let name = cache.hydrate_from_payload(&json!({
            "id": "wrld_heavy",
            "name": "Heavy World",
            "authorId": "usr_author",
            "authorName": "Author",
            "createdAt": "2026-01-01T00:00:00.000Z",
            "description": "Summary detail",
            "imageUrl": "image.png",
            "releaseStatus": "public",
            "thumbnailImageUrl": "thumb.png",
            "updatedAt": "2026-01-02T00:00:00.000Z",
            "version": 7,
            "unityPackages": [{ "assetUrl": "https://example.test/large.bundle" }],
            "instances": [["123", 4]],
            "tags": ["author_tag_large"]
        }));

        assert_eq!(name.as_deref(), Some("Heavy World"));
        assert_eq!(cache.get_name("wrld_heavy").as_deref(), Some("Heavy World"));
        assert_eq!(
            cache
                .working
                .get("wrld_heavy")
                .map(|summary| summary.name.clone())
                .as_deref(),
            Some("Heavy World")
        );

        let row = world_cache_get(db.as_ref(), "wrld_heavy".into())
            .unwrap()
            .unwrap();
        assert_eq!(row.name, "Heavy World");
        assert_eq!(row.description, "Summary detail");
        assert_eq!(row.version, 7);
    }

    #[test]
    fn hydrate_from_vrchat_payload_preserves_snake_case_timestamps() {
        let (_dir, db) = test_db("hydrate-vrchat-timestamps");
        let cache = WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        cache.hydrate_from_payload(&json!({
            "id": "wrld_timestamps",
            "name": "Timestamped World",
            "created_at": "2026-01-01T00:00:00.000Z",
            "updated_at": "2026-01-02T00:00:00.000Z",
            "releaseStatus": "public",
            "imageUrl": "image.png"
        }));

        let row = world_cache_get(db.as_ref(), "wrld_timestamps".into())
            .unwrap()
            .unwrap();
        assert_eq!(row.created_at, "2026-01-01T00:00:00.000Z");
        assert_eq!(row.updated_at, "2026-01-02T00:00:00.000Z");
    }

    #[test]
    fn summary_lookup_starts_empty_then_loads_db_row_into_memory() {
        let (_dir, db) = test_db("summary-db-fallback");
        world_cache_upsert(
            db.as_ref(),
            world_entry("wrld_db_only", "DB Only World", "2026-01-02T00:00:00.000Z"),
        )
        .unwrap();
        let cache = WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        assert_eq!(cache.get_name("wrld_db_only"), None);

        let summary = cache
            .get_summary("wrld_db_only")
            .unwrap()
            .expect("DB row should be loaded on demand");

        assert_eq!(summary.name, "DB Only World");
        world_cache_remove(db.as_ref(), "wrld_db_only".into()).unwrap();
        let memory_summary = cache
            .get_summary("wrld_db_only")
            .unwrap()
            .expect("memory hit should not query the removed DB row");
        assert_eq!(memory_summary.name, "DB Only World");
        assert_eq!(
            cache.get_name("wrld_db_only").as_deref(),
            Some("DB Only World")
        );
    }

    #[test]
    fn summary_lookup_ignores_invalid_db_shells() {
        let (_dir, db) = test_db("summary-invalid-shell");
        world_cache_upsert(
            db.as_ref(),
            world_entry("wrld_shell", "", "2026-01-02T00:00:00.000Z"),
        )
        .unwrap();
        let cache = WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        assert!(cache.get_summary("wrld_shell").unwrap().is_none());
        assert_eq!(cache.get_name("wrld_shell"), None);
    }

    #[tokio::test]
    async fn summary_resolution_uses_db_before_remote_api() {
        let (dir, db) = test_db("summary-db-before-api");
        world_cache_upsert(
            db.as_ref(),
            world_entry(
                "wrld_db_first",
                "DB First World",
                "2026-01-02T00:00:00.000Z",
            ),
        )
        .unwrap();
        let storage =
            vrcx_0_persistence::storage::StorageService::new(&dir.path.join("storage.json"))
                .unwrap();
        let web = WebClient::new(
            &storage,
            db.as_ref(),
            "wss://pipeline.vrchat.cloud".to_string(),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();
        let cache = WorldCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        let summary = cache
            .resolve_summary(&web, "http://127.0.0.1:9/api/1", "wrld_db_first")
            .await
            .expect("DB row should resolve without remote API");

        assert_eq!(summary.name, "DB First World");
    }

    #[test]
    fn resolve_guards_are_scoped_by_normalized_endpoint() {
        let (_dir, db) = test_db("endpoint-scoped-guards");
        let cache = WorldCache::new(db, 8, Duration::from_secs(60));
        let world_id = "wrld_shared";

        let first = resolve_key(" https://one.example/api/1/ ", world_id);
        let same = resolve_key("https://one.example/api/1", world_id);
        let other = resolve_key("https://two.example/api/1", world_id);

        cache.record_failure(&first);
        assert!(cache.recently_failed(&same));
        assert!(!cache.recently_failed(&other));

        let first_lock = cache.inflight_lock(&first);
        let same_lock = cache.inflight_lock(&same);
        let other_lock = cache.inflight_lock(&other);
        assert!(Arc::ptr_eq(&first_lock, &same_lock));
        assert!(!Arc::ptr_eq(&first_lock, &other_lock));
    }

    #[test]
    fn capacity_bounds_every_hydrated_world() {
        let (_dir, db) = test_db("bounded-summaries");
        let cache = WorldCache::new(Arc::clone(&db), 1, Duration::from_secs(60));
        cache.hydrate_from_payload(&json!({
            "id": "wrld_first",
            "name": "First World",
            "releaseStatus": "public",
            "imageUrl": "image.png"
        }));
        cache.hydrate_from_payload(&json!({
            "id": "wrld_second",
            "name": "Second World",
            "releaseStatus": "public",
            "imageUrl": "image.png"
        }));
        cache.working.run_pending_tasks();

        assert!(cache.working.entry_count() <= 1);
    }
}
