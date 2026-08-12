use serde::{Deserialize, Serialize};
use specta::Type;

use crate::LocalApiError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LocalApiServerState {
    Disabled,
    WaitingForGame,
    Running,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LocalApiFailureCode {
    InvalidPort,
    PortInUse,
    Bind,
    Config,
    Io,
    TokenGeneration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalApiFailure {
    pub code: LocalApiFailureCode,
    pub message: String,
    pub port: Option<u16>,
}

impl LocalApiFailure {
    pub fn from_error(error: &LocalApiError) -> Self {
        let (code, port) = match error {
            LocalApiError::InvalidPort { port } => (LocalApiFailureCode::InvalidPort, Some(*port)),
            LocalApiError::PortInUse { port } => (LocalApiFailureCode::PortInUse, Some(*port)),
            LocalApiError::Bind { port, .. } => (LocalApiFailureCode::Bind, Some(*port)),
            LocalApiError::Config(_) => (LocalApiFailureCode::Config, None),
            LocalApiError::Io(_) => (LocalApiFailureCode::Io, None),
            LocalApiError::TokenGeneration(_) => (LocalApiFailureCode::TokenGeneration, None),
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
pub struct LocalApiStatus {
    pub enabled: bool,
    pub allow_lan_connections: bool,
    pub state: LocalApiServerState,
    pub port: u16,
    pub token: String,
    pub active_connections: u32,
    pub last_error: Option<LocalApiFailure>,
}
