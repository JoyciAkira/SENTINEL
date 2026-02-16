//! Swarm Agent Framework
//!
//! Core trait and implementations for swarm agents with deterministic personalities.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use blake3::Hash;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use super::{
    communication::{CommunicationBus, SwarmMessage},
    communication::{Proposal, Vote},
    consensus::ContinuousConsensus,
    emergence::AgentType,
    llm::{LLMRequest, SwarmLLMClient},
    memory::SwarmMemory,
    AgentOutput, Task,
};

/// Unique identifier for an agent (deterministic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub [u8; 16]);

impl AgentId {
    /// Generate deterministic ID from goal hash, agent type, and index
    ///
    /// Same inputs → Same ID (deterministic)
    pub fn deterministic(goal_hash: &Hash, agent_type: &AgentType, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(goal_hash.as_bytes());
        hasher.update(agent_type.to_string().as_bytes());
        hasher.update(&index.to_le_bytes());

        let hash = hasher.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&hash.as_bytes()[0..16]);

        Self(id)
    }

    /// Generate random ID (for non-deterministic use cases)
    pub fn random() -> Self {
        let mut id = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut id);
        Self(id)
    }
}

/// Deterministic agent personality - affects creativity and decision making
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentPersonality {
    /// Bias toward simple vs complex solutions (0.0 = complex, 1.0 = simple)
    pub simplicity_bias: f64,

    /// Bias toward performance vs readability (0.0 = readable, 1.0 = performance)
    pub performance_bias: f64,

    /// Bias toward innovation vs standards (0.0 = standard, 1.0 = innovative)
    pub innovation_bias: f64,

    /// Risk tolerance (0.0 = cautious, 1.0 = risk-tolerant)
    pub risk_tolerance: f64,

    /// Communication verbosity (0.0 = terse, 1.0 = verbose)
    pub verbosity: f64,

    /// Authority level (0.0 - 1.0, affects voting power)
    pub authority: f64,
}

impl AgentPersonality {
    /// Generate deterministic personality from goal hash
    ///
    /// Different positions in hash → different traits
    /// Same goal + type → same personality (deterministic)
    pub fn from_goal(goal_hash: &Hash, agent_type: &AgentType) -> Self {
        let bytes = goal_hash.as_bytes();
        let type_seed = agent_type
            .to_string()
            .bytes()
            .fold(0u8, |a, b| a.wrapping_add(b));

        // Derive each trait from different byte positions + type seed
        // This ensures different agent types have different personalities for same goal
        Self {
            simplicity_bias: Self::byte_to_f64(bytes[0].wrapping_add(type_seed)),
            performance_bias: Self::byte_to_f64(bytes[1].wrapping_add(type_seed)),
            innovation_bias: Self::byte_to_f64(bytes[2].wrapping_add(type_seed)),
            risk_tolerance: Self::byte_to_f64(bytes[3].wrapping_add(type_seed)),
            verbosity: Self::byte_to_f64(bytes[4].wrapping_add(type_seed)),
            authority: Self::derive_authority(agent_type),
        }
    }

    fn byte_to_f64(byte: u8) -> f64 {
        byte as f64 / 255.0
    }

    fn derive_authority(agent_type: &AgentType) -> f64 {
        match agent_type {
            AgentType::Manager => 0.99,
            AgentType::SecurityAuditor => 0.95,
            AgentType::AuthArchitect => 0.90,
            AgentType::ReviewAgent => 0.88,
            AgentType::DatabaseArchitect => 0.85,
            AgentType::APICoder | AgentType::FrontendCoder => 0.80,
            AgentType::JWTCoder => 0.82,
            AgentType::PerformanceOptimizer => 0.75,
            AgentType::DevOpsEngineer => 0.70,
            AgentType::TestWriter => 0.65,
            AgentType::DocWriter => 0.60,
        }
    }

    /// Generate system prompt based on personality
    pub fn system_prompt(&self, agent_type: &AgentType) -> String {
        let base = match agent_type {
            AgentType::AuthArchitect => "You are an authentication architecture expert. Design secure, scalable auth systems.",
            AgentType::SecurityAuditor => "You are a security auditor. Find vulnerabilities and ensure best practices.",
            AgentType::JWTCoder => "You are a JWT implementation specialist. Write correct, secure JWT code.",
            AgentType::APICoder => "You are an API design and implementation expert. Build clean REST/GraphQL APIs.",
            AgentType::FrontendCoder => "You are a frontend development expert. Write modern, responsive UI code.",
            AgentType::DatabaseArchitect => "You are a database architect. Design efficient schemas and queries.",
            AgentType::TestWriter => "You are a test automation expert. Write comprehensive, maintainable tests.",
            AgentType::DocWriter => "You are a technical writer. Create clear, helpful documentation.",
            AgentType::ReviewAgent => "You are a code reviewer. Ensure quality, consistency, and best practices.",
            AgentType::PerformanceOptimizer => "You are a performance engineer. Optimize for speed and efficiency.",
            AgentType::DevOpsEngineer => "You are a DevOps engineer. Handle deployment, CI/CD, and infrastructure.",
            AgentType::Manager => "You are a swarm manager. Coordinate agents and resolve conflicts.",
        };

        let personality_traits = format!(
            "\n\nYour personality traits (deterministic):\n\
            - Simplicity preference: {:.0}% ({})\n\
            - Performance priority: {:.0}% ({})\n\
            - Innovation level: {:.0}% ({})\n\
            - Risk tolerance: {:.0}% ({})\n\
            - Communication: {:.0}% ({})",
            self.simplicity_bias * 100.0,
            if self.simplicity_bias > 0.5 {
                "prefer simple solutions"
            } else {
                "accept complexity when needed"
            },
            self.performance_bias * 100.0,
            if self.performance_bias > 0.5 {
                "performance-first"
            } else {
                "readability-first"
            },
            self.innovation_bias * 100.0,
            if self.innovation_bias > 0.5 {
                "innovative/experimental"
            } else {
                "conservative/standard"
            },
            self.risk_tolerance * 100.0,
            if self.risk_tolerance > 0.5 {
                "risk-tolerant"
            } else {
                "risk-averse"
            },
            self.verbosity * 100.0,
            if self.verbosity > 0.5 {
                "verbose/detailed"
            } else {
                "concise"
            }
        );

        format!("{}\n{}\n\nYou are part of a multi-agent swarm. Communicate with other agents via the shared memory and consensus system.",
            base, personality_traits)
    }

    /// Vote on proposal based on personality alignment
    pub fn vote_on(&self, proposal: &Proposal) -> Vote {
        // Calculate alignment between personality and proposal
        let alignment = self.calculate_alignment(proposal);

        if alignment > 0.7 {
            Vote::Approve
        } else if alignment < 0.3 {
            Vote::Reject
        } else {
            Vote::Abstain
        }
    }

    fn calculate_alignment(&self, proposal: &Proposal) -> f64 {
        // Simplified alignment calculation
        // In real implementation, would analyze proposal content vs personality
        0.5 + (self.risk_tolerance - 0.5) * 0.3 + (self.innovation_bias - 0.5) * 0.2
    }
}

/// Core trait for all swarm agents
#[async_trait]
pub trait SwarmAgent: Send + Sync {
    /// Unique identifier
    fn id(&self) -> AgentId;

    /// Agent type
    fn agent_type(&self) -> AgentType;

    /// Agent personality
    fn personality(&self) -> &AgentPersonality;

    /// Authority level (0.0 - 1.0)
    fn authority(&self) -> f64 {
        self.personality().authority
    }

    /// Main execution loop
    async fn run(&mut self) -> Result<AgentOutput>;

    /// Handle incoming message from swarm
    async fn on_message(&mut self, msg: SwarmMessage) -> Result<()>;

    /// Vote on proposal
    fn vote(&self, proposal: &Proposal) -> Vote {
        self.personality().vote_on(proposal)
    }
}

/// Concrete agent implementation
pub struct ConcreteAgent {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub personality: AgentPersonality,
    pub current_task: Option<Task>,
    pub message_rx: broadcast::Receiver<SwarmMessage>,
    pub memory: Arc<SwarmMemory>,
    pub consensus: Arc<ContinuousConsensus>,
    pub llm_client: Arc<SwarmLLMClient>,
    pub start_time: Instant,
    pub output: Option<AgentOutput>,
}

impl ConcreteAgent {
    pub fn new(
        id: AgentId,
        agent_type: AgentType,
        personality: AgentPersonality,
        message_rx: broadcast::Receiver<SwarmMessage>,
        memory: Arc<SwarmMemory>,
        consensus: Arc<ContinuousConsensus>,
        llm_client: Arc<SwarmLLMClient>,
    ) -> Self {
        Self {
            id,
            agent_type,
            personality,
            current_task: None,
            message_rx,
            memory,
            consensus,
            llm_client,
            start_time: Instant::now(),
            output: None,
        }
    }

    /// Process messages while working
    async fn process_messages(&mut self) -> Result<()> {
        while let Ok(msg) = self.message_rx.try_recv() {
            self.on_message(msg).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl SwarmAgent for ConcreteAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn agent_type(&self) -> AgentType {
        self.agent_type
    }

    fn personality(&self) -> &AgentPersonality {
        &self.personality
    }

    async fn run(&mut self) -> Result<AgentOutput> {
        tracing::info!(
            "Agent {:?} ({:?}) starting execution",
            self.id,
            self.agent_type
        );

        // Get task from memory
        let task = self
            .memory
            .get_task(self.id)
            .await
            .ok_or_else(|| anyhow!("No task assigned to agent {:?}", self.id))?;

        self.current_task = Some(task.clone());

        // Process any pending messages
        self.process_messages().await?;

        // Generate system prompt
        let system_prompt = self.personality.system_prompt(&self.agent_type);

        // Call LLM
        let context = self.memory.get_context_string(self.id).await;
        let request = LLMRequest {
            system: system_prompt,
            user: task.description.clone(),
            context,
        };

        let response = self.llm_client.execute(request).await?;

        // Parse files from LLM response
        let parsed_files = crate::swarm::parser::parse_llm_response(&response.content);
        let files_written: Vec<String> = parsed_files.iter().map(|f| f.path.clone()).collect();

        // Log extracted files
        if !parsed_files.is_empty() {
            tracing::info!(
                "Agent {:?} extracted {} files from LLM response",
                self.id,
                parsed_files.len()
            );
            for file in &parsed_files {
                tracing::debug!("  - {} ({} bytes)", file.path, file.content.len());
            }
        }

        // Parse output
        let output = AgentOutput {
            agent_id: self.id,
            agent_type: self.agent_type,
            task_id: task.id.clone(),
            content: response.content,
            files_written,
            patterns_shared: Vec::new(),
            execution_time_ms: self.start_time.elapsed().as_millis() as u64,
            consensus_approvals: 0,
        };

        // Store in memory
        self.memory.store_output(self.id, output.clone()).await;

        // Broadcast completion
        let msg = SwarmMessage::TaskCompleted {
            by: self.id,
            output: output.clone(),
        };

        // Note: Broadcast would happen via communication bus in real impl

        tracing::info!(
            "Agent {:?} completed in {}ms",
            self.id,
            output.execution_time_ms
        );

        self.output = Some(output.clone());
        Ok(output)
    }

    async fn on_message(&mut self, msg: SwarmMessage) -> Result<()> {
        match msg {
            SwarmMessage::Proposal { proposal, .. } => {
                let vote = self.vote(&proposal);
                self.consensus
                    .submit_vote(proposal.id, self.id, vote)
                    .await?;
            }
            SwarmMessage::PatternShare { pattern, .. } => {
                // Convert communication Pattern to memory Pattern and adopt if relevant
                let memory_pattern = super::memory::Pattern {
                    id: pattern.id,
                    title: pattern.title,
                    description: pattern.description,
                    code_template: None,
                    applicable_to: pattern
                        .applicable_to
                        .iter()
                        .map(|at| at.to_string())
                        .collect(),
                    success_rate: pattern.confidence,
                    usage_count: 1,
                };
                self.memory.adopt_pattern(self.id, memory_pattern).await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn vote(&self, proposal: &Proposal) -> Vote {
        self.personality.vote_on(proposal)
    }
}

/// Manager agent - coordinates other agents
pub struct ManagerAgent {
    pub id: AgentId,
    pub personality: AgentPersonality,
    pub workers: Arc<RwLock<Vec<AgentId>>>,
    pub message_rx: broadcast::Receiver<SwarmMessage>,
    pub memory: Arc<SwarmMemory>,
    pub consensus: Arc<ContinuousConsensus>,
    pub llm_client: Arc<SwarmLLMClient>,
}

impl ManagerAgent {
    fn should_adopt(&self, pattern: &super::memory::Pattern) -> bool {
        // Simplified: always adopt for now
        true
    }
}

#[async_trait]
impl SwarmAgent for ManagerAgent {
    fn id(&self) -> AgentId {
        self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Manager
    }

    fn personality(&self) -> &AgentPersonality {
        &self.personality
    }

    async fn run(&mut self) -> Result<AgentOutput> {
        tracing::info!("Manager agent {:?} coordinating swarm", self.id);

        // Manager doesn't do direct work, just coordinates
        // Monitor workers and resolve conflicts

        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(AgentOutput {
            agent_id: self.id,
            agent_type: AgentType::Manager,
            task_id: "coordination".to_string(),
            content: "Swarm coordination completed".to_string(),
            files_written: Vec::new(),
            patterns_shared: Vec::new(),
            execution_time_ms: 100,
            consensus_approvals: 0,
        })
    }

    async fn on_message(&mut self, _msg: SwarmMessage) -> Result<()> {
        // Manager handles coordination messages
        Ok(())
    }
}

/// Factory for creating agents
pub struct AgentFactory;

impl AgentFactory {
    /// Create standard agent
    pub async fn create(
        id: AgentId,
        agent_type: AgentType,
        personality: AgentPersonality,
        message_rx: broadcast::Receiver<SwarmMessage>,
        memory: Arc<SwarmMemory>,
        consensus: Arc<ContinuousConsensus>,
        llm_client: Arc<SwarmLLMClient>,
    ) -> Result<Box<dyn SwarmAgent>> {
        let agent = ConcreteAgent::new(
            id,
            agent_type,
            personality,
            message_rx,
            memory,
            consensus,
            llm_client,
        );

        Ok(Box::new(agent))
    }

    /// Create manager agent
    pub async fn create_manager(
        id: AgentId,
        workers: Arc<RwLock<std::collections::HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,
        message_rx: broadcast::Receiver<SwarmMessage>,
        memory: Arc<SwarmMemory>,
        consensus: Arc<ContinuousConsensus>,
        llm_client: Arc<SwarmLLMClient>,
    ) -> Result<Box<dyn SwarmAgent>> {
        let worker_ids: Vec<_> = workers.read().await.keys().cloned().collect();

        // Get hash from memory for deterministic personality
        let goal_hash = blake3::hash(b"manager");
        let personality = AgentPersonality::from_goal(&goal_hash, &AgentType::Manager);

        let manager = ManagerAgent {
            id,
            personality,
            workers: Arc::new(RwLock::new(worker_ids)),
            message_rx,
            memory,
            consensus,
            llm_client,
        };

        Ok(Box::new(manager))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_deterministic() {
        let goal_hash = blake3::hash(b"test");
        let agent_type = AgentType::APICoder;

        let id1 = AgentId::deterministic(&goal_hash, &agent_type, 0);
        let id2 = AgentId::deterministic(&goal_hash, &agent_type, 0);

        assert_eq!(id1.0, id2.0);
    }

    #[test]
    fn test_personality_deterministic() {
        let goal_hash = blake3::hash(b"test goal");

        let p1 = AgentPersonality::from_goal(&goal_hash, &AgentType::APICoder);
        let p2 = AgentPersonality::from_goal(&goal_hash, &AgentType::APICoder);

        assert_eq!(p1, p2);
    }

    #[test]
    fn test_different_types_different_personalities() {
        let goal_hash = blake3::hash(b"test goal");

        let p1 = AgentPersonality::from_goal(&goal_hash, &AgentType::APICoder);
        let p2 = AgentPersonality::from_goal(&goal_hash, &AgentType::AuthArchitect);

        // Should have different biases (but same authority per type)
        assert_ne!(p1.simplicity_bias, p2.simplicity_bias);
    }

    #[test]
    fn test_authority_levels() {
        let goal_hash = blake3::hash(b"test");

        let manager = AgentPersonality::from_goal(&goal_hash, &AgentType::Manager);
        let coder = AgentPersonality::from_goal(&goal_hash, &AgentType::APICoder);
        let doc = AgentPersonality::from_goal(&goal_hash, &AgentType::DocWriter);

        assert!(manager.authority > coder.authority);
        assert!(coder.authority > doc.authority);
    }
}
