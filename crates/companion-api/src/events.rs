use serde::{Deserialize, Serialize};
use specta::Type;

use crate::CompanionApiError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CompanionApiStartFailureReason {
    PortInUse,
    Bind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanionApiStartFailedPayload {
    pub port: u16,
    pub reason: CompanionApiStartFailureReason,
}

impl CompanionApiStartFailedPayload {
    pub fn from_error(error: &CompanionApiError, fallback_port: u16) -> Self {
        match error {
            CompanionApiError::PortInUse { port } => Self {
                port: *port,
                reason: CompanionApiStartFailureReason::PortInUse,
            },
            CompanionApiError::Bind { port, .. } => Self {
                port: *port,
                reason: CompanionApiStartFailureReason::Bind,
            },
            _ => Self {
                port: fallback_port,
                reason: CompanionApiStartFailureReason::Bind,
            },
        }
    }
}

vrcx_0_application_core::runtime_event_payload!(
    CompanionApiStartFailedPayload,
    "companionApiStartFailed"
);
