use serde::Serialize;

use vrcx_0_application::AuthenticatedRuntimePhaseSnapshot;
use vrcx_0_application_core::BackendRuntimeSnapshot;

use super::RuntimeHostState;

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackendRuntimeCombinedSnapshot {
    pub backend_runtime: BackendRuntimeSnapshot,
    pub authenticated_runtime_phase: AuthenticatedRuntimePhaseSnapshot,
}

impl RuntimeHostState {
    pub fn backend_runtime_combined_snapshot(&self) -> BackendRuntimeCombinedSnapshot {
        BackendRuntimeCombinedSnapshot {
            backend_runtime: self.snapshot_backend_runtime(),
            authenticated_runtime_phase: self.authenticated_runtime.snapshot(),
        }
    }
}
