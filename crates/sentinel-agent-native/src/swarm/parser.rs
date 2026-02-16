//! LLM Response Parser
//!
//! Parses LLM responses to extract code blocks, file paths, and metadata.
//! Supports multiple code block formats and language detection.

use regex::Regex;
use std::collections::HashMap;

/// Parsed file from LLM response
#[derive(Debug, Clone)]
pub struct ParsedFile {
    /// File path (extracted from markdown or comment)
    pub path: String,
    /// File content
    pub content: String,
    /// Language detected
    pub language: String,
    /// Whether this is a complete file or partial
    pub is_complete: bool,
}

/// LLM response parser
pub struct LLMResponseParser;

impl LLMResponseParser {
    /// Parse response and extract all files
    pub fn parse(response: &str) -> Vec<ParsedFile> {
        let mut files = Vec::new();

        // Extract code blocks with file paths
        files.extend(Self::extract_markdown_code_blocks(response));

        // Extract files from comments (e.g., // filepath: src/main.rs)
        files.extend(Self::extract_files_from_comments(response));

        // Extract XML-style file blocks
        files.extend(Self::extract_xml_file_blocks(response));

        // Deduplicate by path
        let mut seen = HashMap::new();
        files.retain(|f| {
            if seen.contains_key(&f.path) {
                false
            } else {
                seen.insert(f.path.clone(), true);
                true
            }
        });

        files
    }

    /// Extract markdown code blocks with file paths in headers
    fn extract_markdown_code_blocks(response: &str) -> Vec<ParsedFile> {
        let mut files = Vec::new();

        // Pattern: ```language:filepath or ```filepath
        // Also captures: ```rust:src/main.rs or ```src/main.rs
        // (?s) makes . match newlines for multiline code blocks
        let code_block_regex = Regex::new(r"(?s)```(?:(\w+):)?([^\n\r]+)?\n(.*?)```").unwrap();

        for cap in code_block_regex.captures_iter(response) {
            let language = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "text".to_string());

            let path = cap
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .filter(|p| !p.is_empty() && Self::looks_like_filepath(p))
                .unwrap_or_else(|| {
                    // Try to detect language from code content
                    let content = cap.get(3).map(|m| m.as_str()).unwrap_or("");
                    format!("generated.{})", Self::extension_from_language(&language))
                });

            let content = cap
                .get(3)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            files.push(ParsedFile {
                path,
                content: content.trim().to_string(),
                language,
                is_complete: true,
            });
        }

        files
    }

    /// Extract files from special comments
    fn extract_files_from_comments(response: &str) -> Vec<ParsedFile> {
        let mut files = Vec::new();

        // Pattern: // File: src/main.rs or # File: src/main.py
        // Capture everything until next File comment or end
        let file_comment_regex =
            Regex::new(r"(?:^|\n)(?://|#|<!--)\s*(?:File|filepath|path):\s*([^\n]+)\n").unwrap();

        // Find all file comment positions
        let matches: Vec<_> = file_comment_regex.captures_iter(response).collect();

        for (i, cap) in matches.iter().enumerate() {
            let path = cap
                .get(1)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            if path.is_empty() || !Self::looks_like_filepath(&path) {
                continue;
            }

            // Get the start position of content (after the File comment line)
            let mat = cap.get(0).unwrap();
            let content_start = mat.end();

            // Get the end position (start of next File comment or end of string)
            let content_end = if i + 1 < matches.len() {
                matches[i + 1].get(0).unwrap().start()
            } else {
                response.len()
            };

            let content = response[content_start..content_end].to_string();
            let language = Self::detect_language(&path);

            files.push(ParsedFile {
                path,
                content: content.trim().to_string(),
                language,
                is_complete: true,
            });
        }

        files
    }

    /// Extract XML-style file blocks
    fn extract_xml_file_blocks(response: &str) -> Vec<ParsedFile> {
        let mut files = Vec::new();

        // Pattern: <file path="src/main.rs">...</file>
        // Note: Using simpler pattern to avoid regex complexity
        if let Some(start) = response.find("<file path=") {
            if let Some(end) = response[start..].find("</file>") {
                let block = &response[start..start + end + 7];
                if let Some(path_start) = block.find("\"") {
                    if let Some(path_end) = block[path_start + 1..].find("\"") {
                        let path = block[path_start + 1..path_start + 1 + path_end].to_string();
                        if let Some(content_start) = block.find(">") {
                            let content = block[content_start + 1..block.len() - 7].to_string();

                            if !path.is_empty() {
                                let language = Self::detect_language(&path);
                                files.push(ParsedFile {
                                    path,
                                    content: content.trim().to_string(),
                                    language,
                                    is_complete: true,
                                });
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Check if string looks like a file path
    fn looks_like_filepath(s: &str) -> bool {
        // Contains path separators or common extensions
        s.contains('/') || s.contains('.') || s.contains("\\")
    }

    /// Detect language from file path
    fn detect_language(path: &str) -> String {
        let ext = path.split('.').last().unwrap_or("");

        match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "tsx" => "typescript",
            "jsx" => "javascript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" => "c",
            "h" | "hpp" => "cpp",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" => "kotlin",
            "scala" => "scala",
            "r" => "r",
            "sql" => "sql",
            "sh" => "bash",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            "md" => "markdown",
            "html" => "html",
            "css" => "css",
            "scss" | "sass" => "scss",
            _ => "text",
        }
        .to_string()
    }

    /// Get file extension from language name
    fn extension_from_language(language: &str) -> String {
        match language.to_lowercase().as_str() {
            "rust" => "rs",
            "python" => "py",
            "javascript" => "js",
            "typescript" => "ts",
            "go" => "go",
            "java" => "java",
            "cpp" | "c++" => "cpp",
            "c" => "c",
            "ruby" => "rb",
            "php" => "php",
            "swift" => "swift",
            "kotlin" => "kt",
            "scala" => "scala",
            "r" => "r",
            "sql" => "sql",
            "bash" | "shell" => "sh",
            "yaml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            "markdown" => "md",
            "html" => "html",
            "css" => "css",
            "scss" => "scss",
            _ => "txt",
        }
        .to_string()
    }

    /// Extract thinking/reasoning sections from response
    pub fn extract_thinking(response: &str) -> Option<String> {
        // Pattern: <thinking>...</thinking> or [THINKING]...[/THINKING]
        // (?s) makes . match newlines for multiline thinking blocks
        let thinking_regex =
            Regex::new(r"(?s)<thinking>(.*?)</thinking>|\[THINKING\](.*?)\[/THINKING\]").unwrap();

        thinking_regex.captures(response).map(|cap| {
            cap.get(1)
                .or(cap.get(2))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default()
        })
    }

    /// Clean response (remove thinking blocks for display)
    pub fn clean_response(response: &str) -> String {
        // (?s) makes . match newlines for multiline thinking blocks
        let thinking_regex =
            Regex::new(r"(?s)<thinking>.*?</thinking>|\[THINKING\].*?\[/THINKING\]").unwrap();

        thinking_regex.replace_all(response, "").trim().to_string()
    }
}

/// Convenience function to parse LLM response
pub fn parse_llm_response(response: &str) -> Vec<ParsedFile> {
    LLMResponseParser::parse(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown_code_blocks() {
        let response = r#"
Here's the authentication code:

```rust:src/auth.rs
use jsonwebtoken::{encode, decode};

pub fn generate_token() -> String {
    "token".to_string()
}
```

And the main file:

```rust:src/main.rs
fn main() {
    println!("Hello");
}
```
"#;

        let files = parse_llm_response(response);
        assert_eq!(files.len(), 2);

        assert_eq!(files[0].path, "src/auth.rs");
        assert_eq!(files[0].language, "rust");
        assert!(files[0].content.contains("generate_token"));

        assert_eq!(files[1].path, "src/main.rs");
        assert_eq!(files[1].language, "rust");
    }

    #[test]
    fn test_parse_file_comments() {
        let response = r#"
// File: src/utils.py
def helper():
    return 42

// File: src/main.py
import utils
print(utils.helper())
"#;

        let files = parse_llm_response(response);
        assert!(!files.is_empty());

        let python_files: Vec<_> = files.iter().filter(|f| f.path.ends_with(".py")).collect();
        assert!(!python_files.is_empty());
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(LLMResponseParser::detect_language("main.rs"), "rust");
        assert_eq!(LLMResponseParser::detect_language("script.py"), "python");
        assert_eq!(LLMResponseParser::detect_language("app.ts"), "typescript");
        assert_eq!(
            LLMResponseParser::detect_language("server.js"),
            "javascript"
        );
    }

    #[test]
    fn test_extract_thinking() {
        let response = r#"
<thinking>
I need to create a JWT authentication system.
This should include token generation and validation.
</thinking>

Here's the code:
```rust
pub fn generate_token() {}
```
"#;

        let thinking = LLMResponseParser::extract_thinking(response);
        assert!(thinking.is_some());
        assert!(thinking.unwrap().contains("JWT authentication"));
    }

    #[test]
    fn test_clean_response() {
        let response = r#"
<thinking>
Internal reasoning here
</thinking>

Actual code here.
"#;

        let cleaned = LLMResponseParser::clean_response(response);
        assert!(!cleaned.contains("thinking"));
        assert!(cleaned.contains("Actual code"));
    }

    #[test]
    fn test_no_duplicates() {
        let response = r#"
```rust:src/main.rs
fn main() {}
```

```rust:src/main.rs
fn main() { updated }
```
"#;

        let files = parse_llm_response(response);
        assert_eq!(files.len(), 1); // Should deduplicate
        assert_eq!(files[0].path, "src/main.rs");
    }
}
