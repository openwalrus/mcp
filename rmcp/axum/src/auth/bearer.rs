//! Bearer token authentication plugin.
//!
//! Extracts a Bearer token from the `Authorization` header and delegates
//! validation to a [`Validator`](super::Validator).
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, BearerAuth, Validator};
//!
//! #[derive(Clone)]
//! struct MyValidator;
//!
//! impl Validator for MyValidator {
//!     type Claims = String;
//!     type Error = String;
//!
//!     async fn validate(&self, token: &str) -> Result<String, String> {
//!         if token == "secret" { Ok("ok".into()) } else { Err("bad".into()) }
//!     }
//! }
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(BearerAuth::new(MyValidator)));
//! ```

use crate::auth::{Authenticator, Validator};

/// Bearer token authenticator.
///
/// Extracts the token from `Authorization: Bearer <token>` and passes it
/// to the inner [`Validator`].
#[derive(Clone)]
pub struct BearerAuth<V> {
    validator: V,
}

impl<V> BearerAuth<V> {
    pub fn new(validator: V) -> Self {
        Self { validator }
    }
}

impl<V> Authenticator for BearerAuth<V>
where
    V: Validator,
{
    type Claims = V::Claims;
    type Error = String;

    async fn authenticate(
        &self,
        parts: &http::request::Parts,
    ) -> Result<Self::Claims, Self::Error> {
        let token = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| "missing or malformed Authorization header".to_string())?;

        self.validator
            .validate(token)
            .await
            .map_err(|e| e.to_string())
    }
}
