//! Pluggable authentication middleware for MCP servers.
//!
//! Provides a tower middleware that validates incoming requests using a
//! user-defined [`Authenticator`] trait. On success, the authenticated
//! claims are inserted into HTTP extensions and become accessible in MCP
//! tool handlers via `Extension(parts): Extension<Parts>`.
//!
//! # Example
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, Authenticator};
//! use http::request::Parts;
//!
//! #[derive(Clone)]
//! struct MyAuth;
//!
//! #[derive(Clone)]
//! struct Claims { user_id: String }
//!
//! impl Authenticator for MyAuth {
//!     type Claims = Claims;
//!     type Error = String;
//!
//!     async fn authenticate(&self, parts: &Parts) -> Result<Self::Claims, Self::Error> {
//!         let token = parts.headers
//!             .get("authorization")
//!             .and_then(|v| v.to_str().ok())
//!             .and_then(|v| v.strip_prefix("Bearer "))
//!             .ok_or("missing token")?;
//!         // validate token...
//!         Ok(Claims { user_id: "user1".into() })
//!     }
//! }
//!
//! let service = StreamableHttpService::new(/* ... */);
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(MyAuth));
//! ```

use futures::future::BoxFuture;
use http::{Request, Response, StatusCode};
use std::task::{Context, Poll};

/// Trait for validating incoming MCP requests.
///
/// Implement this with your auth logic (JWT validation, API key lookup, etc.).
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

/// Tower [`Layer`](tower::Layer) that applies [`AuthService`].
#[derive(Clone)]
pub struct AuthLayer<A> {
    authenticator: A,
}

impl<A> AuthLayer<A> {
    pub fn new(authenticator: A) -> Self {
        Self { authenticator }
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
            inner,
        }
    }
}

/// Tower service that authenticates requests before forwarding them.
#[derive(Clone)]
pub struct AuthService<A, S> {
    authenticator: A,
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
                    let response = Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(axum::body::Body::from(err.to_string()))
                        .expect("valid response");
                    Ok(response)
                }
            }
        })
    }
}
