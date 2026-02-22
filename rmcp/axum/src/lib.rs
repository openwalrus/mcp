//! # rmcp-axum
//!
//! Extensions for building MCP servers with [rmcp](https://docs.rs/rmcp) and
//! [axum](https://docs.rs/axum).
//!
//! ## Auth Middleware
//!
//! Provides a pluggable [`Authenticator`] trait and tower middleware for
//! validating requests before they reach the MCP service.
//!
//! ```rust,ignore
//! use rmcp::transport::streamable_http_server::{
//!     StreamableHttpService, StreamableHttpServerConfig,
//!     session::local::LocalSessionManager,
//! };
//! use rmcp_axum::auth::{AuthLayer, Authenticator};
//!
//! let service = StreamableHttpService::new(
//!     || Ok(MyMcpService::new()),
//!     LocalSessionManager::default().into(),
//!     StreamableHttpServerConfig::default(),
//! );
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(MyAuth));
//!
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//! axum::serve(listener, app).await?;
//! ```

pub use axum;

pub mod auth;
