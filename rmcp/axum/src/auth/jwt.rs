//! JWT validation plugin for OAuth authentication.
//!
//! Validates JWT Bearer tokens by verifying signatures against a JWKS
//! endpoint. Implements [`Validator`](super::Validator) producing
//! [`OAuthClaims`].
//!
//! Requires the `jwt` feature.
//!
//! ```rust,ignore
//! use rmcp_axum::auth::{AuthLayer, BearerAuth, jwt::JwtValidator};
//!
//! let validator = JwtValidator::from_jwks_url(
//!     "https://auth.example.com/.well-known/jwks.json",
//! )
//! .audience("my-mcp-server")
//! .issuer("https://auth.example.com")
//! .build()
//! .await
//! .expect("failed to fetch JWKS");
//!
//! let app = axum::Router::new()
//!     .nest_service("/mcp", service)
//!     .layer(AuthLayer::new(BearerAuth::new(validator)));
//! ```

use crate::auth::Validator;
use anyhow::{Context, Result, anyhow};
use jsonwebtoken::{DecodingKey, TokenData, Validation, decode, jwk::JwkSet};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Standard OAuth 2.1 token claims.
#[derive(Clone, Debug)]
pub struct OAuthClaims {
    /// The subject (user/client identifier).
    pub sub: String,
    /// The issuer.
    pub iss: Option<String>,
    /// The audience.
    pub aud: Option<Vec<String>>,
    /// Granted scopes (space-separated in the token, parsed here).
    pub scope: Vec<String>,
    /// Expiration time (seconds since epoch).
    pub exp: Option<u64>,
}

/// Raw JWT claims deserialized from the token payload.
#[derive(Debug, Serialize, Deserialize)]
struct RawClaims {
    sub: Option<String>,
    iss: Option<String>,
    aud: Option<Audience>,
    scope: Option<String>,
    exp: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    fn into_vec(self) -> Vec<String> {
        match self {
            Audience::Single(s) => vec![s],
            Audience::Multiple(v) => v,
        }
    }
}

/// Builder for [`JwtValidator`].
pub struct JwtValidatorBuilder {
    jwks_url: String,
    audience: Option<String>,
    issuer: Option<String>,
}

impl JwtValidatorBuilder {
    /// Require the `aud` claim to match this value.
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Require the `iss` claim to match this value.
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Fetch the JWKS and build the validator.
    pub async fn build(self) -> Result<JwtValidator> {
        let jwks = fetch_jwks(&self.jwks_url).await?;

        let mut validation = Validation::default();
        if let Some(ref aud) = self.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }
        if let Some(ref iss) = self.issuer {
            validation.set_issuer(&[iss]);
        }

        Ok(JwtValidator {
            inner: Arc::new(JwtValidatorInner {
                jwks: RwLock::new(jwks),
                jwks_url: self.jwks_url,
                validation,
            }),
        })
    }
}

struct JwtValidatorInner {
    jwks: RwLock<JwkSet>,
    jwks_url: String,
    validation: Validation,
}

/// JWT validator that verifies tokens against a JWKS endpoint.
#[derive(Clone)]
pub struct JwtValidator {
    inner: Arc<JwtValidatorInner>,
}

impl JwtValidator {
    /// Start building a JWT validator from a JWKS URL.
    pub fn from_jwks_url(url: impl Into<String>) -> JwtValidatorBuilder {
        JwtValidatorBuilder {
            jwks_url: url.into(),
            audience: None,
            issuer: None,
        }
    }

    /// Refresh the JWKS from the configured endpoint.
    pub async fn refresh_jwks(&self) -> Result<()> {
        let jwks = fetch_jwks(&self.inner.jwks_url).await?;
        *self.inner.jwks.write().await = jwks;
        Ok(())
    }

    fn decode_token(&self, token: &str, jwks: &JwkSet) -> Result<OAuthClaims> {
        let header = jsonwebtoken::decode_header(token).context("invalid JWT header")?;
        let kid = header
            .kid
            .as_deref()
            .ok_or_else(|| anyhow!("JWT missing kid header"))?;
        let jwk = jwks
            .find(kid)
            .ok_or_else(|| anyhow!("no matching key for kid: {kid}"))?;
        let key = DecodingKey::from_jwk(jwk).context("invalid JWK")?;
        let data: TokenData<RawClaims> =
            decode(token, &key, &self.inner.validation).context("JWT validation failed")?;

        let claims = data.claims;
        Ok(OAuthClaims {
            sub: claims.sub.unwrap_or_default(),
            iss: claims.iss,
            aud: claims.aud.map(Audience::into_vec),
            scope: claims
                .scope
                .map(|s| s.split_whitespace().map(String::from).collect())
                .unwrap_or_default(),
            exp: claims.exp,
        })
    }
}

impl Validator for JwtValidator {
    type Claims = OAuthClaims;
    type Error = anyhow::Error;

    async fn validate(&self, token: &str) -> Result<OAuthClaims> {
        // Try with current JWKS.
        let jwks = self.inner.jwks.read().await;
        match self.decode_token(token, &jwks) {
            Ok(claims) => return Ok(claims),
            Err(e) => {
                // If key not found, try refreshing JWKS (key rotation).
                if !format!("{e}").contains("no matching key") {
                    return Err(e);
                }
            }
        }
        drop(jwks);

        // Refresh and retry once.
        self.refresh_jwks().await.context("JWKS refresh failed")?;

        let jwks = self.inner.jwks.read().await;
        self.decode_token(token, &jwks)
    }
}

async fn fetch_jwks(url: &str) -> Result<JwkSet> {
    let resp = reqwest::get(url).await.context("failed to fetch JWKS")?;
    let jwks = resp
        .json::<JwkSet>()
        .await
        .context("failed to parse JWKS")?;
    Ok(jwks)
}
