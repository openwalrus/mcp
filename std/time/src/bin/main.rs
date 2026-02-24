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
    let _cli = Cli::parse();
    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }
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
