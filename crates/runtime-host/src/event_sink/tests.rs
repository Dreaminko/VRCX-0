use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use vrcx_0_application_core::RuntimeEventSink;

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
    fn emit(&self, event: &str, payload: Value) {
        self.events.lock().unwrap().push(RecordedEvent {
            name: event.to_string(),
            payload,
        });
    }
}

#[test]
fn ordinary_event_is_forwarded_unchanged() {
    let recording = RecordingSink::default();
    let sink = RuntimeHostEventSink::new(recording.clone());
    let payload = json!({ "status": "connected" });

    sink.emit("realtimeWsStatus", payload.clone());

    assert_eq!(
        recording.events(),
        vec![RecordedEvent {
            name: "realtimeWsStatus".into(),
            payload,
        }]
    );
}
