//! Atomic Contract definition for Sentinel
//!
//! This module defines the formal contracts for "Atomic Truth" execution.

use serde::{Deserialize, Serialize};

/// A formal contract for an atomic coding task.
///
/// This is the foundation of the "Atomic Truth" vision. Every atomic task
/// must satisfy this contract before being committed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicContract {
    /// The specific inputs required for this atom.
    pub inputs: Vec<InputSpec>,

    /// The expected outputs this atom must produce.
    pub outputs: Vec<OutputSpec>,

    /// Rules that must hold true throughout the execution.
    pub invariants: Vec<Invariant>,

    /// Isolation level for this atom.
    pub isolation_level: IsolationLevel,
}

/// Specification for a single input to an atom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSpec {
    pub name: String,
    pub r#type: String,
    pub description: String,
    pub is_required: bool,
}

/// Specification for a single output produced by an atom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputSpec {
    pub name: String,
    pub r#type: String,
    pub description: String,
}

/// A rule that must be maintained (invariant).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    pub description: String,
    pub check_command: Option<String>,
    pub severity: InvariantSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvariantSeverity {
    Critical,
    Warning,
    Advisory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IsolationLevel {
    /// No external access, purely functional.
    Strict,
    /// Access to specific files only.
    Sandboxed,
    /// Normal filesystem access (monitored).
    Monitored,
}

impl AtomicContract {
    pub fn new() -> Self {
        Self {
            inputs: Vec::new(),
            outputs: Vec::new(),
            invariants: Vec::new(),
            isolation_level: IsolationLevel::Sandboxed,
        }
    }
}

impl Default for AtomicContract {
    fn default() -> Self {
        Self::new()
    }
}
