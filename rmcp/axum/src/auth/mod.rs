//! Authentication middleware for MCP servers.
//!
//! Provides a tower middleware that validates incoming requests using a
//! user-defined [`Authenticator`] trait. On success, the authenticated
//! claims are inserted into HTTP extensions and become accessible in MCP
//! tool handlers via `Extension(parts): Extension<Parts>`.
//!
//! When configured with a [`ResourceServerConfig`](oauth::ResourceServerConfig),
//! the middleware emits spec-compliant `WWW-Authenticate` headers in 401
//! responses per the MCP authorization specification.
//!
//! # Example
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, BearerAuth, Validator};
//! use rmcp_axum::auth::oauth::ResourceServerConfig;
//!
//! #[derive(Clone)]
//! struct MyValidator;
//!
//! impl Validator for MyValidator {
//!     type Claims = String;
//!     type Error = String;
//!
//!     async fn validate(&self, token: &str) -> Result<String, String> {
//!         // validate token...
//!         Ok("user1".into())
//!     }
//! }
//!
//! let rs_config = ResourceServerConfig {
//!     resource_metadata_url:
//!         "https://mcp.example.com/.well-known/oauth-protected-resource".into(),
//!     default_scope: Some("mcp:tools".into()),
//! };
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(BearerAuth::new(MyValidator)).with_resource_server(rs_config));
//! ```

mod bearer;

pub mod oauth;

#[cfg(feature = "jwt")]
pub mod jwt;

pub use bearer::BearerAuth;

use futures::future::BoxFuture;
use http::{Request, Response, StatusCode};
use oauth::{ResourceServerConfig, www_authenticate_401};
use std::task::{Context, Poll};

/// Trait for validating incoming MCP requests.
///
/// Implement this with your auth logic (JWT validation, etc.).
/// On success, `Claims` is inserted into `http::Extensions`.
pub trait Authenticator: Clone + Send + Sync + 'static {
    /// The claims type produced on successful authentication.
    type Claims: Clone + Send + Sync + 'static;

    /// The error type returned on authentication failure.
    type Error: std::fmt::Display + Send;

    /// Validate the request and return claims, or an error.
    fn authenticate(
        &self,
        parts: &http::request::Parts,
    ) -> impl Future<Output = Result<Self::Claims, Self::Error>> + Send;
}

/// Trait for validating a credential string (e.g., a Bearer token).
///
/// Users implement this to provide their validation logic, then wrap it
/// in [`BearerAuth`] which handles credential extraction from the
/// `Authorization` header.
///
/// ```rust,ignore
/// use rmcp_axum::auth::{Validator, BearerAuth, AuthLayer};
///
/// #[derive(Clone)]
/// struct MyValidator;
///
/// impl Validator for MyValidator {
///     type Claims = String;
///     type Error = String;
///
///     async fn validate(&self, credential: &str) -> Result<String, String> {
///         if credential == "secret" {
///             Ok("authenticated".into())
///         } else {
///             Err("invalid".into())
///         }
///     }
/// }
///
/// let app = axum::Router::new()
///     .nest_service("/mcp", service)
///     .layer(AuthLayer::new(BearerAuth::new(MyValidator)));
/// ```
pub trait Validator: Clone + Send + Sync + 'static {
    /// The claims type produced on successful validation.
    type Claims: Clone + Send + Sync + 'static;

    /// The error type returned on validation failure.
    type Error: std::fmt::Display + Send;

    /// Validate the credential string and return claims, or an error.
    fn validate(
        &self,
        credential: &str,
    ) -> impl Future<Output = Result<Self::Claims, Self::Error>> + Send;
}

/// Tower [`Layer`](tower::Layer) that applies [`AuthService`].
#[derive(Clone)]
pub struct AuthLayer<A> {
    authenticator: A,
    resource_server: Option<ResourceServerConfig>,
}

impl<A> AuthLayer<A> {
    pub fn new(authenticator: A) -> Self {
        Self {
            authenticator,
            resource_server: None,
        }
    }

    /// Configure OAuth resource server metadata for spec-compliant error
    /// responses.
    ///
    /// When set, 401 responses will include a `WWW-Authenticate` header with
    /// `resource_metadata` and `scope` parameters per the MCP authorization
    /// specification.
    pub fn with_resource_server(mut self, config: ResourceServerConfig) -> Self {
        self.resource_server = Some(config);
        self
    }
}

impl<A, S> tower::Layer<S> for AuthLayer<A>
where
    A: Clone,
{
    type Service = AuthService<A, S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            authenticator: self.authenticator.clone(),
            resource_server: self.resource_server.clone(),
            inner,
        }
    }
}

/// Tower service that authenticates requests before forwarding them.
#[derive(Clone)]
pub struct AuthService<A, S> {
    authenticator: A,
    resource_server: Option<ResourceServerConfig>,
    inner: S,
}

impl<A, S, B> tower::Service<Request<B>> for AuthService<A, S>
where
    A: Authenticator,
    S: tower::Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Send,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let authenticator = self.authenticator.clone();
        let resource_server = self.resource_server.clone();
        let mut inner = self.inner.clone();
        // swap to ensure poll_ready state is preserved
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            let (parts, body) = req.into_parts();

            match authenticator.authenticate(&parts).await {
                Ok(claims) => {
                    let mut req = Request::from_parts(parts, body);
                    req.extensions_mut().insert(claims);
                    inner.call(req).await
                }
                Err(err) => {
                    let mut builder = Response::builder().status(StatusCode::UNAUTHORIZED);
                    if let Some(ref config) = resource_server {
                        builder = builder
                            .header(http::header::WWW_AUTHENTICATE, www_authenticate_401(config));
                    }
                    let response = builder
                        .body(axum::body::Body::from(err.to_string()))
                        .expect("valid response");
                    Ok(response)
                }
            }
        })
    }
}
