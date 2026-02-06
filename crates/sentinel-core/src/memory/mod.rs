//! # Memory Manifold (Layer 4)
//!
//! Infinite context system for AI coding agents.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │         Memory Manifold (Orchestrator)          │
//! │  - Query routing & fusion                       │
//! │  - Compression & consolidation                  │
//! │  - Relevance scoring                            │
//! └─────────────────────────────────────────────────┘
//!          │              │              │
//!    ┌─────┴─────┐  ┌─────┴─────┐  ┌────┴─────┐
//!    │  Working  │  │ Episodic  │  │ Semantic │
//!    │  Memory   │  │  Memory   │  │  Memory  │
//!    │ (10 LRU)  │  │ (Vectors) │  │  (Graph) │
//!    └───────────┘  └───────────┘  └──────────┘
//! ```
//!
//! ## Design Principles
//!
//! 1. **Hierarchical Storage**: Hot → Warm → Cold
//! 2. **Automatic Compression**: Merge similar memories
//! 3. **Semantic Retrieval**: Find by meaning, not just keywords
//! 4. **Forgetting Curve**: Decay unused memories gracefully
//!
//! ## Example
//!
//! ```rust
//! use sentinel_core::memory::{MemoryManifold, MemoryItem, MemoryType};
//! use uuid::Uuid;
//!
//! let mut manifold = MemoryManifold::new();
//!
//! // Store a memory
//! let item = MemoryItem::new(
//!     "Implemented authentication system".to_string(),
//!     MemoryType::Action,
//! );
//! manifold.store(item);
//!
//! // Query by semantic similarity
//! let results = manifold.query("How did we handle user login?", 5);
//! ```

pub mod embeddings;
pub mod episodic;
pub mod manifold;
pub mod semantic;
pub mod working;

pub use embeddings::Embedder;
pub use episodic::{EpisodicMemory, MemoryEmbedding};
pub use manifold::MemoryManifold;
pub use semantic::{ConceptNode, ConceptRelation, SemanticMemory};
pub use working::WorkingMemory;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A memory item stored in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique identifier
    pub id: Uuid,

    /// Memory content
    pub content: String,

    /// Type of memory
    pub memory_type: MemoryType,

    /// When the memory was created
    pub created_at: DateTime<Utc>,

    /// When the memory was last accessed
    pub last_accessed: DateTime<Utc>,

    /// How many times this memory has been accessed
    pub access_count: u32,

    /// Importance score (0.0-1.0)
    pub importance: f64,

    /// Confidence score for factual reliability (0.0-1.0)
    pub confidence: f64,

    /// Provenance of this memory
    pub origin: MemoryOrigin,

    /// Expiration timestamp for stale memory control
    pub expires_at: Option<DateTime<Utc>>,

    /// Last explicit verification timestamp
    pub last_verified_at: Option<DateTime<Utc>>,

    /// Associated goal IDs
    pub goal_ids: Vec<Uuid>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Optional metadata
    pub metadata: serde_json::Value,
}

/// Types of memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// An action taken by the agent
    Action,

    /// A decision made
    Decision,

    /// An observation about the environment
    Observation,

    /// A learned pattern or insight
    Insight,

    /// An error or failure
    Error,

    /// A successful outcome
    Success,

    /// A conversation or interaction
    Conversation,

    /// Code-related memory
    Code,
}

/// Provenance of a memory entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryOrigin {
    /// User-provided information
    UserInput,
    /// Derived from agent execution
    AgentExecution,
    /// Derived from tool output
    ToolOutput,
    /// Imported from external synchronization
    ExternalSync,
    /// Internally inferred or synthesized
    SystemDerived,
}

impl MemoryItem {
    /// Create a new memory item
    pub fn new(content: String, memory_type: MemoryType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content,
            memory_type,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            importance: 0.5, // Default medium importance
            confidence: 0.5,
            origin: MemoryOrigin::SystemDerived,
            expires_at: None,
            last_verified_at: Some(now),
            goal_ids: Vec::new(),
            tags: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a builder for memory items
    pub fn builder() -> MemoryItemBuilder {
        MemoryItemBuilder::default()
    }

    /// Mark this memory as accessed
    pub fn access(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }

    /// Mark memory as verified now and optionally adjust confidence.
    pub fn mark_verified(&mut self, confidence: Option<f64>) {
        self.last_verified_at = Some(Utc::now());
        if let Some(value) = confidence {
            self.confidence = value.clamp(0.0, 1.0);
        }
    }

    /// Check if memory is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expiry| Utc::now() > expiry)
            .unwrap_or(false)
    }

    /// Memory can be used in retrieval and decision support.
    pub fn is_active(&self) -> bool {
        !self.is_expired()
    }

    /// Score freshness from verification recency.
    pub fn verification_freshness_score(&self) -> f64 {
        let Some(last_verified) = self.last_verified_at else {
            return 0.3;
        };
        let age_seconds = (Utc::now() - last_verified).num_seconds() as f64;
        let half_life = 7.0 * 86400.0; // 7 days
        (-age_seconds / half_life).exp().clamp(0.0, 1.0)
    }

    /// Trust score combines confidence and verification freshness.
    pub fn trust_score(&self) -> f64 {
        let freshness = self.verification_freshness_score();
        (0.7 * self.confidence + 0.3 * freshness).clamp(0.0, 1.0)
    }

    /// Calculate recency score (0.0-1.0, higher = more recent)
    pub fn recency_score(&self) -> f64 {
        let age_seconds = (Utc::now() - self.last_accessed).num_seconds() as f64;
        let half_life = 86400.0; // 24 hours
        (-age_seconds / half_life).exp()
    }

    /// Calculate frequency score (0.0-1.0, higher = more accessed)
    pub fn frequency_score(&self) -> f64 {
        // Logarithmic scaling to prevent dominance of very frequent items
        (1.0 + self.access_count as f64).ln() / 10.0
    }

    /// Calculate overall relevance score
    pub fn relevance_score(&self) -> f64 {
        if self.is_expired() {
            return 0.0;
        }
        // Weighted combination of importance, recency, and frequency
        let base =
            0.4 * self.importance + 0.25 * self.recency_score() + 0.15 * self.frequency_score();
        let trust = 0.2 * self.trust_score();
        (base + trust).clamp(0.0, 1.0)
    }
}

/// Builder for memory items
#[derive(Default)]
pub struct MemoryItemBuilder {
    content: Option<String>,
    memory_type: Option<MemoryType>,
    importance: Option<f64>,
    confidence: Option<f64>,
    origin: Option<MemoryOrigin>,
    expires_at: Option<DateTime<Utc>>,
    last_verified_at: Option<DateTime<Utc>>,
    goal_ids: Vec<Uuid>,
    tags: Vec<String>,
    metadata: Option<serde_json::Value>,
}

impl MemoryItemBuilder {
    /// Set the content
    pub fn content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the memory type
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }

    /// Set the importance
    pub fn importance(mut self, importance: f64) -> Self {
        self.importance = Some(importance.clamp(0.0, 1.0));
        self
    }

    /// Set confidence
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Set memory origin
    pub fn origin(mut self, origin: MemoryOrigin) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Set expiration absolute timestamp
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set expiration duration from now
    pub fn expires_in(mut self, ttl: Duration) -> Self {
        self.expires_at = Some(Utc::now() + ttl);
        self
    }

    /// Set last verified timestamp
    pub fn last_verified_at(mut self, verified_at: DateTime<Utc>) -> Self {
        self.last_verified_at = Some(verified_at);
        self
    }

    /// Add a goal ID
    pub fn goal_id(mut self, goal_id: Uuid) -> Self {
        self.goal_ids.push(goal_id);
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the memory item
    pub fn build(self) -> Result<MemoryItem, String> {
        let content = self.content.ok_or("content is required")?;
        let memory_type = self.memory_type.ok_or("memory_type is required")?;

        let now = Utc::now();
        Ok(MemoryItem {
            id: Uuid::new_v4(),
            content,
            memory_type,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            importance: self.importance.unwrap_or(0.5),
            confidence: self.confidence.unwrap_or(0.5),
            origin: self.origin.unwrap_or(MemoryOrigin::SystemDerived),
            expires_at: self.expires_at,
            last_verified_at: self.last_verified_at.or(Some(now)),
            goal_ids: self.goal_ids,
            tags: self.tags,
            metadata: self.metadata.unwrap_or(serde_json::Value::Null),
        })
    }
}

/// Query result with relevance score
#[derive(Debug, Clone)]
pub struct MemoryQueryResult {
    /// The memory item
    pub item: MemoryItem,

    /// Relevance score (0.0-1.0)
    pub score: f64,

    /// Source of the memory
    pub source: MemorySource,
}

/// Source of a memory
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySource {
    /// From working memory (hot cache)
    Working,

    /// From episodic memory (vector store)
    Episodic,

    /// From semantic memory (knowledge graph)
    Semantic,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_item_creation() {
        let item = MemoryItem::new("Test memory".to_string(), MemoryType::Action);
        assert_eq!(item.content, "Test memory");
        assert_eq!(item.memory_type, MemoryType::Action);
        assert_eq!(item.access_count, 0);
        assert_eq!(item.importance, 0.5);
        assert_eq!(item.confidence, 0.5);
        assert_eq!(item.origin, MemoryOrigin::SystemDerived);
        assert!(item.is_active());
    }

    #[test]
    fn test_memory_item_expiration() {
        let mut item = MemoryItem::new("Expiring memory".to_string(), MemoryType::Observation);
        item.expires_at = Some(Utc::now() - Duration::seconds(1));
        assert!(item.is_expired());
        assert!(!item.is_active());
        assert_eq!(item.relevance_score(), 0.0);
    }

    #[test]
    fn test_memory_item_trust_score() {
        let mut item = MemoryItem::new("Trusted memory".to_string(), MemoryType::Insight);
        item.confidence = 0.9;
        item.last_verified_at = Some(Utc::now());
        assert!(item.trust_score() > 0.8);
    }

    #[test]
    fn test_memory_item_builder() {
        let goal_id = Uuid::new_v4();
        let item = MemoryItem::builder()
            .content("Built feature".to_string())
            .memory_type(MemoryType::Success)
            .importance(0.9)
            .goal_id(goal_id)
            .tag("authentication".to_string())
            .build()
            .unwrap();

        assert_eq!(item.content, "Built feature");
        assert_eq!(item.importance, 0.9);
        assert_eq!(item.goal_ids.len(), 1);
        assert_eq!(item.tags.len(), 1);
    }

    #[test]
    fn test_memory_access() {
        let mut item = MemoryItem::new("Test".to_string(), MemoryType::Observation);
        let initial_time = item.last_accessed;

        std::thread::sleep(std::time::Duration::from_millis(10));
        item.access();

        assert_eq!(item.access_count, 1);
        assert!(item.last_accessed > initial_time);
    }

    #[test]
    fn test_recency_score() {
        let item = MemoryItem::new("Recent".to_string(), MemoryType::Action);
        let score = item.recency_score();

        // Fresh memory should have high recency
        assert!(score > 0.9);
    }

    #[test]
    fn test_frequency_score() {
        let mut item = MemoryItem::new("Frequent".to_string(), MemoryType::Action);

        // Access multiple times
        for _ in 0..10 {
            item.access();
        }

        let score = item.frequency_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_relevance_score() {
        let mut item = MemoryItem::new("Important".to_string(), MemoryType::Insight);
        item.importance = 0.9;
        item.access_count = 5;

        let score = item.relevance_score();
        assert!(score > 0.0 && score <= 1.0);
    }
}
