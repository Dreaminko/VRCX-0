use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use vrcx_0_application_realtime::{normalize_v1_notification, normalize_v2_notification};
use vrcx_0_persistence::{
    notifications::notification_friend_requests_sync,
    realtime::{write_realtime_batch, RealtimePersistenceBatch},
    DatabaseService,
};
use vrcx_0_vrchat_client::{
    http_api::{ApiJsonResponse, ApiScope, HttpApiRequestInput},
    notifications::{
        hidden_friend_requests_get_input, notifications_v1_get_input, notifications_v2_get_input,
    },
};

use crate::{Error, Result, RuntimeAuthScope, RuntimeAuthScopeSnapshot, WebClient};

const NOTIFICATION_SYNC_PAGE_SIZE: i64 = 100;
const NOTIFICATION_SYNC_MAX_PAGES: usize = 50;

pub struct NotificationSyncDeps<'a> {
    pub db: &'a DatabaseService,
    pub web: &'a WebClient,
    pub auth_scope: &'a RuntimeAuthScope,
    pub expected_scope: RuntimeAuthScopeSnapshot,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSyncOutcome {
    pub v1_count: usize,
    pub v2_count: usize,
    pub hidden_friend_request_count: usize,
    pub truncated: bool,
}

struct NotificationPages {
    rows: Vec<Value>,
    complete: bool,
}

pub async fn sync_notifications(
    deps: &NotificationSyncDeps<'_>,
) -> Result<NotificationSyncOutcome> {
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
    let endpoint = deps.expected_scope.endpoint.clone();
    let v1_pages = fetch_notification_pages(deps, |n, offset| {
        notifications_v1_get_input(endpoint.clone(), n, offset)
    })
    .await?;
    let v2_pages = fetch_notification_pages(deps, |n, offset| {
        notifications_v2_get_input(endpoint.clone(), n, offset)
    })
    .await?;
    let hidden_pages = fetch_notification_pages(deps, |n, offset| {
        hidden_friend_requests_get_input(endpoint.clone(), n, offset)
    })
    .await?;
    ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;

    let now = Utc::now().to_rfc3339();
    let v1 = v1_pages
        .rows
        .iter()
        .map(|notification| normalize_v1_notification(notification, &now))
        .filter(valid_notification)
        .collect::<Vec<_>>();
    let mut hidden_rows = hidden_pages
        .rows
        .iter()
        .map(|notification| normalize_v1_notification(notification, &now))
        .filter(valid_notification)
        .collect::<Vec<_>>();
    for notification in &mut hidden_rows {
        if let Some(object) = notification.as_object_mut() {
            object.insert("type".into(), Value::String("ignoredFriendRequest".into()));
        }
    }
    let v1_count = v1.len();
    let (visible_friend_requests, regular_v1): (Vec<_>, Vec<_>) = v1
        .into_iter()
        .partition(|notification| notification_type(notification) == "friendRequest");
    let v2_rows = v2_pages
        .rows
        .iter()
        .map(|notification| {
            normalize_v2_notification(notification, &deps.expected_scope.endpoint, &now)
        })
        .filter(valid_notification)
        .collect::<Vec<_>>();

    let hidden_friend_request_count = hidden_rows.len();
    notification_friend_requests_sync(
        deps.db,
        deps.expected_scope.current_user_id.clone(),
        visible_friend_requests,
        v1_pages.complete,
        hidden_rows,
        hidden_pages.complete,
    )?;
    let v2_count = v2_rows.len();
    write_realtime_batch(
        deps.db,
        &deps.expected_scope.current_user_id,
        &RealtimePersistenceBatch {
            notification_v1_upserts: regular_v1,
            notification_v2_upserts: v2_rows,
            ..RealtimePersistenceBatch::default()
        },
    )?;

    Ok(NotificationSyncOutcome {
        v1_count,
        v2_count,
        hidden_friend_request_count,
        truncated: !v1_pages.complete || !v2_pages.complete || !hidden_pages.complete,
    })
}

async fn fetch_notification_pages(
    deps: &NotificationSyncDeps<'_>,
    build_request: impl Fn(i64, i64) -> HttpApiRequestInput,
) -> Result<NotificationPages> {
    let mut rows = Vec::new();
    for page in 0..NOTIFICATION_SYNC_MAX_PAGES {
        ensure_scope_matches(&deps.auth_scope.snapshot(), &deps.expected_scope)?;
        let request = build_request(
            NOTIFICATION_SYNC_PAGE_SIZE,
            page as i64 * NOTIFICATION_SYNC_PAGE_SIZE,
        );
        let response = deps
            .web
            .execute_api(request, ApiScope::Vrchat, deps.db)
            .await?;
        let response = ApiJsonResponse::from(&response);
        if response.is_failure() {
            return Err(Error::Custom(
                response.error_message_with_http_status("VRChat notification sync failed"),
            ));
        }
        let page_rows = response.json.as_array().cloned().ok_or_else(|| {
            Error::Custom("VRChat notification sync returned a non-array response.".into())
        })?;
        let complete = page_rows.len() < NOTIFICATION_SYNC_PAGE_SIZE as usize;
        rows.extend(page_rows);
        if complete {
            return Ok(NotificationPages {
                rows,
                complete: true,
            });
        }
    }
    Ok(NotificationPages {
        rows,
        complete: false,
    })
}

fn valid_notification(notification: &Value) -> bool {
    notification
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|id| !id.trim().is_empty())
        && !notification_type(notification).is_empty()
}

fn notification_type(notification: &Value) -> &str {
    notification
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
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
            "Notification sync authentication scope changed.".into(),
        ))
    }
}
