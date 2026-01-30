//! Formal predicates for goal success criteria
//!
//! This module implements formally verifiable predicates that define
//! when a goal is considered complete. Predicates are composable and
//! can be evaluated deterministically.

use crate::error::{PredicateError, Result};
use crate::types::Comparison;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// Formal predicate for success criteria
///
/// Predicates are the foundation of Sentinel's verification system.
/// They must be:
/// - Deterministic: Same input → same output
/// - Verifiable: Can be checked automatically
/// - Composable: Can be combined with AND/OR/NOT
///
/// # Examples
///
/// ```
/// use sentinel_core::goal_manifold::predicate::Predicate;
/// use std::path::PathBuf;
///
/// let pred = Predicate::FileExists(PathBuf::from("src/main.rs"));
/// // Later, evaluate against project state
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Predicate {
    /// File exists at given path
    FileExists(PathBuf),

    /// Directory exists at given path
    DirectoryExists(PathBuf),

    /// Tests pass in a given suite
    TestsPassing {
        /// Suite name
        suite: String,
        /// Minimum coverage required (0.0-1.0)
        min_coverage: f64,
    },

    /// API endpoint responds correctly
    ApiEndpoint {
        /// Endpoint URL
        url: String,
        /// Expected HTTP status code
        expected_status: u16,
        /// Expected string in body
        expected_body_contains: Option<String>,
    },

    /// Performance metric meets threshold
    Performance {
        /// Metric name
        metric: String,
        /// Threshold value
        threshold: f64,
        /// Comparison operator
        comparison: Comparison,
    },

    /// Command executes successfully
    CommandSucceeds {
        /// Command to execute
        command: String,
        /// Arguments for command
        args: Vec<String>,
        /// Expected exit code
        expected_exit_code: i32,
    },

    /// Custom predicate (code that returns bool)
    Custom {
        /// Code to execute
        code: String,
        /// Language of code
        language: PredicateLanguage,
        /// Human-readable description
        description: String,
    },

    /// Logical AND - all predicates must be true
    And(Vec<Predicate>),

    /// Logical OR - at least one predicate must be true
    Or(Vec<Predicate>),

    /// Logical NOT - predicate must be false
    Not(Box<Predicate>),

    /// Always true (for testing)
    AlwaysTrue,

    /// Always false (for testing)
    AlwaysFalse,
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Predicate::FileExists(path) => write!(f, "FileExists({:?})", path),
            Predicate::DirectoryExists(path) => write!(f, "DirectoryExists({:?})", path),
            Predicate::TestsPassing { suite, .. } => write!(f, "TestsPassing({})", suite),
            Predicate::ApiEndpoint { url, .. } => write!(f, "ApiEndpoint({})", url),
            Predicate::Performance { metric, .. } => write!(f, "Performance({})", metric),
            Predicate::CommandSucceeds { command, .. } => write!(f, "CommandSucceeds({})", command),
            Predicate::Custom { description, .. } => write!(f, "Custom({})", description),
            Predicate::And(preds) => write!(f, "And({} items)", preds.len()),
            Predicate::Or(preds) => write!(f, "Or({} items)", preds.len()),
            Predicate::Not(pred) => write!(f, "Not({})", pred),
            Predicate::AlwaysTrue => write!(f, "AlwaysTrue"),
            Predicate::AlwaysFalse => write!(f, "AlwaysFalse"),
        }
    }
}

/// Languages supported for custom predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PredicateLanguage {
    JavaScript,
    Python,
    Rust,
    Shell,
}

impl Predicate {
    /// Evaluate the predicate against a project state
    ///
    /// This is a placeholder that will be implemented with actual
    /// project state evaluation in the future.
    pub async fn evaluate(&self, _state: &PredicateState) -> Result<bool> {
        match self {
            Predicate::FileExists(path) => Ok(std::fs::metadata(path).is_ok()),

            Predicate::DirectoryExists(path) => {
                Ok(std::fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false))
            }

            Predicate::TestsPassing {
                suite,
                min_coverage: _,
            } => {
                // Placeholder - will integrate with test runner
                Err(PredicateError::TestSuiteNotFound(suite.clone()).into())
            }

            Predicate::ApiEndpoint {
                url,
                expected_status: _,
                expected_body_contains: _,
            } => {
                // Placeholder - will integrate with HTTP client
                Err(PredicateError::EndpointUnreachable(url.clone()).into())
            }

            Predicate::Performance {
                metric: _,
                threshold: _,
                comparison: _,
            } => {
                // Placeholder - will integrate with metrics system
                Ok(false)
            }

            Predicate::CommandSucceeds {
                command,
                args,
                expected_exit_code,
            } => {
                // Execute command and check exit code
                let output = tokio::process::Command::new(command)
                    .args(args)
                    .output()
                    .await
                    .map_err(|e| PredicateError::CustomPredicateFailed(e.to_string()))?;

                Ok(output.status.code() == Some(*expected_exit_code))
            }

            Predicate::Custom {
                code: _,
                language: _,
                description: _,
            } => {
                // Placeholder - will integrate with code execution sandbox
                Err(PredicateError::CustomPredicateFailed(
                    "Custom predicates not yet implemented".to_string(),
                )
                .into())
            }

            Predicate::And(predicates) => {
                for pred in predicates {
                    if !Box::pin(pred.evaluate(_state)).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            Predicate::Or(predicates) => {
                for pred in predicates {
                    if Box::pin(pred.evaluate(_state)).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }

            Predicate::Not(pred) => Ok(!Box::pin(pred.evaluate(_state)).await?),

            Predicate::AlwaysTrue => Ok(true),
            Predicate::AlwaysFalse => Ok(false),
        }
    }

    /// Simplify the predicate using logical rules
    ///
    /// This performs compile-time optimization of predicate trees.
    ///
    /// # Examples
    ///
    /// ```
    /// use sentinel_core::goal_manifold::predicate::Predicate;
    ///
    /// let pred = Predicate::And(vec![
    ///     Predicate::AlwaysTrue,
    ///     Predicate::FileExists("main.rs".into()),
    /// ]);
    ///
    /// let simplified = pred.simplify();
    /// // AlwaysTrue AND X = X
    /// ```
    pub fn simplify(self) -> Self {
        match self {
            // AND simplifications
            Predicate::And(mut preds) => {
                // Remove AlwaysTrue
                preds.retain(|p| !matches!(p, Predicate::AlwaysTrue));

                // If any is AlwaysFalse, entire AND is false
                if preds.iter().any(|p| matches!(p, Predicate::AlwaysFalse)) {
                    return Predicate::AlwaysFalse;
                }

                // Recursively simplify children
                let preds: Vec<_> = preds.into_iter().map(|p| p.simplify()).collect();

                match preds.len() {
                    0 => Predicate::AlwaysTrue,
                    1 => preds.into_iter().next().unwrap(),
                    _ => Predicate::And(preds),
                }
            }

            // OR simplifications
            Predicate::Or(mut preds) => {
                // Remove AlwaysFalse
                preds.retain(|p| !matches!(p, Predicate::AlwaysFalse));

                // If any is AlwaysTrue, entire OR is true
                if preds.iter().any(|p| matches!(p, Predicate::AlwaysTrue)) {
                    return Predicate::AlwaysTrue;
                }

                // Recursively simplify children
                let preds: Vec<_> = preds.into_iter().map(|p| p.simplify()).collect();

                match preds.len() {
                    0 => Predicate::AlwaysFalse,
                    1 => preds.into_iter().next().unwrap(),
                    _ => Predicate::Or(preds),
                }
            }

            // NOT simplifications
            Predicate::Not(pred) => match *pred {
                Predicate::AlwaysTrue => Predicate::AlwaysFalse,
                Predicate::AlwaysFalse => Predicate::AlwaysTrue,
                Predicate::Not(inner) => inner.simplify(), // Double negation
                other => Predicate::Not(Box::new(other.simplify())),
            },

            // No simplification needed
            other => other,
        }
    }

    /// Estimate complexity of evaluating this predicate (0-10 scale)
    pub fn complexity(&self) -> u8 {
        match self {
            Predicate::AlwaysTrue | Predicate::AlwaysFalse => 0,
            Predicate::FileExists(_) | Predicate::DirectoryExists(_) => 1,
            Predicate::TestsPassing { .. } => 7,
            Predicate::ApiEndpoint { .. } => 5,
            Predicate::Performance { .. } => 6,
            Predicate::CommandSucceeds { .. } => 4,
            Predicate::Custom { .. } => 8,
            Predicate::And(preds) | Predicate::Or(preds) => {
                preds.iter().map(|p| p.complexity()).max().unwrap_or(0) + 1
            }
            Predicate::Not(pred) => pred.complexity() + 1,
        }
    }

    /// Check if this predicate requires external resources (network, filesystem)
    pub fn requires_external_resources(&self) -> bool {
        match self {
            Predicate::FileExists(_)
            | Predicate::DirectoryExists(_)
            | Predicate::TestsPassing { .. }
            | Predicate::ApiEndpoint { .. }
            | Predicate::CommandSucceeds { .. }
            | Predicate::Custom { .. } => true,

            Predicate::And(preds) | Predicate::Or(preds) => {
                preds.iter().any(|p| p.requires_external_resources())
            }

            Predicate::Not(pred) => pred.requires_external_resources(),

            Predicate::AlwaysTrue | Predicate::AlwaysFalse | Predicate::Performance { .. } => false,
        }
    }
}

/// State representation for predicate evaluation
///
/// This is a placeholder that will be expanded as we implement
/// the full project state tracking system.
#[derive(Debug, Clone)]
pub struct PredicateState {
    pub working_directory: PathBuf,
    // TODO: Add test results, metrics, etc.
}

impl PredicateState {
    pub fn new(working_directory: PathBuf) -> Self {
        Self { working_directory }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_simplify_and() {
        let pred = Predicate::And(vec![
            Predicate::AlwaysTrue,
            Predicate::FileExists("main.rs".into()),
        ]);

        let simplified = pred.simplify();
        assert!(matches!(simplified, Predicate::FileExists(_)));
    }

    #[test]
    fn test_predicate_simplify_and_with_false() {
        let pred = Predicate::And(vec![
            Predicate::AlwaysFalse,
            Predicate::FileExists("main.rs".into()),
        ]);

        let simplified = pred.simplify();
        assert!(matches!(simplified, Predicate::AlwaysFalse));
    }

    #[test]
    fn test_predicate_simplify_or() {
        let pred = Predicate::Or(vec![
            Predicate::AlwaysFalse,
            Predicate::FileExists("main.rs".into()),
        ]);

        let simplified = pred.simplify();
        assert!(matches!(simplified, Predicate::FileExists(_)));
    }

    #[test]
    fn test_predicate_simplify_double_negation() {
        let pred = Predicate::Not(Box::new(Predicate::Not(Box::new(Predicate::FileExists(
            "main.rs".into(),
        )))));

        let simplified = pred.simplify();
        assert!(matches!(simplified, Predicate::FileExists(_)));
    }

    #[test]
    fn test_predicate_complexity() {
        assert_eq!(Predicate::AlwaysTrue.complexity(), 0);
        assert_eq!(Predicate::FileExists("main.rs".into()).complexity(), 1);
        assert!(
            Predicate::TestsPassing {
                suite: "unit".to_string(),
                min_coverage: 0.8
            }
            .complexity()
                > 5
        );
    }

    #[test]
    fn test_predicate_requires_external_resources() {
        assert!(!Predicate::AlwaysTrue.requires_external_resources());
        assert!(Predicate::FileExists("main.rs".into()).requires_external_resources());
        assert!(Predicate::TestsPassing {
            suite: "unit".to_string(),
            min_coverage: 0.8
        }
        .requires_external_resources());
    }

    #[tokio::test]
    async fn test_predicate_evaluate_always_true() {
        let state = PredicateState::new(PathBuf::from("."));
        let result = Predicate::AlwaysTrue.evaluate(&state).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_predicate_evaluate_and() {
        let state = PredicateState::new(PathBuf::from("."));
        let pred = Predicate::And(vec![Predicate::AlwaysTrue, Predicate::AlwaysTrue]);

        let result = pred.evaluate(&state).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_predicate_evaluate_or() {
        let state = PredicateState::new(PathBuf::from("."));
        let pred = Predicate::Or(vec![Predicate::AlwaysFalse, Predicate::AlwaysTrue]);

        let result = pred.evaluate(&state).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
