use serde::{Deserialize, Serialize};
use specta::Type;

use crate::CompanionApiError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CompanionApiServerState {
    Disabled,
    WaitingForGame,
    Running,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CompanionApiFailureCode {
    InvalidPort,
    PortInUse,
    Bind,
    Config,
    Io,
    TokenGeneration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompanionApiFailure {
    pub code: CompanionApiFailureCode,
    pub message: String,
    pub port: Option<u16>,
}

impl CompanionApiFailure {
    pub fn from_error(error: &CompanionApiError) -> Self {
        let (code, port) = match error {
            CompanionApiError::InvalidPort { port } => {
                (CompanionApiFailureCode::InvalidPort, Some(*port))
            }
            CompanionApiError::PortInUse { port } => {
                (CompanionApiFailureCode::PortInUse, Some(*port))
            }
            CompanionApiError::Bind { port, .. } => (CompanionApiFailureCode::Bind, Some(*port)),
            CompanionApiError::Config(_) => (CompanionApiFailureCode::Config, None),
            CompanionApiError::Io(_) => (CompanionApiFailureCode::Io, None),
            CompanionApiError::TokenGeneration(_) => {
                (CompanionApiFailureCode::TokenGeneration, None)
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
pub struct CompanionApiStatus {
    pub enabled: bool,
    pub allow_lan_connections: bool,
    pub state: CompanionApiServerState,
    pub port: u16,
    pub token: String,
    pub active_connections: u32,
    pub last_error: Option<CompanionApiFailure>,
}
