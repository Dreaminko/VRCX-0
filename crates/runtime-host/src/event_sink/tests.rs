use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use vrcx_0_application_core::{
    BackendRuntime, BackendRuntimeTelemetry, BackendRuntimeTelemetryKind, RuntimeEventBus,
    RuntimeEventPayload, RuntimeEventSink,
};
use vrcx_0_core::realtime::{RealtimeWsStatus, RealtimeWsStatusPayload};

use super::*;

#[derive(Clone, Debug, PartialEq)]
struct RecordedEvent {
    name: String,
    payload: Value,
}

#[derive(Clone, Default)]
struct RecordingSink {
    events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl RecordingSink {
    fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl RuntimeEventSink for RecordingSink {
    fn emit(&self, event: &str, payload: Value, _typed_payload: &dyn std::any::Any) {
        self.events.lock().unwrap().push(RecordedEvent {
            name: event.to_string(),
            payload,
        });
    }
}

#[derive(Default)]
struct RecordingProfileExtension {
    now_playing: Mutex<Value>,
}

#[derive(serde::Serialize, specta::Type)]
struct ProfileTestEvent {
    payload: Value,
}

impl RuntimeEventPayload for ProfileTestEvent {
    const EVENT_NAME: &'static str = "profileTestEvent";
}

impl RecordingProfileExtension {
    fn now_playing(&self) -> Value {
        self.now_playing.lock().unwrap().clone()
    }
}

impl RuntimeHostProfileExtension for RecordingProfileExtension {
    fn observe_runtime_event(&self, payload: &dyn std::any::Any) {
        if let Some(event) = payload.downcast_ref::<ProfileTestEvent>() {
            *self.now_playing.lock().unwrap() = event.payload.clone();
        }
    }
}

#[test]
fn ordinary_event_is_forwarded_unchanged_before_one_derived_telemetry_event() {
    let backend_runtime = BackendRuntime::new();
    let recording = RecordingSink::default();
    let sink = RuntimeHostEventSink::new(backend_runtime, None, recording.clone());
    let typed_payload = RealtimeWsStatusPayload {
        status: RealtimeWsStatus::Connected,
        websocket_domain: "pipeline.vrchat.cloud".into(),
        at: "2026-01-01T00:00:00Z".into(),
        client_run_id: None,
        generation: None,
        session_generation: None,
        reason: None,
        status_code: None,
    };
    let payload = serde_json::to_value(&typed_payload).unwrap();

    sink.emit("realtimeWsStatus", payload.clone(), &typed_payload);

    let events = recording.events();
    assert_eq!(events.len(), 3);
    assert_eq!(
        events[0],
        RecordedEvent {
            name: "realtimeWsStatus".to_string(),
            payload,
        }
    );
    assert_eq!(events[1].name, "realtimeProjectionSync");
    assert_eq!(events[1].payload["snapshot"]["wsStatus"], "connected");
    assert_eq!(events[2].name, "backendRuntimeTelemetry");
    assert_eq!(events[2].payload["kind"], "wsStatus");
    assert_eq!(events[2].payload["detail"], "connected");
    assert_eq!(events[2].payload["snapshot"]["wsStatus"], "connected");
}

#[test]
fn typed_backend_runtime_telemetry_passes_through_without_observation() {
    let backend_runtime = BackendRuntime::new();
    let recording = RecordingSink::default();
    let sink = RuntimeHostEventSink::new(backend_runtime.clone(), None, recording.clone());
    let typed_payload = BackendRuntimeTelemetry {
        kind: BackendRuntimeTelemetryKind::RuntimeStarted,
        detail: "ready".into(),
        snapshot: backend_runtime.snapshot(),
    };
    let payload = serde_json::to_value(&typed_payload).unwrap();

    sink.emit("backendRuntimeTelemetry", payload.clone(), &typed_payload);

    let events = recording.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].name, "realtimeProjectionSync");
    assert_eq!(events[0].payload["snapshot"], payload["snapshot"]);
    assert_eq!(
        events[1],
        RecordedEvent {
            name: "backendRuntimeTelemetry".into(),
            payload,
        }
    );
}

#[test]
fn typed_runtime_observation_is_normalized_into_snapshot_telemetry() {
    let bus = RuntimeEventBus::new();
    let recording = RecordingSink::default();
    bus.set_sink(RuntimeHostEventSink::new(
        BackendRuntime::new(),
        None,
        recording.clone(),
    ));

    bus.emit_ws_message_observed("notification");

    let events = recording.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].name, "realtimeProjectionSync");
    let event = &events[1];
    assert_eq!(event.name, "backendRuntimeTelemetry");
    assert_eq!(event.payload["kind"], "wsMessage");
    assert_eq!(event.payload["detail"], "notification");
    assert_eq!(
        event.payload["snapshot"]["wsMessageCounts"]["notification"],
        1
    );
}

#[test]
fn event_is_observed_by_context_and_forwarded() {
    let context = Arc::new(RecordingProfileExtension::default());
    let recording = RecordingSink::default();
    let sink = RuntimeHostEventSink::new(
        BackendRuntime::new(),
        Some(context.clone()),
        recording.clone(),
    );
    let typed_payload = ProfileTestEvent {
        payload: json!({ "name": "Test Track", "position": 42 }),
    };
    let payload = serde_json::to_value(&typed_payload).unwrap();

    sink.emit(
        ProfileTestEvent::EVENT_NAME,
        payload.clone(),
        &typed_payload,
    );

    assert_eq!(context.now_playing()["name"], "Test Track");
    assert_eq!(context.now_playing()["position"], 42);
    assert_eq!(
        recording.events(),
        vec![RecordedEvent {
            name: ProfileTestEvent::EVENT_NAME.to_string(),
            payload,
        }]
    );
}

#[test]
fn event_sink_does_not_retain_profile_extension() {
    let context = Arc::new(RecordingProfileExtension::default());
    let weak_context = Arc::downgrade(&context);

    let _sink = RuntimeHostEventSink::new(
        BackendRuntime::new(),
        Some(context),
        RecordingSink::default(),
    );

    assert!(weak_context.upgrade().is_none());
}
