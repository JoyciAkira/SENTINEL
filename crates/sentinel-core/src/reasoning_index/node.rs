//! Index Node - Hierarchical tree structure for documents
//!
//! Each node represents a section of a document with:
//! - Title and summary
//! - Page range
//! - Nested children (subsections)
//! - Metadata for retrieval

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an index node
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new() -> Self {
        Self(format!("node-{}", Uuid::new_v4()))
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A node in the document tree index
///
/// Represents a section, chapter, or logical division of a document.
/// Nodes form a hierarchical tree structure optimized for reasoning-based
/// navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexNode {
    /// Unique identifier for this node
    pub id: NodeId,

    /// Human-readable title of this section
    pub title: String,

    /// LLM-generated summary of this section's content
    pub summary: String,

    /// Starting page number (1-indexed)
    pub start_page: usize,

    /// Ending page number (inclusive)
    pub end_page: usize,

    /// Nested child nodes (subsections)
    #[serde(default)]
    pub children: Vec<IndexNode>,

    /// Keywords extracted from this section
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Document-specific metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Depth in the tree (0 = root)
    #[serde(default)]
    pub depth: usize,

    /// Number of tokens in this section (approximate)
    #[serde(default)]
    pub token_count: usize,
}

impl IndexNode {
    /// Create a new node with title and page range
    pub fn new(title: impl Into<String>, start_page: usize, end_page: usize) -> Self {
        Self {
            id: NodeId::new(),
            title: title.into(),
            summary: String::new(),
            start_page,
            end_page,
            children: Vec::new(),
            keywords: Vec::new(),
            metadata: HashMap::new(),
            depth: 0,
            token_count: 0,
        }
    }

    /// Set the summary
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Add a child node
    pub fn add_child(&mut self, mut child: IndexNode) {
        child.depth = self.depth + 1;
        self.children.push(child);
    }

    /// Get total number of pages covered by this node and descendants
    pub fn total_pages(&self) -> usize {
        self.end_page - self.start_page + 1
    }

    /// Check if this node contains a specific page
    pub fn contains_page(&self, page: usize) -> bool {
        page >= self.start_page && page <= self.end_page
    }

    /// Count total nodes in this subtree
    pub fn count_nodes(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_nodes()).sum::<usize>()
    }

    /// Get maximum depth in this subtree
    pub fn max_depth(&self) -> usize {
        if self.children.is_empty() {
            self.depth
        } else {
            self.children
                .iter()
                .map(|c| c.max_depth())
                .max()
                .unwrap_or(self.depth)
        }
    }

    /// Find a node by ID
    pub fn find_by_id(&self, id: &NodeId) -> Option<&IndexNode> {
        if &self.id == id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a node by ID (mutable)
    pub fn find_by_id_mut(&mut self, id: &NodeId) -> Option<&mut IndexNode> {
        if &self.id == id {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_by_id_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Collect all leaf nodes
    pub fn collect_leaves(&self) -> Vec<&IndexNode> {
        if self.children.is_empty() {
            vec![self]
        } else {
            self.children
                .iter()
                .flat_map(|c| c.collect_leaves())
                .collect()
        }
    }

    /// Convert to a flat list of all nodes (depth-first)
    pub fn flatten(&self) -> Vec<&IndexNode> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.flatten());
        }
        result
    }

    /// Generate a table-of-contents string
    pub fn to_toc(&self) -> String {
        let indent = "  ".repeat(self.depth);
        let mut lines = vec![format!(
            "{}{} (pp. {}-{}): {}",
            indent,
            self.title,
            self.start_page,
            self.end_page,
            if self.summary.is_empty() {
                ""
            } else {
                &self.summary[..self.summary.len().min(60)]
            }
        )];
        for child in &self.children {
            lines.push(child.to_toc());
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = IndexNode::new("Introduction", 1, 5);
        assert_eq!(node.title, "Introduction");
        assert_eq!(node.start_page, 1);
        assert_eq!(node.end_page, 5);
        assert!(node.children.is_empty());
    }

    #[test]
    fn test_node_with_summary() {
        let node = IndexNode::new("Introduction", 1, 5)
            .with_summary("This chapter introduces the concepts.");
        assert!(!node.summary.is_empty());
    }

    #[test]
    fn test_add_child() {
        let mut parent = IndexNode::new("Chapter 1", 1, 20);
        let child = IndexNode::new("Section 1.1", 1, 10);
        parent.add_child(child);

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].depth, 1);
    }

    #[test]
    fn test_contains_page() {
        let node = IndexNode::new("Chapter", 10, 20);
        assert!(node.contains_page(10));
        assert!(node.contains_page(15));
        assert!(node.contains_page(20));
        assert!(!node.contains_page(9));
        assert!(!node.contains_page(21));
    }

    #[test]
    fn test_count_nodes() {
        let mut root = IndexNode::new("Root", 1, 100);
        root.add_child(IndexNode::new("Child 1", 1, 50));
        root.add_child(IndexNode::new("Child 2", 51, 100));

        assert_eq!(root.count_nodes(), 3);
    }

    #[test]
    fn test_find_by_id() {
        let mut root = IndexNode::new("Root", 1, 100);
        let child = IndexNode::new("Child", 1, 50);
        let child_id = child.id.clone();
        root.add_child(child);

        assert!(root.find_by_id(&child_id).is_some());
        assert!(root.find_by_id(&NodeId::new()).is_none());
    }

    #[test]
    fn test_collect_leaves() {
        let mut root = IndexNode::new("Root", 1, 100);
        root.add_child(IndexNode::new("Leaf 1", 1, 50));
        root.add_child(IndexNode::new("Leaf 2", 51, 100));

        let leaves = root.collect_leaves();
        assert_eq!(leaves.len(), 2);
    }

    #[test]
    fn test_flatten() {
        let mut root = IndexNode::new("Root", 1, 100);
        root.add_child(IndexNode::new("Child 1", 1, 50));
        root.add_child(IndexNode::new("Child 2", 51, 100));

        let flat = root.flatten();
        assert_eq!(flat.len(), 3);
    }

    #[test]
    fn test_to_toc() {
        let mut root = IndexNode::new("Book", 1, 100)
            .with_summary("A book about Rust.");
        root.add_child(IndexNode::new("Chapter 1", 1, 50));

        let toc = root.to_toc();
        assert!(toc.contains("Book"));
        assert!(toc.contains("Chapter 1"));
    }
}