use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use serde_json::Value;
use vrcx_0_application::WebClient;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::ApiScope;
use vrcx_0_vrchat_client::users::user_get_input;

const FETCH_TIMEOUT_MS: u64 = 5_000;
const SUCCESS_TTL: Duration = Duration::from_secs(15 * 60);
const FAILURE_TTL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct UserImageCache {
    success: Mutex<HashMap<String, (String, Instant)>>,
    failures: Mutex<HashMap<String, Instant>>,
    inflight: Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>,
}

impl UserImageCache {
    pub fn new() -> Self {
        Self::default()
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
        let mut map = lock(&self.success);
        let (url, at) = map.get(key)?;
        if at.elapsed() >= SUCCESS_TTL {
            map.remove(key);
            return None;
        }
        Some(url.clone())
    }

    fn store(&self, key: &str, url: &str) {
        let mut map = lock(&self.success);
        map.retain(|_, (_, at)| at.elapsed() < SUCCESS_TTL);
        map.insert(key.to_string(), (url.to_string(), Instant::now()));
    }

    fn recently_failed(&self, key: &str) -> bool {
        lock(&self.failures)
            .get(key)
            .is_some_and(|at| at.elapsed() < FAILURE_TTL)
    }

    fn record_failure(&self, key: &str) {
        let mut map = lock(&self.failures);
        map.retain(|_, at| at.elapsed() < FAILURE_TTL);
        map.insert(key.to_string(), Instant::now());
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
    image_url_from_user(&user, allow_user_icon)
}

fn image_url_from_user(user: &Value, allow_user_icon: bool) -> Option<String> {
    let object = user.as_object()?;
    [
        allow_user_icon.then(|| string_field(object, "userIcon")),
        Some(string_field(object, "profilePicOverride")),
        Some(string_field(object, "currentAvatarThumbnailImageUrl")),
    ]
    .into_iter()
    .flatten()
    .find(|url| !url.is_empty())
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_user_icon_when_allowed() {
        let user = json!({
            "userIcon": "https://img/icon.png",
            "profilePicOverride": "https://img/override.png",
            "currentAvatarThumbnailImageUrl": "https://img/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, true).as_deref(),
            Some("https://img/icon.png")
        );
    }

    #[test]
    fn skips_user_icon_when_not_allowed() {
        let user = json!({
            "userIcon": "https://img/icon.png",
            "profilePicOverride": "https://img/override.png",
            "currentAvatarThumbnailImageUrl": "https://img/avatar.png",
        });
        assert_eq!(
            image_url_from_user(&user, false).as_deref(),
            Some("https://img/override.png")
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
            image_url_from_user(&user, true).as_deref(),
            Some("https://img/avatar.png")
        );
    }

    #[test]
    fn returns_none_without_any_image() {
        let user = json!({ "displayName": "Nobody" });
        assert!(image_url_from_user(&user, true).is_none());
    }
}
