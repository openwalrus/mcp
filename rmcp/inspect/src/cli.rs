use clap::{Parser, Subcommand};

/// Inspect MCP servers and generate registry metadata.
#[derive(Parser, Debug)]
#[command(name = "rmcp-inspect", version, about)]
pub struct Cli {
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
}
