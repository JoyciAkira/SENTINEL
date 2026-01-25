//! Project State representation for alignment computation
//!
//! This module defines the state space for a project, which is used
//! to compute alignment with goals and predict future trajectories.

use crate::goal_manifold::goal::Goal;
use crate::goal_manifold::predicate::ProjectState as PredicateState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Complete representation of project state
///
/// ProjectState captures all relevant information about a project
/// at a given point in time. It's used for:
/// - Alignment computation
/// - Monte Carlo simulation
/// - Gradient calculation
///
/// # State Space Dimensions
///
/// The state space is high-dimensional, including:
/// - Files and their contents
/// - Test results
/// - Goal completion status
/// - Code metrics
/// - API endpoint status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Working directory
    pub working_directory: PathBuf,

    /// Files in the project (path -> hash)
    pub files: HashMap<PathBuf, FileState>,

    /// Test suite results
    pub test_results: HashMap<String, TestResults>,

    /// Goal completion status
    pub goal_states: HashMap<Uuid, GoalState>,

    /// Code metrics
    pub metrics: CodeMetrics,

    /// Timestamp of this state
    pub timestamp: crate::types::Timestamp,
}

/// State of a file in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// File path (relative to project root)
    pub path: PathBuf,

    /// Content hash (for change detection)
    pub content_hash: [u8; 32],

    /// File size in bytes
    pub size: u64,

    /// Last modified timestamp
    pub modified: crate::types::Timestamp,

    /// File type
    pub file_type: FileType,
}

/// Type of file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Source,
    Test,
    Config,
    Documentation,
    Other,
}

/// Test suite results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Suite name
    pub suite: String,

    /// Total number of tests
    pub total: usize,

    /// Number of passed tests
    pub passed: usize,

    /// Number of failed tests
    pub failed: usize,

    /// Number of skipped tests
    pub skipped: usize,

    /// Code coverage (0.0-1.0)
    pub coverage: f64,

    /// Execution time in seconds
    pub duration: f64,
}

impl TestResults {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.passed == self.total - self.skipped
    }

    /// Get pass rate (0.0-1.0)
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }
}

/// State of a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalState {
    /// Goal ID
    pub id: Uuid,

    /// Current status
    pub status: crate::types::GoalStatus,

    /// Completion percentage (0.0-1.0)
    pub completion: f64,

    /// Number of satisfied success criteria
    pub criteria_satisfied: usize,

    /// Total number of success criteria
    pub criteria_total: usize,
}

impl GoalState {
    /// Create from a Goal
    pub fn from_goal(goal: &Goal) -> Self {
        Self {
            id: goal.id,
            status: goal.status,
            completion: 0.0,
            criteria_satisfied: 0,
            criteria_total: goal.success_criteria.len(),
        }
    }

    /// Check if goal is complete
    pub fn is_complete(&self) -> bool {
        self.status == crate::types::GoalStatus::Completed
    }

    /// Get progress (0.0-1.0)
    pub fn progress(&self) -> f64 {
        if self.criteria_total == 0 {
            return 0.0;
        }
        self.criteria_satisfied as f64 / self.criteria_total as f64
    }
}

/// Code metrics for the project
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Lines of code
    pub lines_of_code: usize,

    /// Number of functions
    pub function_count: usize,

    /// Number of classes/structs
    pub class_count: usize,

    /// Cyclomatic complexity (average)
    pub avg_complexity: f64,

    /// Technical debt ratio (0.0-1.0)
    pub tech_debt_ratio: f64,

    /// Test coverage (0.0-1.0)
    pub test_coverage: f64,
}

impl ProjectState {
    /// Create a new empty project state
    pub fn new(working_directory: PathBuf) -> Self {
        Self {
            working_directory,
            files: HashMap::new(),
            test_results: HashMap::new(),
            goal_states: HashMap::new(),
            metrics: CodeMetrics::default(),
            timestamp: crate::types::now(),
        }
    }

    /// Convert to PredicateState for predicate evaluation
    pub fn to_predicate_state(&self) -> PredicateState {
        PredicateState::new(self.working_directory.clone())
    }

    /// Get the number of dimensions in this state space
    ///
    /// This is used for gradient computation.
    pub fn dimension_count(&self) -> usize {
        // Dimensions:
        // - Files (each file is a dimension)
        // - Test results (each suite is a dimension)
        // - Goals (each goal is a dimension)
        // - Metrics (fixed number of dimensions)
        self.files.len() + self.test_results.len() + self.goal_states.len() + 6
    }

    /// Get all dimensions as a list
    pub fn get_dimensions(&self) -> Vec<StateDimension> {
        let mut dimensions = Vec::new();

        // File dimensions
        for path in self.files.keys() {
            dimensions.push(StateDimension::File(path.clone()));
        }

        // Test dimensions
        for suite in self.test_results.keys() {
            dimensions.push(StateDimension::TestSuite(suite.clone()));
        }

        // Goal dimensions
        for goal_id in self.goal_states.keys() {
            dimensions.push(StateDimension::Goal(*goal_id));
        }

        // Metric dimensions
        dimensions.push(StateDimension::Metric(MetricType::LinesOfCode));
        dimensions.push(StateDimension::Metric(MetricType::FunctionCount));
        dimensions.push(StateDimension::Metric(MetricType::ClassCount));
        dimensions.push(StateDimension::Metric(MetricType::AvgComplexity));
        dimensions.push(StateDimension::Metric(MetricType::TechDebtRatio));
        dimensions.push(StateDimension::Metric(MetricType::TestCoverage));

        dimensions
    }

    /// Perturb a dimension by a small amount
    ///
    /// This is used for numerical gradient computation.
    pub fn perturb(&self, dimension: &StateDimension, epsilon: f64) -> Self {
        let mut perturbed = self.clone();

        match dimension {
            StateDimension::Goal(goal_id) => {
                if let Some(goal_state) = perturbed.goal_states.get_mut(goal_id) {
                    goal_state.completion = (goal_state.completion + epsilon).clamp(0.0, 1.0);
                }
            }
            StateDimension::Metric(metric_type) => {
                match metric_type {
                    MetricType::LinesOfCode => {
                        perturbed.metrics.lines_of_code =
                            (perturbed.metrics.lines_of_code as f64 + epsilon).max(0.0) as usize;
                    }
                    MetricType::TestCoverage => {
                        perturbed.metrics.test_coverage =
                            (perturbed.metrics.test_coverage + epsilon).clamp(0.0, 1.0);
                    }
                    MetricType::TechDebtRatio => {
                        perturbed.metrics.tech_debt_ratio =
                            (perturbed.metrics.tech_debt_ratio + epsilon).clamp(0.0, 1.0);
                    }
                    MetricType::AvgComplexity => {
                        perturbed.metrics.avg_complexity =
                            (perturbed.metrics.avg_complexity + epsilon).max(0.0);
                    }
                    _ => {}
                }
            }
            _ => {
                // Other dimensions can be perturbed in future implementations
            }
        }

        perturbed
    }

    /// Compute distance to another state
    ///
    /// Uses Euclidean distance in normalized state space.
    pub fn distance(&self, other: &ProjectState) -> f64 {
        let mut sum_sq = 0.0;

        // Goal completion distances
        for (goal_id, goal_state) in &self.goal_states {
            if let Some(other_goal) = other.goal_states.get(goal_id) {
                let diff = goal_state.completion - other_goal.completion;
                sum_sq += diff * diff;
            }
        }

        // Metric distances (normalized)
        let metrics_diff = vec![
            (self.metrics.test_coverage - other.metrics.test_coverage),
            (self.metrics.tech_debt_ratio - other.metrics.tech_debt_ratio),
            (self.metrics.avg_complexity - other.metrics.avg_complexity) / 10.0, // Normalize
        ];

        for diff in metrics_diff {
            sum_sq += diff * diff;
        }

        sum_sq.sqrt()
    }
}

/// A dimension in the state space
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StateDimension {
    /// A file in the project
    File(PathBuf),

    /// A test suite
    TestSuite(String),

    /// A goal
    Goal(Uuid),

    /// A code metric
    Metric(MetricType),
}

/// Types of code metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    LinesOfCode,
    FunctionCount,
    ClassCount,
    AvgComplexity,
    TechDebtRatio,
    TestCoverage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_state_creation() {
        let state = ProjectState::new(PathBuf::from("/test"));
        assert_eq!(state.files.len(), 0);
        assert_eq!(state.test_results.len(), 0);
        assert_eq!(state.goal_states.len(), 0);
    }

    #[test]
    fn test_test_results() {
        let results = TestResults {
            suite: "unit".to_string(),
            total: 100,
            passed: 95,
            failed: 5,
            skipped: 0,
            coverage: 0.85,
            duration: 10.5,
        };

        assert!(!results.all_passed());
        assert_eq!(results.pass_rate(), 0.95);
    }

    #[test]
    fn test_goal_state_progress() {
        let mut goal_state = GoalState {
            id: Uuid::new_v4(),
            status: crate::types::GoalStatus::InProgress,
            completion: 0.0,
            criteria_satisfied: 3,
            criteria_total: 5,
        };

        assert_eq!(goal_state.progress(), 0.6);
        assert!(!goal_state.is_complete());
    }

    #[test]
    fn test_state_dimensions() {
        let mut state = ProjectState::new(PathBuf::from("/test"));

        state
            .goal_states
            .insert(Uuid::new_v4(), GoalState::from_goal(&test_goal()));
        state.test_results.insert(
            "unit".to_string(),
            TestResults {
                suite: "unit".to_string(),
                total: 10,
                passed: 10,
                failed: 0,
                skipped: 0,
                coverage: 0.9,
                duration: 1.0,
            },
        );

        let dimensions = state.get_dimensions();
        assert!(dimensions.len() > 6); // At least metrics + goal + test
    }

    #[test]
    fn test_state_perturbation() {
        let state = ProjectState::new(PathBuf::from("/test"));
        let goal_id = Uuid::new_v4();

        let mut state_with_goal = state.clone();
        state_with_goal
            .goal_states
            .insert(goal_id, GoalState::from_goal(&test_goal()));

        let perturbed = state_with_goal.perturb(&StateDimension::Goal(goal_id), 0.1);

        assert_ne!(
            state_with_goal.goal_states[&goal_id].completion,
            perturbed.goal_states[&goal_id].completion
        );
    }

    #[test]
    fn test_state_distance() {
        let state1 = ProjectState::new(PathBuf::from("/test"));
        let state2 = ProjectState::new(PathBuf::from("/test"));

        let distance = state1.distance(&state2);
        assert_eq!(distance, 0.0); // Same state
    }

    fn test_goal() -> Goal {
        use crate::goal_manifold::goal::Goal;
        use crate::goal_manifold::predicate::Predicate;

        Goal::builder()
            .description("Test goal")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap()
    }
}
