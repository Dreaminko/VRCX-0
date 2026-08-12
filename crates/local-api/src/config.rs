use crate::LocalApiError;

pub const DEFAULT_LOCAL_API_PORT: u16 = 8799;
pub const LOCAL_API_ENABLED_CONFIG_KEY: &str = "localApiEnabled";
pub const LOCAL_API_PORT_CONFIG_KEY: &str = "localApiPort";
pub const LOCAL_API_TOKEN_CONFIG_KEY: &str = "localApiToken";
pub const LOCAL_API_ALLOW_LAN_CONFIG_KEY: &str = "localApiAllowLanConnections";

pub trait LocalApiConfigStore: Send + Sync {
    fn get_bool(&self, key: &str, default: bool) -> Result<bool, LocalApiError>;
    fn get_string(&self, key: &str, default: &str) -> Result<String, LocalApiError>;
    fn set_bool(&self, key: &str, value: bool) -> Result<(), LocalApiError>;
    fn set_string(&self, key: &str, value: &str) -> Result<(), LocalApiError>;
}
