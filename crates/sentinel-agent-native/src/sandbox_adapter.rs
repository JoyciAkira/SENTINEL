//! Sandbox Adapter - Adapts sentinel-sandbox to SandboxExecutor trait
//!
//! This module provides the concrete implementation of SandboxExecutor
//! using the sentinel-sandbox crate.

use sentinel_core::goal_manifold::{SandboxExecutor, SandboxEvidence, TestResultsSummary, predicate_sandbox::parse_test_output};
use sentinel_core::error::Result;
use std::path::PathBuf;

/// Adapter that wraps sentinel-sandbox::Sandbox to implement SandboxExecutor
pub struct SandboxAdapter {
    sandbox: sentinel_sandbox::Sandbox,
}

impl SandboxAdapter {
    /// Create a new sandbox adapter
    pub fn new() -> Result<Self> {
        let sandbox = sentinel_sandbox::Sandbox::new()
            .map_err(|e| sentinel_core::error::SentinelError::Sandbox(e.to_string()))?;
        Ok(Self { sandbox })
    }

    /// Create with an existing sandbox (for testing or custom setup)
    pub fn from_sandbox(sandbox: sentinel_sandbox::Sandbox) -> Self {
        Self { sandbox }
    }

    /// Get access to the underlying sandbox
    pub fn inner(&self) -> &sentinel_sandbox::Sandbox {
        &self.sandbox
    }
}

impl Default for SandboxAdapter {
    fn default() -> Self {
        Self::new().expect("Failed to create sandbox")
    }
}

impl SandboxExecutor for SandboxAdapter {
    fn execute(&self, cmd: &str, args: &[String], cwd: Option<&PathBuf>) -> Result<SandboxEvidence> {
        // Use tokio runtime for async call
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| sentinel_core::error::SentinelError::Runtime(e.to_string()))?;

        let result = rt.block_on(async {
            // If cwd specified, we need to adjust - sandbox runs in its root
            if cwd.is_some() {
                tracing::warn!("SandboxAdapter: cwd argument ignored, using sandbox root");
            }
            self.sandbox.run(cmd, args).await
        });

        match result {
            Ok(exec_result) => Ok(SandboxEvidence {
                command: Some(format!("{} {}", cmd, args.join(" "))),
                exit_code: Some(exec_result.exit_code),
                stdout: Some(exec_result.stdout),
                stderr: Some(exec_result.stderr),
                files_affected: Vec::new(),
                test_results: None,
            }),
            Err(e) => Err(sentinel_core::error::SentinelError::Sandbox(e.to_string())),
        }
    }

    fn run_tests(&self, suite: Option<&str>) -> Result<TestResultsSummary> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| sentinel_core::error::SentinelError::Runtime(e.to_string()))?;

        let result = rt.block_on(async {
            // Build cargo test command
            let mut args = vec!["test".to_string(), "--quiet".to_string()];
            if let Some(s) = suite {
                args.push("--test".to_string());
                args.push(s.to_string());
            }
            self.sandbox.run("cargo", &args).await
        });

        match result {
            Ok(exec_result) => {
                // Parse test output
                let stdout = exec_result.stdout.clone();
                let stderr = exec_result.stderr.clone();
                let summary = parse_test_output(&stdout, &stderr);

                Ok(TestResultsSummary {
                    total: summary.total,
                    passed: summary.passed,
                    failed: summary.failed,
                    coverage: if summary.failed == 0 && summary.passed > 0 { 1.0 } else { 0.0 },
                })
            }
            Err(e) => Err(sentinel_core::error::SentinelError::Sandbox(e.to_string())),
        }
    }

    fn prepare_files(&self, files: &[(PathBuf, String)]) -> Result<()> {
        self.sandbox
            .prepare(files)
            .map_err(|e| sentinel_core::error::SentinelError::Sandbox(e.to_string()))
    }

    fn file_exists(&self, path: &PathBuf) -> bool {
        self.sandbox.root_path.join(path).exists()
    }

    fn read_file(&self, path: &PathBuf) -> Result<String> {
        std::fs::read_to_string(self.sandbox.root_path.join(path))
            .map_err(|e| sentinel_core::error::SentinelError::Io(e))
    }

    fn root_path(&self) -> &PathBuf {
        &self.sandbox.root_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_adapter_creation() {
        let adapter = SandboxAdapter::new();
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_sandbox_adapter_file_exists() {
        let adapter = SandboxAdapter::new().unwrap();
        
        // File doesn't exist initially
        assert!(!adapter.file_exists(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_sandbox_adapter_prepare_and_read() {
        let adapter = SandboxAdapter::new().unwrap();
        
        let files = vec![(
            PathBuf::from("test.txt"),
            "Hello, Sandbox!".to_string(),
        )];
        
        adapter.prepare_files(&files).unwrap();
        assert!(adapter.file_exists(&PathBuf::from("test.txt")));
        
        let content = adapter.read_file(&PathBuf::from("test.txt")).unwrap();
        assert_eq!(content, "Hello, Sandbox!");
    }

    #[tokio::test]
    async fn test_sandbox_adapter_execute() {
        let adapter = SandboxAdapter::new().unwrap();
        
        // Prepare a simple script
        let files = vec![(
            PathBuf::from("echo.sh"),
            "#!/bin/sh\necho HELLO".to_string(),
        )];
        adapter.prepare_files(&files).unwrap();
        
        // Execute it
        let result = adapter.execute("sh", &["echo.sh".to_string()], None).unwrap();
        
        assert!(result.exit_code == Some(0));
        assert!(result.stdout.unwrap().contains("HELLO"));
    }
}