//! SENTINEL SWARM - Deterministic Multi-Agent Intelligence
//!
//! This module implements a revolutionary swarm intelligence system where agents:
//! 1. Emerge deterministically from task context (not predefined)
//! 2. Self-organize into hierarchies (managers emerge when needed)
//! 3. Reach continuous consensus (every 100ms)
//! 4. Cross-pollinate knowledge in real-time
//! 5. Predict and pre-spawn resources
//! 6. Resolve conflicts creatively via synthesis
//! 7. Evolve across sessions via DNA
//!
//! All behavior is deterministic: same goal → same swarm → same results

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use blake3::Hash;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

// Re-export submodules
pub mod agent;
pub mod balancer;
pub mod circuit_breaker;
pub mod communication;
pub mod conflict;
pub mod consensus;
pub mod emergence;
pub mod human_in_the_loop;
pub mod llm;
pub mod marketplace;
pub mod memory;
pub mod parser;
pub mod predictor;
pub mod smart_routing;

pub use agent::{AgentFactory, AgentId, AgentPersonality, ConcreteAgent, SwarmAgent};
pub use balancer::{AgentHealth, HealthStatus, RebalanceStrategy, SwarmBalancer};
pub use communication::{
    CommunicationBus, MessagePayload, Proposal, ProposalId, SwarmMessage, Vote,
};
pub use conflict::{ArbiterAgent, Conflict, ConflictResolutionEngine, ConflictType, Resolution};
pub use consensus::{ContinuousConsensus, ProposalStatus};
pub use emergence::{
    AgentType, DetectedPattern, Domain, GoalAnalysis, GoalAnalyzer, SecurityLevel,
};
pub use llm::{LLMRequest, LLMResponse, SwarmLLMClient};
pub use human_in_the_loop::{ApprovalRequest, ApprovalType, HumanInTheLoop};
pub use marketplace::{AgentMarketplace, AgentTemplate};
pub use memory::{Episode, MemoryEntry, Pattern, SwarmMemory};
pub use predictor::{PredictiveOrchestrator, TaskPattern, TaskPrediction};
pub use smart_routing::{AgentCapabilities, RoutingStats, SmartRouter};

/// Unique identifier for a swarm agent (deterministic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SwarmId(pub [u8; 16]);

impl SwarmId {
    /// Generate deterministic ID from goal hash
    pub fn from_goal(goal_hash: &Hash) -> Self {
        let mut id = [0u8; 16];
        id.copy_from_slice(&goal_hash.as_bytes()[0..16]);
        Self(id)
    }
}

/// Core swarm coordinator - central hub for all swarm operations
pub struct SwarmCoordinator {
    /// Unique swarm identifier
    pub id: SwarmId,

    /// Original goal that spawned this swarm
    pub goal: String,

    /// Deterministic hash of goal
    pub goal_hash: Hash,

    /// Swarm configuration
    pub config: SwarmConfig,

    /// Active agents in swarm
    pub agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,

    /// Communication bus for inter-agent messaging
    pub communication_bus: Arc<CommunicationBus>,

    /// Continuous consensus engine
    pub consensus: Arc<ContinuousConsensus>,

    /// Shared swarm memory
    pub memory: Arc<SwarmMemory>,

    /// Conflict resolution engine
    pub conflict_resolver: Arc<ConflictResolutionEngine>,

    /// Predictive orchestrator
    pub predictor: Arc<PredictiveOrchestrator>,

    /// Load balancer
    pub balancer: Arc<SwarmBalancer>,

    /// LLM client for all agents
    pub llm_client: Arc<SwarmLLMClient>,

    /// Manager agent (if >3 workers)
    pub manager: Arc<RwLock<Option<AgentId>>>,

    /// Swarm DNA for evolution
    pub dna: Arc<RwLock<SwarmDNA>>,

    /// Execution results
    pub results: Arc<RwLock<Vec<AgentOutput>>>,

    /// Start time
    pub start_time: Instant,
}

/// Swarm DNA - evolves across sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmDNA {
    /// Generation number
    pub generation: u64,

    /// Successful patterns learned
    pub successful_patterns: Vec<Pattern>,

    /// Successful agent personalities
    pub successful_personalities: Vec<AgentPersonality>,

    /// Conflict resolutions learned
    pub conflict_resolutions: Vec<ConflictResolution>,

    /// Performance metrics history
    pub performance_history: Vec<GenerationMetrics>,
}

impl SwarmDNA {
    pub fn new() -> Self {
        Self {
            generation: 1,
            successful_patterns: Vec::new(),
            successful_personalities: Vec::new(),
            conflict_resolutions: Vec::new(),
            performance_history: Vec::new(),
        }
    }

    /// Evolve DNA based on session results
    pub fn evolve(&mut self, session: &SessionResult) {
        // Extract successful patterns
        for success in &session.successes {
            self.successful_patterns.push(success.pattern.clone());
        }

        // Extract successful personalities
        for agent in &session.agents {
            if agent.performance > 0.85 {
                self.successful_personalities
                    .push(agent.personality.clone());
            }
        }

        // Increment generation
        self.generation += 1;
    }
}

/// Metrics for one generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetrics {
    pub generation: u64,
    pub avg_execution_time_ms: u64,
    pub conflict_count: u32,
    pub consensus_reached_count: u32,
    pub agent_count: usize,
    pub timestamp: u64,
}

/// Session result for evolution
#[derive(Debug, Clone)]
pub struct SessionResult {
    pub successes: Vec<SuccessRecord>,
    pub agents: Vec<AgentResult>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SuccessRecord {
    pub pattern: Pattern,
    pub agent_id: AgentId,
}

#[derive(Debug, Clone)]
pub struct AgentResult {
    pub id: AgentId,
    pub personality: AgentPersonality,
    pub performance: f64,
}

/// Task assigned to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_type: AgentType,
    pub dependencies: Vec<String>,
    pub priority: f64, // 0.0 - 1.0
}

/// Output from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub agent_id: AgentId,
    pub agent_type: AgentType,
    pub task_id: String,
    pub content: String,
    pub files_written: Vec<String>,
    pub patterns_shared: Vec<Pattern>,
    pub execution_time_ms: u64,
    pub consensus_approvals: u32,
}

/// Swarm execution configuration
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Quorum threshold for consensus (0.0 - 1.0)
    pub quorum_threshold: f64,

    /// Consensus round interval (ms)
    pub consensus_interval_ms: u64,

    /// Max concurrent LLM calls
    pub max_concurrent_llm: usize,

    /// Enable predictive orchestration
    pub enable_prediction: bool,

    /// Enable auto-balancing
    pub enable_balancing: bool,

    /// Vote timeout (ms)
    pub vote_timeout_ms: u64,

    /// Maximum number of agents allowed (prevents DoS)
    pub max_agents: usize,

    /// Maximum execution time for entire swarm (seconds)
    pub max_execution_time_secs: u64,

    /// Maximum memory per agent (MB)
    pub max_memory_mb: usize,

    /// Enable circuit breaker for LLM providers
    pub enable_circuit_breaker: bool,

    /// Number of retries for failed LLM calls
    pub llm_retry_count: u32,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            quorum_threshold: 0.75,
            consensus_interval_ms: 100,
            max_concurrent_llm: 3,
            enable_prediction: true,
            enable_balancing: true,
            vote_timeout_ms: 2000,
            max_agents: 10,               // Prevent runaway agent spawning
            max_execution_time_secs: 300, // 5 minute timeout
            max_memory_mb: 512,           // 512MB per agent
            enable_circuit_breaker: true,
            llm_retry_count: 3,
        }
    }
}

impl SwarmCoordinator {
    /// Create new swarm coordinator from goal (deterministic)
    pub async fn from_goal(
        goal: impl Into<String>,
        llm_client: Arc<SwarmLLMClient>,
        config: SwarmConfig,
    ) -> Result<Self> {
        let goal = goal.into();
        let goal_hash = blake3::hash(goal.as_bytes());
        let id = SwarmId::from_goal(&goal_hash);

        // Initialize components
        let communication_bus = Arc::new(CommunicationBus::new());
        let memory = Arc::new(SwarmMemory::new());
        let consensus = Arc::new(ContinuousConsensus::new(
            config.quorum_threshold,
            config.vote_timeout_ms,
            communication_bus.clone(),
        ));
        let conflict_resolver = Arc::new(ConflictResolutionEngine::new());
        let predictor = Arc::new(PredictiveOrchestrator::new());
        let balancer = Arc::new(SwarmBalancer::new());
        let dna = Arc::new(RwLock::new(SwarmDNA::new()));

        Ok(Self {
            id,
            goal: goal.clone(),
            goal_hash,
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            communication_bus,
            consensus,
            memory,
            conflict_resolver,
            predictor,
            balancer,
            llm_client,
            manager: Arc::new(RwLock::new(None)),
            dna,
            results: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        })
    }

    /// Spawn deterministic agents based on goal analysis
    pub async fn spawn_agents(&self) -> Result<Vec<AgentId>> {
        // Analyze goal to determine which agents should emerge
        let analysis = GoalAnalyzer::analyze(&self.goal).await?;

        // Check max agents limit
        if analysis.required_agents.len() > self.config.max_agents {
            tracing::warn!(
                "Goal would spawn {} agents but max is {}. Truncating.",
                analysis.required_agents.len(),
                self.config.max_agents
            );
        }

        let mut agent_ids = Vec::new();

        // Spawn each required agent with deterministic ID (up to max)
        for (idx, agent_type) in analysis
            .required_agents
            .iter()
            .take(self.config.max_agents)
            .enumerate()
        {
            let id = AgentId::deterministic(&self.goal_hash, agent_type, idx as u32);

            // Generate deterministic personality
            let personality = AgentPersonality::from_goal(&self.goal_hash, agent_type);

            // Create agent
            let agent = AgentFactory::create(
                id,
                agent_type.clone(),
                personality,
                self.communication_bus.subscribe(),
                self.memory.clone(),
                self.consensus.clone(),
                self.llm_client.clone(),
            )
            .await?;

            // Store agent
            self.agents
                .write()
                .await
                .insert(id, Arc::new(Mutex::new(agent)));
            agent_ids.push(id);

            tracing::info!("Spawned agent {:?} ({:?})", id, agent_type);
        }

        // If >3 agents, spawn manager
        if agent_ids.len() > 3 {
            self.spawn_manager().await?;
        }

        tracing::info!("Swarm initialized with {} agents", agent_ids.len());

        Ok(agent_ids)
    }

    /// Spawn manager agent when needed
    async fn spawn_manager(&self) -> Result<AgentId> {
        let manager_type = AgentType::Manager;
        let idx = self.agents.read().await.len() as u32;
        let id = AgentId::deterministic(&self.goal_hash, &manager_type, idx);

        let personality = AgentPersonality::from_goal(&self.goal_hash, &manager_type);

        let manager = AgentFactory::create_manager(
            id,
            self.agents.clone(),
            self.communication_bus.subscribe(),
            self.memory.clone(),
            self.consensus.clone(),
            self.llm_client.clone(),
        )
        .await?;

        self.agents
            .write()
            .await
            .insert(id, Arc::new(Mutex::new(manager)));
        *self.manager.write().await = Some(id);

        tracing::info!("Spawned manager agent {:?}", id);

        Ok(id)
    }

    /// Execute all agents in parallel
    pub async fn execute_parallel(&self) -> Result<Vec<AgentOutput>> {
        let agents = self.agents.read().await;
        let mut handles = Vec::new();

        // Spawn execution task for each agent
        for (id, agent) in agents.iter() {
            let agent = agent.clone();
            let id = *id;

            let handle = tokio::spawn(async move {
                let mut agent = agent.lock().await;
                agent.run().await
            });

            handles.push((id, handle));
        }

        // Collect results
        let mut results = Vec::new();
        for (id, handle) in handles {
            match handle.await {
                Ok(Ok(output)) => {
                    tracing::info!("Agent {:?} completed successfully", id);
                    results.push(output);
                }
                Ok(Err(e)) => {
                    tracing::error!("Agent {:?} failed: {}", id, e);
                }
                Err(e) => {
                    tracing::error!("Agent {:?} panicked: {}", id, e);
                }
            }
        }

        // Store results
        *self.results.write().await = results.clone();

        Ok(results)
    }

    /// Start continuous consensus loop
    pub async fn start_consensus(&self) -> Result<()> {
        let consensus = self.consensus.clone();

        tokio::spawn(async move {
            if let Err(e) = consensus.run().await {
                tracing::error!("Consensus loop error: {}", e);
            }
        });

        Ok(())
    }

    /// Start predictive orchestration
    pub async fn start_prediction(&self) -> Result<()> {
        if !self.predictor.is_enabled() {
            return Ok(());
        }

        let predictor = self.predictor.clone();
        let agents = self.agents.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Monitor agent progress and predict next needs
                if let Err(e) = predictor.monitor_and_prefetch(agents.clone()).await {
                    tracing::debug!("Predictor error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Start load balancer
    pub async fn start_balancer(&self) -> Result<()> {
        let balancer = self.balancer.clone();
        let agents = self.agents.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                if let Err(e) = balancer.check_and_rebalance(agents.clone()).await {
                    tracing::warn!("Balancer error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Run complete swarm execution with timeout
    pub async fn run(&self) -> Result<SwarmExecutionResult> {
        tracing::info!("Starting swarm execution for goal: {}", self.goal);
        tracing::info!(
            "Max execution time: {}s",
            self.config.max_execution_time_secs
        );

        // Wrap entire execution in timeout
        let timeout_duration = Duration::from_secs(self.config.max_execution_time_secs);

        match tokio::time::timeout(timeout_duration, self.run_inner()).await {
            Ok(result) => result,
            Err(_) => {
                tracing::error!(
                    "Swarm execution timed out after {}s",
                    self.config.max_execution_time_secs
                );
                Err(anyhow!(
                    "Swarm execution timed out after {}s. Consider increasing max_execution_time_secs or simplifying the goal.",
                    self.config.max_execution_time_secs
                ))
            }
        }
    }

    /// Inner execution logic (without timeout wrapper)
    async fn run_inner(&self) -> Result<SwarmExecutionResult> {
        // 1. Spawn agents
        let agent_ids = self.spawn_agents().await?;

        // 2. Start background services
        self.start_consensus().await?;
        self.start_prediction().await?;
        self.start_balancer().await?;

        // 3. Execute agents in parallel
        let outputs = self.execute_parallel().await?;

        // 4. Detect and resolve conflicts
        let conflicts = self.conflict_resolver.detect_conflicts(&outputs).await;
        let conflicts_detected = conflicts.len();
        let mut resolutions = Vec::new();

        for conflict in conflicts {
            match self.conflict_resolver.resolve(conflict).await {
                Ok(resolution) => resolutions.push(resolution),
                Err(e) => tracing::error!("Conflict resolution failed: {}", e),
            }
        }

        // 5. Compile final result
        let result = SwarmExecutionResult {
            swarm_id: self.id,
            goal: self.goal.clone(),
            agent_count: agent_ids.len(),
            outputs,
            conflicts_detected,
            conflicts_resolved: resolutions.len(),
            execution_time_ms: self.start_time.elapsed().as_millis() as u64,
            consensus_rounds: self.consensus.get_round().await,
        };

        // 6. Evolve DNA
        let session = self.create_session_result(&result).await;
        self.dna.write().await.evolve(&session);

        tracing::info!(
            "Swarm execution completed in {}ms",
            result.execution_time_ms
        );

        Ok(result)
    }

    /// Create session result for evolution
    async fn create_session_result(&self, result: &SwarmExecutionResult) -> SessionResult {
        let agents: Vec<_> = self
            .agents
            .read()
            .await
            .iter()
            .map(|(id, _)| {
                AgentResult {
                    id: *id,
                    personality: AgentPersonality::from_goal(&self.goal_hash, &AgentType::APICoder), // Default
                    performance: 0.9, // Simplified
                }
            })
            .collect();

        SessionResult {
            successes: Vec::new(), // Simplified
            agents,
            execution_time_ms: result.execution_time_ms,
        }
    }

    /// Get agent by ID
    pub async fn get_agent(&self, id: AgentId) -> Option<Arc<Mutex<Box<dyn SwarmAgent>>>> {
        self.agents.read().await.get(&id).cloned()
    }

    /// Broadcast message to all agents
    pub async fn broadcast(&self, message: SwarmMessage) -> Result<()> {
        self.communication_bus.broadcast(message).await
    }
}

/// Final swarm execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmExecutionResult {
    pub swarm_id: SwarmId,
    pub goal: String,
    pub agent_count: usize,
    pub outputs: Vec<AgentOutput>,
    pub conflicts_detected: usize,
    pub conflicts_resolved: usize,
    pub execution_time_ms: u64,
    pub consensus_rounds: u64,
}

/// Conflict resolution record for DNA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub conflict_type: String,
    pub resolution: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub success: bool,
    pub message: String,
}

/// Get current timestamp in milliseconds
fn now() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Get current agent ID (placeholder for now)
fn current_agent_id() -> AgentId {
    AgentId::deterministic(&blake3::hash(b"system"), &AgentType::Manager, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swarm_id_deterministic() {
        let goal = "test goal";
        let hash = blake3::hash(goal.as_bytes());

        let id1 = SwarmId::from_goal(&hash);
        let id2 = SwarmId::from_goal(&hash);

        assert_eq!(id1.0, id2.0);
    }

    #[test]
    fn test_agent_id_deterministic() {
        let goal_hash = blake3::hash(b"test");
        let agent_type = AgentType::APICoder;

        let id1 = AgentId::deterministic(&goal_hash, &agent_type, 0);
        let id2 = AgentId::deterministic(&goal_hash, &agent_type, 0);

        assert_eq!(id1, id2);
    }
}
