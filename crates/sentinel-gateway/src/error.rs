//! Error types for the Gateway

use thiserror::Error;

/// Gateway error type
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limit exceeded for {0}")]
    RateLimitExceeded(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Token expired for provider: {0}")]
    TokenExpired(String),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for GatewayError {
    fn from(e: serde_json::Error) -> Self {
        GatewayError::Serialization(e.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for GatewayError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        GatewayError::WebSocket(e.to_string())
    }
}

/// Result type for Gateway operations
pub type Result<T> = std::result::Result<T, GatewayError>;