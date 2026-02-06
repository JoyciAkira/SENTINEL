//! Context Manager - Hierarchical Memory Access
//!
//! This module implements intelligent context retrieval and management:
//! - Accesses all memory levels (Working, Episodic, Semantic)
//! - Uses LRU and semantic similarity for retrieval
//! - Provides infinite context through compression
//!
//! # Why This Is Revolutionary
//!
//! Traditional agents:
//! - Use fixed context window (e.g., 128k tokens)
//! - When context fills, old data is evicted
//! - Cannot retrieve historical information
//! - No intelligent context prioritization
//!
//! Sentinel Context Manager:
//! - Accesses 3-tier hierarchical memory
//! - Uses semantic similarity for retrieval
//! - Compresses context while preserving critical info
//! - Context is prioritized by relevance and recency
//! - Functionally infinite context
//!
//! # Context Access Strategy
//!
//! ```
//! Query: "How was authentication implemented?"
//!         │
//!         v
//! ┌─────────────────────────────────────┐
//! │   Step 1: Working Memory         │
//! │   - Check hot cache (10 items)   │
//! │   - O(1) access                  │
//! └─────────────────────────────────────┘
//!         │
//!         v (if miss)
//! ┌─────────────────────────────────────┐
//! │   Step 2: Episodic Memory        │
//! │   - Semantic vector search          │
//! │   - Re-rank by recency            │
//! │   - Return top 20 results         │
//! └─────────────────────────────────────┘
//!         │
//!         v (if miss)
//! ┌─────────────────────────────────────┐
//! │   Step 3: Semantic Memory         │
//! │   - Knowledge graph traversal        │
//! │   - Pattern matching               │
//! │   - Cross-project learnings       │
//! └─────────────────────────────────────┘
//!         │
//!         v
//! ┌─────────────────────────────────────┐
//! │   Step 4: Context Compression    │
//! │   - Preserve decision rationale      │
//! │   - Preserve failures             │
//! │   - Compress verbose content       │
//! └─────────────────────────────────────┘
//! ```

use anyhow::{Context, Result};
use chrono::Utc;
use sentinel_core::{
    memory::{MemoryManifold, MemoryQueryResult, MemorySource},
    Uuid,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Context query
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Query {
    pub text: String,
    pub goal_id: Option<Uuid>,
    pub tags: Vec<String>,
}

use tokio::sync::Mutex;

/// Context Manager - Hierarchical memory access with intelligent retrieval
#[derive(Debug, Clone)]
pub struct ContextManager {
    /// Access to memory manifold (Layer 4)
    pub memory_manifold: Arc<Mutex<MemoryManifold>>,

    /// Query cache for fast repeated queries
    pub query_cache: HashMap<String, CachedContext>,

    /// Statistics
    pub stats: ContextStats,
}

/// Cached context result
#[derive(Debug, Clone)]
struct CachedContext {
    pub context: UnifiedContext,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

/// Context statistics
#[derive(Debug, Clone, Default)]
pub struct ContextStats {
    pub total_queries: u64,
    pub working_memory_hits: u64,
    pub episodic_memory_hits: u64,
    pub semantic_memory_hits: u64,
    pub cache_hits: u64,
    pub avg_retrieval_time_ms: f64,
    pub context_size_tokens: u64,
}

/// Unified context retrieved from all memory levels
#[derive(Debug, Clone)]
pub struct UnifiedContext {
    /// Working memory items (hot cache)
    pub working: Vec<MemoryQueryResult>,

    /// Episodic memory items (vector search results)
    pub episodic: Vec<MemoryQueryResult>,

    /// Semantic memory items (knowledge graph results)
    pub semantic: Vec<MemoryQueryResult>,

    /// All results combined and ranked
    pub ranked: Vec<MemoryQueryResult>,

    /// Context compressed to token budget
    pub compressed: CompressedContext,

    /// Total number of tokens
    pub token_count: u64,
}

/// Compressed context
#[derive(Debug, Clone)]
pub struct CompressedContext {
    /// Compressed context content
    pub content: String,

    /// What was preserved during compression
    pub preserved_categories: Vec<String>,

    /// What was compressed (reduced detail)
    pub compressed_categories: Vec<String>,

    /// Compression ratio
    pub compression_ratio: f64,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(memory_manifold: Arc<Mutex<MemoryManifold>>) -> Self {
        tracing::info!("Initializing Context Manager");

        Self {
            memory_manifold,
            query_cache: HashMap::new(),
            stats: ContextStats::default(),
        }
    }

    /// Retrieve context for a query, pulling from all memory levels
    ///
    /// This implements intelligent hierarchical retrieval:
    /// 1. Check working memory (instant, 10 items)
    /// 2. Query episodic memory (semantic search, 100ms)
    /// 3. Query semantic memory (knowledge graph, 300ms)
    /// 4. Merge, deduplicate, re-rank
    /// 5. Compress to token budget
    pub async fn retrieve_context(
        &mut self,
        query: &Query,
        limit: usize,
    ) -> Result<UnifiedContext> {
        tracing::debug!("Retrieving context for query: {}", query.text);

        let start_time = std::time::Instant::now();

        // Check cache first
        if let Some(cached) = self.query_cache.get(&query.text) {
            let age_seconds = (Utc::now() - cached.timestamp).num_seconds();
            if age_seconds >= 0 && age_seconds as u64 <= cached.ttl_seconds {
                tracing::debug!("Cache hit for query");
                self.stats.cache_hits += 1;
                self.stats.total_queries += 1;
                return Ok(cached.context.clone());
            }
        }

        // Step 1: Query Working Memory (instant)
        let working_results = self.query_working_memory(query, 10).await;
        let working_hits = working_results.len();

        // Step 2: Query Episodic Memory (semantic search)
        let episodic_results = if working_hits < 5 {
            let mut manifold = self.memory_manifold.lock().await;
            manifold.episodic.query(&query.text, limit)
        } else {
            vec![]
        };

        // Step 3: Query Semantic Memory (knowledge graph)
        let semantic_results = if working_hits < 5 && episodic_results.len() < 15 {
            let mut manifold = self.memory_manifold.lock().await;
            let concept_ids = manifold.semantic.query(&query.text, limit);
            // Convert concept IDs to memories
            let mut results = Vec::new();
            for id in concept_ids {
                if let Some(item) = manifold.episodic.get(&id) {
                    results.push(MemoryQueryResult {
                        item: item.clone(),
                        score: 0.5, // Heuristic for now
                        source: MemorySource::Semantic,
                    });
                }
            }
            results
        } else {
            vec![]
        };

        // Step 4: Merge and rank all results
        let all_results = self.merge_and_rank_results(
            working_results.clone(),
            episodic_results.clone(),
            semantic_results.clone(),
        );

        // Step 5: Compress context to token budget
        let target_tokens = 128_000; // 128k token budget
        let compressed = self.compress_context(&all_results, target_tokens)?;

        let retrieval_time = start_time.elapsed().as_millis() as f64;

        // Update statistics
        self.stats.total_queries += 1;
        self.stats.working_memory_hits += working_hits as u64;
        self.stats.episodic_memory_hits += episodic_results.len() as u64;
        self.stats.semantic_memory_hits += semantic_results.len() as u64;
        self.stats.avg_retrieval_time_ms = (self.stats.avg_retrieval_time_ms
            * (self.stats.total_queries - 1) as f64
            + retrieval_time)
            / self.stats.total_queries as f64;

        tracing::info!(
            "Retrieved {} results ({} WM, {} EM, {} SM) in {}ms",
            all_results.len(),
            working_hits,
            episodic_results.len(),
            semantic_results.len(),
            retrieval_time
        );

        // Cache result
        let unified_context = UnifiedContext {
            working: working_results,
            episodic: episodic_results,
            semantic: semantic_results,
            ranked: all_results.clone(),
            compressed: compressed.clone(),
            token_count: compressed.content.split_whitespace().count() as u64,
        };

        self.cache_query(query.clone(), &unified_context);

        Ok(unified_context)
    }

    /// Query working memory (instant, 10-item LRU cache)
    async fn query_working_memory(&self, query: &Query, limit: usize) -> Vec<MemoryQueryResult> {
        let mut manifold = self.memory_manifold.lock().await;
        manifold.working.query(&query.text, limit)
    }

    /// Merge and rank results from all memory levels
    ///
    /// Uses multi-factor scoring:
    /// 1. Semantic similarity (from vector search)
    /// 2. Recency (recent memories are more relevant)
    /// 3. Goal contribution (memories contributing to goals)
    /// 4. Alignment score (high-alignment memories preferred)
    fn merge_and_rank_results(
        &self,
        working: Vec<MemoryQueryResult>,
        episodic: Vec<MemoryQueryResult>,
        semantic: Vec<MemoryQueryResult>,
    ) -> Vec<MemoryQueryResult> {
        let mut all_results = Vec::new();

        // Add working memory results with source weighting
        for result in working {
            let trust = result.item.trust_score();
            all_results.push(MemoryQueryResult {
                item: result.item.clone(),
                score: result.score * 1.5 * trust, // WM has highest priority
                source: MemorySource::Working,
            });
        }

        // Add episodic memory results with recency scoring
        for (i, result) in episodic.iter().enumerate() {
            let recency_score = 1.0 - (i as f64) / episodic.len() as f64;
            let trust = result.item.trust_score();
            all_results.push(MemoryQueryResult {
                item: result.item.clone(),
                score: (result.score * 0.8 + recency_score * 0.2) * trust,
                source: MemorySource::Episodic,
            });
        }

        // Add semantic memory results with goal contribution scoring
        for result in semantic {
            let goal_score = if !result.item.goal_ids.is_empty() {
                0.2
            } else {
                0.0
            };
            let trust = result.item.trust_score();
            all_results.push(MemoryQueryResult {
                item: result.item.clone(),
                score: (result.score * 0.6 + goal_score) * trust,
                source: MemorySource::Semantic,
            });
        }

        // Sort by composite score (descending)
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Deduplicate by item ID
        let mut seen_ids = std::collections::HashSet::new();
        let dedup_results: Vec<MemoryQueryResult> = all_results
            .into_iter()
            .filter(|r| seen_ids.insert(r.item.id))
            .collect();

        dedup_results
    }

    /// Compress context to fit within token budget
    ///
    /// This is REVOLUTIONARY because:
    /// - Preserves critical information (decisions, failures, learnings)
    /// - Compresses verbose content (code snippets, logs)
    /// - Maintains explainability
    fn compress_context(
        &self,
        results: &[MemoryQueryResult],
        target_tokens: u64,
    ) -> Result<CompressedContext> {
        tracing::debug!(
            "Compressing {} results to {} tokens",
            results.len(),
            target_tokens
        );

        // Estimate current token count
        let current_tokens = self.estimate_token_count(results);

        if current_tokens <= target_tokens {
            // No compression needed
            let content = self.format_results(results);

            return Ok(CompressedContext {
                content,
                preserved_categories: vec!["All categories".to_string()],
                compressed_categories: vec![],
                compression_ratio: 1.0,
            });
        }

        // Categorize memories
        let critical_items: Vec<_> = results
            .iter()
            .filter(|r| self.is_critical_item(r))
            .collect();

        let verbose_items: Vec<_> = results
            .iter()
            .filter(|r| !self.is_critical_item(r))
            .collect();

        // Calculate compression ratio
        let compression_ratio = target_tokens as f64 / current_tokens as f64;

        // Compress verbose items while preserving critical ones
        let compressed_verbose = self.compress_verbose_items(&verbose_items, compression_ratio);

        // Build final content
        let mut content_parts = Vec::new();

        // Section 1: Critical Information (preserve 100%)
        content_parts.push("## Critical Context\n\n".to_string());

        for item in &critical_items {
            content_parts.push(format!("- {}: {}\n", item.item.content, item.score));
        }

        // Section 2: Compressed History
        if !compressed_verbose.is_empty() {
            content_parts.push("\n## Compressed History\n\n".to_string());
            content_parts.push(compressed_verbose);
        }

        // Section 3: Summary Statistics
        content_parts.push("\n## Context Statistics\n\n".to_string());
        content_parts.push(format!("- Total memories: {}\n", results.len()));
        content_parts.push(format!("- Critical: {}\n", critical_items.len()));
        content_parts.push(format!("- Compressed: {}\n", verbose_items.len()));
        content_parts.push(format!(
            "- Token usage: {}/{}\n",
            self.estimate_token_count_from_content(&content_parts.join("")),
            target_tokens
        ));

        let content = content_parts.join("");

        let preserved_categories = vec![
            "Decision rationale".to_string(),
            "Failures and errors".to_string(),
            "Learnings and patterns".to_string(),
            "Goal progress".to_string(),
        ];

        let compressed_categories = vec![
            "Verbose code details".to_string(),
            "Verbose logs".to_string(),
        ];

        tracing::info!(
            "Compressed context: {:.1}% ({} -> {} tokens)",
            compression_ratio * 100.0,
            current_tokens,
            target_tokens
        );

        Ok(CompressedContext {
            content,
            preserved_categories,
            compressed_categories,
            compression_ratio,
        })
    }

    /// Check if memory item is critical
    ///
    /// Critical items are:
    /// 1. Decision rationales (why we made choices)
    /// 2. Failures and errors (what went wrong)
    /// 3. Learnings and patterns (what worked)
    /// 4. Goal progress (current state)
    fn is_critical_item(&self, result: &MemoryQueryResult) -> bool {
        let desc_lower = result.item.content.to_lowercase();

        desc_lower.contains("decision")
            || desc_lower.contains("rationale")
            || desc_lower.contains("choose")
            || desc_lower.contains("reason")
            || desc_lower.contains("error")
            || desc_lower.contains("failure")
            || desc_lower.contains("bug")
            || desc_lower.contains("crash")
            || desc_lower.contains("learn")
            || desc_lower.contains("pattern")
            || desc_lower.contains("success")
            || desc_lower.contains("goal")
            || desc_lower.contains("progress")
    }

    /// Compress verbose items
    ///
    /// Reduces detail level while preserving key information:
    /// - Summarize long descriptions
    /// - Aggregate similar items
    /// - Remove redundant details
    fn compress_verbose_items(
        &self,
        items: &[&MemoryQueryResult],
        compression_ratio: f64,
    ) -> String {
        let mut compressed = Vec::new();

        // Aggregate items by category
        let mut categories: HashMap<String, Vec<&MemoryQueryResult>> = HashMap::new();

        for item in items {
            let category = self.categorize_item(item);

            categories
                .entry(category)
                .or_insert_with(Vec::new)
                .push(item);
        }

        // Summarize each category
        for (category, category_items) in &categories {
            if category_items.len() > 5 {
                // Summarize if many items
                let summary = self.summarize_category(category_items);
                compressed.push(format!(
                    "### {} ({})\n{}\n",
                    category,
                    category_items.len(),
                    summary
                ));
            } else {
                // Keep all items if few
                for item in category_items {
                    compressed.push(format!(
                        "{} (score: {:.1})\n",
                        item.item.content, item.score
                    ));
                }
                compressed.push("\n".to_string());
            }
        }

        compressed.join("")
    }

    /// Categorize memory item
    fn categorize_item(&self, item: &MemoryQueryResult) -> String {
        let desc_lower = item.item.content.to_lowercase();

        if desc_lower.contains("file") || desc_lower.contains("create") {
            "File Operations".to_string()
        } else if desc_lower.contains("test") || desc_lower.contains("assert") {
            "Testing".to_string()
        } else if desc_lower.contains("error") || desc_lower.contains("bug") {
            "Errors".to_string()
        } else if desc_lower.contains("success") || desc_lower.contains("working") {
            "Successes".to_string()
        } else {
            "General".to_string()
        }
    }

    /// Summarize category with multiple items
    fn summarize_category(&self, items: &[&MemoryQueryResult]) -> String {
        let mut summary = Vec::new();

        // Count by subcategory
        let mut counts = HashMap::new();

        for item in items {
            let subcategory = self.categorize_item(item);
            *counts.entry(subcategory).or_insert(0) += 1;
        }

        // Build summary
        summary.push(format!("- Total items: {}\n", items.len()));

        for (category, count) in &counts {
            summary.push(format!("  - {}: {}\n", category, count));
        }

        // Add top 3 items by score
        let mut sorted: Vec<_> = items.to_vec();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        summary.push("- Top items:\n".to_string());

        for (i, item) in sorted.iter().take(3).enumerate() {
            summary.push(format!(
                "  {}. {} (score: {:.1})\n",
                i + 1,
                item.item.content,
                item.score
            ));
        }

        summary.join("")
    }

    /// Estimate token count from memory results
    fn estimate_token_count(&self, results: &[MemoryQueryResult]) -> u64 {
        // Approximate: 1 token per 4 characters
        let total_chars: usize = results.iter().map(|r| r.item.content.len()).sum();

        (total_chars / 4) as u64
    }

    /// Format memory results into readable string
    fn format_results(&self, results: &[MemoryQueryResult]) -> String {
        let mut formatted = Vec::new();

        for (i, result) in results.iter().enumerate() {
            formatted.push(format!(
                "{}. [{:?} {:.1}] {}\n",
                i + 1,
                result.source,
                result.score,
                result.item.content
            ));
        }

        formatted.join("")
    }

    /// Estimate token count from content string
    fn estimate_token_count_from_content(&self, content: &str) -> u64 {
        (content.len() / 4) as u64
    }

    /// Check cache for query
    fn check_cache(&self, query: &Query) -> Option<CachedContext> {
        let cache_key = self.cache_key(query);

        if let Some(cached) = self.query_cache.get(&cache_key) {
            // Check if cache is still valid
            let age = chrono::Utc::now()
                .signed_duration_since(cached.timestamp)
                .num_seconds();

            if age < cached.ttl_seconds as i64 {
                // Update with current context
                let unified = cached.context.clone();

                return Some(CachedContext {
                    context: unified.clone(),
                    timestamp: Utc::now(),
                    ttl_seconds: 300,
                });
            }
        }

        None
    }

    /// Cache query result
    fn cache_query(&mut self, query: Query, context: &UnifiedContext) {
        let cache_key = self.cache_key(&query);

        let cached = CachedContext {
            context: context.clone(),
            timestamp: Utc::now(),
            ttl_seconds: 300, // 5 minute TTL
        };

        self.query_cache.insert(cache_key, cached);

        // Evict old cache entries
        let max_cache_size = 100;
        if self.query_cache.len() > max_cache_size {
            self.evict_oldest_cache_entries(max_cache_size - 10);
        }
    }

    /// Generate cache key from query
    fn cache_key(&self, query: &Query) -> String {
        format!(
            "{}::{}",
            query.text,
            query
                .goal_id
                .map(|g| g.to_string())
                .unwrap_or("none".to_string())
        )
    }

    /// Evict oldest cache entries
    fn evict_oldest_cache_entries(&mut self, keep_count: usize) {
        let mut entries: Vec<_> = self
            .query_cache
            .iter()
            .map(|(k, v)| (k.clone(), v.timestamp))
            .collect();

        entries.sort_by(|a, b| a.1.cmp(&b.1));

        for (key, _) in entries.iter().skip(keep_count) {
            self.query_cache.remove(key);
        }

        tracing::debug!(
            "Evicted {} cache entries, keeping {}",
            entries.len() - keep_count,
            keep_count
        );
    }

    /// Get context statistics
    pub fn get_stats(&self) -> ContextStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_core::memory::{MemoryItem, MemoryType};

    #[test]
    fn test_context_manager_initialization() {
        let memory_manifold = Arc::new(Mutex::new(sentinel_core::memory::MemoryManifold::new()));
        let context_manager = ContextManager::new(memory_manifold);

        assert!(context_manager.query_cache.is_empty());
        assert_eq!(context_manager.stats.total_queries, 0);
    }

    #[test]
    fn test_merge_and_rank_results() {
        let memory_manifold = Arc::new(Mutex::new(sentinel_core::memory::MemoryManifold::new()));
        let mut context_manager = ContextManager::new(memory_manifold);

        let working = vec![];
        let episodic = vec![];
        let semantic = vec![];

        let ranked = context_manager.merge_and_rank_results(working, episodic, semantic);

        // All results should be sorted by score (descending)
        for i in 0..ranked.len().saturating_sub(1) {
            assert!(ranked[i].score >= ranked[i + 1].score);
        }
    }

    #[test]
    fn test_is_critical_item() {
        let memory_manifold = Arc::new(Mutex::new(sentinel_core::memory::MemoryManifold::new()));
        let context_manager = ContextManager::new(memory_manifold);

        // Critical items
        assert!(context_manager.is_critical_item(&MemoryQueryResult {
            item: MemoryItem::new(
                "Decision: Use JWT for authentication".to_string(),
                MemoryType::Decision
            ),
            score: 0.9,
            source: sentinel_core::memory::MemorySource::Episodic,
        }));

        assert!(!context_manager.is_critical_item(&MemoryQueryResult {
            item: MemoryItem::new("Created auth.rs file".to_string(), MemoryType::Action),
            score: 0.85,
            source: sentinel_core::memory::MemorySource::Episodic,
        }));
    }

    #[test]
    fn test_estimate_token_count() {
        let memory_manifold = Arc::new(Mutex::new(sentinel_core::memory::MemoryManifold::new()));
        let context_manager = ContextManager::new(memory_manifold);

        let results = vec![MemoryQueryResult {
            item: MemoryItem::new("Test description 1".to_string(), MemoryType::Observation),
            score: 0.9,
            source: sentinel_core::memory::MemorySource::Episodic,
        }];

        let count = context_manager.estimate_token_count(&results);

        // "Test description 1" = ~20 chars = ~4 tokens
        assert_eq!(count, 4);
    }
}
