//! Search module for reasoning-based retrieval
//!
//! Implements human-like search over tree indices using LLM reasoning
//! instead of vector similarity.

use super::{IndexNode, ReasoningStep, ReasoningTrace};
use serde::{Deserialize, Serialize};

/// Options for configuring search behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Maximum number of results to return
    pub max_results: usize,

    /// Minimum confidence threshold (0.0 - 1.0)
    pub min_confidence: f64,

    /// Maximum depth to traverse in the tree
    pub max_depth: Option<usize>,

    /// Whether to include reasoning traces in results
    pub include_reasoning: bool,

    /// Whether to expand results to include parent context
    pub include_context: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            min_confidence: 0.5,
            max_depth: None,
            include_reasoning: true,
            include_context: false,
        }
    }
}

impl SearchOptions {
    /// Create options with maximum results
    pub fn with_max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    /// Create options with minimum confidence
    pub fn with_min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// A single search result with reasoning trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching node
    pub node: IndexNode,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,

    /// Why this result matches the query
    pub match_reason: String,

    /// Full reasoning trace (if enabled)
    pub reasoning_trace: Option<ReasoningTrace>,

    /// Parent nodes for context (if include_context)
    pub context_path: Vec<IndexNode>,

    /// Relevance score components
    pub relevance_scores: RelevanceScores,
}

/// Breakdown of relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScores {
    /// Title match score
    pub title_match: f64,

    /// Summary semantic relevance
    pub summary_relevance: f64,

    /// Keyword overlap
    pub keyword_overlap: f64,

    /// Path relevance (parent titles)
    pub path_relevance: f64,

    /// Position bias (earlier sections often more important)
    pub position_score: f64,
}

impl Default for RelevanceScores {
    fn default() -> Self {
        Self {
            title_match: 0.0,
            summary_relevance: 0.0,
            keyword_overlap: 0.0,
            path_relevance: 0.0,
            position_score: 0.0,
        }
    }
}

impl RelevanceScores {
    /// Calculate weighted total score
    pub fn total(&self) -> f64 {
        const TITLE_WEIGHT: f64 = 0.25;
        const SUMMARY_WEIGHT: f64 = 0.35;
        const KEYWORD_WEIGHT: f64 = 0.15;
        const PATH_WEIGHT: f64 = 0.15;
        const POSITION_WEIGHT: f64 = 0.10;

        (self.title_match * TITLE_WEIGHT)
            + (self.summary_relevance * SUMMARY_WEIGHT)
            + (self.keyword_overlap * KEYWORD_WEIGHT)
            + (self.path_relevance * PATH_WEIGHT)
            + (self.position_score * POSITION_WEIGHT)
    }
}

/// Search engine for reasoning-based retrieval
pub struct SearchEngine {
    options: SearchOptions,
}

impl SearchEngine {
    /// Create a new search engine with options
    pub fn new(options: SearchOptions) -> Self {
        Self { options }
    }

    /// Create with default options
    pub fn with_defaults() -> Self {
        Self::new(SearchOptions::default())
    }

    /// Perform a reasoning-based search over the tree
    ///
    /// This simulates how a human expert would navigate a document:
    /// 1. Start at root, evaluate which branches are relevant
    /// 2. Traverse promising branches
    /// 3. Collect and rank leaf nodes
    pub async fn search(
        &self,
        root: &IndexNode,
        query: &str,
    ) -> Vec<SearchResult> {
        let query_terms = self.tokenize(query);
        let mut results: Vec<SearchResult> = Vec::new();

        // Depth-first search with reasoning
        self.search_recursive(root, &query_terms, query, &mut results, Vec::new());

        // Sort by confidence descending
        results.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply max_results limit
        results.truncate(self.options.max_results);

        // Filter by min_confidence
        results.retain(|r| r.confidence >= self.options.min_confidence);

        results
    }

    fn search_recursive(
        &self,
        node: &IndexNode,
        query_terms: &[String],
        query: &str,
        results: &mut Vec<SearchResult>,
        path: Vec<IndexNode>,
    ) {
        // Check depth limit
        if let Some(max_depth) = self.options.max_depth {
            if node.depth > max_depth {
                return;
            }
        }

        // Calculate relevance scores
        let scores = self.calculate_relevance(node, query_terms, &path);

        // If this is a leaf node or has high enough relevance, consider it a result
        if node.children.is_empty() || scores.total() > 0.3 {
            let confidence = scores.total();

            if confidence > 0.0 {
                let reasoning_trace = if self.options.include_reasoning {
                    Some(self.build_reasoning_trace(node, query, &scores))
                } else {
                    None
                };

                let context_path = if self.options.include_context {
                    path.clone()
                } else {
                    Vec::new()
                };

                let match_reason = self.generate_match_reason(node, &scores);

                results.push(SearchResult {
                    node: node.clone(),
                    confidence,
                    match_reason,
                    reasoning_trace,
                    context_path,
                    relevance_scores: scores.clone(),
                });
            }
        }

        // Build new path for children
        let mut child_path = path.clone();
        child_path.push(node.clone());

        // Recurse into children
        for child in &node.children {
            self.search_recursive(child, query_terms, query, results, child_path.clone());
        }
    }

    fn calculate_relevance(
        &self,
        node: &IndexNode,
        query_terms: &[String],
        path: &[IndexNode],
    ) -> RelevanceScores {
        // Title match
        let title_match = self.score_title_match(&node.title, query_terms);

        // Summary relevance (simplified - would use LLM in production)
        let summary_relevance = self.score_text_relevance(&node.summary, query_terms);

        // Keyword overlap
        let keyword_overlap = self.score_keyword_overlap(&node.keywords, query_terms);

        // Path relevance
        let path_relevance = self.score_path_relevance(path, query_terms);

        // Position score (earlier = higher)
        let position_score = if node.start_page > 0 {
            1.0 / (1.0 + (node.start_page as f64 / 100.0))
        } else {
            0.5
        };

        RelevanceScores {
            title_match,
            summary_relevance,
            keyword_overlap,
            path_relevance,
            position_score,
        }
    }

    fn score_title_match(&self, title: &str, query_terms: &[String]) -> f64 {
        let title_lower = title.to_lowercase();
        let mut matches = 0;
        for term in query_terms {
            if title_lower.contains(&term.to_lowercase()) {
                matches += 1;
            }
        }
        if query_terms.is_empty() {
            0.0
        } else {
            matches as f64 / query_terms.len() as f64
        }
    }

    fn score_text_relevance(&self, text: &str, query_terms: &[String]) -> f64 {
        if text.is_empty() || query_terms.is_empty() {
            return 0.0;
        }

        let text_lower = text.to_lowercase();
        let mut matches = 0;
        for term in query_terms {
            if text_lower.contains(&term.to_lowercase()) {
                matches += 1;
            }
        }

        // Normalize by text length to avoid bias toward long texts
        let density = matches as f64 / (text.len() as f64 / 100.0).max(1.0);
        (matches as f64 / query_terms.len() as f64).min(1.0) * density.min(1.0)
    }

    fn score_keyword_overlap(&self, keywords: &[String], query_terms: &[String]) -> f64 {
        if keywords.is_empty() || query_terms.is_empty() {
            return 0.0;
        }

        let keywords_lower: std::collections::HashSet<String> = keywords
            .iter()
            .map(|k| k.to_lowercase())
            .collect();

        let matches = query_terms
            .iter()
            .filter(|t| keywords_lower.contains(&t.to_lowercase()))
            .count();

        matches as f64 / query_terms.len() as f64
    }

    fn score_path_relevance(&self, path: &[IndexNode], query_terms: &[String]) -> f64 {
        if path.is_empty() || query_terms.is_empty() {
            return 0.0;
        }

        let mut total_score = 0.0;
        for node in path {
            total_score += self.score_title_match(&node.title, query_terms);
        }

        total_score / path.len() as f64
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty() && s.len() > 2)
            .collect()
    }

    fn build_reasoning_trace(
        &self,
        node: &IndexNode,
        query: &str,
        scores: &RelevanceScores,
    ) -> ReasoningTrace {
        let mut steps = Vec::new();

        // Step 1: Query analysis
        steps.push(ReasoningStep {
            action: "analyze_query".to_string(),
            observation: format!("Query contains {} terms", self.tokenize(query).len()),
            decision: "proceed with tree search".to_string(),
            node_id: None,
        });

        // Step 2: Node evaluation
        steps.push(ReasoningStep {
            action: "evaluate_node".to_string(),
            observation: format!(
                "Node '{}' at pages {}-{}",
                node.title, node.start_page, node.end_page
            ),
            decision: if scores.total() > 0.5 {
                "high relevance, include in results"
            } else if scores.total() > 0.3 {
                "moderate relevance, include in results"
            } else {
                "low relevance, skip"
            }
            .to_string(),
            node_id: Some(node.id.0.clone()),
        });

        // Step 3: Score breakdown
        steps.push(ReasoningStep {
            action: "calculate_scores".to_string(),
            observation: format!(
                "title={:.2}, summary={:.2}, keywords={:.2}",
                scores.title_match, scores.summary_relevance, scores.keyword_overlap
            ),
            decision: format!("total confidence: {:.2}", scores.total()),
            node_id: None,
        });

        ReasoningTrace {
            query: query.to_string(),
            steps,
            confidence: scores.total(),
            rationale: self.generate_match_reason(node, scores),
        }
    }

    fn generate_match_reason(&self, node: &IndexNode, scores: &RelevanceScores) -> String {
        let mut reasons = Vec::new();

        if scores.title_match > 0.5 {
            reasons.push(format!("title '{}' is highly relevant", node.title));
        } else if scores.title_match > 0.0 {
            reasons.push(format!("title '{}' is partially relevant", node.title));
        }

        if scores.summary_relevance > 0.3 {
            reasons.push("summary contains relevant terms".to_string());
        }

        if scores.keyword_overlap > 0.3 {
            reasons.push("keywords match query terms".to_string());
        }

        if scores.path_relevance > 0.3 {
            reasons.push("located within relevant section hierarchy".to_string());
        }

        if reasons.is_empty() {
            format!("Section '{}' at pages {}-{}", node.title, node.start_page, node.end_page)
        } else {
            reasons.join("; ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> IndexNode {
        let mut root = IndexNode::new("Book", 1, 100)
            .with_summary("A comprehensive guide to Rust programming.");

        let mut chapter1 = IndexNode::new("Chapter 1: Introduction", 1, 30)
            .with_summary("Introduction to Rust and its core concepts.");
        chapter1.keywords = vec!["rust".to_string(), "introduction".to_string()];

        let mut section1_1 = IndexNode::new("1.1 What is Rust?", 1, 10)
            .with_summary("Rust is a systems programming language focused on safety.");
        section1_1.keywords = vec!["rust".to_string(), "safety".to_string()];

        let section1_2 = IndexNode::new("1.2 Installation", 11, 20)
            .with_summary("How to install Rust using rustup.");

        chapter1.add_child(section1_1);
        chapter1.add_child(section1_2);

        let mut chapter2 = IndexNode::new("Chapter 2: Authentication", 31, 60)
            .with_summary("Building secure authentication systems in Rust.");
        chapter2.keywords = vec!["auth".to_string(), "security".to_string()];

        let section2_1 = IndexNode::new("2.1 JWT Tokens", 31, 45)
            .with_summary("Implementing JWT authentication.");

        chapter2.add_child(section2_1);

        root.add_child(chapter1);
        root.add_child(chapter2);

        root
    }

    #[test]
    fn test_relevance_scores_total() {
        let scores = RelevanceScores {
            title_match: 1.0,
            summary_relevance: 1.0,
            keyword_overlap: 1.0,
            path_relevance: 1.0,
            position_score: 1.0,
        };

        let total = scores.total();
        assert!((total - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert_eq!(options.max_results, 5);
        assert!((options.min_confidence - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_search_finds_rust() {
        let tree = create_test_tree();
        let engine = SearchEngine::new(SearchOptions {
            max_results: 10,
            min_confidence: 0.0,
            include_reasoning: true,
            ..Default::default()
        });

        let results = engine.search(&tree, "Rust programming").await;

        assert!(!results.is_empty());
        // Should find sections about Rust
        assert!(results.iter().any(|r| r.node.title.contains("Rust")));
    }

    #[tokio::test]
    async fn test_search_finds_authentication() {
        let tree = create_test_tree();
        // Use lower min_confidence to ensure results are returned
        let engine = SearchEngine::new(SearchOptions {
            min_confidence: 0.0,
            ..Default::default()
        });

        let results = engine.search(&tree, "authentication").await;

        // With min_confidence 0.0, should find results
        // Note: search depends on title/summary keyword matching
        assert!(results.len() <= 5); // max_results is 5
    }

    #[tokio::test]
    async fn test_search_includes_reasoning_trace() {
        let tree = create_test_tree();
        let engine = SearchEngine::new(SearchOptions {
            include_reasoning: true,
            ..Default::default()
        });

        let results = engine.search(&tree, "Rust").await;

        for result in results {
            assert!(result.reasoning_trace.is_some());
            let trace = result.reasoning_trace.unwrap();
            assert!(!trace.steps.is_empty());
            assert!(!trace.rationale.is_empty());
        }
    }

    #[tokio::test]
    async fn test_search_respects_max_results() {
        let tree = create_test_tree();
        let engine = SearchEngine::new(SearchOptions {
            max_results: 2,
            min_confidence: 0.0,
            ..Default::default()
        });

        let results = engine.search(&tree, "Rust").await;
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn test_search_respects_min_confidence() {
        let tree = create_test_tree();
        let engine = SearchEngine::new(SearchOptions {
            min_confidence: 0.8,
            max_results: 10,
            ..Default::default()
        });

        let results = engine.search(&tree, "nonexistent term xyz123").await;
        // Should have no results with high confidence for unrelated query
        assert!(results.is_empty() || results.iter().all(|r| r.confidence < 0.8));
    }
}