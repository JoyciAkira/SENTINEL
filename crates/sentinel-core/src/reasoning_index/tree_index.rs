//! Tree Index - Main structure for vectorless document indexing
//!
//! A hierarchical tree structure for documents that enables reasoning-based
//! retrieval without vector databases.

use super::{IndexNode, SearchOptions, SearchResult};
use super::search::SearchEngine;
use super::node::NodeId;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A tree-based document index for reasoning-based retrieval
///
/// This is the main structure for PageIndex-style vectorless RAG.
/// It stores a hierarchical tree of document sections that can be
/// searched using LLM reasoning instead of vector similarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIndex {
    /// Root node of the tree
    pub root: IndexNode,

    /// Source document path (if from file)
    pub source_path: Option<PathBuf>,

    /// Document metadata
    pub metadata: IndexMetadata,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Metadata about the indexed document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Total number of pages
    pub total_pages: usize,

    /// Total number of nodes
    pub total_nodes: usize,

    /// Maximum depth of the tree
    pub max_depth: usize,

    /// Document type (markdown, pdf, text, rust, etc.)
    pub document_type: String,

    /// Index version for compatibility
    pub version: String,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self {
            total_pages: 0,
            total_nodes: 0,
            max_depth: 0,
            document_type: "unknown".to_string(),
            version: "1.0.0".to_string(),
        }
    }
}

impl TreeIndex {
    /// Create a new tree index from a root node
    pub fn new(root: IndexNode, source_path: Option<PathBuf>) -> Self {
        let total_nodes = root.count_nodes();
        let max_depth = root.max_depth();
        let total_pages = root.end_page;
        
        let document_type = source_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            root,
            source_path,
            metadata: IndexMetadata {
                total_pages,
                total_nodes,
                max_depth,
                document_type,
                version: "1.0.0".to_string(),
            },
            created_at: chrono::Utc::now(),
        }
    }

    /// Perform a reasoning-based search
    ///
    /// This simulates how a human expert navigates a document:
    /// - Starts at root
    /// - Traverses promising branches
    /// - Collects and ranks relevant sections
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let index = TreeIndex::from_markdown("doc.md").await?;
    /// let results = index.search("Find authentication code").await?;
    ///
    /// for result in results {
    ///     println!("Found: {} (confidence: {:.2})", 
    ///         result.node.title, result.confidence);
    /// }
    /// ```
    pub async fn search(&self, query: &str) -> Vec<SearchResult> {
        let engine = SearchEngine::with_defaults();
        engine.search(&self.root, query).await
    }

    /// Search with custom options
    pub async fn search_with_options(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Vec<SearchResult> {
        let engine = SearchEngine::new(options);
        engine.search(&self.root, query).await
    }

    /// Get the table of contents as a string
    pub fn to_toc(&self) -> String {
        self.root.to_toc()
    }

    /// Find a node by its ID
    pub fn find_by_id(&self, id: &str) -> Option<&IndexNode> {
        let node_id = NodeId::from_str(id);
        self.root.find_by_id(&node_id)
    }

    /// Get all leaf nodes (sections without children)
    pub fn get_leaves(&self) -> Vec<&IndexNode> {
        self.root.collect_leaves()
    }

    /// Get all nodes as a flat list
    pub fn flatten(&self) -> Vec<&IndexNode> {
        self.root.flatten()
    }

    /// Get sections containing a specific page
    pub fn get_sections_for_page(&self, page: usize) -> Vec<&IndexNode> {
        self.flatten()
            .into_iter()
            .filter(|node| node.contains_page(page))
            .collect()
    }

    /// Export the index to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::SentinelError::Serialization(e))
    }

    /// Import an index from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| crate::error::SentinelError::Serialization(e))
    }

    /// Save index to a file
    pub async fn save(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let json = self.to_json()?;
        tokio::fs::write(path.as_ref(), json).await
            .map_err(|e| crate::error::SentinelError::Io(e))?;
        Ok(())
    }

    /// Load index from a file
    pub async fn load(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = tokio::fs::read_to_string(path.as_ref()).await
            .map_err(|e| crate::error::SentinelError::Io(e))?;
        Self::from_json(&content)
    }

    /// Get statistics about the index
    pub fn stats(&self) -> IndexStats {
        let leaves = self.get_leaves();
        let avg_depth = if leaves.is_empty() {
            0.0
        } else {
            leaves.iter().map(|n| n.depth as f64).sum::<f64>() / leaves.len() as f64
        };

        IndexStats {
            total_nodes: self.metadata.total_nodes,
            total_pages: self.metadata.total_pages,
            max_depth: self.metadata.max_depth,
            avg_depth,
            leaf_count: leaves.len(),
        }
    }
}

/// Statistics about a tree index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_nodes: usize,
    pub total_pages: usize,
    pub max_depth: usize,
    pub avg_depth: f64,
    pub leaf_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_index() -> TreeIndex {
        let mut root = IndexNode::new("Test Document", 1, 100)
            .with_summary("A test document for unit testing.");

        let mut chapter1 = IndexNode::new("Chapter 1", 1, 50)
            .with_summary("First chapter content.");
        chapter1.add_child(IndexNode::new("Section 1.1", 1, 25));
        chapter1.add_child(IndexNode::new("Section 1.2", 26, 50));

        let mut chapter2 = IndexNode::new("Chapter 2", 51, 100)
            .with_summary("Second chapter content.");
        chapter2.add_child(IndexNode::new("Section 2.1", 51, 75));

        root.add_child(chapter1);
        root.add_child(chapter2);

        TreeIndex::new(root, None)
    }

    #[test]
    fn test_tree_index_creation() {
        let index = create_test_index();
        
        assert_eq!(index.root.title, "Test Document");
        assert_eq!(index.metadata.total_nodes, 6); // root + 2 chapters + 3 sections
        assert!(index.metadata.max_depth >= 2);
    }

    #[test]
    fn test_tree_index_stats() {
        let index = create_test_index();
        let stats = index.stats();
        
        assert_eq!(stats.total_nodes, 6);
        assert_eq!(stats.leaf_count, 3); // 3 sections
    }

    #[tokio::test]
    async fn test_tree_index_search() {
        let index = create_test_index();
        let results = index.search("Chapter 1").await;
        
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.node.title.contains("Chapter 1")));
    }

    #[test]
    fn test_tree_index_to_toc() {
        let index = create_test_index();
        let toc = index.to_toc();
        
        assert!(toc.contains("Test Document"));
        assert!(toc.contains("Chapter 1"));
        assert!(toc.contains("Section 1.1"));
    }

    #[test]
    fn test_tree_index_get_leaves() {
        let index = create_test_index();
        let leaves = index.get_leaves();
        
        assert_eq!(leaves.len(), 3);
        assert!(leaves.iter().all(|n| n.children.is_empty()));
    }

    #[test]
    fn test_tree_index_get_sections_for_page() {
        let index = create_test_index();
        let sections = index.get_sections_for_page(30);
        
        // Page 30 should be in Section 1.2
        assert!(sections.iter().any(|s| s.title == "Section 1.2"));
    }

    #[test]
    fn test_tree_index_json_roundtrip() {
        let index = create_test_index();
        let json = index.to_json().unwrap();
        let restored = TreeIndex::from_json(&json).unwrap();
        
        assert_eq!(restored.root.title, index.root.title);
        assert_eq!(restored.metadata.total_nodes, index.metadata.total_nodes);
    }

    #[test]
    fn test_tree_index_find_by_id() {
        let index = create_test_index();
        let first_child = &index.root.children[0];
        
        let found = index.find_by_id(&first_child.id.0);
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, first_child.title);
    }
}