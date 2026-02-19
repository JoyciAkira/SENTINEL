//! OAuth provider management for LLM providers
//!
//! Supports Claude (Anthropic Pro/Max), ChatGPT, and Gemini OAuth authentication
//! with automatic token rotation and fallback.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{GatewayError, Result};

/// Supported OAuth providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProviderType {
    Claude,
    ChatGPT,
    Gemini,
}

impl std::fmt::Display for OAuthProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProviderType::Claude => write!(f, "claude"),
            OAuthProviderType::ChatGPT => write!(f, "chatgpt"),
            OAuthProviderType::Gemini => write!(f, "gemini"),
        }
    }
}

/// OAuth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    /// Access token
    pub access_token: String,

    /// Refresh token (optional)
    pub refresh_token: Option<String>,

    /// Token type (usually "Bearer")
    pub token_type: String,

    /// Expires in seconds
    pub expires_in: u64,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Scopes
    pub scopes: Vec<String>,
}

impl OAuthToken {
    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let elapsed = (Utc::now() - self.created_at).num_seconds() as u64;
        // Consider expired 5 minutes before actual expiration
        elapsed >= self.expires_in.saturating_sub(300)
    }

    /// Check if token needs refresh
    pub fn needs_refresh(&self) -> bool {
        let elapsed = (Utc::now() - self.created_at).num_seconds() as u64;
        // Refresh when 50% of lifetime has passed
        elapsed >= self.expires_in / 2
    }

    /// Time until expiration
    pub fn time_until_expiry(&self) -> std::time::Duration {
        let elapsed = (Utc::now() - self.created_at).num_seconds() as u64;
        let remaining = self.expires_in.saturating_sub(elapsed);
        std::time::Duration::from_secs(remaining)
    }
}

/// OAuth provider configuration
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    /// Provider type
    pub provider_type: OAuthProviderType,

    /// Authorization URL
    pub auth_url: String,

    /// Token URL
    pub token_url: String,

    /// Client ID (optional for some providers)
    pub client_id: Option<String>,

    /// Client secret (optional)
    pub client_secret: Option<String>,

    /// Redirect URI
    pub redirect_uri: Option<String>,

    /// Default scopes
    pub scopes: Vec<String>,

    /// Is enabled
    pub enabled: bool,
}

impl OAuthProviderConfig {
    /// Claude (Anthropic) configuration
    pub fn claude() -> Self {
        Self {
            provider_type: OAuthProviderType::Claude,
            auth_url: "https://claude.ai/oauth/authorize".to_string(),
            token_url: "https://claude.ai/oauth/token".to_string(),
            client_id: None, // Claude uses session-based auth
            client_secret: None,
            redirect_uri: None,
            scopes: vec!["read".to_string(), "write".to_string()],
            enabled: true,
        }
    }

    /// ChatGPT configuration
    pub fn chatgpt() -> Self {
        Self {
            provider_type: OAuthProviderType::ChatGPT,
            auth_url: "https://auth.openai.com/authorize".to_string(),
            token_url: "https://auth.openai.com/oauth/token".to_string(),
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            enabled: true,
        }
    }

    /// Gemini configuration
    pub fn gemini() -> Self {
        Self {
            provider_type: OAuthProviderType::Gemini,
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            scopes: vec![
                "https://www.googleapis.com/auth/generative-language".to_string(),
            ],
            enabled: true,
        }
    }
}

/// OAuth provider with token
#[derive(Debug)]
pub struct OAuthProvider {
    /// Provider configuration
    pub config: OAuthProviderConfig,

    /// Current token
    token: Option<OAuthToken>,

    /// Token file path for persistence
    token_path: Option<String>,
}

impl OAuthProvider {
    pub fn new(config: OAuthProviderConfig) -> Self {
        Self {
            config,
            token: None,
            token_path: None,
        }
    }

    /// Set token file path for persistence
    pub fn with_persistence(mut self, path: impl Into<String>) -> Self {
        self.token_path = Some(path.into());
        self
    }

    /// Set token
    pub fn set_token(&mut self, token: OAuthToken) {
        self.token = Some(token.clone());
        
        // Persist if path is set
        if let Some(ref path) = self.token_path {
            if let Ok(json) = serde_json::to_string(&token) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    /// Get current token
    pub fn token(&self) -> Option<&OAuthToken> {
        self.token.as_ref()
    }

    /// Check if provider has valid token
    pub fn has_valid_token(&self) -> bool {
        self.token.as_ref().map(|t| !t.is_expired()).unwrap_or(false)
    }

    /// Check if token needs refresh
    pub fn needs_refresh(&self) -> bool {
        self.token.as_ref().map(|t| t.needs_refresh()).unwrap_or(false)
    }

    /// Load token from file
    pub fn load_token(&mut self) -> Result<()> {
        if let Some(ref path) = self.token_path {
            if std::path::Path::new(path).exists() {
                let content = std::fs::read_to_string(path)?;
                let token: OAuthToken = serde_json::from_str(&content)?;
                self.token = Some(token);
                tracing::info!("Loaded token for {}", self.config.provider_type);
            }
        }
        Ok(())
    }

    /// Clear token
    pub fn clear_token(&mut self) {
        self.token = None;
        if let Some(ref path) = self.token_path {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// OAuth manager - handles all OAuth providers
pub struct OAuthManager {
    /// Registered providers
    providers: Arc<RwLock<HashMap<OAuthProviderType, OAuthProvider>>>,

    /// Fallback order
    fallback_order: Arc<RwLock<Vec<OAuthProviderType>>>,

    /// Auto-refresh enabled
    auto_refresh: bool,
}

impl OAuthManager {
    pub fn new(auto_refresh: bool) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            fallback_order: Arc::new(RwLock::new(vec![
                OAuthProviderType::Claude,
                OAuthProviderType::ChatGPT,
                OAuthProviderType::Gemini,
            ])),
            auto_refresh,
        }
    }

    /// Register a provider
    pub async fn register_provider(&self, provider: OAuthProvider) {
        let provider_type = provider.config.provider_type;
        let mut providers = self.providers.write().await;
        providers.insert(provider_type, provider);
        tracing::info!("Registered OAuth provider: {}", provider_type);
    }

    /// Get a provider by type
    pub async fn get_provider(&self, provider_type: OAuthProviderType) -> Option<OAuthProvider> {
        let providers = self.providers.read().await;
        providers.get(&provider_type).cloned()
    }

    /// Get available provider with valid token (with fallback)
    pub async fn get_available_provider(&self) -> Option<(OAuthProviderType, OAuthToken)> {
        let fallback_order = self.fallback_order.read().await;
        let providers = self.providers.read().await;

        for provider_type in fallback_order.iter() {
            if let Some(provider) = providers.get(provider_type) {
                if provider.has_valid_token() {
                    if let Some(token) = provider.token() {
                        return Some((*provider_type, token.clone()));
                    }
                }
            }
        }

        None
    }

    /// Set token for a provider
    pub async fn set_token(&self, provider_type: OAuthProviderType, token: OAuthToken) -> Result<()> {
        let mut providers = self.providers.write().await;
        if let Some(provider) = providers.get_mut(&provider_type) {
            provider.set_token(token);
            Ok(())
        } else {
            Err(GatewayError::OAuthError(format!(
                "Provider not registered: {}",
                provider_type
            )))
        }
    }

    /// Check if any provider has valid token
    pub async fn has_any_valid_token(&self) -> bool {
        let providers = self.providers.read().await;
        providers.values().any(|p| p.has_valid_token())
    }

    /// Get all providers needing refresh
    pub async fn providers_needing_refresh(&self) -> Vec<OAuthProviderType> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .filter(|(_, p)| p.needs_refresh())
            .map(|(t, _)| *t)
            .collect()
    }

    /// Set fallback order
    pub async fn set_fallback_order(&self, order: Vec<OAuthProviderType>) {
        let mut fallback = self.fallback_order.write().await;
        *fallback = order;
    }

    /// Get provider status
    pub async fn status(&self) -> HashMap<OAuthProviderType, ProviderStatus> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .map(|(t, p)| {
                let status = ProviderStatus {
                    enabled: p.config.enabled,
                    has_token: p.token.is_some(),
                    token_valid: p.has_valid_token(),
                    needs_refresh: p.needs_refresh(),
                };
                (*t, status)
            })
            .collect()
    }
}

impl Clone for OAuthProvider {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            token: self.token.clone(),
            token_path: self.token_path.clone(),
        }
    }
}

/// Provider status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub enabled: bool,
    pub has_token: bool,
    pub token_valid: bool,
    pub needs_refresh: bool,
}

/// OAuth flow helper for CLI-based authentication
pub struct OAuthFlow;

impl OAuthFlow {
    /// Generate authorization URL for manual authentication
    pub fn generate_auth_url(config: &OAuthProviderConfig, state: &str) -> String {
        let mut url = format!(
            "{}?response_type=code&state={}",
            config.auth_url, state
        );

        if let Some(ref client_id) = config.client_id {
            url.push_str(&format!("&client_id={}", client_id));
        }

        if let Some(ref redirect_uri) = config.redirect_uri {
            url.push_str(&format!("&redirect_uri={}", urlencoding::encode(redirect_uri)));
        }

        if !config.scopes.is_empty() {
            url.push_str("&scope=");
            url.push_str(&config.scopes.join("%20"));
        }

        url
    }

    /// Extract authorization code from callback URL
    pub fn extract_code(callback_url: &str) -> Option<String> {
        let url = url::Url::parse(callback_url).ok()?;
        let params: HashMap<String, String> = url
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        params.get("code").cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_token_expiry() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: 3600, // 1 hour
            created_at: Utc::now(),
            scopes: vec![],
        };

        assert!(!token.is_expired());
        assert!(!token.needs_refresh()); // Just created, shouldn't need refresh yet
    }

    #[test]
    fn test_oauth_token_expired() {
        let token = OAuthToken {
            access_token: "test".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_in: 100,
            created_at: Utc::now() - chrono::Duration::seconds(200),
            scopes: vec![],
        };

        assert!(token.is_expired());
    }

    #[test]
    fn test_provider_configs() {
        let claude = OAuthProviderConfig::claude();
        assert_eq!(claude.provider_type, OAuthProviderType::Claude);

        let chatgpt = OAuthProviderConfig::chatgpt();
        assert_eq!(chatgpt.provider_type, OAuthProviderType::ChatGPT);

        let gemini = OAuthProviderConfig::gemini();
        assert_eq!(gemini.provider_type, OAuthProviderType::Gemini);
    }

    #[tokio::test]
    async fn test_oauth_manager() {
        let manager = OAuthManager::new(true);

        let provider = OAuthProvider::new(OAuthProviderConfig::claude());
        manager.register_provider(provider).await;

        assert!(manager.get_provider(OAuthProviderType::Claude).await.is_some());
        assert!(!manager.has_any_valid_token().await);
    }

    #[test]
    fn test_auth_url_generation() {
        let config = OAuthProviderConfig::claude();
        let url = OAuthFlow::generate_auth_url(&config, "test-state");

        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=test-state"));
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}