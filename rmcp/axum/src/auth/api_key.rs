//! API key authentication plugin.
//!
//! Extracts a credential from a configurable header and delegates
//! validation to a [`Validator`](super::Validator).
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, ApiKeyAuth, Validator};
//!
//! #[derive(Clone)]
//! struct KeyStore;
//!
//! impl Validator for KeyStore {
//!     type Claims = String;
//!     type Error = String;
//!
//!     async fn validate(&self, key: &str) -> Result<String, String> {
//!         if key == "secret-key" { Ok(key.into()) } else { Err("invalid".into()) }
//!     }
//! }
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(ApiKeyAuth::new("x-api-key", KeyStore)));
//! ```

use crate::auth::{Authenticator, Validator};

/// API key authenticator.
///
/// Extracts the value of a specified header and passes it to the inner
/// [`Validator`].
#[derive(Clone)]
pub struct ApiKeyAuth<V> {
    header: http::HeaderName,
    validator: V,
}

impl<V> ApiKeyAuth<V> {
    pub fn new<H>(header: H, validator: V) -> Self
    where
        H: TryInto<http::HeaderName>,
        H::Error: std::fmt::Debug,
    {
        Self {
            header: header.try_into().expect("valid header name"),
            validator,
        }
    }
}

impl<V> Authenticator for ApiKeyAuth<V>
where
    V: Validator,
{
    type Claims = V::Claims;
    type Error = String;

    async fn authenticate(
        &self,
        parts: &http::request::Parts,
    ) -> Result<Self::Claims, Self::Error> {
        let key = parts
            .headers
            .get(&self.header)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| format!("missing {} header", self.header))?;

        self.validator.validate(key).await.map_err(|e| e.to_string())
    }
}
