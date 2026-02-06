//! Goal data structure and operations
//!
//! This module implements the Goal type, which represents a single
//! objective in the goal manifold. Goals have formal success criteria,
//! dependencies, and verifiable completion state.

use crate::error::{GoalError, Result};
use crate::goal_manifold::atomic::AtomicContract;
use crate::goal_manifold::predicate::Predicate;
use crate::types::{GoalLock, GoalStatus, ProbabilityDistribution, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single goal in the Goal Manifold
///
/// Goals are the fundamental unit of work in Sentinel. Each goal has:
/// - Unique identifier
/// - Formal success criteria (predicates)
/// - Dependencies on other goals
/// - Complexity estimate (probability distribution)
/// - Value contribution to root objective
/// - Multi-agent locks (Social Manifold)
///
/// # Invariants
///
/// - `value_to_root` must be in range [0.0, 1.0]
/// - `success_criteria` must not be empty
/// - Status transitions must be valid (enforced by `transition_to`)
///
/// # Examples
///
/// ```
/// use sentinel_core::goal_manifold::goal::Goal;
/// use sentinel_core::goal_manifold::predicate::Predicate;
/// use sentinel_core::types::ProbabilityDistribution;
///
/// let goal = Goal::builder()
///     .description("Implement authentication system")
///     .add_success_criterion(Predicate::TestsPassing {
///         suite: "auth".to_string(),
///         min_coverage: 0.8,
///     })
///     .value_to_root(0.3)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    /// Unique identifier
    pub id: Uuid,

    /// Human-readable description
    pub description: String,

    /// Formally verifiable success criteria
    ///
    /// All predicates must evaluate to true for goal to be considered complete.
    pub success_criteria: Vec<Predicate>,

    /// Goals that must complete before this one can start
    pub dependencies: Vec<Uuid>,

    /// Goals that must NOT be worked on simultaneously with this one
    ///
    /// This is useful for preventing resource conflicts or logical inconsistencies.
    pub anti_dependencies: Vec<Uuid>,

    /// Estimated complexity (probability distribution)
    ///
    /// We use distributions instead of single values to capture uncertainty.
    pub complexity_estimate: ProbabilityDistribution,

    /// How much this goal contributes to root objective [0.0, 1.0]
    ///
    /// This is used for prioritization and alignment scoring.
    pub value_to_root: f64,

    /// Current execution status
    pub status: GoalStatus,

    /// Current lock holder (Social Manifold - Layer 8)
    pub current_lock: Option<GoalLock>,

    /// Formal atomic contract for this goal (Atomic Truth - Phase 4)
    pub atomic_contract: Option<AtomicContract>,

    /// Optional parent goal (for hierarchical decomposition)
    pub parent_id: Option<Uuid>,

    /// Specific tests to validate this goal
    pub validation_tests: Vec<String>,

    /// Metadata for tracking and learning
    pub metadata: GoalMetadata,

    /// Creation timestamp
    pub created_at: Timestamp,

    /// Last update timestamp
    pub updated_at: Timestamp,
}

/// Metadata associated with a goal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GoalMetadata {
    /// Tags for categorization
    pub tags: Vec<String>,

    /// Number of times this goal has been attempted
    pub retry_count: u32,

    /// If failed, the reason
    pub failure_reason: Option<String>,

    /// If blocked, the reason and blocker IDs
    pub blocked_reason: Option<String>,
    pub blocker_ids: Vec<Uuid>,

    /// Notes from execution
    pub notes: Vec<String>,
}

impl Goal {
    /// Create a new goal with default values
    pub fn new(description: impl Into<String>) -> Self {
        let now = crate::types::now();

        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            success_criteria: Vec::new(),
            dependencies: Vec::new(),
            anti_dependencies: Vec::new(),
            complexity_estimate: ProbabilityDistribution::point(5.0),
            value_to_root: 0.0,
            status: GoalStatus::Pending,
            current_lock: None,
            atomic_contract: None,
            parent_id: None,
            validation_tests: Vec::new(),
            metadata: GoalMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a builder for fluent goal construction
    pub fn builder() -> GoalBuilder {
        GoalBuilder::default()
    }

    /// Validate the goal's invariants
    ///
    /// Returns `Err` if any invariant is violated.
    pub fn validate(&self) -> Result<()> {
        // Value must be in valid range
        if !(0.0..=1.0).contains(&self.value_to_root) {
            return Err(GoalError::InvalidValue(self.value_to_root).into());
        }

        // Must have at least one success criterion
        if self.success_criteria.is_empty() {
            return Err(GoalError::EmptySuccessCriteria.into());
        }

        // Complexity mean should be reasonable (1-10 scale)
        let complexity_mean = self.complexity_estimate.mean;
        if !(0.0..=10.0).contains(&complexity_mean) {
            return Err(GoalError::InvalidComplexity(format!(
                "Complexity mean {} outside valid range [0.0, 10.0]",
                complexity_mean
            ))
            .into());
        }

        Ok(())
    }

    /// Transition to a new status
    ///
    /// This enforces valid state transitions and updates timestamps.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the transition is invalid.
    pub fn transition_to(&mut self, new_status: GoalStatus) -> Result<()> {
        if !self.status.can_transition_to(new_status) {
            return Err(GoalError::InvalidStateTransition {
                from: self.status.to_string(),
                to: new_status.to_string(),
            }
            .into());
        }

        self.status = new_status;
        self.updated_at = crate::types::now();

        Ok(())
    }

    /// Mark goal as blocked
    pub fn block(&mut self, reason: impl Into<String>, blocker_ids: Vec<Uuid>) -> Result<()> {
        self.transition_to(GoalStatus::Blocked)?;
        self.metadata.blocked_reason = Some(reason.into());
        self.metadata.blocker_ids = blocker_ids;
        Ok(())
    }

    /// Mark goal as failed
    pub fn fail(&mut self, reason: impl Into<String>) -> Result<()> {
        self.transition_to(GoalStatus::Failed)?;
        self.metadata.failure_reason = Some(reason.into());
        self.metadata.retry_count += 1;
        Ok(())
    }

    /// Mark goal as ready (dependencies satisfied)
    pub fn mark_ready(&mut self) -> Result<()> {
        self.transition_to(GoalStatus::Ready)
    }

    /// Start working on goal
    pub fn start(&mut self) -> Result<()> {
        self.transition_to(GoalStatus::InProgress)
    }

    /// Begin validation
    pub fn begin_validation(&mut self) -> Result<()> {
        self.transition_to(GoalStatus::Validating)
    }

    /// Mark goal as completed
    pub fn complete(&mut self) -> Result<()> {
        self.transition_to(GoalStatus::Completed)
    }

    /// Mark goal as deprecated (no longer relevant)
    pub fn deprecate(&mut self) -> Result<()> {
        self.transition_to(GoalStatus::Deprecated)
    }

    /// Add a note to the goal
    pub fn add_note(&mut self, note: impl Into<String>) {
        self.metadata.notes.push(note.into());
        self.updated_at = crate::types::now();
    }

    /// Add a tag to the goal
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
            self.updated_at = crate::types::now();
        }
    }

    /// Check if goal is terminal (completed or deprecated)
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Check if goal is currently being worked on
    pub fn is_working(&self) -> bool {
        self.status.is_working()
    }

    /// Estimate time to complete (based on complexity distribution)
    ///
    /// Returns expected hours to complete, with confidence interval.
    pub fn estimated_time(&self) -> (f64, f64, f64) {
        // Simple heuristic: complexity * 2 hours
        let mean = self.complexity_estimate.mean * 2.0;
        let (lower, upper) = self.complexity_estimate.confidence_interval(0.95);

        (mean, lower * 2.0, upper * 2.0)
    }
}

/// Builder for constructing goals fluently
#[derive(Debug, Default)]
pub struct GoalBuilder {
    description: Option<String>,
    success_criteria: Vec<Predicate>,
    dependencies: Vec<Uuid>,
    anti_dependencies: Vec<Uuid>,
    complexity_estimate: Option<ProbabilityDistribution>,
    value_to_root: f64,
    atomic_contract: Option<AtomicContract>,
    parent_id: Option<Uuid>,
    validation_tests: Vec<String>,
    tags: Vec<String>,
}

impl GoalBuilder {
    /// Set the goal description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a success criterion
    pub fn add_success_criterion(mut self, predicate: Predicate) -> Self {
        self.success_criteria.push(predicate);
        self
    }

    /// Set all success criteria
    pub fn success_criteria(mut self, criteria: Vec<Predicate>) -> Self {
        self.success_criteria = criteria;
        self
    }

    /// Add a dependency
    pub fn add_dependency(mut self, dep: Uuid) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set all dependencies
    pub fn dependencies(mut self, deps: Vec<Uuid>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Add an anti-dependency
    pub fn add_anti_dependency(mut self, anti_dep: Uuid) -> Self {
        self.anti_dependencies.push(anti_dep);
        self
    }

    /// Set complexity estimate
    pub fn complexity(mut self, complexity: ProbabilityDistribution) -> Self {
        self.complexity_estimate = Some(complexity);
        self
    }

    /// Set value to root
    pub fn value_to_root(mut self, value: f64) -> Self {
        self.value_to_root = value;
        self
    }

    /// Set atomic contract
    pub fn atomic_contract(mut self, contract: AtomicContract) -> Self {
        self.atomic_contract = Some(contract);
        self
    }

    /// Set parent goal
    pub fn parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set validation tests
    pub fn validation_tests(mut self, tests: Vec<String>) -> Self {
        self.validation_tests = tests;
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Build the goal
    ///
    /// # Errors
    ///
    /// Returns `Err` if validation fails.
    pub fn build(self) -> Result<Goal> {
        let description = self
            .description
            .ok_or_else(|| GoalError::InvalidComplexity("Description required".to_string()))?;

        let now = crate::types::now();

        let goal = Goal {
            id: Uuid::new_v4(),
            description,
            success_criteria: self.success_criteria,
            dependencies: self.dependencies,
            anti_dependencies: self.anti_dependencies,
            complexity_estimate: self
                .complexity_estimate
                .unwrap_or_else(|| ProbabilityDistribution::point(5.0)),
            value_to_root: self.value_to_root,
            status: GoalStatus::Pending,
            current_lock: None,
            atomic_contract: self.atomic_contract,
            parent_id: self.parent_id,
            validation_tests: self.validation_tests,
            metadata: GoalMetadata {
                tags: self.tags,
                ..Default::default()
            },
            created_at: now,
            updated_at: now,
        };

        // Validate before returning
        goal.validate()?;

        Ok(goal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let goal = Goal::new("Test goal");
        assert_eq!(goal.description, "Test goal");
        assert_eq!(goal.status, GoalStatus::Pending);
    }

    #[test]
    fn test_goal_builder() {
        let goal = Goal::builder()
            .description("Test goal")
            .add_success_criterion(Predicate::AlwaysTrue)
            .value_to_root(0.5)
            .add_tag("test")
            .build()
            .unwrap();

        assert_eq!(goal.description, "Test goal");
        assert_eq!(goal.value_to_root, 0.5);
        assert_eq!(goal.success_criteria.len(), 1);
        assert!(goal.metadata.tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_goal_validation_empty_criteria() {
        let goal = Goal::new("Test");
        assert!(goal.validate().is_err());
    }

    #[test]
    fn test_goal_validation_invalid_value() {
        let mut goal = Goal::new("Test");
        goal.success_criteria.push(Predicate::AlwaysTrue);
        goal.value_to_root = 1.5; // Invalid: > 1.0

        assert!(goal.validate().is_err());
    }

    #[test]
    fn test_goal_state_transitions() {
        let mut goal = Goal::builder()
            .description("Test")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        assert_eq!(goal.status, GoalStatus::Pending);

        goal.mark_ready().unwrap();
        assert_eq!(goal.status, GoalStatus::Ready);

        goal.start().unwrap();
        assert_eq!(goal.status, GoalStatus::InProgress);

        goal.begin_validation().unwrap();
        assert_eq!(goal.status, GoalStatus::Validating);

        goal.complete().unwrap();
        assert_eq!(goal.status, GoalStatus::Completed);

        // Cannot transition from completed
        assert!(goal.mark_ready().is_err());
    }

    #[test]
    fn test_goal_block() {
        let mut goal = Goal::builder()
            .description("Test")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let blocker = Uuid::new_v4();
        goal.block("Waiting for dependency", vec![blocker]).unwrap();

        assert_eq!(goal.status, GoalStatus::Blocked);
        assert_eq!(
            goal.metadata.blocked_reason,
            Some("Waiting for dependency".to_string())
        );
        assert_eq!(goal.metadata.blocker_ids, vec![blocker]);
    }

    #[test]
    fn test_goal_fail() {
        let mut goal = Goal::builder()
            .description("Test")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        goal.mark_ready().unwrap();
        goal.start().unwrap();
        goal.fail("Tests failed").unwrap();

        assert_eq!(goal.status, GoalStatus::Failed);
        assert_eq!(
            goal.metadata.failure_reason,
            Some("Tests failed".to_string())
        );
        assert_eq!(goal.metadata.retry_count, 1);
    }

    #[test]
    fn test_goal_notes_and_tags() {
        let mut goal = Goal::builder()
            .description("Test")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        goal.add_note("First attempt");
        goal.add_tag("critical");
        goal.add_tag("frontend");

        assert_eq!(goal.metadata.notes.len(), 1);
        assert_eq!(goal.metadata.tags.len(), 2);
    }

    #[test]
    fn test_goal_estimated_time() {
        let goal = Goal::builder()
            .description("Test")
            .add_success_criterion(Predicate::AlwaysTrue)
            .complexity(ProbabilityDistribution::normal(5.0, 1.0))
            .build()
            .unwrap();

        let (mean, lower, upper) = goal.estimated_time();

        assert_eq!(mean, 10.0); // 5.0 * 2
        assert!(lower < mean);
        assert!(upper > mean);
    }
}
