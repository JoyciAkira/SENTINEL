//! LLM-Powered Agent Communication System
//!
//! This module extends the base agent communication from sentinel-core
//! with real LLM-powered message processing and reasoning.
//!
//! Each agent can:
//! - Receive messages from other agents
//! - Process messages using LLM
//! - Generate intelligent responses
//! - Share insights and collaborate

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use uuid::Uuid;

use crate::llm_integration::{LLMChatClient, LLMChatCompletion, LLMContext};
use sentinel_core::outcome_compiler::agent_communication::{
    AgentCapability, AgentId, AgentInfo, AgentMessage, AgentStatus, HandoffContext,
    MessagePayload, ModuleImplementationStatus, UrgencyLevel,
};

/// An intelligent agent that can communicate and reason with LLM
pub struct IntelligentAgent {
    /// Agent identifier
    pub id: AgentId,
    /// Agent metadata
    pub info: AgentInfo,
    /// LLM client for reasoning
    llm_client: Arc<dyn LLMChatClient>,
    /// Message receiver
    receiver: mpsc::Receiver<AgentMessage>,
    /// Broadcast channel for receiving broadcasts
    broadcast_rx: broadcast::Receiver<AgentMessage>,
    /// Outbound message sender
    outbound_tx: mpsc::Sender<AgentOutboundMessage>,
    /// Message history
    message_history: Arc<RwLock<Vec<TimestampedMessage>>>,
    /// Agent's working memory
    working_memory: Arc<RwLock<WorkingMemory>>,
    /// System prompt for this agent
    system_prompt: String,
}

/// Outbound message wrapper
#[derive(Debug, Clone)]
pub struct AgentOutboundMessage {
    pub from: AgentId,
    pub to: Option<AgentId>, // None = broadcast
    pub payload: MessagePayload,
}

/// Message with timestamp
#[derive(Debug, Clone)]
pub struct TimestampedMessage {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from: AgentId,
    pub payload: MessagePayload,
}

/// Agent's working memory
#[derive(Debug, Clone, Default)]
pub struct WorkingMemory {
    pub current_task: Option<String>,
    pub recent_insights: Vec<String>,
    pub learned_patterns: Vec<String>,
    pub active_collaborations: Vec<AgentId>,
}

impl IntelligentAgent {
    /// Create a new intelligent agent
    pub fn new(
        name: impl Into<String>,
        capabilities: Vec<AgentCapability>,
        llm_client: Arc<dyn LLMChatClient>,
        receiver: mpsc::Receiver<AgentMessage>,
        broadcast_rx: broadcast::Receiver<AgentMessage>,
        outbound_tx: mpsc::Sender<AgentOutboundMessage>,
        system_prompt: impl Into<String>,
    ) -> Self {
        let id = AgentId::new();
        let info = AgentInfo {
            id: id.clone(),
            name: name.into(),
            capabilities,
            current_task: None,
            status: AgentStatus::Idle,
            last_heartbeat: chrono::Utc::now(),
        };

        Self {
            id,
            info,
            llm_client,
            receiver,
            broadcast_rx,
            outbound_tx,
            message_history: Arc::new(RwLock::new(Vec::new())),
            working_memory: Arc::new(RwLock::new(WorkingMemory::default())),
            system_prompt: system_prompt.into(),
        }
    }

    /// Start the agent's message processing loop
    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Agent {} starting message loop", self.info.name);
        
        loop {
            tokio::select! {
                // Receive direct messages
                Some(msg) = self.receiver.recv() => {
                    self.process_message(msg).await?;
                }
                // Receive broadcast messages
                Ok(msg) = self.broadcast_rx.recv() => {
                    // Don't process our own broadcasts
                    if msg.from_agent() != Some(&self.id) {
                        self.process_message(msg).await?;
                    }
                }
                else => break,
            }
        }
        
        Ok(())
    }

    /// Process an incoming message using LLM
    async fn process_message(&self, msg: AgentMessage) -> Result<()> {
        // Extract payload
        let payload = match &msg {
            AgentMessage::Direct { payload, .. } => payload.clone(),
            AgentMessage::Broadcast { payload, .. } => payload.clone(),
            AgentMessage::Request { payload, .. } => payload.clone(),
            _ => return Ok(()),
        };

        // Store in history
        {
            let mut history = self.message_history.write().await;
            history.push(TimestampedMessage {
                timestamp: chrono::Utc::now(),
                from: msg.from_agent().cloned().unwrap_or_else(AgentId::new),
                payload: payload.clone(),
            });
        }

        // Generate LLM response
        let response = self.generate_llm_response(&payload).await?;
        
        // Process the LLM response into actions
        self.execute_response_action(&payload, &response).await?;
        
        Ok(())
    }

    /// Generate response using LLM
    async fn generate_llm_response(&self, payload: &MessagePayload) -> Result<String> {
        let user_prompt = format!(
            "You are agent '{}' with capabilities: {:?}.\n\nReceived message:\n{}\n\n\
             Analyze this message and decide:\n\
             1. Should you respond? (yes/no)\n\
             2. What action should you take?\n\
             3. What is your response?",
            self.info.name,
            self.info.capabilities,
            format_payload(payload)
        );

        let completion = self
            .llm_client
            .chat_completion(&self.system_prompt, &user_prompt)
            .await?;

        Ok(completion.content)
    }

    /// Execute action based on LLM response
    async fn execute_response_action(
        &self,
        original: &MessagePayload,
        llm_response: &str,
    ) -> Result<()> {
        // Parse LLM response to determine action
        // This is simplified - in production would use structured parsing
        
        if llm_response.to_lowercase().contains("share pattern") {
            let pattern_msg = AgentOutboundMessage {
                from: self.id.clone(),
                to: None, // broadcast
                payload: MessagePayload::PatternShare {
                    title: format!("Insight from {}", self.info.name),
                    description: llm_response.to_string(),
                    code_snippet: "".to_string(),
                    applicable_to: vec!["all".to_string()],
                },
            };
            let _ = self.outbound_tx.send(pattern_msg).await;
        } else if llm_response.to_lowercase().contains("request help") {
            let help_msg = AgentOutboundMessage {
                from: self.id.clone(),
                to: None,
                payload: MessagePayload::HelpRequest {
                    question: llm_response.to_string(),
                    context: format_payload(original),
                    urgency: UrgencyLevel::Medium,
                },
            };
            let _ = self.outbound_tx.send(help_msg).await;
        }

        Ok(())
    }

    /// Send a direct message to another agent
    pub async fn send_to(&self, to: &AgentId, payload: MessagePayload) -> Result<()> {
        let msg = AgentOutboundMessage {
            from: self.id.clone(),
            to: Some(to.clone()),
            payload,
        };
            self.outbound_tx
                .send(msg)
                .await
                .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;
        Ok(())
    }

    /// Broadcast a message to all agents
    pub async fn broadcast(&self, payload: MessagePayload) -> Result<()> {
        let msg = AgentOutboundMessage {
            from: self.id.clone(),
            to: None,
            payload,
        };
        self.outbound_tx
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Broadcast failed: {}", e))?;
        Ok(())
    }

    /// Share a pattern with other agents
    pub async fn share_pattern(
        &self,
        title: impl Into<String>,
        description: impl Into<String>,
        code_snippet: impl Into<String>,
    ) -> Result<()> {
        self.broadcast(MessagePayload::PatternShare {
            title: title.into(),
            description: description.into(),
            code_snippet: code_snippet.into(),
            applicable_to: vec!["all".to_string()],
        })
        .await
    }

    /// Request help from other agents
    pub async fn request_help(
        &self,
        question: impl Into<String>,
        context: impl Into<String>,
        urgency: UrgencyLevel,
    ) -> Result<()> {
        self.broadcast(MessagePayload::HelpRequest {
            question: question.into(),
            context: context.into(),
            urgency,
        })
        .await
    }
}

/// LLM-powered agent communication orchestrator
pub struct LLMAgentOrchestrator {
    /// LLM client shared across agents
    llm_client: Arc<dyn LLMChatClient>,
    /// Agent communication bus
    agents: Arc<RwLock<HashMap<AgentId, AgentHandle>>>,
    /// Broadcast channel sender
    broadcast_tx: broadcast::Sender<AgentMessage>,
    /// Direct channels to agents
    direct_channels: Arc<RwLock<HashMap<AgentId, mpsc::Sender<AgentMessage>>>>,
    /// Outbound message receiver
    outbound_rx: Arc<Mutex<mpsc::Receiver<AgentOutboundMessage>>>,
    /// Outbound message sender (cloned for agents)
    outbound_tx: mpsc::Sender<AgentOutboundMessage>,
}

/// Handle to communicate with a registered agent
#[derive(Clone)]
pub struct AgentHandle {
    pub id: AgentId,
    pub info: AgentInfo,
    sender: mpsc::Sender<AgentMessage>,
}

impl LLMAgentOrchestrator {
    /// Create new orchestrator with LLM client
    pub fn new(llm_client: Arc<dyn LLMChatClient>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        let (outbound_tx, outbound_rx) = mpsc::channel(1000);

        Self {
            llm_client,
            agents: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            direct_channels: Arc::new(RwLock::new(HashMap::new())),
            outbound_rx: Arc::new(Mutex::new(outbound_rx)),
            outbound_tx,
        }
    }

    /// Register a new intelligent agent
    pub async fn register_agent(
        &self,
        name: impl Into<String>,
        capabilities: Vec<AgentCapability>,
        system_prompt: impl Into<String>,
    ) -> Result<(AgentHandle, IntelligentAgent)> {
        let name = name.into();
        let id = AgentId::new();

        // Create channels
        let (direct_tx, direct_rx) = mpsc::channel(100);
        let broadcast_rx = self.broadcast_tx.subscribe();

        // Store direct channel
        {
            let mut channels = self.direct_channels.write().await;
            channels.insert(id.clone(), direct_tx.clone());
        }

        // Create agent info
        let info = AgentInfo {
            id: id.clone(),
            name: name.clone(),
            capabilities: capabilities.clone(),
            current_task: None,
            status: AgentStatus::Idle,
            last_heartbeat: chrono::Utc::now(),
        };

        // Create handle
        let handle = AgentHandle {
            id: id.clone(),
            info: info.clone(),
            sender: direct_tx.clone(),
        };

        // Create intelligent agent
        let agent = IntelligentAgent::new(
            name,
            capabilities,
            self.llm_client.clone(),
            direct_rx,
            broadcast_rx,
            self.outbound_tx.clone(),
            system_prompt,
        );

        // Store handle
        {
            let mut agents = self.agents.write().await;
            agents.insert(id.clone(), handle.clone());
        }

        Ok((handle, agent))
    }

    /// Start the orchestrator's message routing loop
    pub async fn run(&self) -> Result<()> {
        let mut outbound_rx = self.outbound_rx.lock().await;

        while let Some(msg) = outbound_rx.recv().await {
            self.route_message(msg).await?;
        }

        Ok(())
    }

    /// Route a message to its destination
    async fn route_message(&self, msg: AgentOutboundMessage) -> Result<()> {
        match msg.to {
            Some(target_id) => {
                // Direct message
                let channels = self.direct_channels.read().await;
                if let Some(tx) = channels.get(&target_id) {
                    let agent_msg = AgentMessage::Direct {
                        from: msg.from,
                        to: target_id,
                        payload: msg.payload,
                    };
                    let _ = tx.send(agent_msg).await;
                }
            }
            None => {
                // Broadcast
                let agent_msg = AgentMessage::Broadcast {
                    from: msg.from,
                    payload: msg.payload,
                };
                let _ = self.broadcast_tx.send(agent_msg);
            }
        }
        Ok(())
    }

    /// Get list of all registered agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().map(|h| h.info.clone()).collect()
    }

    /// Send message to specific agent
    pub async fn send_to(&self, to: &AgentId, from: &AgentId, payload: MessagePayload) -> Result<()> {
        let channels = self.direct_channels.read().await;
        if let Some(tx) = channels.get(to) {
            let msg = AgentMessage::Direct {
                from: from.clone(),
                to: to.clone(),
                payload,
            };
            tx.send(msg)
                .await
                .map_err(|e| anyhow::anyhow!("Send failed: {}", e))?;
        }
        Ok(())
    }

    /// Broadcast a message from a specific agent to all other agents
    pub async fn broadcast_from(&self, from: &AgentId, payload: MessagePayload) -> Result<()> {
        let msg = AgentMessage::Broadcast {
            from: from.clone(),
            payload,
        };
        self.broadcast_tx
            .send(msg)
            .map_err(|e| anyhow::anyhow!("Broadcast failed: {}", e))?;
        Ok(())
    }
}

/// Format payload for display
fn format_payload(payload: &MessagePayload) -> String {
    match payload {
        MessagePayload::PatternShare { title, description, .. } => {
            format!("Pattern: {} - {}", title, description)
        }
        MessagePayload::HelpRequest { question, context, urgency } => {
            format!("Help Request ({:?}): {} - Context: {}", urgency, question, context)
        }
        MessagePayload::LessonLearned { situation, solution, .. } => {
            format!("Lesson: {} - Solution: {}", situation, solution)
        }
        MessagePayload::StatusUpdate { module_id, status, completion_percentage, .. } => {
            format!("Status: {} is {:?} ({:.0}%)", module_id, status, completion_percentage)
        }
        MessagePayload::ValidationResult { module_id, passed, issues, .. } => {
            format!("Validation: {} - Passed: {} - Issues: {:?}", module_id, passed, issues)
        }
        _ => "Unknown message type".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock LLM client for testing
    #[derive(Debug)]
    struct MockLLMClient;

    #[async_trait::async_trait]
    impl LLMChatClient for MockLLMClient {
        async fn chat_completion(
            &self,
            _system_prompt: &str,
            user_prompt: &str,
        ) -> anyhow::Result<LLMChatCompletion> {
            // Simple mock that responds based on prompt content
            let response = if user_prompt.contains("Help") {
                "I will share a pattern about this topic."
            } else {
                "I acknowledge this message and will take appropriate action."
            };

            Ok(LLMChatCompletion {
                llm_name: "mock".to_string(),
                content: response.to_string(),
                token_cost: 10,
            })
        }
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let llm_client = Arc::new(MockLLMClient);
        let orchestrator = LLMAgentOrchestrator::new(llm_client);

        let (handle, agent) = orchestrator
            .register_agent("TestAgent", vec![AgentCapability::AuthExpert], "You are a test agent.")
            .await
            .unwrap();

        assert_eq!(handle.info.name, "TestAgent");
        assert!(handle.info.capabilities.contains(&AgentCapability::AuthExpert));
    }

    #[tokio::test]
    async fn test_agent_message_routing() {
        let llm_client = Arc::new(MockLLMClient);
        let orchestrator = LLMAgentOrchestrator::new(llm_client);

        // Register two agents
        let (handle1, mut agent1) = orchestrator
            .register_agent("Agent1", vec![AgentCapability::AuthExpert], "You are agent 1.")
            .await
            .unwrap();

        let (handle2, mut agent2) = orchestrator
            .register_agent("Agent2", vec![AgentCapability::ApiExpert], "You are agent 2.")
            .await
            .unwrap();

        // Spawn orchestrator loop
        let orchestrator_clone = LLMAgentOrchestrator {
            llm_client: Arc::new(MockLLMClient),
            agents: orchestrator.agents.clone(),
            broadcast_tx: orchestrator.broadcast_tx.clone(),
            direct_channels: orchestrator.direct_channels.clone(),
            outbound_rx: orchestrator.outbound_rx.clone(),
            outbound_tx: orchestrator.outbound_tx.clone(),
        };

        tokio::spawn(async move {
            orchestrator_clone.run().await.unwrap();
        });

        // Spawn agent loops
        tokio::spawn(async move {
            agent1.run().await.unwrap();
        });

        tokio::spawn(async move {
            agent2.run().await.unwrap();
        });

        // Send message from agent1 to agent2
        let msg = MessagePayload::HelpRequest {
            question: "How do I integrate auth?".to_string(),
            context: "Working on login module".to_string(),
            urgency: UrgencyLevel::Medium,
        };

        orchestrator.send_to(&handle2.id, &handle1.id, msg).await.unwrap();

        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Test passed if no panic
    }
}
