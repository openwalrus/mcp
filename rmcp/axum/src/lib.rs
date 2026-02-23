//! # rmcp-axum
//!
//! Extensions for building MCP servers with [rmcp](https://docs.rs/rmcp) and
//! [axum](https://docs.rs/axum).
//!
//! Implements the server-side (resource server) requirements of the
//! [MCP Authorization specification](https://modelcontextprotocol.io/specification/draft/basic/authorization).
//!
//! ## Features
//!
//! - **Auth middleware** — [`AuthLayer`](auth::AuthLayer) validates Bearer tokens
//!   and emits spec-compliant `WWW-Authenticate` headers on 401 responses.
//! - **Protected Resource Metadata** — [`metadata_router`](auth::oauth::metadata_router)
//!   serves the RFC 9728 `/.well-known/oauth-protected-resource` endpoint.
//! - **JWT validation** — [`JwtValidator`](auth::jwt::JwtValidator) validates
//!   tokens against a JWKS endpoint (feature `jwt`).
//!
//! ## Example
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, BearerAuth, jwt::JwtValidator};
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
//! let jwt = JwtValidator::from_jwks_url(
//!     "https://auth.example.com/.well-known/jwks.json",
//! )
//! .audience("https://mcp.example.com")
//! .issuer("https://auth.example.com")
//! .build()
//! .await
//! .expect("failed to fetch JWKS");
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", mcp_service)
//!     .merge(metadata_router(metadata))
//!     .layer(AuthLayer::new(BearerAuth::new(jwt)).with_resource_server(rs_config));
//! ```

pub use axum;

pub mod auth;
