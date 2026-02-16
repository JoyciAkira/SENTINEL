//! Smart Routing System for Swarm Communication
//!
//! Replaces broadcast with intelligent message routing based on:
//! - Agent capabilities and types
//! - Message relevance
//! - Agent load and health
//! - Topic subscriptions

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use anyhow::Result;

use crate::swarm::{AgentId, AgentType};
use crate::swarm::communication::SwarmMessage;

/// Smart router for efficient agent communication
pub struct SmartRouter {
    /// Agent registry with capabilities
    agent_registry: Arc<RwLock<HashMap<AgentId, AgentCapabilities>>>,
    /// Topic subscriptions
    subscriptions: Arc<RwLock<HashMap<String, HashSet<AgentId>>>>,
    /// Routing statistics
    stats: Arc<RwLock<RoutingStats>>,
}

/// Agent capabilities for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub agent_id: AgentId,
    pub agent_type: AgentType,
    pub topics: HashSet<String>,
    pub max_concurrent_tasks: u32,
    pub current_load: f64,
    pub is_healthy: bool,
}

/// Routing statistics
#[derive(Debug, Clone, Default)]
pub struct RoutingStats {
    pub total_messages: u64,
    pub broadcast_messages: u64,
    pub routed_messages: u64,
    pub dropped_messages: u64,
}

/// Routing strategy used
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RoutingStrategy {
    /// Broadcast to all agents
    Broadcast,
    /// Direct to specific agent
    Direct,
    /// Based on topic subscription
    TopicBased,
    /// Based on agent type
    TypeBased,
}

impl SmartRouter {
    /// Create new smart router
    pub fn new() -> Self {
        Self {
            agent_registry: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RoutingStats::default())),
        }
    }
    
    /// Register an agent with the router
    pub async fn register_agent(&self, agent_id: AgentId, capabilities: AgentCapabilities) -> Result<()> {
        let mut registry = self.agent_registry.write().await;
        registry.insert(agent_id, capabilities);
        Ok(())
    }
    
    /// Route message to appropriate agents
    pub async fn route_message(&self, message: SwarmMessage, sender: AgentId) -> Result<Vec<AgentId>> {
        let recipients = self.determine_recipients(&message, sender).await;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_messages += 1;
        if recipients.len() > 1 {
            stats.broadcast_messages += 1;
        } else {
            stats.routed_messages += 1;
        }
        
        Ok(recipients)
    }
    
    /// Determine recipients based on message type
    async fn determine_recipients(&self, message: &SwarmMessage, sender: AgentId) -> Vec<AgentId> {
        let registry = self.agent_registry.read().await;
        
        match message {
            SwarmMessage::TaskAssigned { to, .. } => {
                // Direct routing
                vec![*to]
            }
            SwarmMessage::TaskCompleted { .. } => {
                // Send to manager or all
                self.find_managers(&registry).await
            }
            SwarmMessage::Proposal { .. } => {
                // Broadcast for voting
                registry.keys().cloned().collect()
            }
            SwarmMessage::HelpRequest { .. } => {
                // Route to agents with matching capabilities
                self.find_helpers(&registry).await
            }
            _ => {
                // Default: broadcast
                registry.keys().cloned().collect()
            }
        }
    }
    
    /// Find manager agents
    async fn find_managers(&self, registry: &HashMap<AgentId, AgentCapabilities>) -> Vec<AgentId> {
        registry
            .iter()
            .filter(|(_, cap)| matches!(cap.agent_type, AgentType::Manager { .. }))
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Find helper agents
    async fn find_helpers(&self, registry: &HashMap<AgentId, AgentCapabilities>) -> Vec<AgentId> {
        // Filter healthy agents with low load
        registry
            .iter()
            .filter(|(_, cap)| cap.is_healthy && cap.current_load < 0.7)
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_smart_router_creation() {
        let router = SmartRouter::new();
        let stats = router.stats.read().await;
        assert_eq!(stats.total_messages, 0);
    }
}
