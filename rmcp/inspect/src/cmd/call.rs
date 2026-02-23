//! Command for calling a tool on an MCP server.

use crate::error::Error;
use rmcp::{
    RoleClient,
    model::{CallToolRequestParams, CallToolResult, JsonObject},
    service::RunningService,
};
use std::borrow::Cow;

/// Parse `key=value` pairs into a JSON object.
///
/// Each value is first attempted as JSON. If parsing fails, it is
/// treated as a plain string.
fn parse_args(args: &[String]) -> Result<Option<JsonObject>, Error> {
    if args.is_empty() {
        return Ok(None);
    }

    let mut map = serde_json::Map::new();
    for arg in args {
        let (key, raw_value) = arg
            .split_once('=')
            .ok_or_else(|| Error::InvalidArg(arg.clone()))?;

        let value = serde_json::from_str(raw_value)
            .unwrap_or_else(|_| serde_json::Value::String(raw_value.to_string()));

        map.insert(key.to_string(), value);
    }

    Ok(Some(map))
}

/// Call a tool on the connected MCP server.
pub async fn call(
    service: &RunningService<RoleClient, ()>,
    name: String,
    args: Vec<String>,
) -> Result<CallToolResult, Error> {
    let arguments = parse_args(&args)?;
    let result = service
        .peer()
        .call_tool(CallToolRequestParams {
            meta: None,
            name: Cow::Owned(name),
            arguments,
            task: None,
        })
        .await?;
    Ok(result)
}
