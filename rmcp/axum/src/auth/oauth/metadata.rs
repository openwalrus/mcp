//! OAuth 2.0 Protected Resource Metadata (RFC 9728).
//!
//! MCP servers MUST implement RFC 9728 to indicate the locations of their
//! authorization servers. This module provides the metadata type and an axum
//! router that serves it at the well-known endpoint.
//!
//! ```rust,ignore
//! use rmcp_axum::auth::oauth::{ProtectedResourceMetadata, metadata_router};
//!
//! let metadata = ProtectedResourceMetadata {
//!     resource: "https://mcp.example.com".into(),
//!     authorization_servers: vec!["https://auth.example.com".into()],
//!     scopes_supported: Some(vec!["mcp:tools".into()]),
//!     bearer_methods_supported: Some(vec!["header".into()]),
//!     resource_documentation: None,
//! };
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", mcp_service)
//!     .merge(metadata_router(metadata));
//! ```

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// OAuth 2.0 Protected Resource Metadata ([RFC 9728](https://datatracker.ietf.org/doc/html/rfc9728)).
///
/// MCP servers MUST include the `authorization_servers` field containing at
/// least one authorization server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    /// The resource identifier — canonical URI of this MCP server.
    ///
    /// This aligns with the `resource` parameter in RFC 8707 and RFC 9728.
    pub resource: String,

    /// Authorization server(s) that can issue tokens for this resource.
    ///
    /// MUST contain at least one entry.
    pub authorization_servers: Vec<String>,

    /// Scopes supported by this resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,

    /// Bearer token methods supported (e.g., `["header"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_methods_supported: Option<Vec<String>>,

    /// URL of the resource documentation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_documentation: Option<String>,
}

/// Create an axum [`Router`](axum::Router) that serves the Protected Resource
/// Metadata at `/.well-known/oauth-protected-resource`.
///
/// Per RFC 9728, this endpoint allows MCP clients to discover the authorization
/// server(s) for this resource. The metadata is served as `application/json`.
///
/// Mount this router alongside your MCP service:
///
/// ```rust,ignore
/// let app = axum::Router::new()
///     .nest_service("/mcp", mcp_service)
///     .merge(metadata_router(metadata));
/// ```
pub fn metadata_router(metadata: ProtectedResourceMetadata) -> axum::Router {
    let metadata = Arc::new(metadata);
    axum::Router::new().route(
        "/.well-known/oauth-protected-resource",
        axum::routing::get(move || {
            let metadata = metadata.clone();
            async move { Json(metadata.as_ref().clone()).into_response() }
        }),
    )
}
