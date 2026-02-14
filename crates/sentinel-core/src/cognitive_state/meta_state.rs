//! Meta-Cognitive State - Agent's awareness of its own thinking
//!
//! This module implements meta-cognition: the agent's ability to reason
//! about its own cognitive processes.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// The agent's awareness of its own cognitive processes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaCognitiveState {
    /// Expected alignment score for current trajectory
    pub expected_alignment: f64,

    /// Confidence in the current plan (0.0-1.0)
    pub confidence_in_plan: f64,

    /// History of prediction accuracy
    /// (difference between predicted and actual alignment)
    pub prediction_accuracy_history: Vec<f64>,

    /// Known cognitive biases
    pub known_biases: Vec<String>,

    /// Current cognitive load (0.0-1.0)
    /// Higher = more complexity being managed
    pub cognitive_load: f64,

    /// Meta-cognitive insights
    pub insights: Vec<MetaInsight>,
}

impl MetaCognitiveState {
    /// Create a new meta-cognitive state
    pub fn new() -> Self {
        Self {
            expected_alignment: 100.0,
            confidence_in_plan: 0.5,
            prediction_accuracy_history: Vec::new(),
            known_biases: Vec::new(),
            cognitive_load: 0.0,
            insights: Vec::new(),
        }
    }

    /// Update prediction accuracy after observing outcome
    pub fn update_prediction_accuracy(&mut self, predicted: f64, actual: f64) {
        let error = (predicted - actual).abs();
        self.prediction_accuracy_history.push(error);

        // Keep only last 100 predictions
        if self.prediction_accuracy_history.len() > 100 {
            self.prediction_accuracy_history.remove(0);
        }

        // Update confidence based on recent accuracy
        let recent_errors: Vec<_> = self
            .prediction_accuracy_history
            .iter()
            .rev()
            .take(10)
            .copied()
            .collect();

        if !recent_errors.is_empty() {
            let mean_error: f64 = recent_errors.iter().sum::<f64>() / recent_errors.len() as f64;
            // Lower error = higher confidence
            self.confidence_in_plan = (1.0 - (mean_error / 100.0)).clamp(0.0, 1.0);
        }
    }

    /// Get average prediction accuracy (lower is better)
    pub fn average_prediction_error(&self) -> f64 {
        if self.prediction_accuracy_history.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.prediction_accuracy_history.iter().sum();
        sum / self.prediction_accuracy_history.len() as f64
    }

    /// Check if predictions are reliable
    pub fn predictions_are_reliable(&self) -> bool {
        self.average_prediction_error() < 10.0 && self.prediction_accuracy_history.len() >= 5
    }

    /// Add a cognitive bias
    pub fn add_bias(&mut self, bias: String) {
        if !self.known_biases.contains(&bias) {
            self.known_biases.push(bias);
        }
    }

    /// Update cognitive load
    pub fn set_cognitive_load(&mut self, load: f64) {
        self.cognitive_load = load.clamp(0.0, 1.0);
    }

    /// Check if cognitive load is too high
    pub fn is_overloaded(&self) -> bool {
        self.cognitive_load > 0.8
    }

    /// Add a meta-cognitive insight
    pub fn add_insight(&mut self, insight: MetaInsight) {
        self.insights.push(insight);
    }

    /// Get recent insights
    pub fn recent_insights(&self, count: usize) -> Vec<&MetaInsight> {
        self.insights.iter().rev().take(count).collect()
    }
}

impl Default for MetaCognitiveState {
    fn default() -> Self {
        Self::new()
    }
}

/// A meta-cognitive insight
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaInsight {
    /// Type of insight
    pub insight_type: InsightType,

    /// Description
    pub description: String,

    /// Confidence in this insight (0.0-1.0)
    pub confidence: f64,

    /// When this insight was formed
    pub formed_at: crate::types::Timestamp,
}

impl MetaInsight {
    /// Create a new insight
    pub fn new(insight_type: InsightType, description: String, confidence: f64) -> Self {
        Self {
            insight_type,
            description,
            confidence: confidence.clamp(0.0, 1.0),
            formed_at: Utc::now(),
        }
    }
}

/// Type of meta-cognitive insight
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    /// Pattern recognized in own behavior
    BehavioralPattern,

    /// Cognitive bias identified
    BiasDetection,

    /// Improved prediction method
    PredictionImprovement,

    /// Better problem-solving strategy
    StrategyImprovement,

    /// Learning about own limitations
    LimitationAwareness,

    /// Successful correction of deviation
    SuccessfulCorrection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_state_creation() {
        let state = MetaCognitiveState::new();
        assert_eq!(state.expected_alignment, 100.0);
        assert_eq!(state.confidence_in_plan, 0.5);
        assert_eq!(state.cognitive_load, 0.0);
    }

    #[test]
    fn test_update_prediction_accuracy() {
        let mut state = MetaCognitiveState::new();

        // Add some predictions
        state.update_prediction_accuracy(90.0, 85.0); // error: 5.0
        state.update_prediction_accuracy(80.0, 82.0); // error: 2.0
        state.update_prediction_accuracy(95.0, 94.0); // error: 1.0

        assert_eq!(state.prediction_accuracy_history.len(), 3);

        // Average error should be (5 + 2 + 1) / 3 = 2.67
        let avg_error = state.average_prediction_error();
        assert!((avg_error - 2.67).abs() < 0.01);
    }

    #[test]
    fn test_predictions_reliable() {
        let mut state = MetaCognitiveState::new();

        // Not reliable yet (not enough data)
        assert!(!state.predictions_are_reliable());

        // Add accurate predictions
        for _ in 0..10 {
            state.update_prediction_accuracy(90.0, 91.0); // error: 1.0
        }

        // Should be reliable now
        assert!(state.predictions_are_reliable());
    }

    #[test]
    fn test_predictions_unreliable() {
        let mut state = MetaCognitiveState::new();

        // Add inaccurate predictions
        for _ in 0..10 {
            state.update_prediction_accuracy(90.0, 70.0); // error: 20.0
        }

        // Should not be reliable (high error)
        assert!(!state.predictions_are_reliable());
    }

    #[test]
    fn test_add_bias() {
        let mut state = MetaCognitiveState::new();
        state.add_bias("Confirmation bias".to_string());
        state.add_bias("Availability bias".to_string());

        assert_eq!(state.known_biases.len(), 2);

        // Adding same bias again shouldn't duplicate
        state.add_bias("Confirmation bias".to_string());
        assert_eq!(state.known_biases.len(), 2);
    }

    #[test]
    fn test_cognitive_load() {
        let mut state = MetaCognitiveState::new();

        state.set_cognitive_load(0.5);
        assert_eq!(state.cognitive_load, 0.5);
        assert!(!state.is_overloaded());

        state.set_cognitive_load(0.9);
        assert!(state.is_overloaded());
    }

    #[test]
    fn test_add_insight() {
        let mut state = MetaCognitiveState::new();

        let insight = MetaInsight::new(
            InsightType::BehavioralPattern,
            "I tend to overestimate completion time".to_string(),
            0.8,
        );

        state.add_insight(insight);
        assert_eq!(state.insights.len(), 1);
    }

    #[test]
    fn test_recent_insights() {
        let mut state = MetaCognitiveState::new();

        for i in 0..5 {
            let insight = MetaInsight::new(
                InsightType::PredictionImprovement,
                format!("Insight {}", i),
                0.7,
            );
            state.add_insight(insight);
        }

        let recent = state.recent_insights(3);
        assert_eq!(recent.len(), 3);

        // Should be in reverse order (most recent first)
        assert_eq!(recent[0].description, "Insight 4");
    }

    #[test]
    fn test_confidence_improves_with_accuracy() {
        let mut state = MetaCognitiveState::new();
        let initial_confidence = state.confidence_in_plan;

        // Add very accurate predictions
        for _ in 0..10 {
            state.update_prediction_accuracy(90.0, 90.5); // error: 0.5
        }

        // Confidence should have improved
        assert!(state.confidence_in_plan > initial_confidence);
    }

    #[test]
    fn test_confidence_decreases_with_inaccuracy() {
        let mut state = MetaCognitiveState::new();
        state.confidence_in_plan = 0.8; // Start high

        // Add inaccurate predictions
        for _ in 0..10 {
            state.update_prediction_accuracy(90.0, 50.0); // error: 40.0
        }

        // Confidence should have decreased
        assert!(state.confidence_in_plan < 0.8);
    }
}
