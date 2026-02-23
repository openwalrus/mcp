//! MCP client connection handling for stdio and remote transports.

use crate::error::Error;
use rmcp::{
    RoleClient, ServiceExt,
    service::RunningService,
    transport::{TokioChildProcess, streamable_http_client::StreamableHttpClientTransportConfig},
};
use tokio::process::Command;

mod inspect;

pub use inspect::Inspect;

/// Parsed target for connecting to an MCP server.
pub enum Target {
    /// Remote server at the given URL.
    Remote { url: String, auth: Option<String> },
    /// Stdio server launched by a command.
    Stdio { program: String, args: Vec<String> },
}

impl Target {
    /// Parse CLI target arguments into a [`Target`].
    ///
    /// If the first element starts with `http://` or `https://`, treat it as
    /// a remote URL. Otherwise treat the entire vec as a stdio command.
    pub fn parse(target: Vec<String>, auth: Option<String>) -> Self {
        let first = &target[0];
        if first.starts_with("http://") || first.starts_with("https://") {
            Target::Remote {
                url: first.clone(),
                auth,
            }
        } else {
            Target::Stdio {
                program: first.clone(),
                args: target[1..].to_vec(),
            }
        }
    }
}

/// Connect to an MCP server and return a running client service.
pub async fn connect(target: Target) -> Result<RunningService<RoleClient, ()>, Error> {
    match target {
        Target::Remote { url, auth } => {
            let config = StreamableHttpClientTransportConfig {
                uri: url.into(),
                ..Default::default()
            };
            let config = if let Some(token) = auth {
                config.auth_header(token)
            } else {
                config
            };
            let transport = rmcp::transport::StreamableHttpClientTransport::from_config(config);
            let service = ().serve(transport).await.map_err(Box::new)?;
            Ok(service)
        }
        Target::Stdio { program, args } => {
            let mut cmd = Command::new(&program);
            cmd.args(&args);
            let transport = TokioChildProcess::new(cmd)?;
            let service = ().serve(transport).await.map_err(Box::new)?;
            Ok(service)
        }
    }
}
