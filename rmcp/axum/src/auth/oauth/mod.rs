//! OAuth 2.1 resource server support for MCP servers.
//!
//! Implements the server-side (resource server) requirements of the
//! [MCP Authorization specification](https://modelcontextprotocol.io/specification/draft/basic/authorization):
//!
//! - **Protected Resource Metadata** ([RFC 9728](https://datatracker.ietf.org/doc/html/rfc9728)):
//!   Serve `/.well-known/oauth-protected-resource` so MCP clients can discover
//!   authorization servers.
//!
//! - **Spec-compliant error responses**: 401 and 403 responses with proper
//!   `WWW-Authenticate` headers per [RFC 6750](https://datatracker.ietf.org/doc/html/rfc6750).
//!
//! # Example
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, BearerAuth};
//! use rmcp_axum::auth::oauth::{
//!     ProtectedResourceMetadata, ResourceServerConfig, metadata_router,
//! };
//!
//! let metadata = ProtectedResourceMetadata {
//!     resource: "https://mcp.example.com".into(),
//!     authorization_servers: vec!["https://auth.example.com".into()],
//!     scopes_supported: Some(vec!["mcp:tools".into()]),
//!     bearer_methods_supported: Some(vec!["header".into()]),
//!     resource_documentation: None,
//! };
//!
//! let rs_config = ResourceServerConfig {
//!     resource_metadata_url:
//!         "https://mcp.example.com/.well-known/oauth-protected-resource".into(),
//!     default_scope: Some("mcp:tools".into()),
//! };
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", mcp_service)
//!     .merge(metadata_router(metadata))
//!     .layer(AuthLayer::new(BearerAuth::new(validator)).with_resource_server(rs_config));
//! ```

mod error;
mod metadata;

pub use error::{
    ResourceServerConfig, insufficient_scope_response, www_authenticate_401, www_authenticate_403,
};
pub use metadata::{ProtectedResourceMetadata, metadata_router};
