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

vrcx_0_application_contracts::runtime_event_payload!(
    CompanionApiStartFailedPayload,
    "companionApiStartFailed"
);

#[cfg(test)]
mod tests {
    use vrcx_0_application_contracts::RuntimeEventPayload;

    use super::{CompanionApiStartFailedPayload, CompanionApiStartFailureReason};

    #[test]
    fn start_failure_keeps_the_event_name_and_camel_case_wire_shape() {
        let payload = CompanionApiStartFailedPayload {
            port: 27272,
            reason: CompanionApiStartFailureReason::PortInUse,
        };

        assert_eq!(
            CompanionApiStartFailedPayload::EVENT_NAME,
            "companionApiStartFailed"
        );
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({ "port": 27272, "reason": "portInUse" })
        );
    }
}
