use std::sync::Arc;

use vrcx_0_application_core::{
    BackendRuntime, BackendRuntimeSnapshot, BackendRuntimeTelemetry, RealtimeProjectionSync,
    RuntimeEventPayload, RuntimeEventSink,
};

use crate::RuntimeHostProfileExtension;

pub struct RuntimeHostEventSink<S> {
    backend_runtime: BackendRuntime,
    profile_extension: Option<Arc<dyn RuntimeHostProfileExtension>>,
    inner: S,
}

impl<S> RuntimeHostEventSink<S> {
    pub fn new(
        backend_runtime: BackendRuntime,
        profile_extension: Option<Arc<dyn RuntimeHostProfileExtension>>,
        inner: S,
    ) -> Self {
        Self {
            backend_runtime,
            profile_extension,
            inner,
        }
    }
}

impl<S> RuntimeHostEventSink<S>
where
    S: RuntimeEventSink,
{
    fn emit_realtime_projection_sync(&self, snapshot: BackendRuntimeSnapshot) {
        match serde_json::to_value(RealtimeProjectionSync { snapshot }) {
            Ok(payload) => self.inner.emit(RealtimeProjectionSync::EVENT_NAME, payload),
            Err(error) => tracing::warn!(
                error = %error,
                "failed to serialize realtime projection sync"
            ),
        }
    }

    fn emit_fallback_backend_runtime_telemetry(&self, payload: serde_json::Value) {
        let kind = payload
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("runtimeTelemetry")
            .to_string();
        let detail = payload
            .get("detail")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                payload
                    .get("messageType")
                    .and_then(serde_json::Value::as_str)
            })
            .map(str::to_string)
            .or_else(|| {
                payload
                    .get("count")
                    .and_then(serde_json::Value::as_u64)
                    .map(|count| count.to_string())
            })
            .unwrap_or_else(|| payload.to_string());
        let snapshot = self.backend_runtime.snapshot();
        let telemetry = BackendRuntimeTelemetry {
            kind,
            detail,
            snapshot: snapshot.clone(),
        };
        match serde_json::to_value(telemetry) {
            Ok(payload) => {
                self.emit_realtime_projection_sync(snapshot);
                self.inner.emit("backendRuntimeTelemetry", payload);
            }
            Err(error) => tracing::warn!(
                error = %error,
                "failed to serialize fallback backend runtime telemetry"
            ),
        }
    }
}

impl<S> RuntimeEventSink for RuntimeHostEventSink<S>
where
    S: RuntimeEventSink,
{
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(extension) = &self.profile_extension {
            extension.observe_runtime_event(event, &payload);
        }

        if event == "backendRuntimeTelemetry" {
            if let Ok(telemetry) =
                serde_json::from_value::<BackendRuntimeTelemetry>(payload.clone())
            {
                self.emit_realtime_projection_sync(telemetry.snapshot);
                self.inner.emit(event, payload);
                return;
            }
            if payload.get("snapshot").is_some() {
                self.emit_fallback_backend_runtime_telemetry(payload);
                return;
            }
        }

        let telemetry = self.backend_runtime.observe_runtime_event(event, &payload);
        if event != "backendRuntimeTelemetry" {
            self.inner.emit(event, payload.clone());
        }

        if let Some(telemetry) = telemetry {
            let snapshot = telemetry.snapshot.clone();
            match serde_json::to_value(telemetry) {
                Ok(payload) => {
                    self.emit_realtime_projection_sync(snapshot);
                    self.inner.emit("backendRuntimeTelemetry", payload);
                }
                Err(error) => tracing::warn!(
                    error = %error,
                    "failed to serialize backend runtime telemetry"
                ),
            }
        } else if event == "backendRuntimeTelemetry" {
            self.emit_fallback_backend_runtime_telemetry(payload);
        }
    }
}

#[cfg(test)]
mod tests;
