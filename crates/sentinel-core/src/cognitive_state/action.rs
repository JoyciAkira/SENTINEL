//! Action types for the Cognitive State Machine
//!
//! Actions represent concrete operations the agent can take on the project.
//! Every action passes through the cognitive gate (before_action) for validation.

use crate::types::Timestamp;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// An action the agent wants to take
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    /// Unique identifier
    pub id: Uuid,

    /// Type of action
    pub action_type: ActionType,

    /// Human-readable description
    pub description: String,

    /// Which goal this action contributes to
    pub goal_id: Option<Uuid>,

    /// Expected value to goal achievement (0.0-1.0)
    pub expected_value: f64,

    /// Timestamp when action was created
    pub created_at: Timestamp,

    /// Metadata
    pub metadata: ActionMetadata,
}

impl Action {
    /// Create a new action
    pub fn new(action_type: ActionType, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            action_type,
            description,
            goal_id: None,
            expected_value: 0.5,
            created_at: Utc::now(),
            metadata: ActionMetadata::default(),
        }
    }

    /// Get action ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Set the goal this action contributes to
    pub fn for_goal(mut self, goal_id: Uuid) -> Self {
        self.goal_id = Some(goal_id);
        self
    }

    /// Set expected value
    pub fn with_expected_value(mut self, value: f64) -> Self {
        self.expected_value = value.clamp(0.0, 1.0);
        self
    }

    /// Check if this action is safe (doesn't violate basic rules)
    pub fn is_safe(&self) -> bool {
        match &self.action_type {
            ActionType::DeleteFile { path, .. } => {
                // Don't delete critical files
                !Self::is_critical_file(path)
            }
            ActionType::RunCommand { command, .. } => {
                // Don't run dangerous commands
                !Self::is_dangerous_command(command)
            }
            _ => true,
        }
    }

    fn is_critical_file(path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();
        path_str.contains("/.git/")
            || path_str.ends_with("Cargo.toml")
            || path_str.ends_with("package.json")
            || path_str.ends_with(".env")
    }

    fn is_dangerous_command(command: &str) -> bool {
        let dangerous = ["rm -rf /", ":(){ :|:& };:", "mkfs", "dd if=/dev/zero"];
        dangerous.iter().any(|&d| command.contains(d))
    }
}

/// Types of actions the agent can take
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ActionType {
    /// Edit an existing file
    EditFile {
        path: PathBuf,
        old_content: String,
        new_content: String,
    },

    /// Create a new file
    CreateFile { path: PathBuf, content: String },

    /// Delete a file
    DeleteFile { path: PathBuf, backup: bool },

    /// Run a command
    RunCommand {
        command: String,
        working_dir: PathBuf,
    },

    /// Run tests
    RunTests { suite: String },

    /// Update goal status
    UpdateGoal {
        goal_id: Uuid,
        new_status: crate::types::GoalStatus,
    },

    /// Custom action
    Custom {
        name: String,
        payload: serde_json::Value,
    },
}

/// Metadata for an action
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// Estimated time to complete (seconds)
    pub estimated_duration: Option<f64>,

    /// Risk level (0.0 = safe, 1.0 = risky)
    pub risk_level: f64,

    /// Whether this action is reversible
    pub reversible: bool,

    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Result of executing an action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// ID of the action that was executed
    pub action_id: Uuid,

    /// Whether the action succeeded
    pub success: bool,

    /// Output from the action
    pub output: String,

    /// Error message if failed
    pub error: Option<String>,

    /// Actual duration (seconds)
    pub duration: f64,

    /// Timestamp when completed
    pub completed_at: Timestamp,

    /// Changes made (for reversibility)
    pub changes: Vec<Change>,
}

impl ActionResult {
    /// Create a successful result
    pub fn success(action_id: Uuid, output: String, duration: f64) -> Self {
        Self {
            action_id,
            success: true,
            output,
            error: None,
            duration,
            completed_at: Utc::now(),
            changes: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(action_id: Uuid, error: String, duration: f64) -> Self {
        Self {
            action_id,
            success: false,
            output: String::new(),
            error: Some(error),
            duration,
            completed_at: Utc::now(),
            changes: Vec::new(),
        }
    }

    /// Add a change record
    pub fn with_change(mut self, change: Change) -> Self {
        self.changes.push(change);
        self
    }
}

/// A change made by an action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Change {
    /// File was created
    FileCreated { path: PathBuf },

    /// File was modified
    FileModified {
        path: PathBuf,
        old_hash: String,
        new_hash: String,
    },

    /// File was deleted
    FileDeleted {
        path: PathBuf,
        backup_path: Option<PathBuf>,
    },

    /// Command was executed
    CommandExecuted { command: String, exit_code: i32 },
}

/// Decision about whether to execute an action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionDecision {
    /// Type of decision
    pub decision_type: DecisionType,

    /// Reason for the decision
    pub reason: String,

    /// Alternative actions (if rejected/deferred)
    pub alternatives: Vec<Action>,

    /// When to retry (if deferred)
    pub retry_after: Option<std::time::Duration>,
}

/// Type of decision
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionType {
    /// Action approved
    Approve,

    /// Action rejected
    Reject,

    /// Propose alternative action
    ProposeAlternative,

    /// Skip action (low value)
    Skip,

    /// Defer action (resource constraints)
    Defer,
}

impl ActionDecision {
    /// Approve an action
    pub fn approve(action: &Action) -> Self {
        Self {
            decision_type: DecisionType::Approve,
            reason: format!("Action approved: {}", action.description),
            alternatives: Vec::new(),
            retry_after: None,
        }
    }

    /// Reject an action
    pub fn reject(reason: String) -> Self {
        Self {
            decision_type: DecisionType::Reject,
            reason,
            alternatives: Vec::new(),
            retry_after: None,
        }
    }

    /// Propose alternative actions
    pub fn propose_alternative(
        rejected: &Action,
        reason: String,
        alternatives: Vec<Action>,
    ) -> Self {
        Self {
            decision_type: DecisionType::ProposeAlternative,
            reason: format!("Rejected '{}': {}", rejected.description, reason),
            alternatives,
            retry_after: None,
        }
    }

    /// Skip an action
    pub fn skip(reason: String) -> Self {
        Self {
            decision_type: DecisionType::Skip,
            reason,
            alternatives: Vec::new(),
            retry_after: None,
        }
    }

    /// Defer an action
    pub fn defer(reason: String, retry_after: std::time::Duration) -> Self {
        Self {
            decision_type: DecisionType::Defer,
            reason,
            alternatives: Vec::new(),
            retry_after: Some(retry_after),
        }
    }

    /// Check if action was approved
    pub fn is_approved(&self) -> bool {
        self.decision_type == DecisionType::Approve
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_creation() {
        let action = Action::new(
            ActionType::CreateFile {
                path: PathBuf::from("test.txt"),
                content: "Hello".to_string(),
            },
            "Create test file".to_string(),
        );

        assert!(action.is_safe());
        assert_eq!(action.expected_value, 0.5);
    }

    #[test]
    fn test_action_safety_critical_file() {
        let action = Action::new(
            ActionType::DeleteFile {
                path: PathBuf::from("Cargo.toml"),
                backup: false,
            },
            "Delete Cargo.toml".to_string(),
        );

        assert!(!action.is_safe());
    }

    #[test]
    fn test_action_safety_dangerous_command() {
        let action = Action::new(
            ActionType::RunCommand {
                command: "rm -rf /".to_string(),
                working_dir: PathBuf::from("."),
            },
            "Dangerous command".to_string(),
        );

        assert!(!action.is_safe());
    }

    #[test]
    fn test_action_decision_approve() {
        let action = Action::new(
            ActionType::RunTests {
                suite: "unit".to_string(),
            },
            "Run tests".to_string(),
        );

        let decision = ActionDecision::approve(&action);
        assert!(decision.is_approved());
    }

    #[test]
    fn test_action_decision_reject() {
        let decision = ActionDecision::reject("Unsafe operation".to_string());
        assert!(!decision.is_approved());
        assert_eq!(decision.decision_type, DecisionType::Reject);
    }

    #[test]
    fn test_action_result_success() {
        let action_id = Uuid::new_v4();
        let result = ActionResult::success(action_id, "OK".to_string(), 1.5);

        assert!(result.success);
        assert_eq!(result.output, "OK");
        assert_eq!(result.duration, 1.5);
    }

    #[test]
    fn test_action_result_failure() {
        let action_id = Uuid::new_v4();
        let result = ActionResult::failure(action_id, "Failed".to_string(), 0.5);

        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "Failed");
    }
}
