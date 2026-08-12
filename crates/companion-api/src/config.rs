use crate::CompanionApiError;

pub const DEFAULT_COMPANION_API_PORT: u16 = 8799;
pub const COMPANION_API_ENABLED_CONFIG_KEY: &str = "companionApiEnabled";
pub const COMPANION_API_PORT_CONFIG_KEY: &str = "companionApiPort";
pub const COMPANION_API_TOKEN_CONFIG_KEY: &str = "companionApiToken";
pub const COMPANION_API_ALLOW_LAN_CONFIG_KEY: &str = "companionApiAllowLanConnections";

pub trait CompanionApiConfigStore: Send + Sync {
    fn get_bool(&self, key: &str, default: bool) -> Result<bool, CompanionApiError>;
    fn get_string(&self, key: &str, default: &str) -> Result<String, CompanionApiError>;
    fn set_bool(&self, key: &str, value: bool) -> Result<(), CompanionApiError>;
    fn set_string(&self, key: &str, value: &str) -> Result<(), CompanionApiError>;
}
