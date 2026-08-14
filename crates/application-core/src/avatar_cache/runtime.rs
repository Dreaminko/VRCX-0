use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use moka::sync::Cache;
use serde_json::Value;
use vrcx_0_core::vrchat_json::AvatarJson;
use vrcx_0_persistence::avatars::{
    avatar_cache_find_by_file_id, avatar_cache_get, avatar_cache_upsert, AvatarCacheOutput,
};
use vrcx_0_persistence::cache_entities::CacheEntityInput;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::avatars::avatar_get_input;
use vrcx_0_vrchat_client::http_api::{normalize_vrchat_api_endpoint, ApiScope};

use crate::web_client::WebClient;

const AVATAR_RESOLVE_FETCH_TIMEOUT_MS: u64 = 5_000;

pub struct AvatarCache {
    working: Cache<AvatarCacheKey, Arc<AvatarCacheEntry>>,
    db: Arc<DatabaseService>,
    inflight: Mutex<HashMap<AvatarCacheKey, Weak<tokio::sync::Mutex<()>>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct AvatarCacheKey {
    user_id: String,
    endpoint: String,
    avatar_id: String,
}

struct AvatarCacheEntry {
    summary: AvatarCacheOutput,
    full: Option<Arc<Value>>,
}

impl AvatarCache {
    pub fn new(db: Arc<DatabaseService>, capacity: u64, working_ttl: Duration) -> Self {
        let capacity = capacity.max(1);
        Self {
            working: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(working_ttl)
                .build(),
            db,
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub fn clear_working(&self) {
        self.working.invalidate_all();
    }

    pub fn invalidate(&self, user_id: &str, endpoint: &str, avatar_id: &str) {
        self.working
            .invalidate(&cache_key(user_id, endpoint, avatar_id));
    }

    pub fn get_summary(
        &self,
        user_id: &str,
        endpoint: &str,
        avatar_id: &str,
    ) -> crate::Result<Option<AvatarCacheOutput>> {
        let key = cache_key(user_id, endpoint, avatar_id);
        if key.avatar_id.is_empty() {
            return Ok(None);
        }
        if let Some(entry) = self.working.get(&key) {
            return Ok(Some(entry.summary.clone()));
        }
        let Some(summary) = avatar_cache_get(self.db.as_ref(), key.avatar_id.clone())? else {
            return Ok(None);
        };
        if !is_meaningful_summary(&summary) {
            return Ok(None);
        }
        self.insert_summary(key, summary.clone());
        Ok(Some(summary))
    }

    pub fn find_by_image_url(
        &self,
        user_id: &str,
        endpoint: &str,
        image_url: &str,
    ) -> crate::Result<Option<Arc<Value>>> {
        let Some(file_id) = extract_file_id(image_url) else {
            return Ok(None);
        };
        let Some(summary) = avatar_cache_find_by_file_id(self.db.as_ref(), &file_id)? else {
            return Ok(None);
        };
        if !is_meaningful_summary(&summary) {
            return Ok(None);
        }
        let key = cache_key(user_id, endpoint, &summary.id);
        self.insert_summary(key, summary.clone());
        Ok(Some(Arc::new(summary_value(&summary)?)))
    }

    pub fn hydrate_from_payload(
        &self,
        user_id: &str,
        endpoint: &str,
        avatar: Value,
    ) -> Option<Arc<Value>> {
        let summary = avatar_summary(&avatar)?;
        let key = cache_key(user_id, endpoint, &summary.id);
        let avatar = Arc::new(avatar);
        self.working.insert(
            key,
            Arc::new(AvatarCacheEntry {
                summary: summary.clone(),
                full: Some(Arc::clone(&avatar)),
            }),
        );
        if let Err(error) = avatar_cache_upsert(
            self.db.as_ref(),
            cache_entity_input(avatar.as_ref(), &summary),
        ) {
            tracing::warn!(avatar_id = %summary.id, "AvatarCache upsert failed: {error}");
        }
        Some(avatar)
    }

    pub async fn resolve(
        &self,
        web: &WebClient,
        user_id: &str,
        endpoint: &str,
        avatar_id: &str,
        full: bool,
        fresh: bool,
    ) -> crate::Result<Option<Arc<Value>>> {
        let key = cache_key(user_id, endpoint, avatar_id);
        if key.avatar_id.is_empty() {
            return Ok(None);
        }
        if !fresh {
            if let Some(value) = self.working_value_for_key(&key, full) {
                return Ok(Some(value));
            }
            if !full {
                if let Some(summary) = self.get_summary(user_id, endpoint, avatar_id)? {
                    return Ok(Some(Arc::new(summary_value(&summary)?)));
                }
            }
        }
        let inflight = self.inflight_lock(&key);
        let _guard = inflight.lock().await;
        if !fresh {
            if let Some(value) = self.working_value_for_key(&key, full) {
                return Ok(Some(value));
            }
            if !full {
                if let Some(summary) = self.get_summary(user_id, endpoint, avatar_id)? {
                    return Ok(Some(Arc::new(summary_value(&summary)?)));
                }
            }
        }
        self.fetch_avatar(web, &key).await.map(Some)
    }

    async fn fetch_avatar(
        &self,
        web: &WebClient,
        key: &AvatarCacheKey,
    ) -> crate::Result<Arc<Value>> {
        let (_, request) = avatar_get_input(key.endpoint.clone(), key.avatar_id.clone())?;
        let response = tokio::time::timeout(
            Duration::from_millis(AVATAR_RESOLVE_FETCH_TIMEOUT_MS),
            web.execute_api(request, ApiScope::Vrchat, self.db.as_ref()),
        )
        .await
        .map_err(|_| {
            crate::Error::Custom(format!("Avatar request timed out: {}", key.avatar_id))
        })??;
        if !(200..=299).contains(&response.status) {
            return Err(crate::Error::Custom(format!(
                "Avatar request failed with status {}: {}",
                response.status, key.avatar_id
            )));
        }
        let avatar = serde_json::from_str::<Value>(&response.data)?;
        self.hydrate_from_payload(&key.user_id, &key.endpoint, avatar)
            .ok_or_else(|| {
                crate::Error::Custom(format!("Invalid avatar response: {}", key.avatar_id))
            })
    }

    fn insert_summary(&self, key: AvatarCacheKey, summary: AvatarCacheOutput) {
        self.working.insert(
            key,
            Arc::new(AvatarCacheEntry {
                summary,
                full: None,
            }),
        );
    }

    fn working_value_for_key(&self, key: &AvatarCacheKey, full: bool) -> Option<Arc<Value>> {
        let entry = self.working.get(key)?;
        if full {
            return entry.full.as_ref().map(Arc::clone);
        }
        entry
            .full
            .as_ref()
            .map(Arc::clone)
            .or_else(|| summary_value(&entry.summary).ok().map(Arc::new))
    }

    #[cfg(test)]
    fn working_value(
        &self,
        user_id: &str,
        endpoint: &str,
        avatar_id: &str,
        full: bool,
    ) -> Option<Arc<Value>> {
        self.working_value_for_key(&cache_key(user_id, endpoint, avatar_id), full)
    }

    fn inflight_lock(&self, key: &AvatarCacheKey) -> Arc<tokio::sync::Mutex<()>> {
        let mut inflight = self
            .inflight
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(existing) = inflight.get(key).and_then(Weak::upgrade) {
            return existing;
        }
        inflight.retain(|_, weak| weak.strong_count() > 0);
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        inflight.insert(key.clone(), Arc::downgrade(&lock));
        lock
    }

    #[cfg(test)]
    fn entry_count(&self) -> u64 {
        self.working.entry_count()
    }

    #[cfg(test)]
    fn run_pending_tasks(&self) {
        self.working.run_pending_tasks();
    }
}

fn cache_key(user_id: &str, endpoint: &str, avatar_id: &str) -> AvatarCacheKey {
    AvatarCacheKey {
        user_id: user_id.trim().to_string(),
        endpoint: normalize_vrchat_api_endpoint(Some(endpoint)),
        avatar_id: avatar_id.trim().to_string(),
    }
}

fn avatar_summary(value: &Value) -> Option<AvatarCacheOutput> {
    let id = text_field(value, "id");
    let name = text_field(value, "name");
    if id.is_empty() || name.is_empty() {
        return None;
    }
    Some(AvatarCacheOutput {
        id,
        author_id: text_field(value, "authorId"),
        author_name: text_field(value, "authorName"),
        created_at: text_field_with_fallback(value, "created_at", "createdAt"),
        description: text_field(value, "description"),
        image_url: text_field(value, "imageUrl"),
        name,
        release_status: AvatarJson::new(value).release_status().unwrap_or_default(),
        thumbnail_image_url: text_field(value, "thumbnailImageUrl"),
        updated_at: text_field_with_fallback(value, "updated_at", "updatedAt"),
        version: value
            .get("version")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
    })
}

fn cache_entity_input(value: &Value, summary: &AvatarCacheOutput) -> CacheEntityInput {
    CacheEntityInput {
        id: Value::String(summary.id.clone()),
        author_id: value_or_null(value, "authorId"),
        author_name: value_or_null(value, "authorName"),
        created_at: value_or_null_with_fallback(value, "created_at", "createdAt"),
        description: value_or_null(value, "description"),
        image_url: value_or_null(value, "imageUrl"),
        name: Value::String(summary.name.clone()),
        release_status: value_or_null(value, "releaseStatus"),
        thumbnail_image_url: value_or_null(value, "thumbnailImageUrl"),
        updated_at: value_or_null_with_fallback(value, "updated_at", "updatedAt"),
        version: value_or_null(value, "version"),
    }
}

fn summary_value(summary: &AvatarCacheOutput) -> crate::Result<Value> {
    Ok(serde_json::to_value(summary)?)
}

fn is_meaningful_summary(summary: &AvatarCacheOutput) -> bool {
    !summary.id.trim().is_empty() && !summary.name.trim().is_empty()
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
    let text = text_field(value, key);
    if text.is_empty() {
        text_field(value, fallback)
    } else {
        text
    }
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

fn extract_file_id(value: &str) -> Option<String> {
    let start = value.find("file_")?;
    let file_id = value[start..]
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .collect::<String>();
    (file_id.len() > "file_".len()).then_some(file_id)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::{json, Value};
    use vrcx_0_persistence::avatars::{avatar_cache_get, avatar_cache_remove, avatar_cache_upsert};
    use vrcx_0_persistence::cache_entities::CacheEntityInput;
    use vrcx_0_persistence::DatabaseService;

    use super::AvatarCache;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "vrcx-0-avatar-cache-{name}-{}-{nonce}",
                std::process::id()
            ));
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

    fn avatar_entry(id: &str, name: &str) -> CacheEntityInput {
        CacheEntityInput {
            id: json!(id),
            author_id: json!("usr_author"),
            author_name: json!("Author"),
            created_at: json!("2026-01-01T00:00:00.000Z"),
            description: json!("Description"),
            image_url: json!(format!("https://example.test/file/file_{id}/1/file")),
            name: json!(name),
            release_status: json!("public"),
            thumbnail_image_url: json!("thumb.png"),
            updated_at: json!("2026-01-02T00:00:00.000Z"),
            version: json!(1),
        }
    }

    #[test]
    fn summary_lookup_starts_empty_then_loads_one_db_row_into_memory() {
        let (_dir, db) = test_db("summary-db-fallback");
        avatar_cache_upsert(db.as_ref(), avatar_entry("avtr_db", "DB Avatar")).unwrap();
        let cache = AvatarCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        assert!(cache
            .working_value(
                "usr_self",
                "https://api.example.test/api/1",
                "avtr_db",
                false
            )
            .is_none());

        let summary = cache
            .get_summary("usr_self", "https://api.example.test/api/1", "avtr_db")
            .unwrap()
            .expect("DB row should load on demand");

        assert_eq!(summary.name, "DB Avatar");
        avatar_cache_remove(db.as_ref(), "avtr_db".into()).unwrap();
        assert_eq!(
            cache
                .get_summary("usr_self", "https://api.example.test/api/1", "avtr_db")
                .unwrap()
                .expect("working cache should serve the removed DB row")
                .name,
            "DB Avatar"
        );
    }

    #[test]
    fn full_reads_do_not_treat_db_summaries_as_complete_api_payloads() {
        let (_dir, db) = test_db("summary-is-not-full");
        avatar_cache_upsert(db.as_ref(), avatar_entry("avtr_db", "DB Avatar")).unwrap();
        let cache = AvatarCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        cache
            .get_summary("usr_self", "https://api.example.test/api/1", "avtr_db")
            .unwrap();

        assert!(cache
            .working_value(
                "usr_self",
                "https://api.example.test/api/1",
                "avtr_db",
                true
            )
            .is_none());
    }

    #[tokio::test]
    async fn ordinary_resolution_uses_db_before_remote_api() {
        let (dir, db) = test_db("db-before-api");
        avatar_cache_upsert(db.as_ref(), avatar_entry("avtr_db", "DB Avatar")).unwrap();
        let storage =
            vrcx_0_persistence::storage::StorageService::new(&dir.path.join("storage.json"))
                .unwrap();
        let web = crate::WebClient::new(
            &storage,
            db.as_ref(),
            "wss://pipeline.vrchat.cloud".to_string(),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap();
        let cache = AvatarCache::new(Arc::clone(&db), 8, Duration::from_secs(60));

        let avatar = cache
            .resolve(
                &web,
                "usr_self",
                "http://127.0.0.1:9/api/1",
                "avtr_db",
                false,
                false,
            )
            .await
            .unwrap()
            .expect("DB summary should resolve without remote API");

        assert_eq!(
            avatar.get("name").and_then(Value::as_str),
            Some("DB Avatar")
        );
    }

    #[test]
    fn full_api_payloads_are_scoped_to_the_authenticated_user() {
        let (_dir, db) = test_db("auth-scoped-full-payload");
        let cache = AvatarCache::new(Arc::clone(&db), 8, Duration::from_secs(60));
        cache.hydrate_from_payload(
            "usr_first",
            "https://api.example.test/api/1",
            json!({
                "id": "avtr_private",
                "name": "Private Avatar",
                "releaseStatus": "private",
                "unityPackages": []
            }),
        );

        assert!(cache
            .working_value(
                "usr_first",
                "https://api.example.test/api/1",
                "avtr_private",
                true
            )
            .is_some());
        assert!(cache
            .working_value(
                "usr_second",
                "https://api.example.test/api/1",
                "avtr_private",
                true
            )
            .is_none());
    }

    #[test]
    fn api_payload_is_bounded_in_moka_and_persists_only_the_summary() {
        let (_dir, db) = test_db("bounded-api-payload");
        let cache = AvatarCache::new(Arc::clone(&db), 1, Duration::from_secs(60));

        for (id, name) in [("avtr_first", "First"), ("avtr_second", "Second")] {
            cache.hydrate_from_payload(
                "usr_self",
                "https://api.example.test/api/1",
                json!({
                    "id": id,
                    "name": name,
                    "authorId": "usr_author",
                    "authorName": "Author",
                    "releaseStatus": "public",
                    "imageUrl": "image.png",
                    "unityPackages": [{ "assetUrl": "https://example.test/large.bundle" }]
                }),
            );
        }
        cache.run_pending_tasks();

        assert!(cache.entry_count() <= 1);
        let persisted = avatar_cache_get(db.as_ref(), "avtr_second".into())
            .unwrap()
            .expect("summary should be persisted");
        assert_eq!(persisted.name, "Second");
        let retained_full_payload =
            ["avtr_first", "avtr_second"]
                .into_iter()
                .find_map(|avatar_id| {
                    cache.working_value(
                        "usr_self",
                        "https://api.example.test/api/1",
                        avatar_id,
                        true,
                    )
                });
        assert!(retained_full_payload.is_some());
    }
}
