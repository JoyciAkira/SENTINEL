# SENTINEL SWARM: Piano di Implementazione Deterministico

## Overview: Da Vision a Realtà

**Timeline**: 4 settimane (20 giorni lavorativi)  
**Sviluppatori**: 1-2 persone  
**Output**: Sistema Swarm Intelligence completamente operativo

---

## Settimana 1: Fondamenta (Giorni 1-5)

### Giorno 1-2: Core Swarm Engine

**File**: `crates/sentinel-agent-native/src/swarm/mod.rs`

```rust
//! Swarm Intelligence Core
//! 
//! Deterministic multi-agent coordination system

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, broadcast};
use blake3::Hash;

/// Unique identifier for a swarm agent (deterministic)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(pub [u8; 16]);

impl AgentId {
    /// Generate deterministic ID from goal hash + agent type + index
    pub fn deterministic(goal_hash: &Hash, agent_type: &str, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(goal_hash.as_bytes());
        hasher.update(agent_type.as_bytes());
        hasher.update(&index.to_le_bytes());
        
        let hash = hasher.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&hash.as_bytes()[0..16]);
        
        Self(id)
    }
}

/// Core swarm coordinator
pub struct SwarmCoordinator {
    /// Goal that spawned this swarm
    pub goal: String,
    
    /// Deterministic hash of goal
    pub goal_hash: Hash,
    
    /// Active agents in swarm
    pub agents: Arc<RwLock<HashMap<AgentId, Box<dyn SwarmAgent>>>>,
    
    /// Communication bus (all agents subscribe)
    pub broadcast_tx: broadcast::Sender<SwarmMessage>,
    
    /// Consensus engine
    pub consensus: Arc<ContinuousConsensus>,
    
    /// Shared memory
    pub memory: Arc<SwarmMemory>,
}

impl SwarmCoordinator {
    /// Create new swarm from goal (deterministic)
    pub async fn from_goal(goal: impl Into<String>) -> Result<Self> {
        let goal = goal.into();
        let goal_hash = blake3::hash(goal.as_bytes());
        
        let (broadcast_tx, _) = broadcast::channel(1000);
        let memory = Arc::new(SwarmMemory::new());
        let consensus = Arc::new(ContinuousConsensus::new(
            broadcast_tx.subscribe(),
            memory.clone(),
        ));
        
        Ok(Self {
            goal,
            goal_hash,
            agents: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            consensus,
            memory,
        })
    }
    
    /// Spawn deterministic agents based on goal analysis
    pub async fn spawn_agents(&self) -> Result<Vec<AgentId>> {
        let analysis = GoalAnalyzer::analyze(&self.goal).await?;
        let mut agent_ids = Vec::new();
        
        for (idx, agent_type) in analysis.required_agents.iter().enumerate() {
            let id = AgentId::deterministic(&self.goal_hash, &agent_type.to_string(), idx as u32);
            
            let personality = AgentPersonality::from_goal(&self.goal_hash, agent_type);
            
            let agent = AgentFactory::create(
                id,
                agent_type.clone(),
                personality,
                self.broadcast_tx.subscribe(),
                self.memory.clone(),
                self.consensus.clone(),
            ).await?;
            
            self.agents.write().await.insert(id, agent);
            agent_ids.push(id);
        }
        
        // If >3 agents, spawn manager
        if agent_ids.len() > 3 {
            self.spawn_manager().await?;
        }
        
        Ok(agent_ids)
    }
}
```

**Tasks**:
- [ ] Implementare `AgentId::deterministic()` con Blake3
- [ ] Creare `SwarmCoordinator` con goal hash
- [ ] Implementare spawn deterministici degli agenti
- [ ] Test: Stesso goal → stessi AgentId

---

### Giorno 3-4: Emergence Engine

**File**: `crates/sentinel-agent-native/src/swarm/emergence.rs`

```rust
//! Deterministic Agent Emergence
//! 
//! Agents don't exist before the task - they emerge from context

use regex::Regex;

/// Analysis result that determines which agents emerge
#[derive(Debug, Clone)]
pub struct GoalAnalysis {
    /// Domain detected (e.g., "security", "database", "frontend")
    pub domain: Domain,
    
    /// Complexity score (0.0 - 1.0)
    pub complexity: f64,
    
    /// Security criticality
    pub security_level: SecurityLevel,
    
    /// Patterns detected
    pub patterns: Vec<DetectedPattern>,
    
    /// Agents that should emerge
    pub required_agents: Vec<AgentType>,
}

pub struct GoalAnalyzer;

impl GoalAnalyzer {
    /// Deterministic analysis of goal
    pub async fn analyze(goal: &str) -> Result<GoalAnalysis> {
        let goal_lower = goal.to_lowercase();
        let mut agents = Vec::new();
        let mut patterns = Vec::new();
        
        // Rule-based detection (deterministic)
        if Self::detect_pattern(&goal_lower, &["auth", "login", "jwt", "oauth", "password"]) {
            agents.push(AgentType::AuthArchitect);
            agents.push(AgentType::SecurityAuditor);
            patterns.push(DetectedPattern::Authentication);
            
            if goal_lower.contains("jwt") {
                agents.push(AgentType::JWTCoder);
                patterns.push(DetectedPattern::JWT);
            }
        }
        
        if Self::detect_pattern(&goal_lower, &["api", "endpoint", "rest", "graphql"]) {
            agents.push(AgentType::APICoder);
            patterns.push(DetectedPattern::API);
        }
        
        if Self::detect_pattern(&goal_lower, &["test", "spec", "jest", "pytest"]) {
            agents.push(AgentType::TestWriter);
            patterns.push(DetectedPattern::Testing);
        }
        
        if Self::detect_pattern(&goal_lower, &["database", "db", "postgres", "mongo"]) {
            agents.push(AgentType::DatabaseArchitect);
            patterns.push(DetectedPattern::Database);
        }
        
        // Always add these for any non-trivial task
        let complexity = Self::calculate_complexity(goal);
        if complexity > 0.3 {
            agents.push(AgentType::DocWriter);
        }
        
        if complexity > 0.5 {
            agents.push(AgentType::ReviewAgent);
        }
        
        // Deduplicate while preserving order
        let unique_agents: Vec<_> = agents.into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        
        Ok(GoalAnalysis {
            domain: Self::detect_domain(&goal_lower),
            complexity,
            security_level: Self::assess_security(&goal_lower),
            patterns,
            required_agents: unique_agents,
        })
    }
    
    fn detect_pattern(goal: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|kw| goal.contains(kw))
    }
    
    fn calculate_complexity(goal: &str) -> f64 {
        // Deterministic complexity based on:
        // - Word count
        // - Technical terms density
        // - Conjunctions (and, with, plus)
        let words = goal.split_whitespace().count();
        let technical_terms = goal.matches(|c: char| c.is_uppercase()).count();
        let conjunctions = goal.matches("and").count() + goal.matches("with").count();
        
        let score = (words as f64 * 0.01) + 
                    (technical_terms as f64 * 0.05) + 
                    (conjunctions as f64 * 0.1);
        
        score.min(1.0)
    }
    
    fn detect_domain(goal: &str) -> Domain {
        if goal.contains("frontend") || goal.contains("ui") || goal.contains("react") {
            Domain::Frontend
        } else if goal.contains("backend") || goal.contains("api") {
            Domain::Backend
        } else if goal.contains("database") || goal.contains("sql") {
            Domain::Database
        } else {
            Domain::General
        }
    }
    
    fn assess_security(goal: &str) -> SecurityLevel {
        let critical_terms = ["auth", "password", "security", "encrypt", "token"];
        let count = critical_terms.iter().filter(|t| goal.contains(*t)).count();
        
        match count {
            0 => SecurityLevel::Low,
            1 => SecurityLevel::Medium,
            _ => SecurityLevel::High,
        }
    }
}

/// Agent types that can emerge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AgentType {
    AuthArchitect,
    SecurityAuditor,
    JWTCoder,
    APICoder,
    TestWriter,
    DocWriter,
    DatabaseArchitect,
    ReviewAgent,
    PerformanceOptimizer,
    Manager, // Emerges when >3 agents
}
```

**Tasks**:
- [ ] Implementare detection patterns per ogni dominio
- [ ] Creare scoring deterministico per complexity
- [ ] Test: Stesso goal → stessa lista di agenti
- [ ] Documentare tutti i pattern riconosciuti

---

### Giorno 5: Agent Personality & Factory

**File**: `crates/sentinel-agent-native/src/swarm/agent.rs`

```rust
//! Swarm Agent Implementation

use async_trait::async_trait;

/// Deterministic agent personality (affects creativity bias)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentPersonality {
    /// Bias toward simple vs complex solutions (0.0-1.0)
    pub simplicity_bias: f64,
    
    /// Bias toward performance vs readability (0.0-1.0)
    pub performance_bias: f64,
    
    /// Bias toward innovation vs standards (0.0-1.0)
    pub innovation_bias: f64,
    
    /// Risk tolerance (0.0-1.0)
    pub risk_tolerance: f64,
    
    /// Communication verbosity (0.0-1.0)
    pub verbosity: f64,
}

impl AgentPersonality {
    /// Generate deterministic personality from goal hash
    pub fn from_goal(goal_hash: &blake3::Hash, agent_type: &AgentType) -> Self {
        let bytes = goal_hash.as_bytes();
        
        // Derive each trait from different byte positions
        // This ensures deterministic but diverse personalities
        Self {
            simplicity_bias: Self::byte_to_f64(bytes[0]),
            performance_bias: Self::byte_to_f64(bytes[1]),
            innovation_bias: Self::byte_to_f64(bytes[2]),
            risk_tolerance: Self::byte_to_f64(bytes[3]),
            verbosity: Self::byte_to_f64(bytes[4]),
        }
    }
    
    fn byte_to_f64(byte: u8) -> f64 {
        byte as f64 / 255.0
    }
    
    /// Generate system prompt based on personality
    pub fn system_prompt(&self, agent_type: &AgentType) -> String {
        let base = match agent_type {
            AgentType::AuthArchitect => "You are an authentication architecture expert.",
            AgentType::SecurityAuditor => "You are a security auditor focused on vulnerabilities.",
            AgentType::JWTCoder => "You are a JWT implementation specialist.",
            AgentType::APICoder => "You are an API design and implementation expert.",
            AgentType::TestWriter => "You are a test automation expert.",
            AgentType::DocWriter => "You are a technical documentation specialist.",
            _ => "You are a software engineering expert.",
        };
        
        let personality_traits = format!(
            "Your personality traits:\\
            - Simplicity preference: {:.0}% ({} solutions)\
            - Performance priority: {:.0}% ({} focused)\
            - Innovation level: {:.0}% ({})\
            - Risk tolerance: {:.0}% ({})\
            ",
            self.simplicity_bias * 100.0,
            if self.simplicity_bias > 0.5 { "prefer simple" } else { "accept complex" },
            self.performance_bias * 100.0,
            if self.performance_bias > 0.5 { "performance" } else { "readability" },
            self.innovation_bias * 100.0,
            if self.innovation_bias > 0.5 { "innovative" } else { "standard" },
            self.risk_tolerance * 100.0,
            if self.risk_tolerance > 0.5 { "risk-tolerant" } else { "cautious" },
        );
        
        format!("{}\n\n{}", base, personality_traits)
    }
}

/// Trait that all swarm agents must implement
#[async_trait]
pub trait SwarmAgent: Send + Sync {
    /// Unique ID
    fn id(&self) -> AgentId;
    
    /// Agent type
    fn agent_type(&self) -> AgentType;
    
    /// Authority level (0.0-1.0, affects voting power)
    fn authority(&self) -> f64;
    
    /// Main execution loop
    async fn run(&mut self) -> Result<AgentOutput>;
    
    /// Handle incoming message
    async fn on_message(&mut self, msg: SwarmMessage) -> Result<()>;
    
    /// Vote on proposal (for consensus)
    async fn vote(&self, proposal: &Proposal) -> Vote;
}

/// Concrete agent implementation
pub struct ConcreteAgent {
    pub id: AgentId,
    pub agent_type: AgentType,
    pub personality: AgentPersonality,
    pub authority: f64,
    pub llm_client: Arc<OpenRouterClient>,
    pub broadcast_rx: broadcast::Receiver<SwarmMessage>,
    pub memory: Arc<SwarmMemory>,
    pub consensus: Arc<ContinuousConsensus>,
}

#[async_trait]
impl SwarmAgent for ConcreteAgent {
    fn id(&self) -> AgentId {
        self.id
    }
    
    fn agent_type(&self) -> AgentType {
        self.agent_type
    }
    
    fn authority(&self) -> f64 {
        self.authority
    }
    
    async fn run(&mut self) -> Result<AgentOutput> {
        // 1. Get task from swarm memory
        let task = self.memory.get_task(self.id).await?;
        
        // 2. Generate using LLM with personality
        let system_prompt = self.personality.system_prompt(&self.agent_type);
        let response = self.llm_client
            .chat_completion(&system_prompt, &task.description)
            .await?;
        
        // 3. Extract and validate output
        let output = self.parse_output(&response.content)?;
        
        // 4. Broadcast result to other agents
        self.broadcast_result(&output).await?;
        
        Ok(output)
    }
    
    async fn on_message(&mut self, msg: SwarmMessage) -> Result<()> {
        match msg {
            SwarmMessage::Proposal { proposal, .. } => {
                let vote = self.vote(&proposal).await;
                self.consensus.submit_vote(self.id, proposal.id, vote).await?;
            }
            SwarmMessage::PatternShare { pattern, .. } => {
                // Adopt pattern if applicable
                if self.should_adopt(&pattern) {
                    self.memory.adopt_pattern(pattern).await?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn vote(&self, proposal: &Proposal) -> Vote {
        // Deterministic voting based on:
        // - Agent type relevance
        // - Personality alignment
        // - Past experience (from DNA)
        
        let relevance = self.calculate_relevance(&proposal);
        let alignment = self.calculate_alignment(&proposal);
        
        if relevance * alignment > 0.7 {
            Vote::Approve
        } else if relevance * alignment < 0.3 {
            Vote::Reject
        } else {
            Vote::Abstain
        }
    }
}
```

**Tasks**:
- [ ] Implementare `AgentPersonality::from_goal()` deterministico
- [ ] Creare `ConcreteAgent` con LLM integration
- [ ] Implementare voting logic
- [ ] Test: Stesso agente → stesso voto per stessa proposal

---

## Settimana 2: Comunicazione e Consenso (Giorni 6-10)

### Giorno 6-7: Communication Bus

**File**: `crates/sentinel-agent-native/src/swarm/communication.rs`

```rust
//! Inter-agent communication system

/// Message types for swarm communication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SwarmMessage {
    /// Task assignment
    TaskAssigned {
        to: AgentId,
        task: Task,
    },
    
    /// Task completed
    TaskCompleted {
        by: AgentId,
        output: AgentOutput,
    },
    
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
    
    /// Pattern sharing
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
    
    /// Handoff (transfer control)
    Handoff {
        from: AgentId,
        to: AgentId,
        context: HandoffContext,
    },
    
    /// System message
    System {
        level: SystemLevel,
        message: String,
    },
}

pub struct CommunicationBus {
    broadcast_tx: broadcast::Sender<SwarmMessage>,
    message_log: Arc<RwLock<Vec<TimestampedMessage>>>,
}

impl CommunicationBus {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(10000);
        
        Self {
            broadcast_tx,
            message_log: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<SwarmMessage> {
        self.broadcast_tx.subscribe()
    }
    
    pub async fn broadcast(&self, msg: SwarmMessage) -> Result<()> {
        // Log message
        self.message_log.write().await.push(TimestampedMessage {
            timestamp: now(),
            message: msg.clone(),
        });
        
        // Broadcast to all agents
        let _ = self.broadcast_tx.send(msg);
        
        Ok(())
    }
    
    pub async fn send_direct(&self, to: AgentId, msg: SwarmMessage) -> Result<()> {
        // Direct messages go through broadcast but are filtered by receiver
        self.broadcast(msg).await
    }
}
```

**Tasks**:
- [ ] Implementare tutti i tipi di messaggio
- [ ] Creare broadcast efficiente
- [ ] Aggiungere message logging
- [ ] Test: Messaggi arrivano a tutti gli agenti

---

### Giorno 8-9: Continuous Consensus

**File**: `crates/sentinel-agent-native/src/swarm/consensus.rs`

```rust
//! Continuous consensus system (every 100ms)

pub struct ContinuousConsensus {
    /// Current consensus round
    round: AtomicU64,
    
    /// Proposals pending consensus
    pending_proposals: Arc<RwLock<HashMap<ProposalId, ProposalState>>>,
    
    /// Consensus history
    consensus_history: Arc<RwLock<Vec<ConsensusRecord>>>,
    
    /// Quorum threshold (e.g., 0.8 for 80%)
    quorum_threshold: f64,
    
    /// Voting timeout (ms)
    vote_timeout_ms: u64,
}

impl ContinuousConsensus {
    pub fn new(quorum_threshold: f64, vote_timeout_ms: u64) -> Self {
        Self {
            round: AtomicU64::new(0),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
            consensus_history: Arc::new(RwLock::new(Vec::new())),
            quorum_threshold,
            vote_timeout_ms,
        }
    }
    
    /// Start consensus loop (runs every 100ms)
    pub async fn run(&self, swarm: &SwarmCoordinator) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            let round = self.round.fetch_add(1, Ordering::SeqCst);
            
            // 1. Process pending proposals
            self.process_pending_proposals(swarm).await?;
            
            // 2. Check for timeouts
            self.check_timeouts().await?;
            
            // 3. Emit consensus heartbeat
            if round % 10 == 0 {
                self.emit_heartbeat().await?;
            }
        }
    }
    
    /// Submit a proposal for consensus
    pub async fn propose(&self, proposal: Proposal, by: AgentId) -> Result<ProposalId> {
        let id = ProposalId::new();
        
        let state = ProposalState {
            id,
            proposal,
            proposed_by: by,
            proposed_at: now(),
            votes: HashMap::new(),
            status: ProposalStatus::Voting,
        };
        
        self.pending_proposals.write().await.insert(id, state);
        
        Ok(id)
    }
    
    /// Submit vote for proposal
    pub async fn vote(&self, proposal_id: ProposalId, by: AgentId, vote: Vote) -> Result<()> {
        let mut proposals = self.pending_proposals.write().await;
        
        if let Some(state) = proposals.get_mut(&proposal_id) {
            state.votes.insert(by, vote);
            
            // Check if consensus reached
            if self.check_consensus(state) {
                state.status = ProposalStatus::Accepted;
                self.record_consensus(state).await?;
            }
        }
        
        Ok(())
    }
    
    fn check_consensus(&self, state: &ProposalState) -> bool {
        let total_votes = state.votes.len() as f64;
        let approve_votes = state.votes.values()
            .filter(|v| matches!(v, Vote::Approve))
            .count() as f64;
        
        let approval_rate = approve_votes / total_votes;
        approval_rate >= self.quorum_threshold
    }
    
    async fn process_pending_proposals(&self, swarm: &SwarmCoordinator) -> Result<()> {
        let proposals = self.pending_proposals.read().await;
        
        for (id, state) in proposals.iter() {
            if state.status == ProposalStatus::Voting {
                // Request votes from all agents
                swarm.broadcast(SwarmMessage::Proposal {
                    id: *id,
                    by: state.proposed_by,
                    proposal: state.proposal.clone(),
                    timestamp: now(),
                }).await?;
            }
        }
        
        Ok(())
    }
}

/// Proposal for consensus
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub action: ProposedAction,
    pub impact: ImpactAssessment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Approve,
    Reject,
    Abstain,
}
```

**Tasks**:
- [ ] Implementare consensus loop ogni 100ms
- [ ] Creare voting system con quorum
- [ ] Aggiungere timeout handling
- [ ] Test: Proposal raggiunge consenso in <500ms

---

### Giorno 10: Swarm Memory

**File**: `crates/sentinel-agent-native/src/swarm/memory.rs`

```rust
//! Shared memory system for all agents

use dashmap::DashMap;

pub struct SwarmMemory {
    /// Working memory (short-term, TTL 1 minute)
    working: Arc<DashMap<String, MemoryEntry>>,
    
    /// Episodic memory (events)
    episodic: Arc<DashMap<String, Vec<Episode>>>,
    
    /// Semantic memory (concepts)
    semantic: Arc<DashMap<String, Concept>>,
    
    /// Procedural memory (patterns)
    procedural: Arc<DashMap<String, Pattern>>,
}

impl SwarmMemory {
    pub fn new() -> Self {
        // Start TTL cleanup task
        let working = Arc::new(DashMap::new());
        let working_clone = working.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                let now = Instant::now();
                working_clone.retain(|_, entry: &mut MemoryEntry| {
                    entry.expires_at > now
                });
            }
        });
        
        Self {
            working,
            episodic: Arc::new(DashMap::new()),
            semantic: Arc::new(DashMap::new()),
            procedural: Arc::new(DashMap::new()),
        }
    }
    
    /// Write to working memory (all agents can see immediately)
    pub fn write(&self, key: impl Into<String>, value: impl Serialize, ttl: Duration) {
        let entry = MemoryEntry {
            value: serde_json::to_vec(&value).unwrap(),
            written_at: Instant::now(),
            expires_at: Instant::now() + ttl,
            written_by: current_agent_id(),
        };
        
        self.working.insert(key.into(), entry);
    }
    
    /// Read from memory (checks all layers)
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        // 1. Check working memory
        if let Some(entry) = self.working.get(key) {
            return serde_json::from_slice(&entry.value).ok();
        }
        
        // 2. Check semantic memory
        if let Some(concept) = self.semantic.get(key) {
            return serde_json::from_slice(&concept.data).ok();
        }
        
        // 3. Check procedural memory
        if let Some(pattern) = self.procedural.get(key) {
            return serde_json::from_slice(&pattern.template).ok();
        }
        
        None
    }
    
    /// Record episode (important event)
    pub fn record_episode(&self, episode: Episode) {
        let key = episode.category.clone();
        
        self.episodic
            .entry(key)
            .or_insert_with(Vec::new)
            .push(episode);
    }
    
    /// Store pattern for cross-pollination
    pub fn store_pattern(&self, pattern: Pattern) {
        self.procedural.insert(pattern.id.clone(), pattern);
    }
}

pub struct MemoryEntry {
    pub value: Vec<u8>,
    pub written_at: Instant,
    pub expires_at: Instant,
    pub written_by: AgentId,
}

pub struct Episode {
    pub timestamp: u64,
    pub category: String,
    pub description: String,
    pub agents_involved: Vec<AgentId>,
    pub outcome: Outcome,
}
```

**Tasks**:
- [ ] Implementare 4 layer di memoria
- [ ] Aggiungere TTL automatico
- [ ] Creare cross-pollination system
- [ ] Test: Scrittura da un agente, lettura da un altro in <1ms

---

## Settimana 3: Feature Avanzate (Giorni 11-15)

### Giorno 11-12: Conflict Resolution Engine

**File**: `crates/sentinel-agent-native/src/swarm/conflict.rs`

```rust
//! Conflict detection and resolution

pub struct ConflictResolutionEngine {
    /// Detected conflicts
    conflicts: Arc<RwLock<Vec<Conflict>>>,
    
    /// Resolution strategies
    strategies: Vec<Box<dyn ResolutionStrategy>>,
    
    /// Conflict journal (for learning)
    journal: Arc<RwLock<ConflictJournal>>,
}

impl ConflictResolutionEngine {
    /// Detect conflicts between agent outputs
    pub async fn detect_conflicts(&self, outputs: &[AgentOutput]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        
        // Check for resource conflicts (same file)
        let mut file_claims: HashMap<String, Vec<AgentId>> = HashMap::new();
        for output in outputs {
            for file in &output.files_written {
                file_claims.entry(file.clone())
                    .or_insert_with(Vec::new)
                    .push(output.agent_id);
            }
        }
        
        for (file, agents) in file_claims {
            if agents.len() > 1 {
                conflicts.push(Conflict {
                    type_: ConflictType::ResourceConflict { resource: file },
                    involved_agents: agents,
                    detected_at: now(),
                });
            }
        }
        
        // Check for technical disagreements
        for i in 0..outputs.len() {
            for j in (i+1)..outputs.len() {
                if self.technical_disagreement(&outputs[i], &outputs[j]) {
                    conflicts.push(Conflict {
                        type_: ConflictType::TechnicalDisagreement {
                            issue: "Architecture mismatch".to_string(),
                        },
                        involved_agents: vec![outputs[i].agent_id, outputs[j].agent_id],
                        detected_at: now(),
                    });
                }
            }
        }
        
        conflicts
    }
    
    /// Resolve conflicts using strategies
    pub async fn resolve(&self, conflict: Conflict) -> Result<Resolution> {
        // 1. Check journal for similar past conflicts
        if let Some(past) = self.journal.read().await.find_similar(&conflict) {
            return Ok(past.resolution.clone());
        }
        
        // 2. Spawn arbiter agent
        let arbiter = ArbiterAgent::spawn(&conflict.involved_agents);
        
        // 3. Arbiter analyzes and proposes synthesis
        let analysis = arbiter.analyze(&conflict).await?;
        let synthesis = arbiter.synthesize(analysis).await?;
        
        // 4. Get approval from involved agents
        let approvals = self.get_approvals(&conflict.involved_agents, &synthesis).await?;
        
        if approvals.iter().filter(|a| **a).count() as f64 / approvals.len() as f64 > 0.7 {
            let resolution = Resolution::Synthesis(synthesis);
            
            // 5. Journal the resolution
            self.journal.write().await.record(ConflictEntry {
                conflict: conflict.clone(),
                resolution: resolution.clone(),
                timestamp: now(),
            });
            
            Ok(resolution)
        } else {
            // Fallback: escalation to human
            Ok(Resolution::Escalate)
        }
    }
}
```

**Tasks**:
- [ ] Implementare conflict detection
- [ ] Creare arbiter agent
- [ ] Aggiungere conflict journal
- [ ] Test: Conflitto rilevato e risolto in <2s

---

### Giorno 13-14: Predictive Orchestrator

**File**: `crates/sentinel-agent-native/src/swarm/predictor.rs`

```rust
//! Predictive task orchestration

pub struct PredictiveOrchestrator {
    /// Task patterns learned
    patterns: Arc<RwLock<Vec<TaskPattern>>>,
    
    /// Prefetch queue
    prefetch_queue: Arc<RwLock<Vec<PrefetchTask>>>,
}

impl PredictiveOrchestrator {
    /// Predict next tasks based on current progress
    pub async fn predict_next(&self, current_task: &Task, progress: f64) -> Vec<TaskPrediction> {
        let mut predictions = Vec::new();
        
        // Pattern: When auth is 60% done, tests will be needed
        if current_task.name.contains("auth") && progress > 0.6 {
            predictions.push(TaskPrediction {
                task: Task {
                    name: "auth_tests".to_string(),
                    description: "Write tests for auth system".to_string(),
                    agent_type: AgentType::TestWriter,
                },
                confidence: 0.95,
                expected_start: Duration::from_secs(3),
            });
        }
        
        // Pattern: After any coder, security audit follows
        if matches!(current_task.agent_type, AgentType::JWTCoder | AgentType::APICoder) {
            predictions.push(TaskPrediction {
                task: Task {
                    name: "security_audit".to_string(),
                    description: "Security review of generated code".to_string(),
                    agent_type: AgentType::SecurityAuditor,
                },
                confidence: 0.85,
                expected_start: Duration::from_secs(2),
            });
        }
        
        predictions
    }
    
    /// Pre-spawn agents for predicted tasks
    pub async fn prefetch_agents(&self, predictions: &[TaskPrediction]) -> Result<()> {
        for pred in predictions {
            if pred.confidence > 0.8 {
                // Pre-spawn agent
                let agent = AgentFactory::create_idle(pred.task.agent_type).await?;
                
                self.prefetch_queue.write().await.push(PrefetchTask {
                    agent,
                    predicted_task: pred.task.clone(),
                    spawn_time: Instant::now(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Get pre-spawned agent if available
    pub async fn get_prefetched(&self, task_type: AgentType) -> Option<Box<dyn SwarmAgent>> {
        let mut queue = self.prefetch_queue.write().await;
        
        if let Some(pos) = queue.iter().position(|pt| pt.predicted_task.agent_type == task_type) {
            let pt = queue.remove(pos);
            return Some(pt.agent);
        }
        
        None
    }
}
```

**Tasks**:
- [ ] Implementare pattern matching
- [ ] Creare prefetch system
- [ ] Aggiungere confidence scoring
- [ ] Test: Agent pre-spawned prima che serva

---

### Giorno 15: Auto-Balancer

**File**: `crates/sentinel-agent-native/src/swarm/balancer.rs`

```rust
//! Automatic load balancing and health monitoring

pub struct SwarmBalancer {
    /// Health checks
    health: Arc<DashMap<AgentId, AgentHealth>>,
    
    /// Rebalance strategies
    strategies: Vec<RebalanceStrategy>,
}

impl SwarmBalancer {
    /// Monitor agent health
    pub async fn monitor(&self, agent_id: AgentId) -> AgentHealth {
        let health = AgentHealth {
            agent_id,
            status: HealthStatus::Healthy,
            last_heartbeat: Instant::now(),
            tasks_per_minute: 5.0,
            avg_response_time_ms: 1200,
        };
        
        self.health.insert(agent_id, health.clone());
        
        health
    }
    
    /// Check all agents and rebalance if needed
    pub async fn rebalance(&self, swarm: &mut SwarmCoordinator) -> Result<()> {
        for entry in self.health.iter() {
            let (id, health) = (entry.key(), entry.value());
            
            match health.status {
                HealthStatus::Slow { .. } => {
                    // Spawn helper agent
                    let helper = swarm.spawn_helper(*id).await?;
                    tracing::info!("Spawned helper for slow agent {:?}", id);
                }
                
                HealthStatus::Stuck { timeout_secs } if timeout_secs > 30 => {
                    // Kill and respawn
                    swarm.kill_agent(*id).await?;
                    let fresh = swarm.respawn_agent(*id).await?;
                    tracing::info!("Respawned stuck agent {:?} as {:?}", id, fresh);
                }
                
                HealthStatus::Overloaded => {
                    // Redistribute workload
                    self.redistribute_workload(*id, swarm).await?;
                }
                
                _ => {} // Healthy
            }
        }
        
        Ok(())
    }
}
```

**Tasks**:
- [ ] Implementare health monitoring
- [ ] Creare rebalance strategies
- [ ] Aggiungere auto-healing
- [ ] Test: Agente bloccato viene sostituito in <5s

---

## Settimana 4: Integrazione e Polish (Giorni 16-20)

### Giorno 16-17: OpenRouter Integration

**File**: `crates/sentinel-agent-native/src/swarm/llm.rs`

```rust
//! LLM integration for swarm agents

pub struct SwarmLLMClient {
    /// OpenRouter client
    inner: Arc<OpenRouterClient>,
    
    /// Rate limiter (prevent API throttling)
    rate_limiter: RateLimiter,
    
    /// Request queue
    request_queue: Arc<Mutex<Vec<LLMRequest>>>,
}

impl SwarmLLMClient {
    /// Execute LLM call with retries and rate limiting
    pub async fn execute(&self, request: LLMRequest) -> Result<LLMResponse> {
        // Wait for rate limit
        self.rate_limiter.acquire().await;
        
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            match self.inner.chat_completion(&request.system, &request.user).await {
                Ok(response) => return Ok(LLMResponse {
                    content: response.content,
                    tokens: response.token_cost,
                    model: response.llm_name,
                }),
                
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    let backoff = Duration::from_millis(500 * attempts);
                    tracing::warn!("LLM call failed, retrying in {:?}: {}", backoff, e);
                    tokio::time::sleep(backoff).await;
                }
                
                Err(e) => return Err(e.into()),
            }
        }
    }
    
    /// Execute multiple requests in parallel (up to limit)
    pub async fn execute_parallel(&self, requests: Vec<LLMRequest>) -> Vec<Result<LLMResponse>> {
        // Limit concurrent calls to avoid rate limiting
        const MAX_CONCURRENT: usize = 3;
        
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
        
        let futures = requests.into_iter().map(|req| {
            let semaphore = semaphore.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                self.execute(req).await
            }
        });
        
        futures::future::join_all(futures).await
    }
}
```

**Tasks**:
- [ ] Integrare OpenRouterClient
- [ ] Aggiungere rate limiting
- [ ] Implementare retry logic
- [ ] Test: 5 agenti chiamano in parallelo senza rate limit

---

### Giorno 18-19: UI Swarm Panel

**File**: `integrations/vscode/webview-ui/src/components/Swarm/SwarmPanel.tsx`

```typescript
// Visualizzazione swarm in tempo reale

interface SwarmPanelProps {
  agents: Agent[];
  consensus: ConsensusState;
  messages: SwarmMessage[];
}

export function SwarmPanel({ agents, consensus, messages }: SwarmPanelProps) {
  return (
    <div className="swarm-panel">
      {/* Active Agents */}
      <div className="agents-grid">
        {agents.map(agent => (
          <AgentCard 
            key={agent.id}
            agent={agent}
            progress={agent.progress}
            status={agent.status}
          />
        ))}
      </div>
      
      {/* Consensus Visualization */}
      <ConsensusVisualization 
        round={consensus.round}
        proposals={consensus.pending}
      />
      
      {/* Message Stream */}
      <MessageStream messages={messages} />
    </div>
  );
}

function AgentCard({ agent, progress, status }: AgentCardProps) {
  return (
    <div className={`agent-card ${status}`}>
      <div className="agent-icon">{getAgentIcon(agent.type)}</div>
      <div className="agent-info">
        <div className="agent-name">{agent.name}</div>
        <div className="agent-task">{agent.currentTask}</div>
      </div>
      <ProgressBar value={progress} />
      {status === 'working' && <ActivityIndicator />}
    </div>
  );
}
```

**Tasks**:
- [ ] Creare componente AgentCard
- [ ] Aggiungere ConsensusVisualization
- [ ] Implementare MessageStream
- [ ] Test: UI aggiorna in tempo reale (<100ms latency)

---

### Giorno 20: Testing e Finalizzazione

**Test Suite Completa**:

```rust
// tests/swarm_integration_test.rs

#[tokio::test]
async fn test_deterministic_emergence() {
    let goal = "Build auth system with JWT";
    
    // Run 3 times
    for _ in 0..3 {
        let swarm = SwarmCoordinator::from_goal(goal).await.unwrap();
        let agents = swarm.spawn_agents().await.unwrap();
        
        // Same goal → same agents (deterministic)
        assert_eq!(agents.len(), 5);
        assert!(has_agent_type(&agents, AgentType::AuthArchitect));
        assert!(has_agent_type(&agents, AgentType::JWTCoder));
    }
}

#[tokio::test]
async fn test_parallel_execution() {
    let swarm = create_test_swarm().await;
    let start = Instant::now();
    
    // Execute in parallel
    let results = swarm.execute_parallel().await.unwrap();
    
    let elapsed = start.elapsed();
    
    // Should complete in <10s (not sequential 30s)
    assert!(elapsed < Duration::from_secs(10));
    assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_continuous_consensus() {
    let consensus = ContinuousConsensus::new(0.8, 1000);
    
    // Submit proposal
    let proposal = Proposal {
        title: "Use Argon2".to_string(),
        description: "For password hashing".to_string(),
        action: ProposedAction::SelectLibrary("argon2".to_string()),
        impact: ImpactAssessment::default(),
    };
    
    let id = consensus.propose(proposal, AgentId::new()).await.unwrap();
    
    // Agents vote
    for agent in &test_agents {
        let vote = agent.vote(&proposal).await;
        consensus.vote(id, agent.id(), vote).await.unwrap();
    }
    
    // Consensus reached
    tokio::time::sleep(Duration::from_millis(200)).await;
    let state = consensus.get_state(id).await.unwrap();
    assert_eq!(state.status, ProposalStatus::Accepted);
}
```

**Tasks**:
- [ ] Test deterministic emergence
- [ ] Test parallel execution
- [ ] Test consensus
- [ ] Test conflict resolution
- [ ] Documentazione finale

---

## Deliverables Finali

### Codice
1. ✅ SwarmCoordinator - Core orchestrator
2. ✅ EmergenceEngine - Deterministic agent spawning
3. ✅ AgentPersonality - Creative but deterministic
4. ✅ CommunicationBus - Inter-agent messaging
5. ✅ ContinuousConsensus - 100ms consensus loop
6. ✅ SwarmMemory - Shared real-time memory
7. ✅ ConflictResolution - Auto-conflict solving
8. ✅ PredictiveOrchestrator - Pre-spawning
9. ✅ SwarmBalancer - Auto-healing
10. ✅ SwarmLLMClient - OpenRouter integration
11. ✅ SwarmPanel.tsx - Real-time UI

### Documentazione
1. ✅ Architettura completa
2. ✅ API reference
3. ✅ User guide
4. ✅ Determinism proof

### Test
1. ✅ Unit tests (>90% coverage)
2. ✅ Integration tests (E2E)
3. ✅ Determinism tests
4. ✅ Performance benchmarks

---

## Success Criteria

- ✅ Stesso goal → stessi agenti (100% deterministico)
- ✅ 5 agenti eseguono in parallelo in <10s
- ✅ Consensus raggiunto in <500ms
- ✅ Conflitti auto-risolti in <2s
- ✅ UI aggiorna in tempo reale (<100ms)
- ✅ Zero API rate limit violations
- ✅ Auto-healing sostituisce agenti bloccati in <5s

**Questo è SENTINEL SWARM. Game-changer.**
