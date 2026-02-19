//! Reasoning-based Document Index (PageIndex-style)
//!
//! Vectorless RAG system that uses LLM reasoning over hierarchical tree structures
//! instead of vector embeddings for document retrieval.
//!
//! # Key Concepts
//!
//! - **No Vector DB**: Uses document structure and LLM reasoning
//! - **No Chunking**: Documents organized into natural sections
//! - **Human-like Retrieval**: Simulates how experts navigate documents
//! - **Explainable**: Every retrieval has a reasoning trace
//!
//! # Example
//!
//! ```rust,ignore
//! use sentinel_core::reasoning_index::TreeIndex;
//!
//! // Build tree index from document
//! let index = TreeIndex::from_pdf("report.pdf").await?;
//!
//! // Reasoning-based search (no vectors!)
//! let results = index.search("Find authentication vulnerabilities").await?;
//! for result in results {
//!     println!("Page {}: {}", result.page, result.summary);
//!     println!("Reasoning: {:?}", result.reasoning_trace);
//! }
//! ```

mod tree_index;
mod search;
mod node;
mod builder;

pub use tree_index::{TreeIndex, IndexMetadata, IndexStats};
pub use search::{SearchResult, SearchOptions, SearchEngine, RelevanceScores};
pub use node::{IndexNode, NodeId};
pub use builder::TreeIndexBuilder;

/// Reasoning trace for explainable retrieval
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningTrace {
    /// The query that was processed
    pub query: String,
    
    /// Steps taken during reasoning
    pub steps: Vec<ReasoningStep>,
    
    /// Final confidence score (0.0 - 1.0)
    pub confidence: f64,
    
    /// Why this result was selected
    pub rationale: String,
}

/// Single step in the reasoning process
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReasoningStep {
    /// What action was taken
    pub action: String,
    
    /// What was observed
    pub observation: String,
    
    /// Decision made
    pub decision: String,
    
    /// Node visited (if any)
    pub node_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_trace_creation() {
        let trace = ReasoningTrace {
            query: "Find auth".to_string(),
            steps: vec![ReasoningStep {
                action: "traverse".to_string(),
                observation: "Found security section".to_string(),
                decision: "explore children".to_string(),
                node_id: Some("node-001".to_string()),
            }],
            confidence: 0.95,
            rationale: "Section explicitly about authentication".to_string(),
        };
        
        assert_eq!(trace.steps.len(), 1);
        assert!(trace.confidence > 0.9);
    }
}