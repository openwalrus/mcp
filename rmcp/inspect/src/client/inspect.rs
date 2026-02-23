//! [`Inspect`] trait for querying MCP server capabilities.

use crate::error::Error;
use rmcp::{
    RoleClient,
    model::{Prompt, Resource, ResourceTemplate, Tool},
    service::RunningService,
};
use rmcp_registry::ServerDetail;

/// Inspection methods for a connected MCP server.
pub trait Inspect {
    /// List all tools exposed by the server.
    fn list_tools(&self) -> impl Future<Output = Result<Vec<Tool>, Error>> + Send;

    /// List all prompts exposed by the server.
    fn list_prompts(&self) -> impl Future<Output = Result<Vec<Prompt>, Error>> + Send;

    /// List all resources exposed by the server.
    fn list_resources(&self) -> impl Future<Output = Result<Vec<Resource>, Error>> + Send;

    /// List all resource templates exposed by the server.
    fn list_resource_templates(
        &self,
    ) -> impl Future<Output = Result<Vec<ResourceTemplate>, Error>> + Send;

    /// Generate server.json-compatible metadata from the live server.
    ///
    /// Queries peer info (from the initialization handshake) and all
    /// capabilities (tools, prompts, resources), assembling them into a
    /// [`ServerDetail`] conforming to the MCP Registry schema.
    fn generate_meta(&self) -> impl Future<Output = Result<ServerDetail, Error>> + Send;
}

impl Inspect for RunningService<RoleClient, ()> {
    async fn list_tools(&self) -> Result<Vec<Tool>, Error> {
        Ok(self.peer().list_all_tools().await?)
    }

    async fn list_prompts(&self) -> Result<Vec<Prompt>, Error> {
        Ok(self.peer().list_all_prompts().await?)
    }

    async fn list_resources(&self) -> Result<Vec<Resource>, Error> {
        Ok(self.peer().list_all_resources().await?)
    }

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>, Error> {
        Ok(self.peer().list_all_resource_templates().await?)
    }

    async fn generate_meta(&self) -> Result<ServerDetail, Error> {
        let peer = self.peer();
        let peer_info = peer.peer_info().ok_or(Error::NoPeerInfo)?;
        let server = &peer_info.server_info;

        let tools = peer.list_all_tools().await?;
        let prompts = peer.list_all_prompts().await?;
        let resources = peer.list_all_resources().await?;

        // Build _meta with capabilities from the live server.
        let mut meta_map = serde_json::Map::new();
        if !tools.is_empty() {
            meta_map.insert("tools".into(), serde_json::to_value(&tools)?);
        }
        if !prompts.is_empty() {
            meta_map.insert("prompts".into(), serde_json::to_value(&prompts)?);
        }
        if !resources.is_empty() {
            meta_map.insert("resources".into(), serde_json::to_value(&resources)?);
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
            title: server.title.as_deref().map(|t| t.parse()).transpose()?,
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
}
