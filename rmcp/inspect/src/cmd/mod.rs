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
                print_tools(&tools);
            }
            CommandAction::Prompt => {
                let prompts = service.list_prompts().await?;
                print_prompts(&prompts);
            }
            CommandAction::Resource => {
                let resources = service.list_resources().await?;
                print_resources(&resources);
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

fn print_tools(tools: &[rmcp::model::Tool]) {
    if tools.is_empty() {
        println!("No tools available.");
        return;
    }
    for (i, tool) in tools.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("  {}", tool.name);
        if let Some(desc) = &tool.description {
            println!("    {desc}");
        }
        let schema = &tool.input_schema;
        if let Some(serde_json::Value::Object(props)) = schema.get("properties") {
            let required: Vec<&str> = schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if !props.is_empty() {
                println!("    Parameters:");
                for (name, prop) in props {
                    let ty = prop.get("type").and_then(|v| v.as_str()).unwrap_or("any");
                    let req = if required.contains(&name.as_str()) {
                        " (required)"
                    } else {
                        ""
                    };
                    let desc = prop
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if desc.is_empty() {
                        println!("      {name}: {ty}{req}");
                    } else {
                        println!("      {name}: {ty}{req} — {desc}");
                    }
                }
            }
        }
    }
}

fn print_prompts(prompts: &[rmcp::model::Prompt]) {
    if prompts.is_empty() {
        println!("No prompts available.");
        return;
    }
    for (i, prompt) in prompts.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("  {}", prompt.name);
        if let Some(desc) = &prompt.description {
            println!("    {desc}");
        }
        if let Some(args) = &prompt.arguments {
            if !args.is_empty() {
                println!("    Arguments:");
                for arg in args {
                    let req = if arg.required == Some(true) {
                        " (required)"
                    } else {
                        ""
                    };
                    let desc = arg.description.as_deref().unwrap_or("");
                    if desc.is_empty() {
                        println!("      {}{req}", arg.name);
                    } else {
                        println!("      {}{req} — {desc}", arg.name);
                    }
                }
            }
        }
    }
}

fn print_resources(resources: &[rmcp::model::Resource]) {
    if resources.is_empty() {
        println!("No resources available.");
        return;
    }
    for (i, resource) in resources.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("  {} ({})", resource.raw.name, resource.raw.uri);
        if let Some(desc) = &resource.raw.description {
            println!("    {desc}");
        }
        if let Some(mime) = &resource.raw.mime_type {
            println!("    Type: {mime}");
        }
    }
}
