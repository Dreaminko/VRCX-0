use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;

use moka::policy::EvictionPolicy;
use moka::sync::Cache;
use serde_json::Value;
use vrcx_0_application_core::WebClient;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::ApiScope;
use vrcx_0_vrchat_client::users::user_get_input;

const FETCH_TIMEOUT_MS: u64 = 5_000;
const SUCCESS_TTL: Duration = Duration::from_secs(15 * 60);
const FAILURE_TTL: Duration = Duration::from_secs(60);
const SUCCESS_CAPACITY: u64 = 128;
const FAILURE_CAPACITY: u64 = 32;

pub struct UserImageCache {
    success: Cache<String, String>,
    failures: Cache<String, ()>,
    inflight: Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>,
}

impl Default for UserImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl UserImageCache {
    pub fn new() -> Self {
        Self {
            success: Cache::builder()
                .max_capacity(SUCCESS_CAPACITY)
                .time_to_live(SUCCESS_TTL)
                .eviction_policy(EvictionPolicy::lru())
                .build(),
            failures: Cache::builder()
                .max_capacity(FAILURE_CAPACITY)
                .time_to_live(FAILURE_TTL)
                .eviction_policy(EvictionPolicy::lru())
                .build(),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub async fn resolve(
        &self,
        web: &WebClient,
        db: &DatabaseService,
        endpoint: &str,
        user_id: &str,
        allow_user_icon: bool,
    ) -> Option<String> {
        let user_id = user_id.trim();
        if !user_id.starts_with("usr_") {
            return None;
        }
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            return None;
        }
        let key = cache_key(user_id, allow_user_icon);
        if let Some(url) = self.cached(&key) {
            return Some(url);
        }
        if self.recently_failed(&key) {
            return None;
        }
        let inflight = self.inflight_lock(&key);
        let _guard = inflight.lock().await;
        if let Some(url) = self.cached(&key) {
            return Some(url);
        }
        if self.recently_failed(&key) {
            return None;
        }
        match fetch_user_image(web, db, endpoint, user_id, allow_user_icon).await {
            Some(url) => {
                self.store(&key, &url);
                Some(url)
            }
            None => {
                self.record_failure(&key);
                None
            }
        }
    }

    pub fn cached_url(&self, user_id: &str, allow_user_icon: bool) -> Option<String> {
        let user_id = user_id.trim();
        if !user_id.starts_with("usr_") {
            return None;
        }
        self.cached(&cache_key(user_id, allow_user_icon))
    }

    fn cached(&self, key: &str) -> Option<String> {
        self.success.get(key)
    }

    fn store(&self, key: &str, url: &str) {
        self.success.insert(key.to_string(), url.to_string());
    }

    fn recently_failed(&self, key: &str) -> bool {
        self.failures.get(key).is_some()
    }

    fn record_failure(&self, key: &str) {
        self.failures.insert(key.to_string(), ());
    }

    fn inflight_lock(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut map = lock(&self.inflight);
        if let Some(existing) = map.get(key).and_then(Weak::upgrade) {
            return existing;
        }
        map.retain(|_, weak| weak.strong_count() > 0);
        let guard = Arc::new(tokio::sync::Mutex::new(()));
        map.insert(key.to_string(), Arc::downgrade(&guard));
        guard
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn cache_key(user_id: &str, allow_user_icon: bool) -> String {
    format!("{user_id}|{}", allow_user_icon as u8)
}

async fn fetch_user_image(
    web: &WebClient,
    db: &DatabaseService,
    endpoint: &str,
    user_id: &str,
    allow_user_icon: bool,
) -> Option<String> {
    let (_, request) = user_get_input(endpoint.to_string(), user_id.to_string()).ok()?;
    let response = tokio::time::timeout(
        Duration::from_millis(FETCH_TIMEOUT_MS),
        web.execute_api(request, ApiScope::Vrchat, db),
    )
    .await
    .ok()?
    .ok()?;
    if !(200..=299).contains(&response.status) {
        return None;
    }
    let user = serde_json::from_str::<Value>(&response.data).ok()?;
    image_url_from_user(&user, allow_user_icon, endpoint)
}

pub struct UserImageSources<'a> {
    pub user_icon: &'a str,
    pub profile_pic_override_thumbnail: &'a str,
    pub profile_pic_override: &'a str,
    pub thumbnail_url: &'a str,
    pub current_avatar_thumbnail_image_url: &'a str,
    pub current_avatar_image_url: &'a str,
}

pub fn user_image_url_128(
    sources: UserImageSources<'_>,
    allow_user_icon: bool,
    endpoint: &str,
) -> Option<String> {
    let url = [
        allow_user_icon.then_some(sources.user_icon),
        Some(sources.profile_pic_override_thumbnail),
        Some(sources.profile_pic_override),
        Some(sources.thumbnail_url),
        Some(sources.current_avatar_thumbnail_image_url),
        Some(sources.current_avatar_image_url),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|url| !url.is_empty())?;
    Some(normalize_avatar_image_url_128(url, endpoint))
}

pub fn normalize_avatar_image_url_128(url: &str, endpoint: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return String::new();
    }
    file_url_to_image_url_128(url, endpoint).unwrap_or_else(|| url.replace("/256", "/128"))
}

fn image_url_from_user(user: &Value, allow_user_icon: bool, endpoint: &str) -> Option<String> {
    let object = user.as_object()?;
    user_image_url_128(
        UserImageSources {
            user_icon: json_string_field(object, "userIcon"),
            profile_pic_override_thumbnail: json_string_field(
                object,
                "profilePicOverrideThumbnail",
            ),
            profile_pic_override: json_string_field(object, "profilePicOverride"),
            thumbnail_url: json_string_field(object, "thumbnailUrl"),
            current_avatar_thumbnail_image_url: json_string_field(
                object,
                "currentAvatarThumbnailImageUrl",
            ),
            current_avatar_image_url: json_string_field(object, "currentAvatarImageUrl"),
        },
        allow_user_icon,
        endpoint,
    )
}

fn file_url_to_image_url_128(url: &str, endpoint: &str) -> Option<String> {
    let normalized = url.trim().trim_end_matches('/');
    let path = normalized.split('?').next().unwrap_or(normalized);
    let segments = path.split('/').collect::<Vec<_>>();
    let file_index = segments
        .windows(2)
        .position(|pair| pair[0] == "file" && pair[1].starts_with("file_"))?;
    let file_id = segments.get(file_index + 1)?;
    let version = segments.get(file_index + 2)?;
    if !is_vrchat_file_id(file_id) || !version.chars().all(|value| value.is_ascii_digit()) {
        return None;
    }
    match segments.get(file_index + 3) {
        None => {}
        Some(&"file") if file_index + 4 == segments.len() => {}
        _ => return None,
    }
    let endpoint = endpoint.trim().trim_end_matches('/');
    if endpoint.is_empty() {
        return None;
    }
    Some(format!("{endpoint}/image/{file_id}/{version}/128"))
}

fn is_vrchat_file_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("file_") else {
        return false;
    };
    !suffix.is_empty()
        && suffix
            .chars()
            .all(|value| value.is_ascii_hexdigit() || value == '-')
}

fn json_string_field<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    object.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_image_cache_uses_bounded_moka_storage() {
        let cache = UserImageCache::new();
        assert_eq!(
            cache.success.policy().max_capacity(),
            Some(SUCCESS_CAPACITY)
        );
        assert_eq!(cache.success.policy().time_to_live(), Some(SUCCESS_TTL));
        assert_eq!(
            cache.failures.policy().max_capacity(),
            Some(FAILURE_CAPACITY)
        );
        assert_eq!(cache.failures.policy().time_to_live(), Some(FAILURE_TTL));

        for index in 0..SUCCESS_CAPACITY * 2 {
            let key = format!("usr_{index}|1");
            cache.store(&key, &format!("https://img.example/{index}"));
            cache.record_failure(&key);
        }
        cache.success.run_pending_tasks();
        cache.failures.run_pending_tasks();

        assert!(cache.success.entry_count() <= SUCCESS_CAPACITY);
        assert!(cache.failures.entry_count() <= FAILURE_CAPACITY);
    }

    #[test]
    fn prefers_user_icon_when_allowed() {
        let user = json!({
            "userIcon": "https://api.vrchat.cloud/api/1/file/file_1234abcd-0000-1111-2222-abcdefabcdef/2/file",
            "profilePicOverride": "https://img/override.png",
            "currentAvatarThumbnailImageUrl": "https://img/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, true, "https://api.vrchat.cloud/api/1").as_deref(),
            Some("https://api.vrchat.cloud/api/1/image/file_1234abcd-0000-1111-2222-abcdefabcdef/2/128")
        );
    }

    #[test]
    fn skips_user_icon_when_not_allowed() {
        let user = json!({
            "userIcon": "https://img/icon.png",
            "profilePicOverrideThumbnail": "https://img/override/256",
            "profilePicOverride": "https://img/override.png",
            "currentAvatarThumbnailImageUrl": "https://img/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, false, "https://api.vrchat.cloud/api/1").as_deref(),
            Some("https://img/override/128")
        );
    }

    #[test]
    fn falls_back_to_avatar_thumbnail() {
        let user = json!({
            "userIcon": "",
            "profilePicOverride": "  ",
            "currentAvatarThumbnailImageUrl": "https://img/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, true, "https://api.vrchat.cloud/api/1").as_deref(),
            Some("https://img/avatar.png")
        );
    }

    #[test]
    fn converts_avatar_image_file_url_to_128() {
        let user = json!({
            "currentAvatarImageUrl": "https://api.vrchat.cloud/api/1/file/file_abcdefab-0000-1111-2222-abcdefabcdef/7/file",
        });
        assert_eq!(
            image_url_from_user(&user, false, "https://api.vrchat.cloud/api/1").as_deref(),
            Some("https://api.vrchat.cloud/api/1/image/file_abcdefab-0000-1111-2222-abcdefabcdef/7/128")
        );
    }

    #[test]
    fn keeps_unrecognized_image_url() {
        let user = json!({
            "thumbnailUrl": "https://img.example/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, false, "https://api.vrchat.cloud/api/1").as_deref(),
            Some("https://img.example/avatar.png")
        );
    }

    #[test]
    fn returns_none_without_any_image() {
        let user = json!({ "displayName": "Nobody" });
        assert!(image_url_from_user(&user, true, "https://api.vrchat.cloud/api/1").is_none());
    }
}
