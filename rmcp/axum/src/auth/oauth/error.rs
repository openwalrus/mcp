//! Spec-compliant OAuth 2.1 error responses for MCP resource servers.
//!
//! Provides helpers for building `WWW-Authenticate` headers as required by
//! [RFC 6750 §3](https://datatracker.ietf.org/doc/html/rfc6750#section-3) and
//! the [MCP Authorization specification](https://modelcontextprotocol.io/specification/draft/basic/authorization).

use http::HeaderValue;

/// Configuration for an MCP server acting as an OAuth 2.1 resource server.
///
/// Used to generate spec-compliant `WWW-Authenticate` headers in 401 and 403
/// responses.
#[derive(Clone, Debug)]
pub struct ResourceServerConfig {
    /// URL to the Protected Resource Metadata document (RFC 9728).
    ///
    /// Included as `resource_metadata="..."` in `WWW-Authenticate` headers.
    pub resource_metadata_url: String,
    /// Default scopes to include in 401 `WWW-Authenticate` challenges.
    ///
    /// Per the MCP spec, servers SHOULD include a `scope` parameter to indicate
    /// the scopes required for accessing the resource.
    pub default_scope: Option<String>,
}

/// Build a `WWW-Authenticate` header value for a 401 Unauthorized response.
///
/// Format: `Bearer resource_metadata="<url>"[, scope="<scopes>"]`
///
/// Per [RFC 9728 §5.1](https://datatracker.ietf.org/doc/html/rfc9728#name-www-authenticate-response)
/// and the MCP authorization spec.
pub fn www_authenticate_401(config: &ResourceServerConfig) -> HeaderValue {
    let mut value = format!(
        "Bearer resource_metadata=\"{}\"",
        config.resource_metadata_url,
    );
    if let Some(ref scope) = config.default_scope {
        value.push_str(&format!(", scope=\"{scope}\""));
    }
    // Safe: we control the format and it's valid ASCII.
    HeaderValue::from_str(&value).expect("valid WWW-Authenticate header")
}

/// Build a `WWW-Authenticate` header value for a 403 Forbidden response
/// with `insufficient_scope` error.
///
/// Format: `Bearer error="insufficient_scope", scope="<required>", resource_metadata="<url>"`
///
/// Per [RFC 6750 §3.1](https://datatracker.ietf.org/doc/html/rfc6750#section-3.1)
/// and the MCP authorization spec.
pub fn www_authenticate_403(config: &ResourceServerConfig, required_scope: &str) -> HeaderValue {
    let value = format!(
        "Bearer error=\"insufficient_scope\", scope=\"{required_scope}\", resource_metadata=\"{}\"",
        config.resource_metadata_url,
    );
    HeaderValue::from_str(&value).expect("valid WWW-Authenticate header")
}

/// Build a 403 Forbidden response with the proper `WWW-Authenticate` header
/// for insufficient scope errors.
///
/// Use this in MCP tool handlers when a request has valid auth but lacks
/// the required scopes.
///
/// ```rust,ignore
/// use rmcp_axum::auth::oauth::{ResourceServerConfig, insufficient_scope_response};
///
/// fn check_scope(config: &ResourceServerConfig) -> Result<(), Response<Body>> {
///     // ... check if claims have required scope ...
///     Err(insufficient_scope_response(config, "files:write"))
/// }
/// ```
pub fn insufficient_scope_response(
    config: &ResourceServerConfig,
    required_scope: &str,
) -> http::Response<axum::body::Body> {
    http::Response::builder()
        .status(http::StatusCode::FORBIDDEN)
        .header(
            http::header::WWW_AUTHENTICATE,
            www_authenticate_403(config, required_scope),
        )
        .body(axum::body::Body::from("insufficient scope"))
        .expect("valid response")
}
