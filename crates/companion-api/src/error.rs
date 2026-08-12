#[derive(Debug, thiserror::Error)]
pub enum CompanionApiError {
    #[error("failed to generate Companion API token")]
    TokenGeneration(#[from] getrandom::Error),
    #[error("Companion API port {port} must be between 1024 and 65535")]
    InvalidPort { port: u16 },
    #[error("Companion API port {port} is already in use")]
    PortInUse { port: u16 },
    #[error("failed to bind Companion API port {port}: {source}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("Companion API configuration error: {0}")]
    Config(String),
    #[error("Companion API IO error: {0}")]
    Io(#[from] std::io::Error),
}
