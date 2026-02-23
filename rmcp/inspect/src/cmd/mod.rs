//! Command-line interface for inspecting MCP servers.

use crate::{
    client::{Inspect, Target, connect},
    error::Error,
};
use clap::{Parser, Subcommand};
pub mod call;

/// Inspect MCP servers and generate registry metadata.
///
/// Usage:
///   rmcp-inspect tool -- ./target/debug/wmcp-time
///   rmcp-inspect call get_current_time timezone=UTC -- ./my-server
///   rmcp-inspect --auth TOKEN tool -- https://example.com/mcp
#[derive(Parser, Debug)]
#[command(name = "rmcp-inspect", version, about, subcommand_negates_reqs = true)]
pub struct App {
    /// Bearer token for authenticating with remote servers.
    #[arg(long = "auth", value_name = "TOKEN")]
    pub auth: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List tools exposed by the server.
    Tool {
        /// Target MCP server (URL or command after `--`).
        #[arg(required = true, num_args = 1.., last = true)]
        target: Vec<String>,
    },
    /// List prompts exposed by the server.
    Prompt {
        /// Target MCP server (URL or command after `--`).
        #[arg(required = true, num_args = 1.., last = true)]
        target: Vec<String>,
    },
    /// List resources exposed by the server.
    Resource {
        /// Target MCP server (URL or command after `--`).
        #[arg(required = true, num_args = 1.., last = true)]
        target: Vec<String>,
    },
    /// Generate server.json-compatible metadata from the live server.
    Meta {
        /// Target MCP server (URL or command after `--`).
        #[arg(required = true, num_args = 1.., last = true)]
        target: Vec<String>,
    },
    /// Call a tool with arguments.
    Call {
        /// Name of the tool to call.
        name: String,

        /// Tool arguments as JSON key=value pairs (e.g. key1=value1 key2=value2).
        /// Values are parsed as JSON; plain strings are treated as JSON strings.
        #[arg(value_name = "KEY=VALUE")]
        args: Vec<String>,

        /// Target MCP server (URL or command after `--`).
        #[arg(required = true, num_args = 1.., last = true)]
        target: Vec<String>,
    },
}

/// Internal action after extracting target from subcommand.
enum CommandAction {
    Tool,
    Prompt,
    Resource,
    Meta,
    Call { name: String, args: Vec<String> },
}

impl App {
    /// Parse CLI arguments and execute the corresponding command.
    pub async fn run() -> Result<(), Error> {
        let app = App::parse();
        if std::env::var_os("RUST_LOG").is_some() {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
        }

        let (target_args, command) = match app.command {
            Command::Tool { target } => (target, CommandAction::Tool),
            Command::Prompt { target } => (target, CommandAction::Prompt),
            Command::Resource { target } => (target, CommandAction::Resource),
            Command::Meta { target } => (target, CommandAction::Meta),
            Command::Call { name, args, target } => (target, CommandAction::Call { name, args }),
        };

        let target = Target::parse(target_args, app.auth);
        let service = connect(target).await?;

        match command {
            CommandAction::Tool => {
                let tools = service.list_tools().await?;
                println!("{}", serde_json::to_string_pretty(&tools)?);
            }
            CommandAction::Prompt => {
                let prompts = service.list_prompts().await?;
                println!("{}", serde_json::to_string_pretty(&prompts)?);
            }
            CommandAction::Resource => {
                let resources = service.list_resources().await?;
                println!("{}", serde_json::to_string_pretty(&resources)?);
            }
            CommandAction::Meta => {
                let meta = service.generate_meta().await?;
                println!("{}", serde_json::to_string_pretty(&meta)?);
            }
            CommandAction::Call { name, args } => {
                let result = call::call(&service, name, args).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        service.cancel().await.ok();
        Ok(())
    }
}
