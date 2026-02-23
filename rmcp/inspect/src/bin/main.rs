use clap::Parser;
use rmcp_inspect::{
    cli::{Cli, Command},
    client::{Target, connect},
    error::Error,
    inspect,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let target = Target::parse(cli.target, cli.auth);
    let service = connect(target).await?;

    match cli.command {
        Command::Tool => {
            let tools = inspect::list_tools(&service).await?;
            println!("{}", serde_json::to_string_pretty(&tools)?);
        }
        Command::Prompt => {
            let prompts = inspect::list_prompts(&service).await?;
            println!("{}", serde_json::to_string_pretty(&prompts)?);
        }
        Command::Resource => {
            let resources = inspect::list_resources(&service).await?;
            println!("{}", serde_json::to_string_pretty(&resources)?);
        }
        Command::Meta => {
            let meta = inspect::generate_meta(&service).await?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    }

    service.cancel().await.ok();
    Ok(())
}
