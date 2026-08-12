mod auth;
mod config;
mod controller;
mod error;
mod events;
mod publisher;
mod session;
mod state;
mod transport;
mod types;
mod wire;

pub use config::{
    CompanionApiConfigStore, COMPANION_API_ALLOW_LAN_CONFIG_KEY, COMPANION_API_ENABLED_CONFIG_KEY,
    COMPANION_API_PORT_CONFIG_KEY, COMPANION_API_TOKEN_CONFIG_KEY, DEFAULT_COMPANION_API_PORT,
};
pub use controller::CompanionApiController;
pub use error::CompanionApiError;
pub use events::{CompanionApiStartFailedPayload, CompanionApiStartFailureReason};
pub use publisher::{
    companion_api_publisher_channel, CompanionApiInput, CompanionApiInputReceiver,
    CompanionApiPublisher,
};
pub use state::{RoomMemberState, RoomState};
pub use types::{
    CompanionApiFailure, CompanionApiFailureCode, CompanionApiServerState, CompanionApiStatus,
};
pub use wire::PROTOCOL_VERSION;
