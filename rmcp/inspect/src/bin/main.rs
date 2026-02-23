use clap::Parser;
use rmcp_inspect::{
    cli::{Cli, Command},
    client::{Inspect, Target, connect},
    error::Error,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let target = Target::parse(cli.target, cli.auth);
    let service = connect(target).await?;

    match cli.command {
        Command::Tool => {
            let tools = service.list_tools().await?;
            println!("{}", serde_json::to_string_pretty(&tools)?);
        }
        Command::Prompt => {
            let prompts = service.list_prompts().await?;
            println!("{}", serde_json::to_string_pretty(&prompts)?);
        }
        Command::Resource => {
            let resources = service.list_resources().await?;
            println!("{}", serde_json::to_string_pretty(&resources)?);
        }
        Command::Meta => {
            let meta = service.generate_meta().await?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }
    }

    service.cancel().await.ok();
    Ok(())
}
