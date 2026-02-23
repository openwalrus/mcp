//! Binary entry point for the wmcp-filesystem MCP server.

use clap::Parser;
use rmcp::ServiceExt;
use wmcp_filesystem::FilesystemServer;

/// Walrus MCP Filesystem Server — provides sandboxed filesystem tools.
#[derive(Parser)]
#[command(name = "wmcp-filesystem", version, about)]
struct Cli {
    /// Allowed directories the server may access.
    #[arg(required = true, num_args = 1..)]
    allowed_dirs: Vec<std::path::PathBuf>,
}

#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }
    let cli = Cli::parse();
    let server = FilesystemServer::new(cli.allowed_dirs);
    let transport = rmcp::transport::stdio();
    server
        .serve(transport)
        .await
        .expect("failed to start server")
        .waiting()
        .await
        .expect("server error");
}
