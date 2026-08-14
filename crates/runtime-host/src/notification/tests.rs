use std::{path::PathBuf, sync::Arc};

use serde_json::json;
use vrcx_0_application_activity::{
    OverlayActivityActorRelation, OverlayActivityCategory, OverlayActivityContent,
    OverlayActivityDelivery, OverlayActivityEntry,
};
use vrcx_0_persistence::DatabaseService;

use super::{
    generic_webhook_payload, normalize_avatar_image_url_128, parse_webhook_fields,
    RealtimeUserImageResolverSlot, RenderedNotification,
};

#[test]
fn generic_webhook_payload_exposes_location_id_and_local_time() {
    let payload = generic_webhook_payload(
        &delivery(),
        &rendered(),
        &["location".into(), "locationId".into(), "localTime".into()],
    );

    assert_eq!(
        payload.get("location").and_then(|value| value.as_str()),
        Some("Named World public")
    );
    assert_eq!(
        payload.get("locationId").and_then(|value| value.as_str()),
        Some("wrld_named:123")
    );
    let local_time = payload
        .get("localTime")
        .and_then(|value| value.as_str())
        .expect("localTime");
    assert_eq!(local_time.len(), "2026-06-18 17:30:00".len());
    assert!(payload.get("timestamp").is_none());
    assert!(payload.get("worldName").is_none());
}

#[test]
fn generic_webhook_fields_ignore_localized_names() {
    let fields = parse_webhook_fields(r#"["locationId","位置","タイトル"]"#);
    let payload = generic_webhook_payload(&delivery(), &rendered(), &fields);

    assert_eq!(payload.as_object().unwrap().len(), 1);
    assert_eq!(
        payload.get("locationId").and_then(|value| value.as_str()),
        Some("wrld_named:123")
    );
    assert!(payload.get("位置").is_none());
    assert!(payload.get("タイトル").is_none());
}

fn rendered() -> RenderedNotification {
    RenderedNotification {
        title: "Traveler".into(),
        body: "joined Named World".into(),
        text: "Traveler joined Named World".into(),
        display_location: "Named World public".into(),
        image_url: String::new(),
    }
}

fn delivery() -> OverlayActivityDelivery {
    OverlayActivityDelivery {
        entry: OverlayActivityEntry {
            sequence: 1,
            source_id: "game-log:join".into(),
            activity_type: "OnPlayerJoined".into(),
            category: OverlayActivityCategory::CurrentInstance,
            created_at: "2026-06-18T08:30:00.000Z".into(),
            actor_user_id: "usr_traveler".into(),
            actor_display_name: "Traveler".into(),
            content: OverlayActivityContent {
                location: "wrld_named:123".into(),
                world_id: "wrld_named".into(),
                display_location: "Named World public".into(),
                world_name: "Named World".into(),
                ..OverlayActivityContent::default()
            },
            actor_relation: OverlayActivityActorRelation::None,
            payload: json!({}),
        },
        desktop: false,
        vr: false,
        hmd: false,
        webhook: true,
        tts: false,
    }
}

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
            "vrcx-0-notification-{name}-{}-{nonce}",
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

fn test_realtime_runtime(
    name: &str,
) -> (
    TestDir,
    Arc<vrcx_0_application_realtime::RealtimeHostRuntime>,
    Arc<DatabaseService>,
    Arc<vrcx_0_application_core::WebClient>,
) {
    let dir = TestDir::new(name);
    let db = Arc::new(DatabaseService::new(&dir.path.join("VRCX-0.sqlite3")).unwrap());
    let storage =
        vrcx_0_persistence::storage::StorageService::new(&dir.path.join("storage.json")).unwrap();
    let web = Arc::new(
        vrcx_0_application_core::WebClient::new(
            &storage,
            db.as_ref(),
            "wss://pipeline.vrchat.cloud".to_string(),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap(),
    );
    let world_cache = Arc::new(vrcx_0_application_core::WorldCache::new(
        Arc::clone(&db),
        512,
        std::time::Duration::from_secs(30 * 60),
    ));
    let event_bus = vrcx_0_application_core::RuntimeEventBus::new();
    let runtime = Arc::new(vrcx_0_application_realtime::RealtimeHostRuntime::new(
        vrcx_0_application_realtime::RealtimeHostRuntimeDeps {
            db: Arc::clone(&db),
            web: Arc::clone(&web),
            event_bus: event_bus.clone(),
            backend_status: vrcx_0_application_core::BackendRuntimeStatusPublisher::new(
                vrcx_0_application_core::BackendRuntime::new(
                    vrcx_0_application_core::RuntimeHostProfile::Desktop,
                ),
                event_bus.clone(),
            ),
            friend_projection_sink: vrcx_0_application_realtime::FriendProjectionSink::new(
                event_bus.clone(),
                None,
            ),
            sync: vrcx_0_application_core::RuntimeSyncEngine::new(),
            tasks: vrcx_0_application_core::TaskSupervisor::new(),
            session: vrcx_0_application_core::HostSessionRuntime::new(),
            auth_scope: vrcx_0_application_core::RuntimeAuthScope::new(),
            remote_mutations: Arc::new(vrcx_0_application_core::RemoteMutationGate::default()),
            local_game_context: Arc::new(
                vrcx_0_application_core::UnavailableLocalGameContextSource,
            ),
            activity_sink: None,
            world_cache,
            print_cleanup: Arc::new(vrcx_0_application_core::NoopPrintCleanupInputSink),
            friend_note_change_sink: None,
            current_user_snapshot_sink: None,
        },
    ));
    (dir, runtime, db, web)
}

#[test]
fn realtime_image_resolver_reads_the_realtime_cache() {
    let (_dir, runtime, _active_session) =
        vrcx_0_application_realtime::test_support::runtime_with_active_session(
            "actor-image-cache-hit",
        )
        .unwrap();
    let runtime = runtime.runtime();
    let endpoint = "https://api.vrchat.cloud/api/1";
    runtime.ingest_user_facts(vec![json!({
        "user": {
            "id": "usr_traveler",
            "displayName": "Traveler",
            "userIcon": "https://api.vrchat.cloud/api/1/file/file_1234abcd-0000-1111-2222-abcdefabcdef/2/file",
        },
        "source": "test",
        "isFriend": true,
    })]);
    let resolver = RealtimeUserImageResolverSlot::default();
    resolver.set(runtime);
    let image_url = resolver
        .cached_url(endpoint, "usr_traveler", true)
        .map(|url| normalize_avatar_image_url_128(&url, endpoint));

    assert_eq!(
        image_url.as_deref(),
        Some(
            "https://api.vrchat.cloud/api/1/image/file_1234abcd-0000-1111-2222-abcdefabcdef/2/128"
        )
    );
}

#[test]
fn realtime_image_resolver_returns_none_when_endpoint_is_missing() {
    let (_dir, _runtime, _db, _web) = test_realtime_runtime("actor-image-cache-miss");
    let resolver = RealtimeUserImageResolverSlot::default();
    let image_url = resolver.cached_url("", "usr_traveler", true);

    assert_eq!(image_url, None);
}

#[test]
fn realtime_user_image_resolver_does_not_retain_runtime() {
    let (_dir, runtime, _db, _web) = test_realtime_runtime("resolver-weak-runtime");
    let weak_runtime = Arc::downgrade(&runtime);
    let resolver = RealtimeUserImageResolverSlot::default();

    resolver.set(&runtime);
    drop(runtime);

    assert!(weak_runtime.upgrade().is_none());
    assert_eq!(resolver.cached_url("", "usr_missing", true), None);
}
