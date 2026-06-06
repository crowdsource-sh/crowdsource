//! Error types for the crowdsource client.

use serde::Deserialize;

/// RFC 7807 Problem Details, as returned by `crowdsource-server`
/// (`crowdsource-server/src/error.rs`): `{ type, title, status, detail }`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProblemDetails {
    #[serde(rename = "type", default)]
    pub problem_type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub detail: Option<String>,
}

impl std::fmt::Display for ProblemDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Prefer the actionable `detail`, then `title`; never render blank.
        let msg = self
            .detail
            .as_deref()
            .or(self.title.as_deref())
            .unwrap_or("request failed");
        write!(f, "{msg} (HTTP {})", self.status)
    }
}

impl std::error::Error for ProblemDetails {}

/// The unified error type for all client operations.
#[derive(Debug, thiserror::Error)]
pub enum CrowdsourceError {
    /// A structured error response from the server (RFC 7807).
    #[error(transparent)]
    Api(#[from] ProblemDetails),

    /// Network / transport failure.
    #[cfg(feature = "client")]
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),

    /// Failed to (de)serialize a request or response body.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid client configuration (bad URL, missing credentials, etc.).
    #[error("configuration error: {0}")]
    Config(String),
}
