//! Distributed Cognitive Consciousness (DCC)
//!
//! Shared memory system enabling agents to access collective knowledge.
//! Eliminates cognitive isolation and enables instant context sharing.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Distributed Cognitive Consciousness            │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │              Shared Memory Layers                   │   │
//! │  ├─────────────────┬─────────────────┬────────────────┤   │
//! │  │ Working Memory  │ Episodic Memory │ Semantic Mem   │   │
//! │  │ (Current Focus) │ (Event History) │ (Patterns)     │   │
//! │  │ - Active goals  │ - Decisions     │ - Best practices│   │
//! │  │ - Context       │ - Outcomes      │ - Anti-patterns│   │
//! │  │ - State         │ - Learnings     │ - Templates    │   │
//! │  └─────────────────┴─────────────────┴────────────────┘   │
//! │                                                             │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐             │
//! │  │ Agent A  │◄──►│ Agent B  │◄──►│ Agent C  │             │
//! │  │(Auth)    │    │(API)     │    │(UI)      │             │
//! │  └──────────┘    └──────────┘    └──────────┘             │
//! │       │                │                │                  │
//! │       └────────────────┼────────────────┘                  │
//! │                        ▼                                   │
//! │              ┌──────────────────┐                         │
//! │              │ Memory Access Bus│                         │
//! │              │ (Read/Write/Query│                         │
//! │              └──────────────────┘                         │
//! │                                                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use crate::error::Result;
use crate::outcome_compiler::agent_communication::AgentId;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Distributed memory store shared across all agents
pub struct DistributedMemory {
    /// Working memory - current focus and context
    working: Arc<RwLock<WorkingMemory>>,
    
    /// Episodic memory - history of events and decisions
    episodic: Arc<RwLock<EpisodicMemory>>,
    
    /// Semantic memory - learned patterns and knowledge
    semantic: Arc<RwLock<SemanticMemory>>,
    
    /// Access statistics for optimization
    stats: Arc<RwLock<MemoryStats>>,
}

/// Working memory - ultra-fast, small capacity, current focus
/// Similar to human working memory (7±2 items)
#[derive(Debug, Clone)]
pub struct WorkingMemory {
    /// Currently active goals
    pub active_goals: Vec<ActiveGoal>,
    
    /// Current context snapshot
    pub context: ContextSnapshot,
    
    /// Recent actions (last 10)
    pub recent_actions: Vec<ActionRecord>,
    
    /// Shared state variables
    pub shared_state: HashMap<String, serde_json::Value>,
    
    /// Attention focus
    pub attention: AttentionFocus,
}

/// An active goal in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveGoal {
    pub goal_id: String,
    pub description: String,
    pub priority: i32,
    pub progress: f64,
    pub owner: Option<AgentId>,
}

/// Context snapshot for quick access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub project_name: String,
    pub current_phase: String,
    pub tech_stack: Vec<String>,
    pub active_constraints: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Record of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub action_id: String,
    pub agent_id: AgentId,
    pub description: String,
    pub result: ActionResult,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionResult {
    Success,
    PartialSuccess,
    Failure,
    Blocked,
}

/// Current attention focus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionFocus {
    pub focused_on: String,
    pub focus_level: f64,  // 0.0-1.0
    pub context_ids: Vec<String>,
}

/// Episodic memory - event history with embeddings
#[derive(Debug, Clone)]
pub struct EpisodicMemory {
    /// All recorded episodes
    episodes: Vec<Episode>,
    
    /// Index by agent
    by_agent: HashMap<String, Vec<usize>>,
    
    /// Index by type
    by_type: HashMap<EpisodeType, Vec<usize>>,
}

/// An episode in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub episode_id: String,
    pub agent_id: AgentId,
    pub episode_type: EpisodeType,
    pub content: EpisodeContent,
    pub outcome: Outcome,
    pub learnings: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub embedding: Option<Vec<f32>>,  // For semantic search
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpisodeType {
    Decision,
    Implementation,
    Validation,
    Communication,
    Learning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeContent {
    Decision {
        context: String,
        options: Vec<String>,
        chosen: String,
        reasoning: String,
    },
    Implementation {
        task: String,
        approach: String,
        code_snippet: String,
    },
    Validation {
        proposal: String,
        validators: Vec<String>,
        result: String,
    },
    Communication {
        message_type: String,
        with_agent: Option<String>,
        content_summary: String,
    },
    Learning {
        pattern: String,
        applicability: Vec<String>,
        confidence: f64,
    },
    Error {
        error_type: String,
        error_message: String,
        resolution: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub metrics: HashMap<String, f64>,
    pub side_effects: Vec<String>,
}

/// Semantic memory - learned patterns and concepts
#[derive(Debug, Clone)]
pub struct SemanticMemory {
    /// Learned patterns
    patterns: HashMap<String, LearnedPattern>,
    
    /// Concept definitions
    concepts: HashMap<String, Concept>,
    
    /// Relationships between concepts
    relationships: Vec<Relationship>,
    
    /// Best practices by domain
    best_practices: HashMap<String, Vec<BestPractice>>,
}

/// A learned pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub pattern_id: String,
    pub name: String,
    pub description: String,
    pub applicability: Vec<String>,
    pub code_template: Option<String>,
    pub success_rate: f64,
    pub usage_count: usize,
    pub learned_from: Vec<String>, // Episode IDs
}

/// A concept in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept_id: String,
    pub name: String,
    pub definition: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub related_concepts: Vec<String>,
}

/// Relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub relationship_type: RelationshipType,
    pub strength: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    DependsOn,
    Implements,
    Uses,
    Extends,
    Contradicts,
}

/// Best practice for a domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestPractice {
    pub practice_id: String,
    pub title: String,
    pub description: String,
    pub rationale: String,
    pub examples: Vec<String>,
    pub violations: Vec<String>,
}

/// Statistics for memory access
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub working_memory_hits: usize,
    pub episodic_memory_queries: usize,
    pub semantic_memory_lookups: usize,
    pub cache_hit_rate: f64,
}

/// Query for memory search
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub query_type: QueryType,
    pub agent_id: Option<AgentId>,
    pub time_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
    pub keywords: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    RecentEpisodes,
    SimilarPatterns,
    RelatedDecisions,
    LearnedLessons,
    ActiveContext,
}

/// Result of memory query
#[derive(Debug, Clone)]
pub struct MemoryQueryResult {
    pub working_memory: Option<WorkingMemory>,
    pub relevant_episodes: Vec<Episode>,
    pub matching_patterns: Vec<LearnedPattern>,
    pub related_concepts: Vec<Concept>,
}

impl DistributedMemory {
    /// Create new distributed memory
    pub fn new() -> Self {
        Self {
            working: Arc::new(RwLock::new(WorkingMemory {
                active_goals: Vec::new(),
                context: ContextSnapshot {
                    project_name: String::new(),
                    current_phase: String::new(),
                    tech_stack: Vec::new(),
                    active_constraints: Vec::new(),
                    timestamp: chrono::Utc::now(),
                },
                recent_actions: Vec::new(),
                shared_state: HashMap::new(),
                attention: AttentionFocus {
                    focused_on: String::new(),
                    focus_level: 0.0,
                    context_ids: Vec::new(),
                },
            })),
            episodic: Arc::new(RwLock::new(EpisodicMemory {
                episodes: Vec::new(),
                by_agent: HashMap::new(),
                by_type: HashMap::new(),
            })),
            semantic: Arc::new(RwLock::new(SemanticMemory {
                patterns: HashMap::new(),
                concepts: HashMap::new(),
                relationships: Vec::new(),
                best_practices: HashMap::new(),
            })),
            stats: Arc::new(RwLock::new(MemoryStats::default())),
        }
    }
    
    /// Working Memory Operations
    
    /// Get current working memory (instant access)
    pub async fn get_working_memory(&self) -> WorkingMemory {
        self.working.read().await.clone()
    }
    
    /// Update working memory
    pub async fn update_working_memory<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut WorkingMemory),
    {
        let mut working = self.working.write().await;
        updater(&mut working);
        
        let mut stats = self.stats.write().await;
        stats.working_memory_hits += 1;
        
        Ok(())
    }
    
    /// Add active goal
    pub async fn add_active_goal(&self, goal: ActiveGoal) -> Result<()> {
        self.update_working_memory(|working| {
            // Keep only top 7 goals (working memory limit)
            working.active_goals.push(goal);
            working.active_goals.sort_by(|a, b| b.priority.cmp(&a.priority));
            if working.active_goals.len() > 7 {
                working.active_goals.truncate(7);
            }
        }).await
    }
    
    /// Record action
    pub async fn record_action(&self, action: ActionRecord) -> Result<()> {
        self.update_working_memory(|working| {
            working.recent_actions.push(action);
            if working.recent_actions.len() > 10 {
                working.recent_actions.remove(0);
            }
        }).await
    }
    
    /// Set shared state
    pub async fn set_shared_state(&self, key: impl Into<String>, value: serde_json::Value) -> Result<()> {
        let key = key.into();
        self.update_working_memory(|working| {
            working.shared_state.insert(key, value);
        }).await
    }
    
    /// Get shared state
    pub async fn get_shared_state(&self, key: &str) -> Option<serde_json::Value> {
        let working = self.working.read().await;
        working.shared_state.get(key).cloned()
    }
    
    /// Episodic Memory Operations
    
    /// Record an episode
    pub async fn record_episode(&self, episode: Episode) -> Result<()> {
        let mut episodic = self.episodic.write().await;
        let index = episodic.episodes.len();
        
        // Index by agent
        episodic.by_agent
            .entry(episode.agent_id.0.clone())
            .or_default()
            .push(index);
        
        // Index by type
        episodic.by_type
            .entry(episode.episode_type)
            .or_default()
            .push(index);
        
        episodic.episodes.push(episode);
        
        let mut stats = self.stats.write().await;
        stats.episodic_memory_queries += 1;
        
        Ok(())
    }
    
    /// Query episodic memory
    pub async fn query_episodes(&self, query: &MemoryQuery) -> Vec<Episode> {
        let episodic = self.episodic.read().await;
        
        let indices: Vec<usize> = match query.query_type {
            QueryType::RecentEpisodes => {
                // Get most recent episodes
                let mut indices: Vec<_> = (0..episodic.episodes.len()).collect();
                indices.reverse(); // Most recent first
                indices
            }
            QueryType::SimilarPatterns => {
                // Search by keywords
                episodic.episodes.iter()
                    .enumerate()
                    .filter(|(_, e)| {
                        query.keywords.iter().any(|kw| {
                            let content = format!("{:?}", e.content).to_lowercase();
                            content.contains(&kw.to_lowercase())
                        })
                    })
                    .map(|(i, _)| i)
                    .collect()
            }
            _ => Vec::new(),
        };
        
        // Filter by agent if specified
        let filtered: Vec<_> = if let Some(ref agent_id) = query.agent_id {
            indices.into_iter()
                .filter(|i| episodic.episodes[*i].agent_id == *agent_id)
                .collect()
        } else {
            indices
        };
        
        // Return episodes
        filtered.iter()
            .take(query.limit)
            .filter_map(|i| episodic.episodes.get(*i))
            .cloned()
            .collect()
    }
    
    /// Semantic Memory Operations
    
    /// Learn a pattern
    pub async fn learn_pattern(&self, pattern: LearnedPattern) -> Result<()> {
        let mut semantic = self.semantic.write().await;
        semantic.patterns.insert(pattern.pattern_id.clone(), pattern);
        
        let mut stats = self.stats.write().await;
        stats.semantic_memory_lookups += 1;
        
        Ok(())
    }
    
    /// Find applicable patterns
    pub async fn find_patterns(&self, context: &str) -> Vec<LearnedPattern> {
        let semantic = self.semantic.read().await;
        
        semantic.patterns.values()
            .filter(|p| {
                // Check applicability
                p.applicability.iter().any(|app| {
                    context.to_lowercase().contains(&app.to_lowercase())
                })
            })
            .sorted_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap())
            .take(5)
            .cloned()
            .collect()
    }
    
    /// Add concept
    pub async fn add_concept(&self, concept: Concept) -> Result<()> {
        let mut semantic = self.semantic.write().await;
        semantic.concepts.insert(concept.concept_id.clone(), concept);
        Ok(())
    }
    
    /// Get concept
    pub async fn get_concept(&self, concept_id: &str) -> Option<Concept> {
        let semantic = self.semantic.read().await;
        semantic.concepts.get(concept_id).cloned()
    }
    
    /// Add relationship
    pub async fn add_relationship(&self, relationship: Relationship) -> Result<()> {
        let mut semantic = self.semantic.write().await;
        semantic.relationships.push(relationship);
        Ok(())
    }
    
    /// Query related concepts
    pub async fn get_related_concepts(&self, concept_id: &str) -> Vec<(Concept, RelationshipType)> {
        let semantic = self.semantic.read().await;
        
        semantic.relationships.iter()
            .filter(|r| r.from == concept_id || r.to == concept_id)
            .filter_map(|r| {
                let related_id = if r.from == concept_id { &r.to } else { &r.from };
                semantic.concepts.get(related_id)
                    .map(|c| (c.clone(), r.relationship_type))
            })
            .collect()
    }
    
    /// Add best practice
    pub async fn add_best_practice(&self, domain: impl Into<String>, practice: BestPractice) -> Result<()> {
        let mut semantic = self.semantic.write().await;
        let domain = domain.into();
        semantic.best_practices
            .entry(domain)
            .or_default()
            .push(practice);
        Ok(())
    }
    
    /// Get best practices for domain
    pub async fn get_best_practices(&self, domain: &str) -> Vec<BestPractice> {
        let semantic = self.semantic.read().await;
        semantic.best_practices
            .get(domain)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Combined Query
    
    /// Query all memory layers
    pub async fn query(&self, query: &MemoryQuery) -> MemoryQueryResult {
        MemoryQueryResult {
            working_memory: Some(self.get_working_memory().await),
            relevant_episodes: self.query_episodes(query).await,
            matching_patterns: if !query.keywords.is_empty() {
                self.find_patterns(&query.keywords.join(" ")).await
            } else {
                Vec::new()
            },
            related_concepts: Vec::new(), // TODO: Semantic search
        }
    }
    
    /// Statistics
    
    /// Get memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        self.stats.read().await.clone()
    }
    
    /// Get memory summary
    pub async fn get_summary(&self) -> MemorySummary {
        let working = self.working.read().await;
        let episodic = self.episodic.read().await;
        let semantic = self.semantic.read().await;
        
        MemorySummary {
            active_goals: working.active_goals.len(),
            total_episodes: episodic.episodes.len(),
            unique_agents: episodic.by_agent.len(),
            learned_patterns: semantic.patterns.len(),
            known_concepts: semantic.concepts.len(),
            best_practice_domains: semantic.best_practices.len(),
        }
    }
}

/// Summary of memory contents
#[derive(Debug, Clone)]
pub struct MemorySummary {
    pub active_goals: usize,
    pub total_episodes: usize,
    pub unique_agents: usize,
    pub learned_patterns: usize,
    pub known_concepts: usize,
    pub best_practice_domains: usize,
}

/// Agent interface to distributed memory
pub struct AgentMemoryHandle {
    agent_id: AgentId,
    memory: Arc<DistributedMemory>,
}

impl AgentMemoryHandle {
    /// Create handle for agent
    pub fn new(agent_id: AgentId, memory: Arc<DistributedMemory>) -> Self {
        Self { agent_id, memory }
    }
    
    /// Access working memory
    pub async fn get_context(&self) -> WorkingMemory {
        self.memory.get_working_memory().await
    }
    
    /// Record episode from this agent
    pub async fn record(&self, episode_type: EpisodeType, content: EpisodeContent, outcome: Outcome) -> Result<()> {
        let episode = Episode {
            episode_id: Uuid::new_v4().to_string(),
            agent_id: self.agent_id.clone(),
            episode_type,
            content,
            outcome,
            learnings: Vec::new(),
            timestamp: chrono::Utc::now(),
            embedding: None,
        };
        
        self.memory.record_episode(episode).await
    }
    
    /// Learn from experience
    pub async fn learn(&self, pattern: LearnedPattern) -> Result<()> {
        self.memory.learn_pattern(pattern).await
    }
    
    /// Recall similar experiences
    pub async fn recall(&self, keywords: Vec<String>, limit: usize) -> Vec<Episode> {
        let query = MemoryQuery {
            query_type: QueryType::SimilarPatterns,
            agent_id: Some(self.agent_id.clone()),
            time_range: None,
            keywords,
            limit,
        };
        
        self.memory.query_episodes(&query).await
    }
    
    /// Query full memory
    pub async fn query(&self, query: &MemoryQuery) -> MemoryQueryResult {
        self.memory.query(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_distributed_memory_creation() {
        let memory = DistributedMemory::new();
        let working = memory.get_working_memory().await;
        assert!(working.active_goals.is_empty());
    }
    
    #[tokio::test]
    async fn test_working_memory_operations() {
        let memory = DistributedMemory::new();
        
        // Add goal
        memory.add_active_goal(ActiveGoal {
            goal_id: "test-1".to_string(),
            description: "Test goal".to_string(),
            priority: 10,
            progress: 0.0,
            owner: None,
        }).await.unwrap();
        
        let working = memory.get_working_memory().await;
        assert_eq!(working.active_goals.len(), 1);
        
        // Set shared state
        memory.set_shared_state("key", serde_json::json!("value")).await.unwrap();
        let value = memory.get_shared_state("key").await;
        assert!(value.is_some());
    }
    
    #[tokio::test]
    async fn test_episodic_memory() {
        let memory = DistributedMemory::new();
        let agent_id = AgentId::new();
        
        let episode = Episode {
            episode_id: "ep-1".to_string(),
            agent_id: agent_id.clone(),
            episode_type: EpisodeType::Decision,
            content: EpisodeContent::Decision {
                context: "Test".to_string(),
                options: vec!["A".to_string()],
                chosen: "A".to_string(),
                reasoning: "Test".to_string(),
            },
            outcome: Outcome {
                success: true,
                metrics: HashMap::new(),
                side_effects: Vec::new(),
            },
            learnings: Vec::new(),
            timestamp: chrono::Utc::now(),
            embedding: None,
        };
        
        memory.record_episode(episode).await.unwrap();
        
        let query = MemoryQuery {
            query_type: QueryType::RecentEpisodes,
            agent_id: Some(agent_id),
            time_range: None,
            keywords: Vec::new(),
            limit: 10,
        };
        
        let episodes = memory.query_episodes(&query).await;
        assert_eq!(episodes.len(), 1);
    }
    
    #[tokio::test]
    async fn test_semantic_memory() {
        let memory = DistributedMemory::new();
        
        let pattern = LearnedPattern {
            pattern_id: "pat-1".to_string(),
            name: "Auth Pattern".to_string(),
            description: "Best auth approach".to_string(),
            applicability: vec!["auth".to_string()],
            code_template: Some("code".to_string()),
            success_rate: 0.95,
            usage_count: 10,
            learned_from: Vec::new(),
        };
        
        memory.learn_pattern(pattern).await.unwrap();
        
        let patterns = memory.find_patterns("auth system").await;
        assert_eq!(patterns.len(), 1);
    }
    
    #[test]
    fn test_memory_query() {
        let query = MemoryQuery {
            query_type: QueryType::RecentEpisodes,
            agent_id: None,
            time_range: None,
            keywords: vec!["auth".to_string()],
            limit: 5,
        };
        
        assert_eq!(query.keywords.len(), 1);
    }
}
