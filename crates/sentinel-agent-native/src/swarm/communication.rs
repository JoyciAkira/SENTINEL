//! Inter-agent Communication System
//!
//! Broadcast-based messaging for swarm coordination.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

use super::{AgentId, AgentOutput, AgentType, Task};

/// Message types for swarm communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmMessage {
    /// Task assigned to agent
    TaskAssigned { to: AgentId, task: Task },

    /// Task completed by agent
    TaskCompleted { by: AgentId, output: AgentOutput },

    /// Proposal for consensus
    Proposal {
        id: ProposalId,
        by: AgentId,
        proposal: Proposal,
        timestamp: u64,
    },

    /// Vote on proposal
    Vote {
        proposal_id: ProposalId,
        by: AgentId,
        vote: Vote,
        reasoning: Option<String>,
    },

    /// Pattern shared by agent
    PatternShare {
        by: AgentId,
        pattern: Pattern,
        confidence: f64,
    },

    /// Help request
    HelpRequest {
        by: AgentId,
        issue: String,
        urgency: UrgencyLevel,
    },

    /// Handoff control
    Handoff {
        from: AgentId,
        to: AgentId,
        context: HandoffContext,
    },

    /// System message
    System { level: SystemLevel, message: String },

    /// Progress update
    Progress {
        by: AgentId,
        task_id: String,
        percent: f64,
        status: String,
    },
}

/// Proposal for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: ProposalId,
    pub title: String,
    pub description: String,
    pub action: ProposedAction,
    pub proposed_by: AgentId,
    pub timestamp: u64,
}

/// Unique proposal ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProposalId(pub [u8; 16]);

impl ProposalId {
    pub fn new() -> Self {
        let mut id = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut id);
        Self(id)
    }
}

/// Types of proposed actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposedAction {
    SelectLibrary(String),
    DesignDecision(String),
    ArchitectureChoice(String),
    ImplementationApproach(String),
    SecurityMeasure(String),
    PerformanceOptimization(String),
}

/// Vote on proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}

/// Pattern shared between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub title: String,
    pub description: String,
    pub applicable_to: Vec<AgentType>,
    pub confidence: f64,
}

/// Urgency level for help requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Context for handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    pub task_state: String,
    pub notes: String,
    pub progress_percent: f64,
}

/// System message level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Message with timestamp
#[derive(Debug, Clone)]
pub struct TimestampedMessage {
    pub timestamp: Instant,
    pub message: SwarmMessage,
}

/// Communication bus for swarm messaging
pub struct CommunicationBus {
    /// Broadcast channel sender
    broadcast_tx: broadcast::Sender<SwarmMessage>,

    /// Message log for history
    message_log: Arc<RwLock<Vec<TimestampedMessage>>>,

    /// Max log size (oldest messages get truncated)
    max_log_size: usize,
}

impl CommunicationBus {
    /// Create new communication bus
    pub fn new() -> Self {
        // Channel capacity: 10000 messages
        let (broadcast_tx, _) = broadcast::channel(10000);

        Self {
            broadcast_tx,
            message_log: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            max_log_size: 10000,
        }
    }

    /// Subscribe to broadcast channel
    pub fn subscribe(&self) -> broadcast::Receiver<SwarmMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Broadcast message to all agents
    pub async fn broadcast(&self, message: SwarmMessage) -> Result<()> {
        // Log message
        let timestamped = TimestampedMessage {
            timestamp: Instant::now(),
            message: message.clone(),
        };

        {
            let mut log = self.message_log.write().await;
            log.push(timestamped);

            // Truncate if exceeds max size
            if log.len() > self.max_log_size {
                let excess = log.len() - self.max_log_size;
                log.drain(0..excess);
            }
        }

        // Broadcast (ignore errors if no receivers)
        let _ = self.broadcast_tx.send(message);

        Ok(())
    }

    /// Send direct message (goes through broadcast but filtered by receiver)
    pub async fn send_direct(&self, _to: AgentId, message: SwarmMessage) -> Result<()> {
        // In broadcast system, direct messages are just filtered by receiver
        self.broadcast(message).await
    }

    /// Get message log
    pub async fn get_log(&self) -> Vec<TimestampedMessage> {
        self.message_log.read().await.clone()
    }

    /// Get messages from specific agent
    pub async fn get_messages_from(&self, agent_id: AgentId) -> Vec<SwarmMessage> {
        let log = self.message_log.read().await;
        log.iter()
            .filter(|tm| Self::message_from(&tm.message, agent_id))
            .map(|tm| tm.message.clone())
            .collect()
    }

    /// Check if message is from specific agent
    fn message_from(msg: &SwarmMessage, agent_id: AgentId) -> bool {
        match msg {
            SwarmMessage::TaskCompleted { by, .. } => *by == agent_id,
            SwarmMessage::Proposal { by, .. } => *by == agent_id,
            SwarmMessage::Vote { by, .. } => *by == agent_id,
            SwarmMessage::PatternShare { by, .. } => *by == agent_id,
            SwarmMessage::HelpRequest { by, .. } => *by == agent_id,
            SwarmMessage::Handoff { from, .. } => *from == agent_id,
            SwarmMessage::Progress { by, .. } => *by == agent_id,
            _ => false,
        }
    }

    /// Get message count
    pub async fn message_count(&self) -> usize {
        self.message_log.read().await.len()
    }

    /// Clear log
    pub async fn clear_log(&self) {
        self.message_log.write().await.clear();
    }
}

impl Default for CommunicationBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Message payload for different message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Text(String),
    Code(String),
    Pattern(Pattern),
    Task(Task),
    Output(AgentOutput),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast() {
        let bus = CommunicationBus::new();
        let mut rx = bus.subscribe();

        let msg = SwarmMessage::System {
            level: SystemLevel::Info,
            message: "Test".to_string(),
        };

        bus.broadcast(msg.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            SwarmMessage::System { message, .. } => {
                assert_eq!(message, "Test");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = CommunicationBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let msg = SwarmMessage::System {
            level: SystemLevel::Info,
            message: "Broadcast".to_string(),
        };

        bus.broadcast(msg).await.unwrap();

        let received1 = rx1.recv().await.unwrap();
        let received2 = rx2.recv().await.unwrap();

        // Both should receive same message
        match (received1, received2) {
            (
                SwarmMessage::System { message: m1, .. },
                SwarmMessage::System { message: m2, .. },
            ) => {
                assert_eq!(m1, m2);
            }
            _ => panic!("Wrong message types"),
        }
    }

    #[tokio::test]
    async fn test_message_log() {
        let bus = CommunicationBus::new();

        for i in 0..5 {
            let msg = SwarmMessage::System {
                level: SystemLevel::Info,
                message: format!("Message {}", i),
            };
            bus.broadcast(msg).await.unwrap();
        }

        let count = bus.message_count().await;
        assert_eq!(count, 5);
    }
}
