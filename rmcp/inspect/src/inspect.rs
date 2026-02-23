use rmcp::{
    RoleClient,
    model::{Prompt, Resource, ResourceTemplate, Tool},
    service::RunningService,
};
use rmcp_registry::ServerDetail;

use crate::error::Error;

/// List all tools exposed by the server.
pub async fn list_tools(
    service: &RunningService<RoleClient, ()>,
) -> Result<Vec<Tool>, Error> {
    Ok(service.list_all_tools().await?)
}

/// List all prompts exposed by the server.
pub async fn list_prompts(
    service: &RunningService<RoleClient, ()>,
) -> Result<Vec<Prompt>, Error> {
    Ok(service.list_all_prompts().await?)
}

/// List all resources exposed by the server.
pub async fn list_resources(
    service: &RunningService<RoleClient, ()>,
) -> Result<Vec<Resource>, Error> {
    Ok(service.list_all_resources().await?)
}

/// List all resource templates exposed by the server.
pub async fn list_resource_templates(
    service: &RunningService<RoleClient, ()>,
) -> Result<Vec<ResourceTemplate>, Error> {
    Ok(service.list_all_resource_templates().await?)
}

/// Generate server.json-compatible metadata from a live MCP server.
///
/// Queries the server's peer info (from the initialization handshake) and
/// all capabilities (tools, prompts, resources), assembling them into a
/// [`ServerDetail`] conforming to the MCP Registry schema.
pub async fn generate_meta(
    service: &RunningService<RoleClient, ()>,
) -> Result<ServerDetail, Error> {
    let peer_info = service.peer_info().ok_or(Error::NoPeerInfo)?;
    let server = &peer_info.server_info;

    let tools = service.list_all_tools().await?;
    let prompts = service.list_all_prompts().await?;
    let resources = service.list_all_resources().await?;

    // Build _meta with capabilities from the live server.
    let mut meta_map = serde_json::Map::new();
    if !tools.is_empty() {
        meta_map.insert(
            "tools".into(),
            serde_json::to_value(&tools)?,
        );
    }
    if !prompts.is_empty() {
        meta_map.insert(
            "prompts".into(),
            serde_json::to_value(&prompts)?,
        );
    }
    if !resources.is_empty() {
        meta_map.insert(
            "resources".into(),
            serde_json::to_value(&resources)?,
        );
    }

    let meta = if meta_map.is_empty() {
        None
    } else {
        Some(rmcp_registry::ServerDetailMeta {
            io_modelcontextprotocol_registry_publisher_provided: meta_map,
        })
    };

    let detail = ServerDetail {
        name: server.name.parse()?,
        description: server
            .description
            .as_deref()
            .unwrap_or(&server.name)
            .parse()?,
        version: server.version.parse()?,
        title: server
            .title
            .as_deref()
            .map(|t| t.parse())
            .transpose()?,
        website_url: server.website_url.clone(),
        schema: Some(
            "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json"
                .into(),
        ),
        meta,
        icons: Vec::new(),
        packages: Vec::new(),
        remotes: Vec::new(),
        repository: None,
    };

    Ok(detail)
}
