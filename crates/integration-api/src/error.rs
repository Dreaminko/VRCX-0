#[derive(Debug, thiserror::Error)]
pub enum IntegrationApiError {
    #[error("failed to generate Integration API token")]
    TokenGeneration(#[from] getrandom::Error),
    #[error("Integration API port {port} must be between 1024 and 65535")]
    InvalidPort { port: u16 },
    #[error("Integration API port {port} is already in use")]
    PortInUse { port: u16 },
    #[error("failed to bind Integration API port {port}: {source}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("Integration API configuration error: {0}")]
    Config(String),
    #[error("Integration API IO error: {0}")]
    Io(#[from] std::io::Error),
}
