use serde::{Deserialize, Serialize};
use specta::Type;

use crate::LocalApiError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LocalApiStartFailureReason {
    PortInUse,
    Bind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalApiStartFailedPayload {
    pub port: u16,
    pub reason: LocalApiStartFailureReason,
}

impl LocalApiStartFailedPayload {
    pub fn from_error(error: &LocalApiError, fallback_port: u16) -> Self {
        match error {
            LocalApiError::PortInUse { port } => Self {
                port: *port,
                reason: LocalApiStartFailureReason::PortInUse,
            },
            LocalApiError::Bind { port, .. } => Self {
                port: *port,
                reason: LocalApiStartFailureReason::Bind,
            },
            _ => Self {
                port: fallback_port,
                reason: LocalApiStartFailureReason::Bind,
            },
        }
    }
}

vrcx_0_application_core::runtime_event_payload!(LocalApiStartFailedPayload, "localApiStartFailed");
