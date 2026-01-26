//! Alignment Field - Continuous validation of goal alignment
//!
//! This module implements the Alignment Field, a continuous mathematical
//! field that measures how well the current project state aligns with
//! the Goal Manifold.
//!
//! # Core Concept
//!
//! The Alignment Field treats project state as a point in high-dimensional
//! space and computes a scalar "alignment score" at each point. The gradient
//! of this field points toward better alignment.
//!
//! Think of it like a topographical map:
//! - Height = Alignment score (0-100)
//! - Gradient = Direction of improvement
//! - Goal = Peak of the mountain
//! - Deviation = Falling into a valley
//!
//! # Usage
//!
//! ```no_run
//! use sentinel_core::alignment::field::AlignmentField;
//! use sentinel_core::alignment::state::ProjectState;
//! use sentinel_core::goal_manifold::GoalManifold;
//! use std::path::PathBuf;
//!
//! # async fn example() -> sentinel_core::error::Result<()> {
//! use sentinel_core::goal_manifold::Intent;
//!
//! let intent = Intent::new("Build feature", Vec::<String>::new());
//! let manifold = GoalManifold::new(intent);
//! let state = ProjectState::new(PathBuf::from("."));
//! let field = AlignmentField::new(manifold);
//!
//! // Compute current alignment
//! let vector = field.compute_alignment(&state).await?;
//! println!("Alignment score: {:.1}", vector.score);
//!
//! // Compute gradient for improvement direction
//! let gradient = field.compute_gradient(&state, 0.01).await?;
//! println!("Gradient magnitude: {:.3}", gradient.magnitude());
//! # Ok(())
//! # }
//! ```

use super::simulator::{MonteCarloSimulator, SimulationConfig};
use super::state::ProjectState;
use super::vector::{AlignmentVector, Vector};
use crate::error::Result;
use crate::goal_manifold::GoalManifold;
use serde::{Deserialize, Serialize};

/// Alignment Field - continuous validation system
///
/// The field computes alignment scores for project states and provides
/// gradient information for continuous improvement.
#[derive(Debug)]
pub struct AlignmentField {
    /// The goal manifold we're aligning to
    manifold: GoalManifold,

    /// Monte Carlo simulator for predictive analysis
    simulator: MonteCarloSimulator,

    /// Configuration for alignment computation
    config: AlignmentConfig,
}

impl AlignmentField {
    /// Create a new alignment field for a goal manifold
    pub fn new(manifold: GoalManifold) -> Self {
        Self {
            manifold,
            simulator: MonteCarloSimulator::new(),
            config: AlignmentConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(manifold: GoalManifold, config: AlignmentConfig) -> Self {
        Self {
            manifold,
            simulator: MonteCarloSimulator::new(),
            config,
        }
    }

    /// Compute alignment vector for current state
    ///
    /// This is the core function that evaluates how well the current
    /// project state aligns with the goal manifold.
    ///
    /// # Algorithm
    ///
    /// 1. For each goal in the manifold:
    ///    - Evaluate success criteria
    ///    - Weight by value_to_root
    /// 2. Compute overall alignment score (0-100)
    /// 3. Compute goal contribution vector
    /// 4. Compute deviation magnitude
    /// 5. Optionally: run Monte Carlo simulation for future prediction
    ///
    /// # Returns
    ///
    /// AlignmentVector with:
    /// - score: Overall alignment (0-100)
    /// - goal_contribution: Vector showing which goals contribute most
    /// - deviation_magnitude: How far from ideal alignment
    /// - confidence: Confidence in this measurement
    pub async fn compute_alignment(&self, state: &ProjectState) -> Result<AlignmentVector> {
        // Get all goals from manifold
        let goals = self.manifold.all_goals();

        if goals.is_empty() {
            // No goals = perfect alignment (nothing to do)
            return Ok(AlignmentVector::new(100.0));
        }

        // Compute contribution from each goal
        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        let mut goal_contributions = Vec::new();

        for goal in goals {
            // Weight by value_to_root (how important this goal is)
            let weight = goal.value_to_root;

            // Compute goal-specific score
            let goal_score = self.compute_goal_score(goal, state).await?;

            // Accumulate weighted score
            total_score += goal_score * weight;
            total_weight += weight;

            // Track individual contribution
            goal_contributions.push(goal_score * weight);
        }

        // Normalize score
        let alignment_score = if total_weight > 0.0 {
            (total_score / total_weight).clamp(0.0, 100.0)
        } else {
            100.0
        };

        // Create contribution vector (normalized)
        let contribution_vector = Vector::new(goal_contributions);

        // Compute deviation magnitude (distance from perfect alignment)
        let deviation = 100.0 - alignment_score;

        // Entropy gradient (rate of alignment change)
        // For now, use a simple heuristic based on test coverage and completion
        let entropy = self.compute_entropy(state);

        // Confidence based on test coverage and goal clarity
        let confidence = self.compute_confidence(state);

        Ok(AlignmentVector {
            score: alignment_score,
            goal_contribution: contribution_vector,
            deviation_magnitude: deviation,
            entropy_gradient: entropy,
            confidence,
        })
    }

    /// Compute gradient of alignment field at current state
    ///
    /// The gradient points in the direction of maximum alignment improvement.
    /// This uses finite difference approximation:
    ///
    /// ∇f(x) ≈ [f(x + εe₁) - f(x)] / ε for each dimension
    ///
    /// # Arguments
    ///
    /// * `state` - Current project state
    /// * `epsilon` - Step size for finite differences (default: 0.01)
    ///
    /// # Returns
    ///
    /// Vector pointing toward better alignment. Magnitude indicates
    /// how much alignment could improve.
    pub async fn compute_gradient(&self, state: &ProjectState, epsilon: f64) -> Result<Vector> {
        // Get current alignment
        let current = self.compute_alignment(state).await?;

        // Get state dimensions
        let dimensions = state.get_dimensions();

        // Compute partial derivative for each dimension
        let mut gradient_components = Vec::new();

        for dimension in dimensions {
            // Perturb state in this dimension
            let perturbed_state = state.perturb(&dimension, epsilon);

            // Compute alignment at perturbed state
            let perturbed = self.compute_alignment(&perturbed_state).await?;

            // Finite difference: (f(x + ε) - f(x)) / ε
            let partial_derivative = (perturbed.score - current.score) / epsilon;

            gradient_components.push(partial_derivative);
        }

        Ok(Vector::new(gradient_components))
    }

    /// Predict future alignment using Monte Carlo simulation
    ///
    /// Runs probabilistic simulations to predict whether the current
    /// trajectory will lead to deviation.
    ///
    /// # Returns
    ///
    /// SimulationResult with deviation probability and expected alignment
    pub async fn predict_alignment(
        &self,
        state: &ProjectState,
    ) -> Result<super::simulator::SimulationResult> {
        let sim_config = SimulationConfig {
            iterations: self.config.monte_carlo_iterations,
            time_horizon: self.config.prediction_horizon,
            uncertainty_model: self.config.uncertainty_model,
            deviation_threshold: self.config.deviation_threshold,
        };

        self.simulator.simulate_action(state, sim_config).await
    }

    /// Compute alignment score for a single goal
    async fn compute_goal_score(
        &self,
        goal: &crate::goal_manifold::goal::Goal,
        state: &ProjectState,
    ) -> Result<f64> {
        use crate::types::GoalStatus;

        // Base score from goal status
        let status_score = match goal.status {
            GoalStatus::Completed => 100.0,
            GoalStatus::Validating => 90.0,
            GoalStatus::InProgress => 50.0,
            GoalStatus::Ready => 20.0,
            GoalStatus::Pending => 10.0,
            GoalStatus::Blocked => 5.0,
            GoalStatus::Failed => 0.0,
            GoalStatus::Deprecated => 0.0,
        };

        // Check if we have state information for this goal
        if let Some(goal_state) = state.goal_states.get(&goal.id) {
            // If we have detailed state, use that
            Ok(if goal_state.is_complete() {
                100.0
            } else {
                goal_state.progress() * 100.0
            })
        } else {
            // Otherwise use status-based score
            Ok(status_score)
        }
    }

    /// Compute entropy (disorder/uncertainty in alignment)
    fn compute_entropy(&self, state: &ProjectState) -> f64 {
        // Simple heuristic: entropy decreases as test coverage increases
        // and as more goals are completed

        let test_entropy = 1.0 - state.metrics.test_coverage;

        let goal_entropy = if state.goal_states.is_empty() {
            1.0
        } else {
            let incomplete = state
                .goal_states
                .values()
                .filter(|g| !g.is_complete())
                .count() as f64;
            let total = state.goal_states.len() as f64;
            incomplete / total
        };

        // Average entropy
        (test_entropy + goal_entropy) / 2.0
    }

    /// Compute confidence in alignment measurement
    fn compute_confidence(&self, state: &ProjectState) -> f64 {
        // Confidence increases with:
        // 1. Test coverage (we can verify claims)
        // 2. Number of goals with clear state
        // 3. Recency of measurements

        let test_confidence = state.metrics.test_coverage;

        let goal_confidence = if state.goal_states.is_empty() {
            0.5 // Moderate confidence without goal states
        } else {
            1.0 // High confidence with goal tracking
        };

        // Weighted average (tests are 60% of confidence)
        0.6 * test_confidence + 0.4 * goal_confidence
    }

    /// Get reference to the goal manifold
    pub fn manifold(&self) -> &GoalManifold {
        &self.manifold
    }

    /// Get mutable reference to configuration
    pub fn config_mut(&mut self) -> &mut AlignmentConfig {
        &mut self.config
    }
}

/// Configuration for alignment field computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// Number of Monte Carlo iterations for prediction
    ///
    /// Higher = more accurate predictions but slower
    /// Default: 1000
    pub monte_carlo_iterations: usize,

    /// How many steps to predict into the future
    ///
    /// Default: 10
    pub prediction_horizon: usize,

    /// Uncertainty model for simulations
    ///
    /// Default: Realistic
    pub uncertainty_model: super::simulator::UncertaintyModel,

    /// Alignment threshold below which we consider deviation
    ///
    /// Default: 60.0
    pub deviation_threshold: f64,

    /// Step size for gradient computation
    ///
    /// Smaller = more accurate but slower
    /// Default: 0.01
    pub gradient_epsilon: f64,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            monte_carlo_iterations: 1000,
            prediction_horizon: 10,
            uncertainty_model: super::simulator::UncertaintyModel::Realistic,
            deviation_threshold: 60.0,
            gradient_epsilon: 0.01,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_manifold::goal::Goal;
    use crate::goal_manifold::predicate::Predicate;
    use crate::goal_manifold::Intent;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_alignment_field_creation() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let field = AlignmentField::new(manifold);

        assert_eq!(field.config.monte_carlo_iterations, 1000);
        assert_eq!(field.config.prediction_horizon, 10);
    }

    #[tokio::test]
    async fn test_empty_manifold_perfect_alignment() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let field = AlignmentField::new(manifold);

        let state = ProjectState::new(PathBuf::from("/test"));
        let alignment = field.compute_alignment(&state).await.unwrap();

        // No goals = perfect alignment
        assert_eq!(alignment.score, 100.0);
    }

    #[tokio::test]
    async fn test_alignment_with_goals() {
        let intent = Intent::new("Build API", vec!["TypeScript", "PostgreSQL"]);
        let mut manifold = GoalManifold::new(intent);

        // Add a completed goal
        let goal1 = Goal::builder()
            .description("Setup project")
            .add_success_criterion(Predicate::AlwaysTrue)
            .value_to_root(0.3)
            .build()
            .unwrap();

        let goal1_id = goal1.id;
        manifold.add_goal(goal1).unwrap();

        // Mark as completed
        let goal1_mut = manifold.get_goal_mut(&goal1_id).unwrap();
        goal1_mut.mark_ready().unwrap();
        goal1_mut.start().unwrap();
        goal1_mut.begin_validation().unwrap();
        goal1_mut.complete().unwrap();

        // Add a pending goal
        let goal2 = Goal::builder()
            .description("Implement API")
            .add_success_criterion(Predicate::AlwaysTrue)
            .value_to_root(0.7)
            .build()
            .unwrap();

        manifold.add_goal(goal2).unwrap();

        let field = AlignmentField::new(manifold);
        let state = ProjectState::new(PathBuf::from("/test"));

        let alignment = field.compute_alignment(&state).await.unwrap();

        // Should be between 0 and 100
        assert!(alignment.score >= 0.0);
        assert!(alignment.score <= 100.0);

        // Should be partial completion (not 100, not 0)
        assert!(alignment.score > 0.0);
        assert!(alignment.score < 100.0);
    }

    #[tokio::test]
    async fn test_gradient_computation() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let field = AlignmentField::new(manifold);

        let state = ProjectState::new(PathBuf::from("/test"));
        let gradient = field.compute_gradient(&state, 0.01).await.unwrap();

        // Gradient should exist (even if zero for perfect alignment)
        assert!(gradient.components.len() > 0);
    }

    #[tokio::test]
    async fn test_prediction() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let field = AlignmentField::new(manifold);

        let state = ProjectState::new(PathBuf::from("/test"));
        let prediction = field.predict_alignment(&state).await.unwrap();

        // Should have valid probability
        assert!(prediction.deviation_probability >= 0.0);
        assert!(prediction.deviation_probability <= 1.0);

        // Should have valid alignment scores
        assert!(prediction.expected_alignment >= 0.0);
        assert!(prediction.expected_alignment <= 100.0);
    }

    #[test]
    fn test_alignment_config_default() {
        let config = AlignmentConfig::default();
        assert_eq!(config.monte_carlo_iterations, 1000);
        assert_eq!(config.prediction_horizon, 10);
        assert_eq!(config.deviation_threshold, 60.0);
        assert_eq!(config.gradient_epsilon, 0.01);
    }
}
