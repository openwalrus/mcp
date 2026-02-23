use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("MCP client initialization error: {0}")]
    ClientInit(#[from] rmcp::service::ClientInitializeError),

    #[error("MCP client error: {0}")]
    Service(#[from] rmcp::ServiceError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("schema conversion error: {0}")]
    Schema(#[from] rmcp_registry::error::ConversionError),

    #[error("server did not provide peer info")]
    NoPeerInfo,
}
