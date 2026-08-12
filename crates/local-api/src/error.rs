#[derive(Debug, thiserror::Error)]
pub enum LocalApiError {
    #[error("failed to generate Local API token")]
    TokenGeneration(#[from] getrandom::Error),
    #[error("Local API port {port} must be between 1024 and 65535")]
    InvalidPort { port: u16 },
    #[error("Local API port {port} is already in use")]
    PortInUse { port: u16 },
    #[error("failed to bind Local API port {port}: {source}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("Local API configuration error: {0}")]
    Config(String),
    #[error("Local API IO error: {0}")]
    Io(#[from] std::io::Error),
}
