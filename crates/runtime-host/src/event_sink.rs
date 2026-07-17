use std::sync::Arc;

use vrcx_0_application_core::{BackendRuntime, RuntimeEventSink};

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

impl<S> RuntimeEventSink for RuntimeHostEventSink<S>
where
    S: RuntimeEventSink,
{
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(extension) = &self.profile_extension {
            extension.observe_runtime_event(event, &payload);
        }

        if event == "backendRuntimeTelemetry" && payload.get("snapshot").is_some() {
            self.inner.emit(event, payload);
            return;
        }

        let telemetry = self.backend_runtime.observe_runtime_event(event, &payload);
        if event != "backendRuntimeTelemetry" {
            self.inner.emit(event, payload.clone());
        }

        if let Some(telemetry) = telemetry {
            match serde_json::to_value(telemetry) {
                Ok(payload) => self.inner.emit("backendRuntimeTelemetry", payload),
                Err(error) => tracing::warn!(
                    error = %error,
                    "failed to serialize backend runtime telemetry"
                ),
            }
        } else if event == "backendRuntimeTelemetry" {
            self.inner.emit(event, payload);
        }
    }
}

#[cfg(test)]
mod tests;
