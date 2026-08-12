use serde::{Deserialize, Serialize};
use specta::Type;

use crate::IntegrationApiError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationApiServerState {
    Disabled,
    WaitingForGame,
    Running,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IntegrationApiFailureCode {
    InvalidPort,
    PortInUse,
    Bind,
    Config,
    Io,
    TokenGeneration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationApiFailure {
    pub code: IntegrationApiFailureCode,
    pub message: String,
    pub port: Option<u16>,
}

impl IntegrationApiFailure {
    pub fn from_error(error: &IntegrationApiError) -> Self {
        let (code, port) = match error {
            IntegrationApiError::InvalidPort { port } => {
                (IntegrationApiFailureCode::InvalidPort, Some(*port))
            }
            IntegrationApiError::PortInUse { port } => {
                (IntegrationApiFailureCode::PortInUse, Some(*port))
            }
            IntegrationApiError::Bind { port, .. } => {
                (IntegrationApiFailureCode::Bind, Some(*port))
            }
            IntegrationApiError::Config(_) => (IntegrationApiFailureCode::Config, None),
            IntegrationApiError::Io(_) => (IntegrationApiFailureCode::Io, None),
            IntegrationApiError::TokenGeneration(_) => {
                (IntegrationApiFailureCode::TokenGeneration, None)
            }
        };
        Self {
            code,
            message: error.to_string(),
            port,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationApiStatus {
    pub enabled: bool,
    pub allow_lan_connections: bool,
    pub state: IntegrationApiServerState,
    pub port: u16,
    pub token: String,
    pub active_connections: u32,
    pub last_error: Option<IntegrationApiFailure>,
}
