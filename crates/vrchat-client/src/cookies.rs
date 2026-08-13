// Portions of this file are derived from reqwest_cookie_store
// (https://github.com/pfernie/reqwest_cookie_store), commit
// c906d634bad35b9f79abdc36f939979e39d1a2e1, src/lib.rs.
//
// Copyright (c) 2017 Patrick Fernie <patrick.fernie@gmail.com>
//
// The original work is dual-licensed under either the MIT License or the Apache
// License, Version 2.0, at your option. It is used here under the MIT License,
// reproduced below.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use cookie_store::{CookieStore, RawCookie, RawCookieParseError};
use reqwest::header::HeaderValue;
use url::Url;

pub struct CookieJarInner {
    dirty: bool,
    store: CookieStore,
}

pub struct CookieJar(std::sync::Mutex<CookieJarInner>);

impl CookieJar {
    pub fn new(store: CookieStore) -> Self {
        Self(std::sync::Mutex::new(CookieJarInner {
            dirty: false,
            store,
        }))
    }

    pub fn mark_dirty(&self) {
        self.map(|inner| inner.dirty = true);
    }

    pub fn clear_dirty(&self) {
        self.map(|inner| inner.dirty = false);
    }

    pub fn flush_if_dirty<F, R>(&self, closure: F) -> Option<R>
    where
        F: FnOnce(&CookieStore) -> R,
    {
        self.map(|inner| {
            inner.dirty.then(|| {
                inner.dirty = false;
                closure(&inner.store)
            })
        })
    }

    pub fn read_with<F, R>(&self, closure: F) -> R
    where
        F: FnOnce(&CookieStore) -> R,
    {
        self.map(|inner| closure(&inner.store))
    }

    pub fn update<F, R>(&self, closure: F) -> R
    where
        F: FnOnce(&mut CookieStore) -> R,
    {
        self.map(|inner| {
            inner.dirty = true;
            closure(&mut inner.store)
        })
    }

    pub fn map<F, R>(&self, closure: F) -> R
    where
        F: FnOnce(&mut CookieJarInner) -> R,
    {
        let mut guard = self.0.lock().unwrap();
        closure(&mut guard)
    }
}

impl reqwest::cookie::CookieStore for CookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, url: &Url) {
        let mut guard = self.0.lock().unwrap();
        guard.dirty = true;
        set_cookies(&mut guard.store, cookie_headers, url);
    }

    fn cookies(&self, url: &Url) -> Option<HeaderValue> {
        let guard = self.0.lock().unwrap();
        let s = guard
            .store
            .get_request_values(url)
            .map(|(name, value)| format!("{}={}", name, value))
            .collect::<Vec<_>>()
            .join("; ");

        if s.is_empty() {
            return None;
        }

        HeaderValue::try_from(s).ok()
    }
}

fn set_cookies(
    cookie_store: &mut CookieStore,
    cookie_headers: &mut dyn Iterator<Item = &HeaderValue>,
    url: &Url,
) {
    let cookies = cookie_headers.filter_map(|val| {
        std::str::from_utf8(val.as_bytes())
            .map_err(RawCookieParseError::from)
            .and_then(RawCookie::parse)
            .map(|c| c.into_owned())
            .ok()
    });
    cookie_store.store_response_cookies(cookies, url);
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CookieEntry {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
}

pub fn serialize_cookie_store(store: &CookieStore) -> Option<String> {
    let mut json = Vec::new();
    #[allow(deprecated)]
    store
        .save_incl_expired_and_nonpersistent_json(&mut json)
        .ok()?;
    Some(B64.encode(json))
}

pub fn deserialize_cookie_store(b64: &str) -> Option<CookieStore> {
    let bytes = B64.decode(b64).ok()?;
    #[allow(deprecated)]
    CookieStore::load_json_all(&*bytes).ok()
}

pub fn deserialize_legacy_cookie_entries(b64: &str) -> Option<Vec<CookieEntry>> {
    let bytes = B64.decode(b64).ok()?;
    serde_json::from_slice::<Vec<CookieEntry>>(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use reqwest::cookie::CookieStore as ReqwestCookieStore;

    use super::*;

    fn cookie_header(jar: &CookieJar, url: &str) -> Option<String> {
        ReqwestCookieStore::cookies(jar, &Url::parse(url).unwrap())
            .map(|value| value.to_str().unwrap().to_string())
    }

    #[test]
    fn fresh_jar_is_clean() {
        let jar = CookieJar::new(CookieStore::default());
        assert!(jar.flush_if_dirty(|_| ()).is_none());
    }

    #[test]
    fn update_marks_dirty_and_flush_clears_once() {
        let jar = CookieJar::new(CookieStore::default());
        jar.update(|_| {});
        assert!(jar.flush_if_dirty(|_| ()).is_some());
        assert!(jar.flush_if_dirty(|_| ()).is_none());
    }

    #[test]
    fn mark_dirty_rearms_after_flush() {
        let jar = CookieJar::new(CookieStore::default());
        jar.update(|_| {});
        assert!(jar.flush_if_dirty(|_| ()).is_some());
        jar.mark_dirty();
        assert!(jar.flush_if_dirty(|_| ()).is_some());
    }

    #[test]
    fn clear_dirty_suppresses_next_flush() {
        let jar = CookieJar::new(CookieStore::default());
        jar.update(|_| {});
        jar.clear_dirty();
        assert!(jar.flush_if_dirty(|_| ()).is_none());
    }

    #[test]
    fn response_cookies_follow_vrchat_domain_path_and_secure_scope() {
        let jar = CookieJar::new(CookieStore::default());
        let response_url = Url::parse("https://api.vrchat.cloud/api/1/auth/user").unwrap();
        let headers = [
            HeaderValue::from_static("auth=session-token; Domain=vrchat.cloud; Path=/api/; Secure"),
            HeaderValue::from_static(
                "twoFactorAuth=remember-token; Domain=api.vrchat.cloud; Path=/; Secure",
            ),
        ];

        ReqwestCookieStore::set_cookies(&jar, &mut headers.iter(), &response_url);

        let api_header = cookie_header(&jar, "https://api.vrchat.cloud/api/1/auth/user").unwrap();
        assert!(api_header.contains("auth=session-token"));
        assert!(api_header.contains("twoFactorAuth=remember-token"));
        assert_eq!(
            cookie_header(&jar, "https://api.vrchat.cloud/home").as_deref(),
            Some("twoFactorAuth=remember-token")
        );
        assert!(cookie_header(&jar, "http://api.vrchat.cloud/api/1/auth/user").is_none());
        assert!(cookie_header(&jar, "https://example.com/api/1/auth/user").is_none());
    }

    #[test]
    fn malformed_set_cookie_does_not_discard_valid_neighbors() {
        let jar = CookieJar::new(CookieStore::default());
        let response_url = Url::parse("https://api.vrchat.cloud/api/1/auth/user").unwrap();
        let headers = [
            HeaderValue::from_static("auth=session-token; Path=/; Secure"),
            HeaderValue::from_static(";"),
            HeaderValue::from_static("twoFactorAuth=remember-token; Path=/; Secure"),
        ];

        ReqwestCookieStore::set_cookies(&jar, &mut headers.iter(), &response_url);

        let header = cookie_header(&jar, response_url.as_str()).unwrap();
        assert!(header.contains("auth=session-token"));
        assert!(header.contains("twoFactorAuth=remember-token"));
    }

    #[test]
    fn session_cookies_survive_store_serialization_round_trip() {
        let jar = CookieJar::new(CookieStore::default());
        let response_url = Url::parse("https://api.vrchat.cloud/api/1/auth/user").unwrap();
        let headers = [
            HeaderValue::from_static("auth=session-token; Domain=.vrchat.cloud; Path=/; Secure"),
            HeaderValue::from_static(
                "twoFactorAuth=remember-token; Domain=.vrchat.cloud; Path=/; Secure",
            ),
        ];
        ReqwestCookieStore::set_cookies(&jar, &mut headers.iter(), &response_url);

        let encoded = jar
            .read_with(serialize_cookie_store)
            .expect("cookie store should serialize");
        let restored = deserialize_cookie_store(&encoded).expect("cookie store should restore");
        let restored = CookieJar::new(restored);

        let header = cookie_header(&restored, response_url.as_str()).unwrap();
        assert!(header.contains("auth=session-token"));
        assert!(header.contains("twoFactorAuth=remember-token"));
    }

    #[test]
    fn legacy_cookie_payload_uses_original_pascal_case_shape() {
        let payload =
            B64.encode(br#"[{"Name":"auth","Value":"token","Domain":".vrchat.cloud","Path":"/"}]"#);

        let entries = deserialize_legacy_cookie_entries(&payload).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "auth");
        assert_eq!(entries[0].value, "token");
        assert_eq!(entries[0].domain, ".vrchat.cloud");
        assert_eq!(entries[0].path, "/");
    }
}
