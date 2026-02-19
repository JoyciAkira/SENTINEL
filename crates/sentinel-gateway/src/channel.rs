//! Channel routing and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{GatewayError, Result};

/// Channel type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// VSCode extension
    VsCode,
    /// CLI client
    Cli,
    /// Web UI
    WebUi,
    /// REST API
    RestApi,
    /// WebSocket direct
    WebSocket,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::VsCode => write!(f, "vscode"),
            ChannelType::Cli => write!(f, "cli"),
            ChannelType::WebUi => write!(f, "webui"),
            ChannelType::RestApi => write!(f, "rest"),
            ChannelType::WebSocket => write!(f, "ws"),
        }
    }
}

/// Unique channel identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub String);

impl ChannelId {
    pub fn new(channel_type: ChannelType) -> Self {
        Self(format!("{}:{}", channel_type, Uuid::new_v4()))
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Channel metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetadata {
    /// Channel type
    pub channel_type: ChannelType,

    /// User agent string
    pub user_agent: Option<String>,

    /// Client version
    pub client_version: Option<String>,

    /// Additional metadata
    pub extra: HashMap<String, String>,
}

/// Message sent through a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Message ID
    pub id: Uuid,

    /// Source channel
    pub source: ChannelId,

    /// Target session (optional, broadcast if None)
    pub target_session: Option<String>,

    /// Message type
    pub message_type: MessageType,

    /// Payload
    pub payload: serde_json::Value,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ChannelMessage {
    pub fn new(source: ChannelId, message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            target_session: None,
            message_type,
            payload,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target_session = Some(target.into());
        self
    }
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request to execute a task
    TaskRequest,

    /// Response to a task
    TaskResponse,

    /// Stream chunk from agent
    StreamChunk,

    /// Stream complete
    StreamComplete,

    /// Error
    Error,

    /// Ping/Pong
    Ping,

    /// Status update
    Status,

    /// Event notification
    Event,

    /// Control message
    Control,
}

/// Outgoing message type (serialized JSON string)
pub type OutgoingMessage = String;

/// Active channel
#[derive(Debug, Clone)]
pub struct Channel {
    /// Channel ID
    pub id: ChannelId,

    /// Channel metadata
    pub metadata: ChannelMetadata,

    /// Sender for outgoing messages (JSON string)
    tx: mpsc::Sender<OutgoingMessage>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl Channel {
    pub fn new(
        channel_type: ChannelType,
        tx: mpsc::Sender<OutgoingMessage>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: ChannelId::new(channel_type),
            metadata: ChannelMetadata {
                channel_type,
                user_agent: None,
                client_version: None,
                extra: HashMap::new(),
            },
            tx,
            created_at: now,
            last_activity: now,
        }
    }

    pub fn with_metadata(mut self, metadata: ChannelMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Send a message to this channel
    pub async fn send(&self, message: ChannelMessage) -> Result<()> {
        let json = serde_json::to_string(&message)?;
        self.tx.send(json).await.map_err(|e| {
            GatewayError::ChannelError(format!("Failed to send message: {}", e))
        })
    }

    /// Update last activity
    pub fn touch(&mut self) {
        self.last_activity = chrono::Utc::now();
    }
}

/// Channel router - manages all active channels and routes messages
pub struct ChannelRouter {
    /// Active channels
    channels: Arc<RwLock<HashMap<ChannelId, Channel>>>,

    /// Broadcast sender for all messages
    broadcast_tx: broadcast::Sender<ChannelMessage>,

    /// Inbound message receiver
    inbound_rx: mpsc::Receiver<ChannelMessage>,

    /// Inbound message sender (for external use)
    inbound_tx: mpsc::Sender<ChannelMessage>,
}

impl ChannelRouter {
    pub fn new(broadcast_capacity: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(broadcast_capacity);
        let (inbound_tx, inbound_rx) = mpsc::channel(1000);

        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            inbound_rx,
            inbound_tx,
        }
    }

    /// Get the inbound message sender
    pub fn inbound_sender(&self) -> mpsc::Sender<ChannelMessage> {
        self.inbound_tx.clone()
    }

    /// Subscribe to all broadcast messages
    pub fn subscribe(&self) -> broadcast::Receiver<ChannelMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Register a new channel
    pub async fn register(&self, mut channel: Channel) -> Result<()> {
        channel.touch();
        let id = channel.id.clone();
        let mut channels = self.channels.write().await;
        channels.insert(id.clone(), channel);
        tracing::info!("Channel registered: {:?}", id);
        Ok(())
    }

    /// Unregister a channel
    pub async fn unregister(&self, id: &ChannelId) -> Result<()> {
        let mut channels = self.channels.write().await;
        if channels.remove(id).is_some() {
            tracing::info!("Channel unregistered: {:?}", id);
            Ok(())
        } else {
            Err(GatewayError::ChannelError(format!(
                "Channel not found: {}",
                id
            )))
        }
    }

    /// Get a channel by ID
    pub async fn get(&self, id: &ChannelId) -> Option<Channel> {
        let channels = self.channels.read().await;
        channels.get(id).cloned()
    }

    /// Route a message to target channel(s)
    pub async fn route(&self, message: ChannelMessage) -> Result<()> {
        // Broadcast to all subscribers
        let _ = self.broadcast_tx.send(message.clone());

        // Also send to specific channel if targeted
        if let Some(ref _target) = message.target_session {
            // Find channels for this session
            let channels = self.channels.read().await;
            for (id, channel) in channels.iter() {
                if id == &message.source {
                    continue; // Don't send back to source
                }
                // In a full implementation, we'd check session mapping here
                let _ = channel.send(message.clone()).await;
            }
        }

        Ok(())
    }

    /// Get all active channels
    pub async fn active_channels(&self) -> Vec<ChannelId> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    /// Get channel count
    pub async fn channel_count(&self) -> usize {
        let channels = self.channels.read().await;
        channels.len()
    }

    /// Process inbound messages
    pub async fn process_inbound(&mut self) -> Option<ChannelMessage> {
        self.inbound_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_type_display() {
        assert_eq!(ChannelType::VsCode.to_string(), "vscode");
        assert_eq!(ChannelType::Cli.to_string(), "cli");
    }

    #[test]
    fn test_channel_id_generation() {
        let id1 = ChannelId::new(ChannelType::VsCode);
        let id2 = ChannelId::new(ChannelType::VsCode);
        assert_ne!(id1, id2); // Each ID should be unique
        assert!(id1.0.starts_with("vscode:"));
    }

    #[tokio::test]
    async fn test_channel_router() {
        let router = ChannelRouter::new(100);

        let (tx, _rx) = mpsc::channel(10);
        let channel = Channel::new(ChannelType::VsCode, tx);
        let id = channel.id.clone();

        router.register(channel).await.unwrap();
        assert_eq!(router.channel_count().await, 1);

        let retrieved = router.get(&id).await;
        assert!(retrieved.is_some());

        router.unregister(&id).await.unwrap();
        assert_eq!(router.channel_count().await, 0);
    }

    #[test]
    fn test_channel_message() {
        let msg = ChannelMessage::new(
            ChannelId::from_str("test:123"),
            MessageType::TaskRequest,
            serde_json::json!({"task": "test"}),
        );

        assert!(msg.target_session.is_none());
        let msg = msg.with_target("session-1");
        assert_eq!(msg.target_session, Some("session-1".to_string()));
    }
}