//! Monte Carlo Simulator for deviation prediction
//!
//! This module implements Monte Carlo simulation to predict whether
//! an action will cause deviation BEFORE executing it.

use super::state::ProjectState;
use super::vector::AlignmentVector;
use crate::error::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Monte Carlo simulator for deviation prediction
///
/// The simulator runs thousands of parallel simulations to predict
/// the likely outcome of an action. This enables PREDICTIVE correction:
/// we can detect and prevent deviations before they happen.
///
/// # How it works
///
/// 1. Given current state and planned action
/// 2. Simulate action execution N times (N=1000 typical)
/// 3. For each simulation:
///    - Apply action with random perturbations (model uncertainty)
///    - Compute alignment of resulting state
/// 4. Analyze distribution of outcomes
/// 5. Predict probability of deviation
///
/// # Example
///
/// ```no_run
/// use sentinel_core::alignment::simulator::{MonteCarloSimulator, SimulationConfig};
/// use sentinel_core::alignment::state::ProjectState;
/// use sentinel_core::goal_manifold::GoalManifold;
/// use std::path::PathBuf;
///
/// # async fn example() -> sentinel_core::error::Result<()> {
/// let simulator = MonteCarloSimulator::new();
/// let state = ProjectState::new(PathBuf::from("."));
/// let config = SimulationConfig::default();
///
/// // Simulate action
/// let result = simulator.simulate_action(&state, config).await?;
///
/// println!("Deviation probability: {:.1}%", result.deviation_probability * 100.0);
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MonteCarloSimulator {
    /// Random number generator
    rng: rand::rngs::ThreadRng,
}

impl MonteCarloSimulator {
    /// Create a new simulator
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }

    /// Simulate an action and predict outcomes
    ///
    /// Runs Monte Carlo simulation to predict the distribution
    /// of possible outcomes from executing an action.
    pub async fn simulate_action(
        &self,
        initial_state: &ProjectState,
        config: SimulationConfig,
    ) -> Result<SimulationResult> {
        let mut futures = Vec::new();

        // Run parallel simulations
        for _i in 0..config.iterations {
            let future = self.simulate_single_trajectory(initial_state, &config);
            futures.push(future);
        }

        // Collect results
        let mut alignment_scores = Vec::new();
        for future in futures {
            let (_final_state, alignment) = future.await?;
            alignment_scores.push(alignment.score);
        }

        // Analyze results
        let result = self.analyze_simulation_results(alignment_scores, &config);

        Ok(result)
    }

    /// Simulate a single trajectory
    async fn simulate_single_trajectory(
        &self,
        initial_state: &ProjectState,
        config: &SimulationConfig,
    ) -> Result<(ProjectState, AlignmentVector)> {
        let mut state = initial_state.clone();

        // Apply action with random perturbations
        state = self.apply_action_with_uncertainty(&state, config);

        // Simulate forward in time
        for _step in 0..config.time_horizon {
            state = self.step_forward(&state, config);
        }

        // Compute final alignment
        let alignment = self.compute_alignment(&state, config);

        Ok((state, alignment))
    }

    /// Apply action with uncertainty modeling
    fn apply_action_with_uncertainty(
        &self,
        state: &ProjectState,
        config: &SimulationConfig,
    ) -> ProjectState {
        let mut new_state = state.clone();

        // Add random perturbations based on uncertainty model
        match config.uncertainty_model {
            UncertaintyModel::Realistic => {
                // Realistic: moderate random variations
                self.add_realistic_noise(&mut new_state);
            }
            UncertaintyModel::Optimistic => {
                // Optimistic: small variations
                self.add_small_noise(&mut new_state);
            }
            UncertaintyModel::Pessimistic => {
                // Pessimistic: large variations
                self.add_large_noise(&mut new_state);
            }
        }

        new_state
    }

    /// Step simulation forward one time step
    fn step_forward(&self, state: &ProjectState, _config: &SimulationConfig) -> ProjectState {
        // Simplified: for now just return the state
        // In full implementation, this would model state evolution
        state.clone()
    }

    /// Compute alignment for a state
    fn compute_alignment(
        &self,
        state: &ProjectState,
        _config: &SimulationConfig,
    ) -> AlignmentVector {
        // Simplified alignment computation
        // In full implementation, this would use AlignmentField

        // For now, use a simple heuristic:
        // Higher test coverage + more completed goals = better alignment
        let test_coverage_score = state.metrics.test_coverage * 40.0;

        let goal_completion_score = if state.goal_states.is_empty() {
            0.0
        } else {
            let completed = state
                .goal_states
                .values()
                .filter(|g| g.is_complete())
                .count() as f64;
            let total = state.goal_states.len() as f64;
            (completed / total) * 60.0
        };

        let score = test_coverage_score + goal_completion_score;

        AlignmentVector::new(score)
    }

    /// Add realistic noise to state
    fn add_realistic_noise(&self, state: &mut ProjectState) {
        let noise = rand::thread_rng().gen_range(-0.05..0.05);
        state.metrics.test_coverage = (state.metrics.test_coverage + noise).clamp(0.0, 1.0);
    }

    /// Add small noise (optimistic)
    fn add_small_noise(&self, state: &mut ProjectState) {
        let noise = rand::thread_rng().gen_range(-0.02..0.02);
        state.metrics.test_coverage = (state.metrics.test_coverage + noise).clamp(0.0, 1.0);
    }

    /// Add large noise (pessimistic)
    fn add_large_noise(&self, state: &mut ProjectState) {
        let noise = rand::thread_rng().gen_range(-0.1..0.1);
        state.metrics.test_coverage = (state.metrics.test_coverage + noise).clamp(0.0, 1.0);
    }

    /// Analyze simulation results
    fn analyze_simulation_results(
        &self,
        alignment_scores: Vec<f64>,
        config: &SimulationConfig,
    ) -> SimulationResult {
        if alignment_scores.is_empty() {
            return SimulationResult {
                deviation_probability: 0.0,
                expected_alignment: 0.0,
                worst_case: 0.0,
                best_case: 0.0,
                std_deviation: 0.0,
                confidence: 0.0,
            };
        }

        // Count deviations (score < threshold)
        let deviations = alignment_scores
            .iter()
            .filter(|&&score| score < config.deviation_threshold)
            .count();

        let deviation_probability = deviations as f64 / alignment_scores.len() as f64;

        // Statistics
        let expected_alignment =
            alignment_scores.iter().sum::<f64>() / alignment_scores.len() as f64;
        let worst_case = alignment_scores
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let best_case = alignment_scores
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // Standard deviation
        let variance = alignment_scores
            .iter()
            .map(|&score| {
                let diff = score - expected_alignment;
                diff * diff
            })
            .sum::<f64>()
            / alignment_scores.len() as f64;
        let std_deviation = variance.sqrt();

        // Confidence based on sample size and std deviation
        let confidence = (1.0 - (std_deviation / 100.0)).clamp(0.0, 1.0);

        SimulationResult {
            deviation_probability,
            expected_alignment,
            worst_case,
            best_case,
            std_deviation,
            confidence,
        }
    }
}

impl Default for MonteCarloSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for Monte Carlo simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Number of simulation iterations
    ///
    /// Higher = more accurate but slower
    /// Typical: 1000
    pub iterations: usize,

    /// How many steps to simulate forward
    ///
    /// Higher = look further into future
    /// Typical: 10
    pub time_horizon: usize,

    /// Uncertainty model to use
    pub uncertainty_model: UncertaintyModel,

    /// Threshold below which alignment is considered deviation
    ///
    /// Typical: 60.0
    pub deviation_threshold: f64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            time_horizon: 10,
            uncertainty_model: UncertaintyModel::Realistic,
            deviation_threshold: 60.0,
        }
    }
}

/// Uncertainty model for simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UncertaintyModel {
    /// Optimistic: assume small variations
    Optimistic,

    /// Realistic: moderate variations
    Realistic,

    /// Pessimistic: assume large variations
    Pessimistic,
}

/// Result of Monte Carlo simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Probability of deviation (0.0-1.0)
    ///
    /// Percentage of simulations that resulted in deviation
    pub deviation_probability: f64,

    /// Expected alignment score (mean of all simulations)
    pub expected_alignment: f64,

    /// Worst case alignment (minimum across simulations)
    pub worst_case: f64,

    /// Best case alignment (maximum across simulations)
    pub best_case: f64,

    /// Standard deviation of alignment scores
    pub std_deviation: f64,

    /// Confidence in this prediction (0.0-1.0)
    pub confidence: f64,
}

impl SimulationResult {
    /// Check if action is likely to cause deviation
    ///
    /// Uses 30% probability threshold (configurable)
    pub fn will_likely_deviate(&self) -> bool {
        self.deviation_probability > 0.3
    }

    /// Check if action is safe (low deviation risk)
    pub fn is_safe(&self) -> bool {
        self.deviation_probability < 0.1
    }

    /// Get risk level
    pub fn risk_level(&self) -> RiskLevel {
        match self.deviation_probability {
            p if p < 0.1 => RiskLevel::Low,
            p if p < 0.3 => RiskLevel::Medium,
            p if p < 0.6 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// Risk level for an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Low risk (<10% deviation probability)
    Low,

    /// Medium risk (10-30%)
    Medium,

    /// High risk (30-60%)
    High,

    /// Critical risk (>60%)
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_monte_carlo_simulation() {
        let simulator = MonteCarloSimulator::new();
        let state = ProjectState::new(PathBuf::from("/test"));

        let config = SimulationConfig {
            iterations: 100, // Fewer for faster tests
            time_horizon: 5,
            uncertainty_model: UncertaintyModel::Realistic,
            deviation_threshold: 60.0,
        };

        let result = simulator.simulate_action(&state, config).await.unwrap();

        // Should have valid results
        assert!(result.deviation_probability >= 0.0);
        assert!(result.deviation_probability <= 1.0);
        assert!(result.worst_case <= result.expected_alignment);
        assert!(result.expected_alignment <= result.best_case);
    }

    #[test]
    fn test_simulation_config_default() {
        let config = SimulationConfig::default();
        assert_eq!(config.iterations, 1000);
        assert_eq!(config.time_horizon, 10);
        assert_eq!(config.deviation_threshold, 60.0);
    }

    #[test]
    fn test_simulation_result_risk_levels() {
        let low_risk = SimulationResult {
            deviation_probability: 0.05,
            expected_alignment: 90.0,
            worst_case: 80.0,
            best_case: 95.0,
            std_deviation: 5.0,
            confidence: 0.95,
        };
        assert!(low_risk.is_safe());
        assert_eq!(low_risk.risk_level(), RiskLevel::Low);

        let high_risk = SimulationResult {
            deviation_probability: 0.5,
            expected_alignment: 50.0,
            worst_case: 30.0,
            best_case: 70.0,
            std_deviation: 15.0,
            confidence: 0.8,
        };
        assert!(high_risk.will_likely_deviate());
        assert_eq!(high_risk.risk_level(), RiskLevel::High);
    }

    #[test]
    fn test_uncertainty_models() {
        let _optimistic = UncertaintyModel::Optimistic;
        let _realistic = UncertaintyModel::Realistic;
        let _pessimistic = UncertaintyModel::Pessimistic;
        // Just test that they exist and can be created
    }
}
