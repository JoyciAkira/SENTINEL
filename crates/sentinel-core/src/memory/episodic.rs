//! Episodic Memory - Vector-based semantic storage
//!
//! Stores unlimited memories with semantic similarity search.

use super::{MemoryItem, MemoryQueryResult, MemorySource};
use std::collections::HashMap;
use uuid::Uuid;

/// Vector embedding for semantic similarity
#[derive(Debug, Clone)]
pub struct MemoryEmbedding {
    /// The memory item
    pub item: MemoryItem,

    /// Vector embedding (simplified - in production use real embeddings)
    pub embedding: Vec<f32>,
}

impl MemoryEmbedding {
    /// Create a new memory embedding
    pub fn new(item: MemoryItem) -> Self {
        let embedding = Self::compute_embedding(&item.content);
        Self { item, embedding }
    }

    /// Compute embedding from text (simplified - use real model in production)
    fn compute_embedding(text: &str) -> Vec<f32> {
        // Simplified: hash-based pseudo-embedding (768 dimensions like BERT)
        // In production, use sentence-transformers or similar
        let mut embedding = vec![0.0; 768];

        // Simple character frequency-based features
        for (i, ch) in text.chars().enumerate() {
            let idx = (ch as usize + i) % 768;
            embedding[idx] += 1.0;
        }

        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        embedding
    }

    /// Compute cosine similarity with another embedding
    pub fn similarity(&self, other: &MemoryEmbedding) -> f32 {
        self.embedding
            .iter()
            .zip(other.embedding.iter())
            .map(|(a, b)| a * b)
            .sum()
    }
}

/// Episodic memory with vector-based retrieval
#[derive(Debug)]
pub struct EpisodicMemory {
    /// All stored memories with embeddings
    memories: HashMap<Uuid, MemoryEmbedding>,

    /// Compression threshold (merge similar memories above this)
    compression_threshold: f32,
}

impl EpisodicMemory {
    /// Create a new episodic memory
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
            compression_threshold: 0.95, // Very high similarity = merge
        }
    }

    /// Create with custom compression threshold
    pub fn with_compression_threshold(threshold: f32) -> Self {
        Self {
            memories: HashMap::new(),
            compression_threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Store a memory
    pub fn store(&mut self, item: MemoryItem) {
        let embedding = MemoryEmbedding::new(item);
        let id = embedding.item.id;
        self.memories.insert(id, embedding);
    }

    /// Get a memory by ID
    pub fn get(&mut self, id: &Uuid) -> Option<&mut MemoryItem> {
        self.memories.get_mut(id).map(|emb| {
            emb.item.access();
            &mut emb.item
        })
    }

    /// Query by semantic similarity
    pub fn query(&mut self, query: &str, limit: usize) -> Vec<MemoryQueryResult> {
        // Create embedding for query
        let query_item = MemoryItem::new(query.to_string(), super::MemoryType::Observation);
        let query_embedding = MemoryEmbedding::new(query_item);

        // Compute similarities
        let mut results: Vec<_> = self
            .memories
            .values_mut()
            .map(|emb| {
                let similarity = query_embedding.similarity(emb);
                emb.item.access();

                MemoryQueryResult {
                    item: emb.item.clone(),
                    score: similarity as f64 * emb.item.relevance_score(),
                    source: MemorySource::Episodic,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        results
    }

    /// Query by goal ID
    pub fn query_by_goal(&self, goal_id: &Uuid, limit: usize) -> Vec<MemoryQueryResult> {
        let mut results: Vec<_> = self
            .memories
            .values()
            .filter(|emb| emb.item.goal_ids.contains(goal_id))
            .map(|emb| MemoryQueryResult {
                item: emb.item.clone(),
                score: emb.item.relevance_score(),
                source: MemorySource::Episodic,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        results
    }

    /// Query by tags
    pub fn query_by_tags(&self, tags: &[String], limit: usize) -> Vec<MemoryQueryResult> {
        let mut results: Vec<_> = self
            .memories
            .values()
            .filter(|emb| tags.iter().any(|tag| emb.item.tags.contains(tag)))
            .map(|emb| MemoryQueryResult {
                item: emb.item.clone(),
                score: emb.item.relevance_score(),
                source: MemorySource::Episodic,
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        results
    }

    /// Compress similar memories (merge very similar ones)
    pub fn compress(&mut self) -> usize {
        let mut to_merge: Vec<(Uuid, Uuid)> = Vec::new();

        // Find pairs with high similarity
        let ids: Vec<Uuid> = self.memories.keys().copied().collect();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                if let (Some(emb1), Some(emb2)) =
                    (self.memories.get(&ids[i]), self.memories.get(&ids[j]))
                {
                    let similarity = emb1.similarity(emb2);
                    if similarity >= self.compression_threshold {
                        to_merge.push((ids[i], ids[j]));
                    }
                }
            }
        }

        // Merge memories (keep the one with higher importance)
        let merged_count = to_merge.len();
        for (id1, id2) in to_merge {
            // Determine which to keep and which to remove
            let (keep_id, remove_id) = {
                let emb1 = self.memories.get(&id1);
                let emb2 = self.memories.get(&id2);

                match (emb1, emb2) {
                    (Some(e1), Some(e2)) => {
                        if e1.item.importance >= e2.item.importance {
                            (id1, id2)
                        } else {
                            (id2, id1)
                        }
                    }
                    _ => continue,
                }
            };

            // Extract data from remove_emb before borrowing keep_emb mutably
            let (access_count, tags, goal_ids) = {
                if let Some(remove_emb) = self.memories.get(&remove_id) {
                    (
                        remove_emb.item.access_count,
                        remove_emb.item.tags.clone(),
                        remove_emb.item.goal_ids.clone(),
                    )
                } else {
                    continue;
                }
            };

            // Now merge into keep_emb
            if let Some(keep_emb) = self.memories.get_mut(&keep_id) {
                // Combine access counts
                keep_emb.item.access_count += access_count;

                // Merge tags
                for tag in tags {
                    if !keep_emb.item.tags.contains(&tag) {
                        keep_emb.item.tags.push(tag);
                    }
                }

                // Merge goal IDs
                for goal_id in goal_ids {
                    if !keep_emb.item.goal_ids.contains(&goal_id) {
                        keep_emb.item.goal_ids.push(goal_id);
                    }
                }
            }

            self.memories.remove(&remove_id);
        }

        merged_count
    }

    /// Get total number of memories
    pub fn len(&self) -> usize {
        self.memories.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }

    /// Clear all memories
    pub fn clear(&mut self) {
        self.memories.clear();
    }

    /// Get memories sorted by importance
    pub fn get_most_important(&self, limit: usize) -> Vec<&MemoryItem> {
        let mut items: Vec<_> = self.memories.values().map(|emb| &emb.item).collect();

        items.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        items.truncate(limit);

        items
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryType;

    #[test]
    fn test_embedding_creation() {
        let item = MemoryItem::new("Test memory".to_string(), MemoryType::Action);
        let embedding = MemoryEmbedding::new(item);

        assert_eq!(embedding.embedding.len(), 768);

        // Check normalization
        let magnitude: f32 = embedding
            .embedding
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_embedding_similarity() {
        let item1 = MemoryItem::new("Hello world".to_string(), MemoryType::Action);
        let item2 = MemoryItem::new("Hello world".to_string(), MemoryType::Action);
        let item3 = MemoryItem::new("Goodbye universe".to_string(), MemoryType::Action);

        let emb1 = MemoryEmbedding::new(item1);
        let emb2 = MemoryEmbedding::new(item2);
        let emb3 = MemoryEmbedding::new(item3);

        // Identical strings should have high similarity
        let sim_same = emb1.similarity(&emb2);
        assert!(sim_same > 0.9);

        // Different strings should have lower similarity
        let sim_diff = emb1.similarity(&emb3);
        assert!(sim_diff < sim_same);
    }

    #[test]
    fn test_episodic_store_and_get() {
        let mut em = EpisodicMemory::new();
        let item = MemoryItem::new("Test memory".to_string(), MemoryType::Action);
        let id = item.id;

        em.store(item);
        assert_eq!(em.len(), 1);

        let retrieved = em.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test memory");
    }

    #[test]
    fn test_semantic_query() {
        let mut em = EpisodicMemory::new();

        em.store(MemoryItem::new(
            "Implemented user authentication system".to_string(),
            MemoryType::Action,
        ));
        em.store(MemoryItem::new(
            "Fixed database connection bug".to_string(),
            MemoryType::Action,
        ));
        em.store(MemoryItem::new(
            "Added login and signup features".to_string(),
            MemoryType::Action,
        ));

        let results = em.query("authentication login", 2);
        assert!(results.len() <= 2);

        // Should find authentication-related memories
        assert!(
            results
                .iter()
                .any(|r| r.item.content.contains("authentication")
                    || r.item.content.contains("login"))
        );
    }

    #[test]
    fn test_query_by_goal() {
        let mut em = EpisodicMemory::new();
        let goal_id = Uuid::new_v4();

        let item1 = MemoryItem::builder()
            .content("Related to goal".to_string())
            .memory_type(MemoryType::Action)
            .goal_id(goal_id)
            .build()
            .unwrap();

        let item2 = MemoryItem::builder()
            .content("Not related".to_string())
            .memory_type(MemoryType::Action)
            .build()
            .unwrap();

        em.store(item1);
        em.store(item2);

        let results = em.query_by_goal(&goal_id, 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].item.content.contains("Related"));
    }

    #[test]
    fn test_query_by_tags() {
        let mut em = EpisodicMemory::new();

        let item1 = MemoryItem::builder()
            .content("Auth feature".to_string())
            .memory_type(MemoryType::Action)
            .tag("authentication".to_string())
            .build()
            .unwrap();

        let item2 = MemoryItem::builder()
            .content("DB feature".to_string())
            .memory_type(MemoryType::Action)
            .tag("database".to_string())
            .build()
            .unwrap();

        em.store(item1);
        em.store(item2);

        let results = em.query_by_tags(&["authentication".to_string()], 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].item.content.contains("Auth"));
    }

    #[test]
    fn test_compression() {
        let mut em = EpisodicMemory::with_compression_threshold(0.95);

        // Store very similar memories
        em.store(
            MemoryItem::builder()
                .content("Test memory A".to_string())
                .memory_type(MemoryType::Action)
                .importance(0.8)
                .build()
                .unwrap(),
        );

        em.store(
            MemoryItem::builder()
                .content("Test memory A".to_string())
                .memory_type(MemoryType::Action)
                .importance(0.6)
                .build()
                .unwrap(),
        );

        assert_eq!(em.len(), 2);

        let merged = em.compress();
        assert!(merged > 0);
        assert_eq!(em.len(), 1);
    }

    #[test]
    fn test_get_most_important() {
        let mut em = EpisodicMemory::new();

        em.store(
            MemoryItem::builder()
                .content("Low importance".to_string())
                .memory_type(MemoryType::Action)
                .importance(0.3)
                .build()
                .unwrap(),
        );

        em.store(
            MemoryItem::builder()
                .content("High importance".to_string())
                .memory_type(MemoryType::Action)
                .importance(0.9)
                .build()
                .unwrap(),
        );

        let important = em.get_most_important(1);
        assert_eq!(important.len(), 1);
        assert_eq!(important[0].content, "High importance");
    }
}
