//! Binary entry point for the rmcp-inspect CLI.

use rmcp_inspect::cmd::App;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = App::run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
