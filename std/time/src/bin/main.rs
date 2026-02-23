//! Binary entry point for the wmcp-time MCP server.

use clap::Parser;
use rmcp::ServiceExt;
use wmcp_time::TimeServer;

/// Walrus MCP Time Server — provides timezone-aware time tools.
#[derive(Parser)]
#[command(name = "wmcp-time", version, about)]
struct Cli {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _cli = Cli::parse();
    let server = TimeServer::new();
    let transport = rmcp::transport::stdio();
    server
        .serve(transport)
        .await
        .expect("failed to start server")
        .waiting()
        .await
        .expect("server error");
}
