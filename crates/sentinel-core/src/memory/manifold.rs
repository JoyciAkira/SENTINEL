//! Memory Manifold - Orchestrator for infinite context
//!
//! Unifies working, episodic, and semantic memory into a coherent system.

use super::{
    ConceptNode, ConceptRelation, EpisodicMemory, MemoryItem, MemoryQueryResult, MemorySource,
    SemanticMemory, WorkingMemory,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Memory manifold orchestrator
#[derive(Debug, Clone)]
pub struct MemoryManifold {
    /// Hot cache (10 items, LRU)
    pub working: WorkingMemory,

    /// Vector-based semantic storage (unlimited)
    pub episodic: EpisodicMemory,

    /// Knowledge graph
    pub semantic: SemanticMemory,

    /// Query statistics
    stats: QueryStats,
}

/// Query statistics
#[derive(Debug, Default, Clone)]
struct QueryStats {
    total_queries: u64,
    working_hits: u64,
    episodic_hits: u64,
    semantic_hits: u64,
}

impl MemoryManifold {
    /// Create a new memory manifold
    pub fn new() -> Self {
        Self {
            working: WorkingMemory::new(),
            episodic: EpisodicMemory::new(),
            semantic: SemanticMemory::new(),
            stats: QueryStats::default(),
        }
    }

    /// Store a memory item
    ///
    /// Automatically stores in:
    /// 1. Working memory (hot cache)
    /// 2. Episodic memory (permanent storage)
    /// 3. Optionally updates semantic memory if concepts are detected
    pub fn store(&mut self, item: MemoryItem) {
        // Store in episodic (permanent)
        self.episodic.store(item.clone());

        // Store in working (may evict LRU)
        if let Some(evicted) = self.working.store(item.clone()) {
            // Evicted item is already in episodic, so it's safe
            drop(evicted);
        }

        // Extract concepts and update semantic memory
        self.extract_and_store_concepts(&item);
    }

    /// Query memories with intelligent routing
    ///
    /// Strategy:
    /// 1. Check working memory first (fastest)
    /// 2. Query episodic memory (semantic similarity)
    /// 3. Query semantic memory (conceptual relationships)
    /// 4. Merge and rank results
    pub fn query(&mut self, query: &str, limit: usize) -> Vec<MemoryQueryResult> {
        self.stats.total_queries += 1;

        let mut all_results = Vec::new();

        // 1. Query working memory
        let working_results = self.working.query(query, limit);
        if !working_results.is_empty() {
            self.stats.working_hits += 1;
        }
        all_results.extend(working_results);

        // 2. Query episodic memory
        let episodic_results = self.episodic.query(query, limit);
        if !episodic_results.is_empty() {
            self.stats.episodic_hits += 1;
        }
        all_results.extend(episodic_results);

        // 3. Query semantic memory and retrieve associated memories
        let concept_ids = self.semantic.query(query, 5);
        if !concept_ids.is_empty() {
            self.stats.semantic_hits += 1;

            for concept_id in concept_ids {
                if let Some(concept) = self.semantic.get_concept(&concept_id) {
                    for memory_id in &concept.memory_ids {
                        if let Some(memory) = self.episodic.get(memory_id) {
                            all_results.push(MemoryQueryResult {
                                item: memory.clone(),
                                score: concept.activation * memory.relevance_score(),
                                source: MemorySource::Semantic,
                            });
                        }
                    }
                }
            }
        }

        // Deduplicate and merge results
        let mut seen = HashMap::new();
        for result in all_results {
            let id = result.item.id;
            seen.entry(id)
                .and_modify(|existing: &mut MemoryQueryResult| {
                    // Keep the result with higher score
                    if result.score > existing.score {
                        *existing = result.clone();
                    }
                })
                .or_insert(result);
        }

        // Sort by score and limit
        let mut final_results: Vec<_> = seen.into_values().collect();
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results.truncate(limit);

        final_results
    }

    /// Query by goal ID
    pub fn query_by_goal(&self, goal_id: &Uuid, limit: usize) -> Vec<MemoryQueryResult> {
        self.episodic.query_by_goal(goal_id, limit)
    }

    /// Query by tags
    pub fn query_by_tags(&self, tags: &[String], limit: usize) -> Vec<MemoryQueryResult> {
        self.episodic.query_by_tags(tags, limit)
    }

    /// Get a specific memory by ID
    pub fn get(&mut self, id: &Uuid) -> Option<&mut MemoryItem> {
        // Try working memory first
        if let Some(item) = self.working.get(id) {
            return Some(item);
        }

        // Fall back to episodic
        self.episodic.get(id)
    }

    /// Add a concept to semantic memory
    pub fn add_concept(&mut self, concept: ConceptNode) {
        self.semantic.add_concept(concept);
    }

    /// Add a relationship between concepts
    pub fn add_relation(&mut self, relation: ConceptRelation) {
        self.semantic.add_relation(relation);
    }

    /// Compress episodic memory (merge similar memories)
    pub fn compress(&mut self) -> usize {
        self.episodic.compress()
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            working_size: self.working.len(),
            working_capacity: self.working.capacity(),
            episodic_size: self.episodic.len(),
            semantic_concepts: self.semantic.len(),
            total_queries: self.stats.total_queries,
            working_hit_rate: if self.stats.total_queries > 0 {
                self.stats.working_hits as f64 / self.stats.total_queries as f64
            } else {
                0.0
            },
            episodic_hit_rate: if self.stats.total_queries > 0 {
                self.stats.episodic_hits as f64 / self.stats.total_queries as f64
            } else {
                0.0
            },
            semantic_hit_rate: if self.stats.total_queries > 0 {
                self.stats.semantic_hits as f64 / self.stats.total_queries as f64
            } else {
                0.0
            },
        }
    }

    /// Extract concepts from memory item and update semantic memory
    fn extract_and_store_concepts(&mut self, item: &MemoryItem) {
        // Simple keyword-based concept extraction
        // In production, use NLP/NER for better extraction
        let keywords = self.extract_keywords(&item.content);

        for keyword in keywords {
            // Check if concept already exists
            if let Some(existing) = self.semantic.get_concept_by_name(&keyword) {
                // Need to update - get ID and update via method
                let concept_id = existing.id;
                // We'll add a method to SemanticMemory to update memory_ids
                self.semantic.add_memory_to_concept(&concept_id, item.id);
            } else {
                // Create new concept
                let concept = ConceptNode {
                    id: Uuid::new_v4(),
                    name: keyword.clone(),
                    description: format!("Concept: {}", keyword),
                    memory_ids: vec![item.id],
                    concept_type: super::semantic::ConceptType::General,
                    activation: 1.0,
                };

                self.semantic.add_concept(concept);
            }
        }
    }

    /// Extract keywords from text (simplified)
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        // Simple extraction: words longer than 4 characters
        // In production, use proper NLP
        text.split_whitespace()
            .filter(|word| word.len() > 4)
            .map(|word| {
                word.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Clear all memories
    pub fn clear(&mut self) {
        self.working.clear();
        self.episodic.clear();
        self.semantic.clear();
        self.stats = QueryStats::default();
    }
}

impl Default for MemoryManifold {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Working memory size
    pub working_size: usize,

    /// Working memory capacity
    pub working_capacity: usize,

    /// Episodic memory size
    pub episodic_size: usize,

    /// Number of semantic concepts
    pub semantic_concepts: usize,

    /// Total queries executed
    pub total_queries: u64,

    /// Working memory hit rate
    pub working_hit_rate: f64,

    /// Episodic memory hit rate
    pub episodic_hit_rate: f64,

    /// Semantic memory hit rate
    pub semantic_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryType;

    #[test]
    fn test_manifold_creation() {
        let manifold = MemoryManifold::new();
        let stats = manifold.stats();

        assert_eq!(stats.working_size, 0);
        assert_eq!(stats.episodic_size, 0);
        assert_eq!(stats.semantic_concepts, 0);
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut manifold = MemoryManifold::new();

        let item = MemoryItem::new(
            "Implemented authentication system".to_string(),
            MemoryType::Action,
        );
        let id = item.id;

        manifold.store(item);

        let stats = manifold.stats();
        assert_eq!(stats.working_size, 1);
        assert_eq!(stats.episodic_size, 1);

        let retrieved = manifold.get(&id);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_query_routing() {
        let mut manifold = MemoryManifold::new();

        manifold.store(MemoryItem::new(
            "Implemented user authentication system".to_string(),
            MemoryType::Action,
        ));

        manifold.store(MemoryItem::new(
            "Fixed database connection bug".to_string(),
            MemoryType::Action,
        ));

        let results = manifold.query("authentication", 5);
        assert!(!results.is_empty());

        // Should find authentication-related memory
        assert!(results
            .iter()
            .any(|r| r.item.content.contains("authentication")));
    }

    #[test]
    fn test_working_memory_eviction() {
        let mut manifold = MemoryManifold::new();

        // Store more than working memory capacity (10)
        for i in 0..15 {
            manifold.store(MemoryItem::new(format!("Memory {}", i), MemoryType::Action));
        }

        let stats = manifold.stats();
        assert_eq!(stats.working_size, 10); // Capped at capacity
        assert_eq!(stats.episodic_size, 15); // All stored
    }

    #[test]
    fn test_concept_extraction() {
        let mut manifold = MemoryManifold::new();

        manifold.store(MemoryItem::new(
            "Implemented authentication and authorization system".to_string(),
            MemoryType::Action,
        ));

        let stats = manifold.stats();
        assert!(stats.semantic_concepts > 0); // Should extract concepts
    }

    #[test]
    fn test_query_by_goal() {
        let mut manifold = MemoryManifold::new();
        let goal_id = Uuid::new_v4();

        let item = MemoryItem::builder()
            .content("Related to goal".to_string())
            .memory_type(MemoryType::Action)
            .goal_id(goal_id)
            .build()
            .unwrap();

        manifold.store(item);

        let results = manifold.query_by_goal(&goal_id, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_query_by_tags() {
        let mut manifold = MemoryManifold::new();

        let item = MemoryItem::builder()
            .content("Auth feature".to_string())
            .memory_type(MemoryType::Action)
            .tag("authentication".to_string())
            .build()
            .unwrap();

        manifold.store(item);

        let results = manifold.query_by_tags(&["authentication".to_string()], 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_compression() {
        let mut manifold = MemoryManifold::new();

        // Store similar memories
        for _ in 0..5 {
            manifold.store(MemoryItem::new(
                "Very similar memory content".to_string(),
                MemoryType::Action,
            ));
        }

        let before = manifold.stats().episodic_size;
        let merged = manifold.compress();
        let after = manifold.stats().episodic_size;

        assert!(merged > 0);
        assert!(after < before);
    }

    #[test]
    fn test_stats_tracking() {
        let mut manifold = MemoryManifold::new();

        manifold.store(MemoryItem::new("Test".to_string(), MemoryType::Action));

        // Execute queries
        manifold.query("test", 5);
        manifold.query("test", 5);

        let stats = manifold.stats();
        assert_eq!(stats.total_queries, 2);
        assert!(stats.working_hit_rate > 0.0);
    }

    #[test]
    fn test_deduplication() {
        let mut manifold = MemoryManifold::new();

        let item = MemoryItem::new(
            "Test memory in both working and episodic".to_string(),
            MemoryType::Action,
        );

        manifold.store(item);

        // Query should deduplicate results from working and episodic
        let results = manifold.query("memory", 10);

        // Should not have duplicates
        let ids: Vec<_> = results.iter().map(|r| r.item.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len());
    }
}
