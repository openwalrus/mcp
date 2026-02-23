//! MCP server providing sandboxed filesystem tools.
//!
//! All operations are restricted to a set of allowed directories configured
//! at server startup. Implements 11 tools following the MCP filesystem server
//! reference specification.

use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler,
};
use std::path::PathBuf;
pub mod tools;
pub mod validate;

/// MCP filesystem server with directory-level access control.
#[derive(Debug, Clone)]
pub struct FilesystemServer {
    pub(crate) allowed_dirs: Vec<PathBuf>,
    pub(crate) tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for FilesystemServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "wmcp-filesystem".into(),
                title: Some("Walrus MCP Filesystem Server".into()),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Filesystem server providing sandboxed file and directory operations.".into(),
            ),
        }
    }
}
