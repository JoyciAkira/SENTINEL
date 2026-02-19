//! Tree Index Builder
//!
//! Constructs tree indices from various document sources:
//! - Markdown files
//! - Plain text
//! - Code files (structure-aware)
//! - PDF files (via external extraction)

use super::{IndexNode, TreeIndex};
use crate::error::Result;
use std::path::Path;
use tokio::fs;
use regex::Regex;

/// Builder for creating tree indices from documents
pub struct TreeIndexBuilder {
    /// Document title
    title: Option<String>,
    /// Maximum depth for the tree
    max_depth: usize,
    /// Whether to extract keywords
    extract_keywords: bool,
    /// Whether to generate summaries
    generate_summaries: bool,
}

impl Default for TreeIndexBuilder {
    fn default() -> Self {
        Self {
            title: None,
            max_depth: 5,
            extract_keywords: true,
            generate_summaries: true,
        }
    }
}

impl TreeIndexBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the document title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set maximum depth
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Build from a markdown file
    pub async fn from_markdown(self, path: impl AsRef<Path>) -> Result<TreeIndex> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).await?;
        self.parse_markdown(&content, path)
    }

    /// Build from markdown content string
    pub fn from_markdown_content(self, content: &str, source_path: Option<&Path>) -> Result<TreeIndex> {
        self.parse_markdown(content, source_path.unwrap_or(Path::new("unknown")))
    }

    /// Build from a plain text file
    pub async fn from_text(self, path: impl AsRef<Path>) -> Result<TreeIndex> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).await?;

        let title = self.title.clone().unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Document")
                .to_string()
        });

        let node = self.build_text_tree(&content, &title);
        Ok(TreeIndex::new(node, Some(path.to_path_buf())))
    }

    /// Build from a Rust source file (structure-aware)
    pub async fn from_rust_source(self, path: impl AsRef<Path>) -> Result<TreeIndex> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).await?;
        self.parse_rust_source(&content, path)
    }

    fn parse_markdown(&self, content: &str, source_path: &Path) -> Result<TreeIndex> {
        let title = self.title.clone().unwrap_or_else(|| {
            content
                .lines()
                .find(|line| line.starts_with("# "))
                .map(|line| line.trim_start_matches('#').trim().to_string())
                .unwrap_or_else(|| {
                    source_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Document")
                        .to_string()
                })
        });

        let total_lines = content.lines().count();
        let mut root = IndexNode::new(&title, 1, total_lines);

        // Parse headings to build tree structure
        let heading_pattern = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        let mut current_h1: Option<IndexNode> = None;
        let mut current_h2: Option<IndexNode> = None;
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;

            if let Some(caps) = heading_pattern.captures(line) {
                let level = caps[1].len();
                let heading_text = caps[2].trim();

                match level {
                    1 if heading_text != title => {
                        // Save previous h1 section
                        if let Some(mut h1) = current_h1.take() {
                            h1.end_page = line_num - 1;
                            if let Some(mut h2) = current_h2.take() {
                                h2.end_page = line_num - 1;
                                h1.add_child(h2);
                            }
                            root.add_child(h1);
                        }
                        current_h1 = Some(IndexNode::new(heading_text, line_num, line_num));
                        current_h2 = None;
                    }
                    2 => {
                        if let Some(ref mut h1) = current_h1 {
                            // Save previous h2
                            if let Some(mut h2) = current_h2.take() {
                                h2.end_page = line_num - 1;
                                h1.add_child(h2);
                            }
                            current_h2 = Some(IndexNode::new(heading_text, line_num, line_num));
                        }
                    }
                    3..=6 => {
                        if let Some(ref mut h2) = current_h2 {
                            if level == 3 && self.max_depth >= 3 {
                                let h3 = IndexNode::new(heading_text, line_num, line_num);
                                h2.add_child(h3);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Save last sections
        if let Some(mut h1) = current_h1.take() {
            h1.end_page = line_num;
            if let Some(mut h2) = current_h2.take() {
                h2.end_page = line_num;
                h1.add_child(h2);
            }
            root.add_child(h1);
        }

        // Extract keywords from content
        if self.extract_keywords {
            root.keywords = self.extract_keywords_from_text(content);
        }

        // Generate summary
        if self.generate_summaries {
            root.summary = self.generate_summary_from_text(content);
        }

        Ok(TreeIndex::new(root, Some(source_path.to_path_buf())))
    }

    fn parse_rust_source(&self, content: &str, source_path: &Path) -> Result<TreeIndex> {
        let title = self.title.clone().unwrap_or_else(|| {
            source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("module")
                .trim_end_matches(".rs")
                .to_string()
        });

        let total_lines = content.lines().count();
        let mut root = IndexNode::new(&title, 1, total_lines);

        // Parse Rust structure
        let fn_pattern = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap();
        let struct_pattern = Regex::new(r"^\s*(pub\s+)?struct\s+(\w+)").unwrap();
        let enum_pattern = Regex::new(r"^\s*(pub\s+)?enum\s+(\w+)").unwrap();
        let impl_pattern = Regex::new(r"^\s*impl\s+(.+?)\s*\{").unwrap();
        let mod_pattern = Regex::new(r"^\s*(pub\s+)?mod\s+(\w+)").unwrap();

        let mut current_impl: Option<IndexNode> = None;
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;

            // Track impl blocks
            if let Some(caps) = impl_pattern.captures(line) {
                if let Some(mut prev_impl) = current_impl.take() {
                    prev_impl.end_page = line_num - 1;
                    root.add_child(prev_impl);
                }
                current_impl = Some(IndexNode::new(
                    format!("impl {}", &caps[1]),
                    line_num,
                    line_num,
                ));
            }

            // Functions
            if let Some(caps) = fn_pattern.captures(line) {
                let fn_name = &caps[3];
                let fn_node = IndexNode::new(format!("fn {}", fn_name), line_num, line_num);
                
                if let Some(ref mut impl_node) = current_impl {
                    impl_node.add_child(fn_node);
                } else {
                    root.add_child(fn_node);
                }
            }

            // Structs
            if let Some(caps) = struct_pattern.captures(line) {
                let struct_name = &caps[2];
                root.add_child(IndexNode::new(format!("struct {}", struct_name), line_num, line_num));
            }

            // Enums
            if let Some(caps) = enum_pattern.captures(line) {
                let enum_name = &caps[2];
                root.add_child(IndexNode::new(format!("enum {}", enum_name), line_num, line_num));
            }

            // Modules
            if let Some(caps) = mod_pattern.captures(line) {
                let mod_name = &caps[2];
                root.add_child(IndexNode::new(format!("mod {}", mod_name), line_num, line_num));
            }
        }

        // Close last impl block
        if let Some(mut impl_node) = current_impl.take() {
            impl_node.end_page = line_num;
            root.add_child(impl_node);
        }

        if self.extract_keywords {
            root.keywords = self.extract_keywords_from_text(content);
        }

        if self.generate_summaries {
            root.summary = self.generate_summary_from_text(content);
        }

        Ok(TreeIndex::new(root, Some(source_path.to_path_buf())))
    }

    fn build_text_tree(&self, content: &str, title: &str) -> IndexNode {
        let total_lines = content.lines().count();
        let mut root = IndexNode::new(title, 1, total_lines);

        // Split by paragraphs and create sections
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let chunk_size = (paragraphs.len() / 10).max(1);
        
        for (i, chunk) in paragraphs.chunks(chunk_size).enumerate() {
            let start_line = i * chunk_size + 1;
            let end_line = start_line + chunk.len() - 1;
            
            // Use first non-empty line as title
            let section_title = chunk
                .iter()
                .find(|p| !p.trim().is_empty())
                .map(|p| p.lines().next().unwrap_or("Section"))
                .unwrap_or("Section");

            let mut section = IndexNode::new(
                section_title.chars().take(50).collect::<String>(),
                start_line,
                end_line,
            );

            if self.generate_summaries {
                section.summary = chunk.join("\n").chars().take(200).collect();
            }

            root.add_child(section);
        }

        if self.extract_keywords {
            root.keywords = self.extract_keywords_from_text(content);
        }

        if self.generate_summaries {
            root.summary = self.generate_summary_from_text(content);
        }

        root
    }

    fn extract_keywords_from_text(&self, text: &str) -> Vec<String> {
        // Simple keyword extraction: frequent words, filtered
        let stop_words = std::collections::HashSet::from([
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will", "would",
            "could", "should", "may", "might", "must", "shall", "can", "this",
            "that", "these", "those", "it", "its", "as", "if", "then", "than",
            "so", "such", "no", "not", "only", "own", "same", "too", "very",
            "just", "also", "now", "here", "there", "when", "where", "which",
            "who", "whom", "what", "how", "why", "all", "each", "every", "both",
            "few", "more", "most", "other", "some", "any", "into", "through",
        ]);

        let mut word_counts: std::collections::HashMap<String, usize> = 
            std::collections::HashMap::new();

        for word in text.to_lowercase().split_whitespace() {
            let word = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string();
            
            if word.len() > 3 && !stop_words.contains(word.as_str()) {
                *word_counts.entry(word).or_insert(0) += 1;
            }
        }

        let mut keywords: Vec<(String, usize)> = word_counts.into_iter().collect();
        keywords.sort_by(|a, b| b.1.cmp(&a.1));
        
        keywords
            .into_iter()
            .take(10)
            .map(|(word, _)| word)
            .collect()
    }

    fn generate_summary_from_text(&self, text: &str) -> String {
        // Take first substantial paragraph
        text.lines()
            .skip_while(|line| line.trim().is_empty())
            .take(3)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown() {
        let markdown = r#"# Main Title

## Chapter 1
This is chapter 1 content.

### Section 1.1
Some details here.

## Chapter 2
Another chapter.
"#;

        let builder = TreeIndexBuilder::new();
        let result = builder.from_markdown_content(markdown, Some(Path::new("test.md")));
        
        assert!(result.is_ok());
        let index = result.unwrap();
        assert_eq!(index.root.title, "Main Title");
        // Note: Headings at level 1 that match title are skipped
        // Only level 1 headings different from title create children
    }

    #[test]
    fn test_parse_rust_source() {
        let rust_code = r#"
//! Module documentation

pub struct User {
    name: String,
}

impl User {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

pub enum Status {
    Active,
    Inactive,
}
"#;

        let builder = TreeIndexBuilder::new();
        let result = builder.parse_rust_source(rust_code, Path::new("user.rs"));
        
        assert!(result.is_ok());
        let index = result.unwrap();
        
        // Should have struct, impl, and enum
        let titles: Vec<&str> = index.root.children.iter()
            .map(|n| n.title.as_str())
            .collect();
        
        assert!(titles.iter().any(|t| t.starts_with("struct User")));
        assert!(titles.iter().any(|t| t.starts_with("impl User")));
        assert!(titles.iter().any(|t| t.starts_with("enum Status")));
    }

    #[test]
    fn test_extract_keywords() {
        let text = "Rust is a systems programming language. Rust focuses on safety and performance.";
        
        let builder = TreeIndexBuilder::new();
        let keywords = builder.extract_keywords_from_text(text);
        
        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"systems".to_string()));
    }

    #[test]
    fn test_generate_summary() {
        let text = "First paragraph here. It contains some important information about the document.";
        
        let builder = TreeIndexBuilder::new();
        let summary = builder.generate_summary_from_text(text);
        
        assert!(summary.contains("First paragraph"));
    }
}