//! Decision tracking - Record why actions were taken
//!
//! This module tracks the reasoning behind each decision, enabling
//! learning and accountability.

use super::action::Action;
use crate::alignment::SimulationResult;
use crate::types::Timestamp;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A decision made by the agent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Decision {
    /// Unique identifier
    pub id: Uuid,

    /// The action that was decided upon
    pub action: Action,

    /// Rationale for this decision
    pub rationale: Rationale,

    /// Alignment prediction at time of decision
    pub alignment_prediction: Option<SimulationResult>,

    /// When this decision was made
    pub timestamp: Timestamp,

    /// Outcome (filled in after action execution)
    pub outcome: Option<DecisionOutcome>,
}

impl Decision {
    /// Create a new decision
    pub fn new(action: Action, rationale: Rationale) -> Self {
        Self {
            id: Uuid::new_v4(),
            action,
            rationale,
            alignment_prediction: None,
            timestamp: Utc::now(),
            outcome: None,
        }
    }

    /// Add alignment prediction
    pub fn with_prediction(mut self, prediction: SimulationResult) -> Self {
        self.alignment_prediction = Some(prediction);
        self
    }

    /// Record outcome after action execution
    pub fn record_outcome(&mut self, outcome: DecisionOutcome) {
        self.outcome = Some(outcome);
    }

    /// Check if prediction matched reality
    pub fn prediction_was_accurate(&self) -> Option<bool> {
        let prediction = self.alignment_prediction.as_ref()?;
        let outcome = self.outcome.as_ref()?;

        // Check if predicted deviation matched actual deviation
        let predicted_deviation = prediction.will_likely_deviate();
        let actual_deviation = outcome.actual_alignment < 60.0;

        Some(predicted_deviation == actual_deviation)
    }
}

/// Rationale for a decision
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rationale {
    /// Is this action justified?
    pub is_justified: bool,

    /// Explanation of why/why not
    pub reason: String,

    /// Expected value from this action (0.0-1.0)
    pub expected_value: f64,

    /// Goals this action contributes to
    pub contributing_goals: Vec<Uuid>,

    /// Alternative actions considered
    pub alternatives_considered: Vec<Action>,
}

impl Rationale {
    /// Create a justified rationale
    pub fn justified(reason: String, expected_value: f64) -> Self {
        Self {
            is_justified: true,
            reason,
            expected_value: expected_value.clamp(0.0, 1.0),
            contributing_goals: Vec::new(),
            alternatives_considered: Vec::new(),
        }
    }

    /// Create an unjustified rationale
    pub fn unjustified(reason: String) -> Self {
        Self {
            is_justified: false,
            reason,
            expected_value: 0.0,
            contributing_goals: Vec::new(),
            alternatives_considered: Vec::new(),
        }
    }

    /// Add contributing goal
    pub fn for_goal(mut self, goal_id: Uuid) -> Self {
        self.contributing_goals.push(goal_id);
        self
    }

    /// Add alternative action that was considered
    pub fn with_alternative(mut self, action: Action) -> Self {
        self.alternatives_considered.push(action);
        self
    }
}

/// Outcome of a decision (filled in after execution)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionOutcome {
    /// Whether action succeeded
    pub success: bool,

    /// Actual alignment score after action
    pub actual_alignment: f64,

    /// Time taken to execute (seconds)
    pub duration: f64,

    /// Unexpected consequences
    pub unexpected_consequences: Vec<String>,

    /// Learnings from this decision
    pub learnings: Vec<String>,
}

impl DecisionOutcome {
    /// Create a successful outcome
    pub fn success(actual_alignment: f64, duration: f64) -> Self {
        Self {
            success: true,
            actual_alignment,
            duration,
            unexpected_consequences: Vec::new(),
            learnings: Vec::new(),
        }
    }

    /// Create a failed outcome
    pub fn failure(actual_alignment: f64, duration: f64) -> Self {
        Self {
            success: false,
            actual_alignment,
            duration,
            unexpected_consequences: Vec::new(),
            learnings: Vec::new(),
        }
    }

    /// Add unexpected consequence
    pub fn with_consequence(mut self, consequence: String) -> Self {
        self.unexpected_consequences.push(consequence);
        self
    }

    /// Add learning
    pub fn with_learning(mut self, learning: String) -> Self {
        self.learnings.push(learning);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive_state::action::ActionType;

    #[test]
    fn test_decision_creation() {
        let action = Action::new(
            ActionType::RunTests {
                suite: "unit".to_string(),
            },
            "Run unit tests".to_string(),
        );

        let rationale = Rationale::justified("Verify functionality".to_string(), 0.8);
        let decision = Decision::new(action, rationale);

        assert!(decision.rationale.is_justified);
        assert_eq!(decision.rationale.expected_value, 0.8);
    }

    #[test]
    fn test_rationale_justified() {
        let rationale = Rationale::justified("Good reason".to_string(), 0.9);
        assert!(rationale.is_justified);
        assert_eq!(rationale.expected_value, 0.9);
    }

    #[test]
    fn test_rationale_unjustified() {
        let rationale = Rationale::unjustified("No good reason".to_string());
        assert!(!rationale.is_justified);
        assert_eq!(rationale.expected_value, 0.0);
    }

    #[test]
    fn test_rationale_for_goal() {
        let goal_id = Uuid::new_v4();
        let rationale = Rationale::justified("Test".to_string(), 0.5).for_goal(goal_id);

        assert_eq!(rationale.contributing_goals.len(), 1);
        assert_eq!(rationale.contributing_goals[0], goal_id);
    }

    #[test]
    fn test_decision_outcome_success() {
        let outcome = DecisionOutcome::success(85.0, 2.5);
        assert!(outcome.success);
        assert_eq!(outcome.actual_alignment, 85.0);
        assert_eq!(outcome.duration, 2.5);
    }

    #[test]
    fn test_decision_outcome_failure() {
        let outcome = DecisionOutcome::failure(45.0, 1.0);
        assert!(!outcome.success);
        assert_eq!(outcome.actual_alignment, 45.0);
    }

    #[test]
    fn test_decision_outcome_with_learning() {
        let outcome =
            DecisionOutcome::success(90.0, 1.5).with_learning("Tests are important".to_string());

        assert_eq!(outcome.learnings.len(), 1);
    }

    #[test]
    fn test_decision_record_outcome() {
        let action = Action::new(
            ActionType::RunTests {
                suite: "integration".to_string(),
            },
            "Run tests".to_string(),
        );

        let rationale = Rationale::justified("Verify".to_string(), 0.7);
        let mut decision = Decision::new(action, rationale);

        let outcome = DecisionOutcome::success(88.0, 3.0);
        decision.record_outcome(outcome);

        assert!(decision.outcome.is_some());
        assert_eq!(decision.outcome.unwrap().actual_alignment, 88.0);
    }
}
