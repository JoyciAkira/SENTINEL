//! Error types for Sentinel Core
//!
//! This module defines all error types used throughout the Sentinel core engine.
//! We use `thiserror` for ergonomic error definitions with automatic Display/Error implementations.

use thiserror::Error;
use uuid::Uuid;

/// Result type alias for Sentinel operations
pub type Result<T> = std::result::Result<T, SentinelError>;

/// Main error type for Sentinel operations
#[derive(Error, Debug)]
pub enum SentinelError {
    /// Goal-related errors
    #[error("Goal error: {0}")]
    Goal(#[from] GoalError),

    /// DAG-related errors
    #[error("DAG error: {0}")]
    Dag(#[from] DagError),

    /// Predicate evaluation errors
    #[error("Predicate error: {0}")]
    Predicate(#[from] PredicateError),

    /// Invariant violation errors
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        source: Box<SentinelError>,
    },
}

/// Errors related to Goal operations
#[derive(Error, Debug, Clone)]
pub enum GoalError {
    #[error("Goal not found: {0}")]
    NotFound(Uuid),

    #[error("Goal already exists: {0}")]
    AlreadyExists(Uuid),

    #[error("Invalid goal state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Goal has unresolved dependencies: {0:?}")]
    UnresolvedDependencies(Vec<Uuid>),

    #[error("Goal complexity estimate invalid: {0}")]
    InvalidComplexity(String),

    #[error("Goal value to root must be in range [0.0, 1.0], got {0}")]
    InvalidValue(f64),

    #[error("Empty success criteria")]
    EmptySuccessCriteria,
}

/// Errors related to DAG operations
#[derive(Error, Debug, Clone)]
pub enum DagError {
    #[error("Cycle detected in goal DAG: {0:?}")]
    CycleDetected(Vec<Uuid>),

    #[error("Node not found in DAG: {0}")]
    NodeNotFound(Uuid),

    #[error("Edge not found in DAG: {from} -> {to}")]
    EdgeNotFound { from: Uuid, to: Uuid },

    #[error("Cannot add edge that would create cycle: {from} -> {to}")]
    WouldCreateCycle { from: Uuid, to: Uuid },

    #[error("Anti-dependency conflict: goal {goal} cannot depend on {blocked}")]
    AntiDependencyConflict { goal: Uuid, blocked: Uuid },

    #[error("DAG is empty")]
    Empty,
}

/// Errors related to Predicate evaluation
#[derive(Error, Debug, Clone)]
pub enum PredicateError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Test suite not found: {0}")]
    TestSuiteNotFound(String),

    #[error("API endpoint unreachable: {0}")]
    EndpointUnreachable(String),

    #[error("Invalid JSON schema: {0}")]
    InvalidJsonSchema(String),

    #[error("Custom predicate execution failed: {0}")]
    CustomPredicateFailed(String),

    #[error("Predicate type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Evaluation timeout after {0}ms")]
    Timeout(u64),
}

impl SentinelError {
    /// Add context to an error
    pub fn context(self, context: impl Into<String>) -> Self {
        Self::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }
}

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to a Result
    fn context(self, context: impl Into<String>) -> Result<T>;

    /// Add lazy context to a Result
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.context(context))
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| e.context(f()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let err = GoalError::NotFound(Uuid::new_v4());
        let err = SentinelError::from(err);
        let err = err.context("Failed to retrieve goal");

        assert!(err.to_string().contains("Failed to retrieve goal"));
    }

    #[test]
    fn test_result_ext() {
        let result: Result<()> = Err(GoalError::EmptySuccessCriteria.into());
        let result = result.context("Goal validation failed");

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Goal validation failed"));
    }
}
