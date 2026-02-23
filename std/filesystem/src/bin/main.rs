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
    tracing_subscriber::fmt::init();
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
