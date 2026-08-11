use std::sync::{Arc, Mutex};

use vrcx_0_integrations::external_api::{
    self, ExternalApiExecuteResponse, ExternalApiScope, ExternalHttpRequestInput,
    ExternalWebExecuteRequest,
};
use vrcx_0_persistence::cookies;
use vrcx_0_persistence::storage::StorageService;
use vrcx_0_persistence::DatabaseService;
use vrcx_0_vrchat_client::http_api::{
    self, normalize_vrchat_api_endpoint, ApiScope, HttpApiExecuteResponse, HttpApiRequestInput,
};
use vrcx_0_vrchat_client::image_fetcher::ImageFetcher;
use vrcx_0_vrchat_client::realtime::RealtimeConnectionOptions;
use vrcx_0_vrchat_client::web_client::{self as transport, WebExecuteRequest};

use crate::Result;

fn vrchat_config_request_endpoint(input: &HttpApiRequestInput, scope: ApiScope) -> Option<String> {
    if scope != ApiScope::Vrchat
        || input.path.as_deref() != Some("config")
        || input
            .query_params
            .as_ref()
            .is_some_and(|params| !params.is_empty())
    {
        return None;
    }
    Some(normalize_vrchat_api_endpoint(input.endpoint.as_deref()))
}

pub struct RealtimeAuthTokenFetch {
    pub response: HttpApiExecuteResponse,
    pub rejected_pooled_status: Option<i32>,
}

pub struct WebClient {
    inner: transport::WebClient,
    realtime_origin: String,
    image_fetcher: Arc<ImageFetcher>,
    vrchat_config: VrchatConfigCache,
}

struct VrchatConfigSnapshot {
    endpoint: String,
    response: HttpApiExecuteResponse,
}

#[derive(Default)]
struct VrchatConfigCache {
    snapshot: Mutex<Option<VrchatConfigSnapshot>>,
}

impl VrchatConfigCache {
    fn get(&self, endpoint: &str) -> Option<HttpApiExecuteResponse> {
        let endpoint = normalize_vrchat_api_endpoint(Some(endpoint));
        self.snapshot.lock().ok().and_then(|snapshot| {
            snapshot
                .as_ref()
                .filter(|snapshot| snapshot.endpoint == endpoint)
                .map(|snapshot| snapshot.response.clone())
        })
    }

    fn store(&self, endpoint: String, response: HttpApiExecuteResponse) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = Some(VrchatConfigSnapshot { endpoint, response });
        }
    }

    fn clear(&self) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            *snapshot = None;
        }
    }
}

impl WebClient {
    pub fn new(
        storage: &StorageService,
        db: &DatabaseService,
        realtime_origin: String,
        app_version: &str,
    ) -> Result<Self> {
        let proxy_url = crate::proxy::load_proxy_url(storage);
        let persisted_cookies = cookies::get_default_cookies(db)?;
        let inner =
            transport::WebClient::new(proxy_url, persisted_cookies.as_deref(), app_version)?;
        let image_fetcher = Arc::new(ImageFetcher::new(
            inner.cookie_jar(),
            inner.proxy_url(),
            app_version,
        )?);
        Ok(Self {
            inner,
            realtime_origin,
            image_fetcher,
            vrchat_config: VrchatConfigCache::default(),
        })
    }

    pub fn save_cookies(&self, db: &DatabaseService) {
        let jar = self.inner.cookie_jar();
        let Some(maybe_b64) = jar.flush_if_dirty(transport::serialize_cookie_store) else {
            return;
        };
        let Some(b64) = maybe_b64 else {
            jar.mark_dirty();
            return;
        };
        if let Err(error) = cookies::save_default_cookies(db, &b64) {
            jar.mark_dirty();
            tracing::warn!("failed to persist cookies: {error}");
        }
    }

    pub fn proxy_url(&self) -> Option<&str> {
        self.inner.proxy_url()
    }

    pub fn image_fetcher(&self) -> Result<Arc<ImageFetcher>> {
        Ok(Arc::clone(&self.image_fetcher))
    }

    pub fn realtime_connection_options(&self) -> RealtimeConnectionOptions {
        RealtimeConnectionOptions {
            origin: self.realtime_origin.clone(),
            proxy_url: self.inner.proxy_url().map(ToString::to_string),
        }
    }

    pub fn clear_cookies(&self) {
        self.inner.clear_cookies();
    }

    pub fn clear_auth_cookies(&self) {
        self.inner.clear_auth_cookies();
    }

    pub fn cookie_diagnostics(&self) -> serde_json::Value {
        let summary = self.inner.auth_cookie_summary();
        serde_json::json!({
            "cookieCount": summary.total_cookie_count,
            "authCookies": summary
                .auth_cookies
                .into_iter()
                .map(|cookie| serde_json::json!({ "domain": cookie.domain, "expired": cookie.expired }))
                .collect::<Vec<_>>(),
        })
    }

    pub fn auth_cookie_value(&self) -> Option<String> {
        self.inner.auth_cookie_value()
    }

    pub fn get_cookies(&self) -> String {
        self.inner.get_cookies()
    }

    pub fn set_cookies(&self, b64: &str) -> Result<()> {
        Ok(self.inner.set_cookies(b64)?)
    }

    pub async fn execute(&self, request: WebExecuteRequest) -> Result<(i32, String)> {
        Ok(self.inner.execute(request).await?)
    }

    pub async fn execute_external(
        &self,
        request: ExternalWebExecuteRequest,
    ) -> Result<(i32, String)> {
        let follow_redirects = request.follow_redirects;
        let request = external_request_to_transport(request);
        if follow_redirects {
            Ok(self.inner.execute(request).await?)
        } else {
            Ok(self.inner.execute_without_redirects(request).await?)
        }
    }

    pub async fn execute_api(
        &self,
        input: HttpApiRequestInput,
        scope: ApiScope,
        db: &DatabaseService,
    ) -> Result<HttpApiExecuteResponse> {
        let vrchat_config_endpoint = vrchat_config_request_endpoint(&input, scope);
        if let Some(response) = vrchat_config_endpoint
            .as_deref()
            .and_then(|endpoint| self.vrchat_config.get(endpoint))
        {
            return Ok(response);
        }
        let request = self.build_api_request(input, scope)?;
        let (status, data) = self.execute(request).await?;
        let response = self.finish_api_request(status, data, db)?;
        if response.status == 200 {
            if let Some(endpoint) = vrchat_config_endpoint {
                self.vrchat_config.store(endpoint, response.clone());
            }
        }
        Ok(response)
    }

    pub fn vrchat_config_snapshot(&self, endpoint: &str) -> Option<HttpApiExecuteResponse> {
        self.vrchat_config.get(endpoint)
    }

    pub fn clear_vrchat_config_snapshot(&self) {
        self.vrchat_config.clear();
    }

    pub async fn fetch_realtime_auth_token(
        &self,
        endpoint: &str,
        db: &DatabaseService,
    ) -> Result<RealtimeAuthTokenFetch> {
        let input = vrcx_0_vrchat_client::auth::session_get_input(endpoint.to_string());
        let scope = ApiScope::Vrchat;
        let (pooled_status, pooled_data) = self
            .inner
            .execute(self.build_api_request(input.clone(), scope)?)
            .await?;
        if (200..300).contains(&pooled_status) {
            return Ok(RealtimeAuthTokenFetch {
                response: self.finish_api_request(pooled_status, pooled_data, db)?,
                rejected_pooled_status: None,
            });
        }

        let (status, data) = self
            .inner
            .execute_fresh_standard(self.build_api_request(input, scope)?)
            .await?;
        Ok(RealtimeAuthTokenFetch {
            response: self.finish_api_request(status, data, db)?,
            rejected_pooled_status: Some(pooled_status),
        })
    }

    pub async fn execute_external_api(
        &self,
        input: ExternalHttpRequestInput,
        scope: ExternalApiScope,
    ) -> Result<ExternalApiExecuteResponse> {
        self.execute_external_api_with_limit(input, scope, None)
            .await
    }

    pub async fn execute_external_api_limited(
        &self,
        input: ExternalHttpRequestInput,
        scope: ExternalApiScope,
        max_response_bytes: usize,
    ) -> Result<ExternalApiExecuteResponse> {
        self.execute_external_api_with_limit(input, scope, Some(max_response_bytes))
            .await
    }

    async fn execute_external_api_with_limit(
        &self,
        input: ExternalHttpRequestInput,
        scope: ExternalApiScope,
        max_response_bytes: Option<usize>,
    ) -> Result<ExternalApiExecuteResponse> {
        let mut request = external_api::build_web_execute_request(input, scope)
            .map_err(|error| crate::Error::Custom(error.to_string()))?;
        request.response_body_limit = max_response_bytes;
        let (status, data) = self.execute_external(request).await?;
        if status == -1 {
            return Err(crate::Error::Custom(data));
        }
        Ok(external_api::execute_response(status, data, scope))
    }

    fn build_api_request(
        &self,
        input: HttpApiRequestInput,
        scope: ApiScope,
    ) -> Result<WebExecuteRequest> {
        http_api::build_web_execute_request(input, scope)
            .map_err(|error| crate::Error::Custom(error.to_string()))
    }

    fn finish_api_request(
        &self,
        status: i32,
        data: String,
        db: &DatabaseService,
    ) -> Result<HttpApiExecuteResponse> {
        self.save_cookies(db);
        if status == -1 {
            return Err(crate::Error::Custom(data));
        }
        Ok(http_api::execute_response(status, data))
    }
}

fn external_request_to_transport(request: ExternalWebExecuteRequest) -> WebExecuteRequest {
    WebExecuteRequest {
        url: request.url,
        method: request.method,
        headers: request.headers,
        body: request.body,
        upload: vrcx_0_vrchat_client::web_client::WebUploadMode::None,
        response_body_limit: request.response_body_limit,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;
    use vrcx_0_vrchat_client::auth::config_get_input;

    use super::*;

    #[test]
    fn config_snapshot_matching_is_limited_to_plain_vrchat_config_gets() {
        let input = config_get_input("https://api.example.test/api/1/".into());
        assert_eq!(
            vrchat_config_request_endpoint(&input, ApiScope::Vrchat).as_deref(),
            Some("https://api.example.test/api/1")
        );
        assert_eq!(
            vrchat_config_request_endpoint(&input, ApiScope::VrchatMedia),
            None
        );

        let mut queried_input = input.clone();
        queried_input.query_params = Some(HashMap::from([("x".into(), json!(1))]));
        assert_eq!(
            vrchat_config_request_endpoint(&queried_input, ApiScope::Vrchat),
            None
        );

        let mut other_input = input;
        other_input.path = Some("auth/user".into());
        assert_eq!(
            vrchat_config_request_endpoint(&other_input, ApiScope::Vrchat),
            None
        );
    }

    #[test]
    fn config_cache_is_endpoint_scoped_and_explicitly_cleared() {
        let cache = VrchatConfigCache::default();
        let response = http_api::execute_response(200, r#"{"sdkUnityVersion":"test"}"#.into());

        cache.store("https://api.example.test/api/1".into(), response.clone());

        assert_eq!(
            cache
                .get("https://api.example.test/api/1/")
                .map(|snapshot| snapshot.data),
            Some(response.data)
        );
        assert!(cache.get("https://api.other.test/api/1").is_none());

        cache.clear();

        assert!(cache.get("https://api.example.test/api/1").is_none());
    }
}
