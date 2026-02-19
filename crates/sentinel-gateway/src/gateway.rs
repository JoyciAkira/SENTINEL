//! Main Gateway implementation
//!
//! Unified WebSocket gateway for SENTINEL multi-channel communication.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::channel::{Channel, ChannelId, ChannelMessage, ChannelRouter, ChannelType, OutgoingMessage};
use crate::config::GatewayConfig;
use crate::oauth::OAuthManager;
use crate::security::{ChannelSecurity, DmPolicy, SecurityDecision, UserId};
use crate::session::SessionManager;
use crate::skills::SkillRegistry;
use crate::{GatewayError, Result};

/// Gateway state shared across handlers
#[derive(Clone)]
pub struct GatewayState {
    pub config: GatewayConfig,
    pub channel_router: Arc<ChannelRouter>,
    pub session_manager: Arc<SessionManager>,
    pub oauth_manager: Arc<OAuthManager>,
    pub security: Arc<ChannelSecurity>,
    pub skill_registry: Arc<SkillRegistry>,
    pub shutdown_tx: broadcast::Sender<()>,
}

impl GatewayState {
    pub fn new(config: GatewayConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config: config.clone(),
            channel_router: Arc::new(ChannelRouter::new(1000)),
            session_manager: Arc::new(SessionManager::new(
                config.session.max_sessions_per_user,
                config.session.timeout_secs,
            )),
            oauth_manager: Arc::new(OAuthManager::new(config.oauth.auto_refresh)),
            security: Arc::new(ChannelSecurity::new(
                config.security.dm_policy.into(),
                config.security.rate_limit_per_minute,
            )),
            skill_registry: Arc::new(SkillRegistry::new()),
            shutdown_tx,
        }
    }
}

/// Convert DmPolicyConfig to DmPolicy
impl From<crate::config::DmPolicyConfig> for DmPolicy {
    fn from(policy: crate::config::DmPolicyConfig) -> Self {
        match policy {
            crate::config::DmPolicyConfig::Pairing => DmPolicy::Pairing,
            crate::config::DmPolicyConfig::Open => DmPolicy::Open,
            crate::config::DmPolicyConfig::Closed => DmPolicy::Closed,
        }
    }
}

/// Main Gateway
pub struct Gateway {
    state: Arc<GatewayState>,
}

impl Gateway {
    /// Create a new gateway with configuration
    pub fn new(config: GatewayConfig) -> Self {
        let state = Arc::new(GatewayState::new(config));
        Self { state }
    }

    /// Create a gateway with default configuration
    pub fn default_config() -> Self {
        Self::new(GatewayConfig::default())
    }

    /// Get gateway state
    pub fn state(&self) -> Arc<GatewayState> {
        self.state.clone()
    }

    /// Build the Axum router
    pub fn build_router(&self) -> Router {
        Router::new()
            .route("/", get(Self::handle_index))
            .route("/ws", get(Self::handle_ws_upgrade))
            .route("/health", get(Self::handle_health))
            .route("/status", get(Self::handle_status))
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .with_state(self.state.clone())
    }

    /// Start the gateway server
    pub async fn start(&self) -> Result<()> {
        let addr = self.state.config.socket_addr();
        let router = self.build_router();

        tracing::info!("🦞 SENTINEL Gateway starting on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| GatewayError::Io(e))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| GatewayError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Shutdown the gateway
    pub fn shutdown(&self) {
        let _ = self.state.shutdown_tx.send(());
        tracing::info!("Gateway shutdown initiated");
    }

    // HTTP handlers

    async fn handle_index() -> Html<&'static str> {
        Html(include_str!("../static/index.html"))
    }

    async fn handle_health() -> impl IntoResponse {
        axum::Json(serde_json::json!({
            "status": "healthy",
            "version": crate::VERSION
        }))
    }

    async fn handle_status(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
        let channel_count = state.channel_router.channel_count().await;
        let session_count = state.session_manager.session_count();

        axum::Json(serde_json::json!({
            "version": crate::VERSION,
            "channels": channel_count,
            "sessions": session_count,
            "security": {
                "dm_policy": format!("{:?}", state.security.policy()),
                "allowlist_count": state.security.allowlist_count(),
            }
        }))
    }

    async fn handle_ws_upgrade(
        ws: WebSocketUpgrade,
        State(state): State<Arc<GatewayState>>,
    ) -> Response {
        ws.on_upgrade(move |socket| Self::handle_ws_connection(socket, state))
    }

    async fn handle_ws_connection(socket: WebSocket, state: Arc<GatewayState>) {
        let (mut tx, mut rx) = socket.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<OutgoingMessage>(100);

        // Create channel (uses OutgoingMessage = String)
        let channel = Channel::new(ChannelType::WebSocket, outgoing_tx);
        let channel_id = channel.id.clone();

        // Register channel
        if let Err(e) = state.channel_router.register(channel).await {
            tracing::error!("Failed to register channel: {}", e);
            return;
        }

        // Create session
        let session_id = state
            .session_manager
            .create_session(channel_id.clone(), None)
            .ok();

        tracing::info!("WebSocket connected: {}", channel_id);

        // Activate session
        if let Some(ref sid) = session_id {
            let _ = state.session_manager.update_session(sid, |s| s.activate());
        }

        // Clone for the outgoing task
        let outgoing_channel_id = channel_id.clone();
        let state_clone = state.clone();
        
        // Spawn outgoing message handler - forwards to WebSocket
        tokio::spawn(async move {
            while let Some(json_msg) = outgoing_rx.recv().await {
                if tx.send(Message::Text(json_msg)).await.is_err() {
                    break;
                }
            }
            tracing::debug!("Outgoing handler stopped for {}", outgoing_channel_id);
        });

        // Process incoming messages
        while let Some(msg) = rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Parse and handle message
                    if let Err(e) = Self::handle_text_message(&state_clone, &channel_id, &text).await {
                        tracing::error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Binary(data)) => {
                    tracing::debug!("Binary message received: {} bytes", data.len());
                }
                Ok(Message::Ping(_)) => {
                    // Pong is automatically sent by axum
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket closing: {}", channel_id);
                    break;
                }
                _ => {}
            }
        }

        // Cleanup
        let _ = state.channel_router.unregister(&channel_id).await;
        if let Some(sid) = session_id {
            let _ = state.session_manager.end_session(&sid);
        }
        tracing::info!("WebSocket disconnected: {}", channel_id);
    }

    async fn handle_text_message(
        state: &Arc<GatewayState>,
        channel_id: &ChannelId,
        text: &str,
    ) -> Result<()> {
        // Parse message
        let message: ChannelMessage = serde_json::from_str(text)?;

        // Security check
        let user_id = UserId::new(&channel_id.0);
        match state.security.validate_sender(&user_id) {
            SecurityDecision::Allow => {}
            SecurityDecision::RequestPairing => {
                let code = state.security.create_pairing(user_id, "websocket");
                tracing::info!("Pairing required. Code: {}", code);
                // In a real implementation, send pairing request back
                return Ok(());
            }
            SecurityDecision::Reject => {
                return Err(GatewayError::SecurityViolation("Sender rejected".to_string()));
            }
        }

        // Route message
        state.channel_router.route(message).await?;

        Ok(())
    }
}

/// Client for connecting to a SENTINEL Gateway
pub struct GatewayClient {
    url: String,
}

impl GatewayClient {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }

    /// Connect to the gateway
    pub async fn connect(&self) -> Result<WebSocketConnection> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&self.url)
            .await
            .map_err(|e| GatewayError::WebSocket(e.to_string()))?;

        Ok(WebSocketConnection::new(ws_stream))
    }
}

/// WebSocket connection wrapper
pub struct WebSocketConnection {
    ws: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl WebSocketConnection {
    fn new(
        ws: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    ) -> Self {
        Self { ws }
    }

    /// Send a message
    pub async fn send(&mut self, msg: &str) -> Result<()> {
        self.ws
            .send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string()))
            .await
            .map_err(|e| GatewayError::WebSocket(e.to_string()))
    }

    /// Receive a message
    pub async fn recv(&mut self) -> Result<Option<String>> {
        match self.ws.next().await {
            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => Ok(Some(text)),
            Some(Ok(_)) => Ok(None),
            Some(Err(e)) => Err(GatewayError::WebSocket(e.to_string())),
            None => Ok(None),
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        self.ws
            .close(None)
            .await
            .map_err(|e| GatewayError::WebSocket(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_creation() {
        let config = GatewayConfig::default();
        let gateway = Gateway::new(config);
        assert!(gateway.state().config.port > 0);
    }

    #[test]
    fn test_gateway_state() {
        let state = GatewayState::new(GatewayConfig::default());
        assert!(state.session_manager.session_count() == 0);
    }

    #[tokio::test]
    async fn test_channel_registration() {
        let state = GatewayState::new(GatewayConfig::default());
        let (tx, _rx) = mpsc::channel(10);
        let channel = Channel::new(ChannelType::VsCode, tx);

        state
            .channel_router
            .register(channel)
            .await
            .unwrap();
        assert_eq!(state.channel_router.channel_count().await, 1);
    }
}