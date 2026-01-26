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

use chrono::{DateTime, Utc};
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
        // Weighted combination of importance, recency, and frequency
        0.5 * self.importance + 0.3 * self.recency_score() + 0.2 * self.frequency_score()
    }
}

/// Builder for memory items
#[derive(Default)]
pub struct MemoryItemBuilder {
    content: Option<String>,
    memory_type: Option<MemoryType>,
    importance: Option<f64>,
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
