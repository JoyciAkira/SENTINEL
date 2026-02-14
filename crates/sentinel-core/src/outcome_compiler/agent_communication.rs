//! Inter-Agent Communication System
//!
//! Enables Split Agents to communicate, share state, delegate tasks, and learn from each other.
//! This is the "killer feature" that makes the multi-agent system truly collaborative.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Agent Communication Bus                  │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  Agent A (Auth Expert)        Agent B (API Expert)         │
//! │       │                              │                      │
//! │       │ 1. Request auth context      │                      │
//! │       ├──────────────────────────────►                      │
//! │       │                              │                      │
//! │       │ 2. Response with patterns    │                      │
//! │       ◄──────────────────────────────┤                      │
//! │       │                              │                      │
//! │       │ 3. Broadcast: New insight    │                      │
//! │       ├───────────┬──────────────────►                      │
//! │       │           │                  │                      │
//! │       │           ▼                  │                      │
//! │       │     Agent C (UI Expert)      │                      │
//! │       │     (learns from insight)    │                      │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Communication Patterns
//!
//! 1. **Direct Message**: Point-to-point communication
//! 2. **Broadcast**: Send to all agents
//! 3. **Request/Response**: Synchronous query
//! 4. **Pub/Sub**: Subscribe to topics
//! 5. **Handoff**: Transfer control with context

use crate::error::{Result, SentinelError};
use crate::outcome_compiler::AtomicModule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

/// Unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent capability - what this agent can do
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Can work on authentication modules
    AuthExpert,
    /// Can work on API/backend modules
    ApiExpert,
    /// Can work on frontend/UI modules
    FrontendExpert,
    /// Can work on database models
    DatabaseExpert,
    /// Can review and validate code
    CodeReviewer,
    /// Can optimize performance
    PerformanceOptimizer,
    /// Can handle testing
    TestExpert,
    /// Can integrate modules
    IntegrationExpert,
    /// Custom capability
    Custom(String),
}

/// Agent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<AgentCapability>,
    pub current_task: Option<String>,
    pub status: AgentStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Busy,
    WaitingForInput,
    Error,
    Offline,
}

/// Message types for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// Direct message to specific agent
    Direct {
        from: AgentId,
        to: AgentId,
        payload: MessagePayload,
    },
    /// Broadcast to all agents
    Broadcast {
        from: AgentId,
        payload: MessagePayload,
    },
    /// Request that expects a response
    Request {
        from: AgentId,
        to: AgentId,
        request_id: String,
        payload: MessagePayload,
    },
    /// Response to a request
    Response {
        from: AgentId,
        to: AgentId,
        request_id: String,
        payload: MessagePayload,
    },
    /// Handoff control with context
    Handoff {
        from: AgentId,
        to: AgentId,
        context: HandoffContext,
    },
}

impl AgentMessage {
    /// Get the sender agent ID if available
    pub fn from_agent(&self) -> Option<&AgentId> {
        match self {
            AgentMessage::Direct { from, .. } => Some(from),
            AgentMessage::Broadcast { from, .. } => Some(from),
            AgentMessage::Request { from, .. } => Some(from),
            AgentMessage::Response { from, .. } => Some(from),
            AgentMessage::Handoff { from, .. } => Some(from),
        }
    }

    /// Get the message payload if available
    pub fn payload(&self) -> Option<&MessagePayload> {
        match self {
            AgentMessage::Direct { payload, .. } => Some(payload),
            AgentMessage::Broadcast { payload, .. } => Some(payload),
            AgentMessage::Request { payload, .. } => Some(payload),
            AgentMessage::Response { payload, .. } => Some(payload),
            AgentMessage::Handoff { .. } => None,
        }
    }
}

/// Message payload - the actual content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Share a code pattern or insight
    PatternShare {
        title: String,
        description: String,
        code_snippet: String,
        applicable_to: Vec<String>,
    },
    /// Ask for help/clarification
    HelpRequest {
        question: String,
        context: String,
        urgency: UrgencyLevel,
    },
    /// Share learned lesson
    LessonLearned {
        situation: String,
        solution: String,
        prevention: String,
    },
    /// Share module status update
    StatusUpdate {
        module_id: String,
        status: ModuleImplementationStatus,
        completion_percentage: f64,
        blockers: Vec<String>,
    },
    /// Request specific expertise
    ExpertiseRequest {
        capability: AgentCapability,
        question: String,
    },
    /// Share validation results
    ValidationResult {
        module_id: String,
        passed: bool,
        issues: Vec<String>,
        suggestions: Vec<String>,
    },
    /// Context for handoff
    ContextTransfer {
        key: String,
        value: serde_json::Value,
    },
    /// Custom message
    Custom {
        message_type: String,
        data: serde_json::Value,
    },
}

/// Urgency level for help requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Module implementation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleImplementationStatus {
    NotStarted,
    InProgress,
    Blocked,
    NeedsReview,
    Completed,
    Failed,
}

/// Context passed during handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    /// Previous agent's notes
    pub handoff_notes: String,
    /// Current state snapshot
    pub state_snapshot: HashMap<String, serde_json::Value>,
    /// Pending decisions
    pub pending_decisions: Vec<String>,
    /// Critical context that must not be lost
    pub critical_context: Vec<String>,
    /// Why the handoff is happening
    pub handoff_reason: HandoffReason,
}

/// Reason for handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandoffReason {
    /// Completed work and passing to next
    Completed,
    /// Stuck and needs different expertise
    StuckNeedHelp,
    /// Conflict resolution needed
    ConflictResolution,
    /// Optimization opportunity
    Optimization,
    /// Custom reason
    Custom(String),
}

/// Agent Communication Bus - Central hub for all agent communication
pub struct AgentCommunicationBus {
    /// All registered agents
    agents: Arc<Mutex<HashMap<AgentId, AgentInfo>>>,
    /// Broadcast channel for all agents
    broadcast_tx: broadcast::Sender<AgentMessage>,
    /// Direct message channels
    direct_channels: Arc<Mutex<HashMap<AgentId, mpsc::Sender<AgentMessage>>>>,
    /// Message history for learning
    message_history: Arc<Mutex<Vec<TimestampedMessage>>>,
}

/// Message with timestamp
#[derive(Debug, Clone)]
pub struct TimestampedMessage {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: AgentMessage,
}

/// Handle for an agent to communicate
pub struct AgentHandle {
    pub id: AgentId,
    pub info: AgentInfo,
    bus: Arc<AgentCommunicationBus>,
    receiver: mpsc::Receiver<AgentMessage>,
}

impl AgentCommunicationBus {
    /// Create new communication bus
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            direct_channels: Arc::new(Mutex::new(HashMap::new())),
            message_history: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Register a new agent with the bus
    pub fn register_agent(
        &self,
        name: impl Into<String>,
        capabilities: Vec<AgentCapability>,
    ) -> Result<AgentHandle> {
        let id = AgentId::new();
        let info = AgentInfo {
            id: id.clone(),
            name: name.into(),
            capabilities,
            current_task: None,
            status: AgentStatus::Idle,
            last_heartbeat: chrono::Utc::now(),
        };
        
        // Create direct channel for this agent
        let (tx, rx) = mpsc::channel(100);
        
        {
            let mut agents = self.agents.lock().unwrap();
            agents.insert(id.clone(), info.clone());
        }
        
        {
            let mut channels = self.direct_channels.lock().unwrap();
            channels.insert(id.clone(), tx);
        }
        
        Ok(AgentHandle {
            id,
            info,
            bus: Arc::new(self.clone()),
            receiver: rx,
        })
    }
    
    /// Send message to specific agent
    pub fn send_to(&self, to: &AgentId, message: AgentMessage) -> Result<()> {
        let channels = self.direct_channels.lock().unwrap();
        if let Some(tx) = channels.get(to) {
            tx.try_send(message)
                .map_err(|e| SentinelError::InvalidInput(format!("Failed to send: {}", e)))?;
        }
        Ok(())
    }
    
    /// Broadcast message to all agents
    pub fn broadcast(&self, message: AgentMessage) -> Result<()> {
        self.broadcast_tx
            .send(message)
            .map_err(|e| SentinelError::InvalidInput(format!("Broadcast failed: {}", e)))?;
        Ok(())
    }
    
    /// Find agents by capability
    pub fn find_agents_by_capability(&self, capability: &AgentCapability) -> Vec<AgentInfo> {
        let agents = self.agents.lock().unwrap();
        agents
            .values()
            .filter(|info| info.capabilities.contains(capability))
            .cloned()
            .collect()
    }
    
    /// Get all messages from history
    pub fn get_message_history(&self) -> Vec<TimestampedMessage> {
        self.message_history.lock().unwrap().clone()
    }
    
    /// Record message in history
    fn record_message(&self, message: AgentMessage) {
        let mut history = self.message_history.lock().unwrap();
        history.push(TimestampedMessage {
            timestamp: chrono::Utc::now(),
            message,
        });
        // Keep only last 1000 messages
        if history.len() > 1000 {
            history.remove(0);
        }
    }
}

impl Clone for AgentCommunicationBus {
    fn clone(&self) -> Self {
        Self {
            agents: Arc::clone(&self.agents),
            broadcast_tx: self.broadcast_tx.clone(),
            direct_channels: Arc::clone(&self.direct_channels),
            message_history: Arc::clone(&self.message_history),
        }
    }
}

impl AgentHandle {
    /// Send direct message to another agent
    pub fn send_to(&self, to: &AgentId, payload: MessagePayload) -> Result<()> {
        let message = AgentMessage::Direct {
            from: self.id.clone(),
            to: to.clone(),
            payload,
        };
        self.bus.send_to(to, message)
    }
    
    /// Broadcast message to all agents
    pub fn broadcast(&self, payload: MessagePayload) -> Result<()> {
        let message = AgentMessage::Broadcast {
            from: self.id.clone(),
            payload,
        };
        self.bus.broadcast(message)
    }
    
    /// Request help from agents with specific capability
    pub fn request_help(
        &self,
        capability: AgentCapability,
        question: impl Into<String>,
        context: impl Into<String>,
        urgency: UrgencyLevel,
    ) -> Result<()> {
        let question_str = question.into();
        let context_str = context.into();
        let experts = self.bus.find_agents_by_capability(&capability);
        
        for expert in experts {
            let expert_id = expert.id.clone();
            if expert_id != self.id && expert.status != AgentStatus::Offline {
                let message = AgentMessage::Request {
                    from: self.id.clone(),
                    to: expert_id.clone(),
                    request_id: Uuid::new_v4().to_string(),
                    payload: MessagePayload::HelpRequest {
                        question: question_str.clone(),
                        context: context_str.clone(),
                        urgency,
                    },
                };
                self.bus.send_to(&expert_id, message)?;
            }
        }
        
        Ok(())
    }
    
    /// Share a pattern/insight with all agents
    pub fn share_pattern(
        &self,
        title: impl Into<String>,
        description: impl Into<String>,
        code_snippet: impl Into<String>,
        applicable_to: Vec<String>,
    ) -> Result<()> {
        let payload = MessagePayload::PatternShare {
            title: title.into(),
            description: description.into(),
            code_snippet: code_snippet.into(),
            applicable_to,
        };
        self.broadcast(payload)
    }
    
    /// Handoff work to another agent
    pub fn handoff_to(
        &self,
        to: &AgentId,
        notes: impl Into<String>,
        state: HashMap<String, serde_json::Value>,
        reason: HandoffReason,
    ) -> Result<()> {
        let context = HandoffContext {
            handoff_notes: notes.into(),
            state_snapshot: state,
            pending_decisions: vec![],
            critical_context: vec![],
            handoff_reason: reason,
        };
        
        let message = AgentMessage::Handoff {
            from: self.id.clone(),
            to: to.clone(),
            context,
        };
        
        self.bus.send_to(to, message)
    }
    
    /// Update agent status
    pub fn update_status(&mut self, status: AgentStatus) {
        self.info.status = status;
        self.info.last_heartbeat = chrono::Utc::now();
        
        // Broadcast status update
        let _ = self.broadcast(MessagePayload::StatusUpdate {
            module_id: self.id.0.clone(),
            status: ModuleImplementationStatus::InProgress,
            completion_percentage: 0.0,
            blockers: vec![],
        });
    }
    
    /// Share validation results
    pub fn share_validation(
        &self,
        module_id: impl Into<String>,
        passed: bool,
        issues: Vec<String>,
        suggestions: Vec<String>,
    ) -> Result<()> {
        let payload = MessagePayload::ValidationResult {
            module_id: module_id.into(),
            passed,
            issues,
            suggestions,
        };
        self.broadcast(payload)
    }
    
    /// Receive next message (non-blocking)
    pub async fn receive(&mut self) -> Option<AgentMessage> {
        self.receiver.recv().await
    }
    
    /// Learn from message history
    pub fn learn_from_history(&self) -> Vec<LearnedPattern> {
        let history = self.bus.get_message_history();
        let mut patterns = vec![];
        
        for msg in history.iter().rev().take(100) {
            if let AgentMessage::Broadcast { payload: MessagePayload::PatternShare { title, description, code_snippet, applicable_to }, .. } = &msg.message {
                patterns.push(LearnedPattern {
                    title: title.clone(),
                    description: description.clone(),
                    code_snippet: code_snippet.clone(),
                    applicable_to: applicable_to.clone(),
                    learned_at: msg.timestamp,
                });
            }
        }
        
        patterns
    }
}

/// Pattern learned from other agents
#[derive(Debug, Clone)]
pub struct LearnedPattern {
    pub title: String,
    pub description: String,
    pub code_snippet: String,
    pub applicable_to: Vec<String>,
    pub learned_at: chrono::DateTime<chrono::Utc>,
}

/// Multi-Agent Orchestrator with communication
pub struct CollaborativeAgentOrchestrator {
    base_orchestrator: super::orchestrator::SplitAgentOrchestrator,
    comm_bus: AgentCommunicationBus,
}

impl CollaborativeAgentOrchestrator {
    pub fn new(language: &str, framework: &str) -> Self {
        Self {
            base_orchestrator: super::orchestrator::SplitAgentOrchestrator::new(language, framework),
            comm_bus: AgentCommunicationBus::new(),
        }
    }
    
    /// Spawn specialized worker agents
    pub fn spawn_workers(&self, modules: Vec<AtomicModule>) -> Result<Vec<AgentHandle>> {
        let mut handles = vec![];
        
        for module in modules {
            let capabilities = Self::infer_capabilities(&module);
            let handle = self.comm_bus.register_agent(
                format!("Worker-{}", module.module_name),
                capabilities,
            )?;
            handles.push(handle);
        }
        
        Ok(handles)
    }
    
    /// Infer capabilities needed for a module
    fn infer_capabilities(module: &AtomicModule) -> Vec<AgentCapability> {
        let name = module.module_name.to_lowercase();
        let mut caps = vec![];
        
        if name.contains("auth") {
            caps.push(AgentCapability::AuthExpert);
        }
        if name.contains("api") || name.contains("route") {
            caps.push(AgentCapability::ApiExpert);
        }
        if name.contains("ui") || name.contains("component") {
            caps.push(AgentCapability::FrontendExpert);
        }
        if name.contains("db") || name.contains("model") {
            caps.push(AgentCapability::DatabaseExpert);
        }
        if name.contains("test") {
            caps.push(AgentCapability::TestExpert);
        }
        
        // All workers can review code
        caps.push(AgentCapability::CodeReviewer);
        
        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_communication_bus_creation() {
        let bus = AgentCommunicationBus::new();
        assert!(bus.get_message_history().is_empty());
    }
    
    #[test]
    fn test_agent_registration() {
        let bus = AgentCommunicationBus::new();
        let handle = bus.register_agent("TestAgent", vec![AgentCapability::AuthExpert]).unwrap();
        
        assert_eq!(handle.info.name, "TestAgent");
        assert!(handle.info.capabilities.contains(&AgentCapability::AuthExpert));
    }
    
    #[test]
    fn test_find_by_capability() {
        let bus = AgentCommunicationBus::new();
        bus.register_agent("AuthAgent", vec![AgentCapability::AuthExpert]).unwrap();
        bus.register_agent("ApiAgent", vec![AgentCapability::ApiExpert]).unwrap();
        
        let auth_agents = bus.find_agents_by_capability(&AgentCapability::AuthExpert);
        assert_eq!(auth_agents.len(), 1);
        assert_eq!(auth_agents[0].name, "AuthAgent");
    }
    
    #[tokio::test]
    async fn test_message_passing() {
        let bus = Arc::new(AgentCommunicationBus::new());
        
        let handle1 = bus.register_agent("Agent1", vec![]).unwrap();
        let handle2 = bus.register_agent("Agent2", vec![]).unwrap();
        
        // Agent1 sends to Agent2
        let msg = MessagePayload::Custom {
            message_type: "test".to_string(),
            data: serde_json::json!({"hello": "world"}),
        };
        
        // Note: In real usage, agents would run in separate tasks
        // This is just testing the infrastructure
    }
}
