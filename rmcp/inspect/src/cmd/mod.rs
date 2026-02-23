//! Command-line interface for inspecting MCP servers.

use crate::{
    client::{Inspect, Target, connect},
    error::Error,
};
use clap::{Parser, Subcommand};
pub mod call;

/// Inspect MCP servers and generate registry metadata.
#[derive(Parser, Debug)]
#[command(name = "rmcp-inspect", version, about)]
pub struct App {
    /// Target MCP server: a URL (http/https) for remote servers,
    /// or a command for stdio servers.
    ///
    /// Use `--` before commands with flags:
    ///   rmcp-inspect -- npx -y @modelcontextprotocol/server-everything tool
    #[arg(required = true, num_args = 1..)]
    pub target: Vec<String>,

    /// Bearer token for authenticating with remote servers.
    #[arg(long = "auth", value_name = "TOKEN")]
    pub auth: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List tools exposed by the server.
    Tool,
    /// List prompts exposed by the server.
    Prompt,
    /// List resources exposed by the server.
    Resource,
    /// Generate server.json-compatible metadata from the live server.
    Meta,
    /// Call a tool with arguments.
    Call {
        /// Name of the tool to call.
        name: String,

        /// Tool arguments as JSON key=value pairs (e.g. key1=value1 key2=value2).
        /// Values are parsed as JSON; plain strings are treated as JSON strings.
        #[arg(value_name = "KEY=VALUE")]
        args: Vec<String>,
    },
}

impl App {
    /// Parse CLI arguments and execute the corresponding command.
    pub async fn run() -> Result<(), Error> {
        let app = App::parse();
        let target = Target::parse(app.target, app.auth);
        let service = connect(target).await?;

        match app.command {
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
            Command::Call { name, args } => {
                let result = call::call(&service, name, args).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        service.cancel().await.ok();
        Ok(())
    }
}
