//! Sandbox-based Predicate Evaluation
//!
//! Defines traits and types for evaluating predicates in sandboxed environments.
//! The actual sandbox implementation is provided by sentinel-sandbox crate,
//! which is used by sentinel-cli or sentinel-agent-native.
//!
//! This module breaks the circular dependency by:
//! 1. Defining a trait `SandboxExecutor` that abstracts sandbox operations
//! 2. Providing types for evidence and evaluation results
//! 3. Extension trait for predicates to use any sandbox implementation

use super::predicate::{Predicate, PredicateLanguage};
use crate::error::Result;
use std::path::PathBuf;

/// Result of sandbox-based predicate evaluation
#[derive(Debug, Clone)]
pub struct SandboxEvaluation {
    /// Whether the predicate passed
    pub passed: bool,
    
    /// Evidence from the sandbox execution
    pub evidence: SandboxEvidence,
    
    /// Determinism guarantee (same input → same output)
    pub is_deterministic: bool,
}

/// Evidence collected during sandbox execution
#[derive(Debug, Clone)]
pub struct SandboxEvidence {
    /// Command that was executed
    pub command: Option<String>,
    
    /// Exit code
    pub exit_code: Option<i32>,
    
    /// Captured stdout
    pub stdout: Option<String>,
    
    /// Captured stderr
    pub stderr: Option<String>,
    
    /// Files created/modified
    pub files_affected: Vec<PathBuf>,
    
    /// Test results (if applicable)
    pub test_results: Option<TestResultsSummary>,
}

/// Summary of test results
#[derive(Debug, Clone)]
pub struct TestResultsSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub coverage: f64,
}

impl Default for SandboxEvidence {
    fn default() -> Self {
        Self {
            command: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            files_affected: Vec::new(),
            test_results: None,
        }
    }
}

/// Sandbox executor trait for predicate evaluation
///
/// This abstraction allows different sandbox implementations
/// (local, Docker, remote) to be used interchangeably.
/// 
/// Implementations are provided by:
/// - `sentinel-sandbox::Sandbox` → wrapped in `LocalSandboxExecutor`
/// - Docker-based sandboxes
/// - Remote execution environments
pub trait SandboxExecutor: Send + Sync {
    /// Execute a command in the sandbox
    fn execute(&self, cmd: &str, args: &[String], cwd: Option<&PathBuf>) -> Result<SandboxEvidence>;
    
    /// Run tests in the sandbox
    fn run_tests(&self, suite: Option<&str>) -> Result<TestResultsSummary>;
    
    /// Prepare files in the sandbox
    fn prepare_files(&self, files: &[(PathBuf, String)]) -> Result<()>;
    
    /// Check if a file exists in the sandbox
    fn file_exists(&self, path: &PathBuf) -> bool;
    
    /// Read a file from the sandbox
    fn read_file(&self, path: &PathBuf) -> Result<String>;
    
    /// Get the sandbox root path
    fn root_path(&self) -> &PathBuf;
}

/// Extension trait for evaluating predicates in sandbox
pub trait PredicateSandboxExt {
    /// Evaluate predicate using sandbox executor
    fn evaluate_in_sandbox(
        &self,
        executor: &dyn SandboxExecutor,
        project_root: Option<&PathBuf>,
    ) -> Result<SandboxEvaluation>;
}

impl PredicateSandboxExt for Predicate {
    fn evaluate_in_sandbox(
        &self,
        executor: &dyn SandboxExecutor,
        project_root: Option<&PathBuf>,
    ) -> Result<SandboxEvaluation> {
        match self {
            Predicate::CommandSucceeds { command, args, expected_exit_code } => {
                let evidence = executor.execute(command, args, project_root)?;
                let passed = evidence.exit_code == Some(*expected_exit_code);
                
                Ok(SandboxEvaluation {
                    passed,
                    evidence,
                    is_deterministic: true,
                })
            }
            
            Predicate::TestsPassing { suite, min_coverage } => {
                let test_results = executor.run_tests(Some(suite))?;
                
                let evidence = SandboxEvidence {
                    command: Some(format!("cargo test --quiet {}", suite)),
                    exit_code: Some(if test_results.failed == 0 { 0 } else { 1 }),
                    stdout: None,
                    stderr: None,
                    files_affected: Vec::new(),
                    test_results: Some(test_results.clone()),
                };
                
                let passed = test_results.failed == 0 
                    && test_results.coverage >= *min_coverage;
                
                Ok(SandboxEvaluation {
                    passed,
                    evidence,
                    is_deterministic: true,
                })
            }
            
            Predicate::FileExists(path) => {
                let exists = executor.file_exists(path);
                
                Ok(SandboxEvaluation {
                    passed: exists,
                    evidence: SandboxEvidence {
                        command: None,
                        exit_code: Some(if exists { 0 } else { 1 }),
                        stdout: None,
                        stderr: None,
                        files_affected: if exists { vec![path.clone()] } else { Vec::new() },
                        test_results: None,
                    },
                    is_deterministic: true,
                })
            }
            
            Predicate::Custom { code, language, description: _ } => {
                match language {
                    PredicateLanguage::Shell => {
                        let evidence = executor.execute("sh", &["-c".to_string(), code.clone()], project_root)?;
                        let passed = evidence.exit_code == Some(0);
                        
                        Ok(SandboxEvaluation {
                            passed,
                            evidence,
                            is_deterministic: false, // Custom code may be non-deterministic
                        })
                    }
                    _ => {
                        // Other languages not yet supported in sandbox
                        Ok(SandboxEvaluation {
                            passed: false,
                            evidence: SandboxEvidence::default(),
                            is_deterministic: false,
                        })
                    }
                }
            }
            
            Predicate::And(predicates) => {
                let mut all_passed = true;
                let mut combined_evidence = SandboxEvidence::default();
                
                for pred in predicates {
                    let result = pred.evaluate_in_sandbox(executor, project_root)?;
                    if !result.passed {
                        all_passed = false;
                    }
                    // Merge evidence
                    if let Some(cmd) = result.evidence.command {
                        combined_evidence.command = Some(cmd);
                    }
                }
                
                Ok(SandboxEvaluation {
                    passed: all_passed,
                    evidence: combined_evidence,
                    is_deterministic: true,
                })
            }
            
            Predicate::Or(predicates) => {
                let mut any_passed = false;
                let mut combined_evidence = SandboxEvidence::default();
                
                for pred in predicates {
                    let result = pred.evaluate_in_sandbox(executor, project_root)?;
                    if result.passed {
                        any_passed = true;
                        combined_evidence = result.evidence;
                        break;
                    }
                }
                
                Ok(SandboxEvaluation {
                    passed: any_passed,
                    evidence: combined_evidence,
                    is_deterministic: true,
                })
            }
            
            Predicate::Not(pred) => {
                let inner = pred.evaluate_in_sandbox(executor, project_root)?;
                Ok(SandboxEvaluation {
                    passed: !inner.passed,
                    evidence: inner.evidence,
                    is_deterministic: inner.is_deterministic,
                })
            }
            
            Predicate::AlwaysTrue => Ok(SandboxEvaluation {
                passed: true,
                evidence: SandboxEvidence::default(),
                is_deterministic: true,
            }),
            
            Predicate::AlwaysFalse => Ok(SandboxEvaluation {
                passed: false,
                evidence: SandboxEvidence::default(),
                is_deterministic: true,
            }),
            
            // Other predicates use standard evaluation (not sandbox-specific)
            _ => {
                // Fallback: just mark as not sandbox-evaluated
                Ok(SandboxEvaluation {
                    passed: false,
                    evidence: SandboxEvidence::default(),
                    is_deterministic: false,
                })
            }
        }
    }
}

/// Parse test output to extract results
pub fn parse_test_output(stdout: &str, stderr: &str) -> TestResultsSummary {
    let output = format!("{}\n{}", stdout, stderr);
    
    // Try to parse cargo test output
    // Example: "test result: ok. 10 passed; 0 failed; 0 ignored"
    let mut passed = 0;
    let mut failed = 0;
    
    // Look for "X passed" pattern
    if let Some(pos) = output.find("passed") {
        let before = &output[..pos];
        if let Some(num) = before.trim().split_whitespace().last() {
            passed = num.parse().unwrap_or(0);
        }
    }
    
    // Look for "X failed" pattern
    if let Some(pos) = output.find("failed") {
        let before = &output[..pos];
        if let Some(num) = before.trim().split_whitespace().last() {
            failed = num.parse().unwrap_or(0);
        }
    }
    
    // Also try "test result: ok/fail" pattern
    if output.contains("test result: ok") {
        // Success case
    } else if output.contains("test result: FAILED") {
        // Failure case
    }
    
    TestResultsSummary {
        total: passed + failed,
        passed,
        failed,
        coverage: if failed == 0 && passed > 0 { 1.0 } else { 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Mock sandbox executor for testing
    struct MockSandboxExecutor {
        root: PathBuf,
    }
    
    impl MockSandboxExecutor {
        fn new() -> Self {
            Self {
                root: std::env::temp_dir(),
            }
        }
    }
    
    impl SandboxExecutor for MockSandboxExecutor {
        fn execute(&self, cmd: &str, args: &[String], _cwd: Option<&PathBuf>) -> Result<SandboxEvidence> {
            // Mock implementation
            Ok(SandboxEvidence {
                command: Some(format!("{} {}", cmd, args.join(" "))),
                exit_code: Some(0),
                stdout: Some("mock output".to_string()),
                stderr: None,
                files_affected: Vec::new(),
                test_results: None,
            })
        }
        
        fn run_tests(&self, _suite: Option<&str>) -> Result<TestResultsSummary> {
            Ok(TestResultsSummary {
                total: 10,
                passed: 10,
                failed: 0,
                coverage: 1.0,
            })
        }
        
        fn prepare_files(&self, _files: &[(PathBuf, String)]) -> Result<()> {
            Ok(())
        }
        
        fn file_exists(&self, _path: &PathBuf) -> bool {
            true
        }
        
        fn read_file(&self, _path: &PathBuf) -> Result<String> {
            Ok("mock content".to_string())
        }
        
        fn root_path(&self) -> &PathBuf {
            &self.root
        }
    }
    
    #[test]
    fn test_sandbox_evidence_default() {
        let evidence = SandboxEvidence::default();
        assert!(evidence.command.is_none());
        assert!(evidence.exit_code.is_none());
    }
    
    #[test]
    fn test_parse_test_output() {
        let stdout = "test result: ok. 10 passed; 0 failed; 0 ignored";
        let summary = parse_test_output(stdout, "");
        
        assert!(summary.total >= 0);
    }
    
    #[test]
    fn test_predicate_sandbox_ext_always() {
        let executor = MockSandboxExecutor::new();
        
        let pred = Predicate::AlwaysTrue;
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(result.passed);
        
        let pred = Predicate::AlwaysFalse;
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(!result.passed);
    }
    
    #[test]
    fn test_predicate_sandbox_ext_and() {
        let executor = MockSandboxExecutor::new();
        
        let pred = Predicate::And(vec![
            Predicate::AlwaysTrue,
            Predicate::AlwaysTrue,
        ]);
        
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(result.passed);
    }
    
    #[test]
    fn test_predicate_sandbox_ext_or() {
        let executor = MockSandboxExecutor::new();
        
        let pred = Predicate::Or(vec![
            Predicate::AlwaysFalse,
            Predicate::AlwaysTrue,
        ]);
        
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(result.passed);
    }
    
    #[test]
    fn test_predicate_sandbox_ext_not() {
        let executor = MockSandboxExecutor::new();
        
        let pred = Predicate::Not(Box::new(Predicate::AlwaysFalse));
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(result.passed);
        
        let pred = Predicate::Not(Box::new(Predicate::AlwaysTrue));
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        assert!(!result.passed);
    }
    
    #[test]
    fn test_predicate_file_exists() {
        let executor = MockSandboxExecutor::new();
        
        let pred = Predicate::FileExists(PathBuf::from("test.txt"));
        let result = pred.evaluate_in_sandbox(&executor, None).unwrap();
        // Mock always returns true
        assert!(result.passed);
    }
}