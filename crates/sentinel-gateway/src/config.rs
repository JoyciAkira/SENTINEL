//! Gateway configuration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::{DEFAULT_HOST, DEFAULT_PORT};

/// Main gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Session configuration
    pub session: SessionSettings,

    /// Security configuration
    pub security: SecuritySettings,

    /// OAuth configuration
    pub oauth: OAuthSettings,

    /// Skills configuration path
    pub skills_path: Option<String>,

    /// Enable tracing
    pub tracing: bool,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            max_connections: 100,
            session: SessionSettings::default(),
            security: SecuritySettings::default(),
            oauth: OAuthSettings::default(),
            skills_path: None,
            tracing: true,
        }
    }
}

impl GatewayConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the host
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set max connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Get the socket address
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }

    /// Load configuration from a file
    pub fn from_file(path: &str) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn to_file(&self, path: &str) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Session settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
    /// Session timeout in seconds
    pub timeout_secs: u64,

    /// Maximum sessions per user
    pub max_sessions_per_user: usize,

    /// Enable session persistence
    pub persistence: bool,

    /// Session storage path (if persistence enabled)
    pub storage_path: Option<String>,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            timeout_secs: 3600, // 1 hour
            max_sessions_per_user: 5,
            persistence: false,
            storage_path: None,
        }
    }
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Enable DM pairing
    pub dm_pairing_enabled: bool,

    /// DM policy
    pub dm_policy: DmPolicyConfig,

    /// Rate limit requests per minute
    pub rate_limit_per_minute: u32,

    /// Enable authentication
    pub auth_required: bool,

    /// API key for authentication (optional)
    pub api_key: Option<String>,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            dm_pairing_enabled: true,
            dm_policy: DmPolicyConfig::Pairing,
            rate_limit_per_minute: 60,
            auth_required: false,
            api_key: None,
        }
    }
}

/// DM Policy configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DmPolicyConfig {
    /// Require pairing code for new senders
    Pairing,
    /// Allow all messages
    Open,
    /// Reject all messages
    Closed,
}

/// OAuth settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthSettings {
    /// Enable OAuth providers
    pub enabled: bool,

    /// Claude OAuth configuration
    pub claude: Option<OAuthProviderConfig>,

    /// ChatGPT OAuth configuration
    pub chatgpt: Option<OAuthProviderConfig>,

    /// Gemini OAuth configuration
    pub gemini: Option<OAuthProviderConfig>,

    /// Fallback order (provider names)
    pub fallback_order: Vec<String>,

    /// Auto-refresh tokens
    pub auto_refresh: bool,

    /// Refresh interval in seconds
    pub refresh_interval_secs: u64,
}

impl Default for OAuthSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            claude: None,
            chatgpt: None,
            gemini: None,
            fallback_order: vec![
                "claude".to_string(),
                "chatgpt".to_string(),
                "gemini".to_string(),
            ],
            auto_refresh: true,
            refresh_interval_secs: 300, // 5 minutes
        }
    }
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProviderConfig {
    /// Client ID (optional, some providers don't need it)
    pub client_id: Option<String>,

    /// Client secret (optional)
    pub client_secret: Option<String>,

    /// Redirect URI
    pub redirect_uri: Option<String>,

    /// Token path
    pub token_path: Option<String>,

    /// Scopes
    pub scopes: Vec<String>,

    /// Enabled
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.host, DEFAULT_HOST);
        assert!(config.session.timeout_secs > 0);
    }

    #[test]
    fn test_config_builder() {
        let config = GatewayConfig::new()
            .with_host("0.0.0.0")
            .with_port(8080)
            .with_max_connections(50);

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_connections, 50);
    }

    #[test]
    fn test_config_serialization() {
        let config = GatewayConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GatewayConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.port, parsed.port);
    }
}