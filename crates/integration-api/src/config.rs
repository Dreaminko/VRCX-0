use crate::IntegrationApiError;

pub const DEFAULT_INTEGRATION_API_PORT: u16 = 8799;
pub const INTEGRATION_API_ENABLED_CONFIG_KEY: &str = "integrationApiEnabled";
pub const INTEGRATION_API_PORT_CONFIG_KEY: &str = "integrationApiPort";
pub const INTEGRATION_API_TOKEN_CONFIG_KEY: &str = "integrationApiToken";
pub const INTEGRATION_API_ALLOW_LAN_CONFIG_KEY: &str = "integrationApiAllowLanConnections";

pub trait IntegrationApiConfigStore: Send + Sync {
    fn get_bool(&self, key: &str, default: bool) -> Result<bool, IntegrationApiError>;
    fn get_string(&self, key: &str, default: &str) -> Result<String, IntegrationApiError>;
    fn set_bool(&self, key: &str, value: bool) -> Result<(), IntegrationApiError>;
    fn set_string(&self, key: &str, value: &str) -> Result<(), IntegrationApiError>;
}
