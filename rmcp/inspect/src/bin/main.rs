//! Binary entry point for the rmcp-inspect CLI.

use rmcp_inspect::{cmd::App, error::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    App::run().await
}
