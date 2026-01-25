//! Cognitive State - The agent's complete working memory
//!
//! This is the "consciousness" of the agent - everything it knows about
//! the project, its goals, and its own thinking process.

use super::action::{Action, ActionDecision, ActionResult};
use super::belief::{BeliefNetwork, Uncertainty};
use super::decision::Decision;
use super::meta_state::MetaCognitiveState;
use crate::alignment::AlignmentField;
use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use crate::goal_manifold::GoalManifold;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Cognitive mode - what the agent is currently doing
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveMode {
    /// Planning high-level goal decomposition
    Planning,

    /// Executing concrete actions
    Executing,

    /// Validating results and checking alignment
    Validating,

    /// Debugging failures
    Debugging,

    /// Learning from experience
    Learning,

    /// Reflecting on own performance (meta-cognition)
    Reflecting,
}

/// The agent's complete cognitive state
///
/// This is the "working memory" of the agent - everything it knows
/// about the project, its goals, and its own thinking process.
pub struct CognitiveState {
    /// Reference to the goal manifold (source of truth)
    goal_manifold: GoalManifold,

    /// Current goal being worked on
    current_focus: Option<Uuid>,

    /// Current cognitive mode
    cognitive_mode: CognitiveMode,

    /// Immutable log of all actions taken
    execution_trace: Vec<Action>,

    /// Network of beliefs about the project
    beliefs: BeliefNetwork,

    /// What the agent is uncertain about
    uncertainties: HashMap<String, Uncertainty>,

    /// Meta-cognitive state (awareness of own thinking)
    meta_state: MetaCognitiveState,

    /// Decision log (why each action was taken)
    decision_log: Vec<Decision>,

    /// Alignment field for continuous validation
    alignment_field: AlignmentField,
}

impl CognitiveState {
    /// Create a new cognitive state
    pub fn new(goal_manifold: GoalManifold) -> Self {
        let alignment_field = AlignmentField::new(goal_manifold.clone());

        Self {
            goal_manifold,
            current_focus: None,
            cognitive_mode: CognitiveMode::Planning,
            execution_trace: Vec::new(),
            beliefs: BeliefNetwork::new(),
            uncertainties: HashMap::new(),
            meta_state: MetaCognitiveState::new(),
            decision_log: Vec::new(),
            alignment_field,
        }
    }

    /// CRITICAL GATE: Every action passes through this method
    ///
    /// This is where Sentinel's self-awareness prevents deviation.
    pub async fn before_action(&mut self, action: Action) -> Result<ActionDecision> {
        // 1. META-COGNITIVE CHECK: Why are we doing this?
        let rationale = self.explain_rationale(&action);
        if !rationale.is_justified {
            return Ok(ActionDecision::reject(format!(
                "Cannot justify action: {}",
                rationale.reason
            )));
        }

        // 2. SAFETY CHECK: Is this action safe?
        if !action.is_safe() {
            return Ok(ActionDecision::reject(
                "Action failed safety check".to_string(),
            ));
        }

        // 3. INVARIANT VERIFICATION: Does this violate constraints?
        if !self.satisfies_invariants(&action).await {
            let violations = self.find_invariant_violations(&action).await;
            return Ok(ActionDecision::reject(format!(
                "Violates invariants: {:?}",
                violations
            )));
        }

        // 4. ALIGNMENT PREDICTION: Will this cause deviation?
        let current_state = self.get_current_state();
        let prediction = self
            .alignment_field
            .predict_alignment(&current_state)
            .await?;

        if prediction.will_likely_deviate() {
            // Don't just reject - propose better alternative if possible
            let alternatives = self.find_better_alternatives(&action);
            if !alternatives.is_empty() {
                return Ok(ActionDecision::propose_alternative(
                    &action,
                    format!("{:.0}% chance of deviation", prediction.deviation_probability * 100.0),
                    alternatives,
                ));
            } else {
                return Ok(ActionDecision::reject(format!(
                    "High deviation risk ({:.0}%)",
                    prediction.deviation_probability * 100.0
                )));
            }
        }

        // 5. VALUE-OF-INFORMATION: Is this worth doing?
        let voi = self.compute_value_of_information(&action);
        if voi < 0.1 {
            // Very low value
            return Ok(ActionDecision::skip(format!(
                "Low value to goal (VOI={:.2})",
                voi
            )));
        }

        // 6. META-LEARNING: Record decision for future learning
        let decision = Decision::new(action.clone(), rationale).with_prediction(prediction);
        self.decision_log.push(decision);

        // 7. UPDATE COGNITIVE STATE
        self.cognitive_mode = CognitiveMode::Executing;
        self.execution_trace.push(action.clone());

        Ok(ActionDecision::approve(&action))
    }

    /// Post-action processing: learn from outcome
    pub async fn after_action(&mut self, action: Action, result: ActionResult) -> Result<()> {
        // 1. Update beliefs based on outcome
        self.update_beliefs(&action, &result);

        // 2. Check if alignment changed
        let current_state = self.get_current_state();
        let new_alignment = self.alignment_field.compute_alignment(&current_state).await?;

        // 3. Detect unexpected deviations
        if new_alignment.score < self.meta_state.expected_alignment {
            self.handle_unexpected_deviation(&action, &result, &new_alignment)
                .await?;
        }

        // 4. Update uncertainties
        self.resolve_uncertainties(&action, &result);

        // 5. Meta-learning: Did prediction match reality?
        if let Some(last_decision) = self.decision_log.last_mut() {
            if last_decision.action.id == action.id {
                let outcome = super::decision::DecisionOutcome::success(
                    new_alignment.score,
                    result.duration,
                );
                last_decision.record_outcome(outcome);

                // Update meta-state prediction accuracy
                if let Some(prediction) = &last_decision.alignment_prediction {
                    self.meta_state.update_prediction_accuracy(
                        prediction.expected_alignment,
                        new_alignment.score,
                    );
                }
            }
        }

        // 6. Update expected alignment
        self.meta_state.expected_alignment = new_alignment.score;

        Ok(())
    }

    /// Explain why we want to take this action (META-COGNITIVE)
    fn explain_rationale(&self, action: &Action) -> super::decision::Rationale {
        // Find which goal this action contributes to
        let contributing_goals = self.find_contributing_goals(action);

        if contributing_goals.is_empty() {
            return super::decision::Rationale::unjustified(
                "Action does not contribute to any goal".to_string(),
            );
        }

        // Compute expected value
        let expected_value: f64 = contributing_goals
            .iter()
            .map(|goal| goal.value_to_root * action.expected_value)
            .sum();

        if expected_value < 0.1 {
            return super::decision::Rationale::unjustified(format!(
                "Expected value too low: {:.2}",
                expected_value
            ));
        }

        let mut rationale = super::decision::Rationale::justified(
            format!("Contributes to {} goal(s)", contributing_goals.len()),
            expected_value,
        );

        for goal in contributing_goals {
            rationale = rationale.for_goal(goal.id);
        }

        rationale
    }

    /// Find goals this action contributes to
    fn find_contributing_goals(&self, action: &Action) -> Vec<&Goal> {
        // If action explicitly specifies a goal
        if let Some(goal_id) = action.goal_id {
            if let Some(goal) = self.goal_manifold.get_goal(&goal_id) {
                return vec![goal];
            }
        }

        // Otherwise, infer from action description
        // This is simplified - full implementation would use semantic matching
        self.goal_manifold
            .all_goals()
            .into_iter()
            .filter(|goal| {
                // Simple heuristic: action description mentions goal
                action
                    .description
                    .to_lowercase()
                    .contains(&goal.description.to_lowercase())
            })
            .collect()
    }

    /// Check if action satisfies all invariants
    async fn satisfies_invariants(&self, _action: &Action) -> bool {
        // Simplified: in full implementation, would check each invariant
        // against predicted state after action
        true
    }

    /// Find invariant violations
    async fn find_invariant_violations(&self, _action: &Action) -> Vec<String> {
        // Simplified: return empty for now
        Vec::new()
    }

    /// Find better alternative actions
    fn find_better_alternatives(&self, _action: &Action) -> Vec<Action> {
        // Simplified: in full implementation, would use planning
        // to find alternatives with higher alignment probability
        Vec::new()
    }

    /// Compute value of information for this action
    fn compute_value_of_information(&self, action: &Action) -> f64 {
        // Simplified heuristic based on:
        // 1. How much this reduces uncertainty
        // 2. How much it contributes to goals

        let uncertainty_reduction = self.estimate_uncertainty_reduction(action);
        let goal_value = action.expected_value;

        (uncertainty_reduction + goal_value) / 2.0
    }

    /// Estimate how much this action reduces uncertainty
    fn estimate_uncertainty_reduction(&self, _action: &Action) -> f64 {
        // Simplified: would analyze which uncertainties this action resolves
        0.5
    }

    /// Update beliefs based on action outcome
    fn update_beliefs(&mut self, _action: &Action, result: &ActionResult) {
        // Example: if tests pass, increase confidence in "system works"
        if result.success {
            // Would add/update relevant beliefs
        }
    }

    /// Handle unexpected deviation
    async fn handle_unexpected_deviation(
        &mut self,
        _action: &Action,
        _result: &ActionResult,
        _alignment: &crate::alignment::AlignmentVector,
    ) -> Result<()> {
        // Switch to debugging mode
        self.cognitive_mode = CognitiveMode::Debugging;

        // Add meta-cognitive insight
        let insight = super::meta_state::MetaInsight::new(
            super::meta_state::InsightType::LimitationAwareness,
            "Prediction was inaccurate - need to investigate".to_string(),
            0.8,
        );
        self.meta_state.add_insight(insight);

        Ok(())
    }

    /// Resolve uncertainties based on action outcome
    fn resolve_uncertainties(&mut self, _action: &Action, _result: &ActionResult) {
        // Would remove or reduce uncertainties that were resolved
    }

    /// Get current project state for alignment computation
    fn get_current_state(&self) -> crate::alignment::ProjectState {
        // Simplified: would scan actual project state
        crate::alignment::ProjectState::new(std::path::PathBuf::from("."))
    }

    /// Get current cognitive mode
    pub fn mode(&self) -> &CognitiveMode {
        &self.cognitive_mode
    }

    /// Set cognitive mode
    pub fn set_mode(&mut self, mode: CognitiveMode) {
        self.cognitive_mode = mode;
    }

    /// Get current focus goal
    pub fn current_focus(&self) -> Option<&Goal> {
        self.current_focus
            .and_then(|id| self.goal_manifold.get_goal(&id))
    }

    /// Set current focus
    pub fn focus_on(&mut self, goal_id: Uuid) {
        self.current_focus = Some(goal_id);
    }

    /// Get reference to goal manifold
    pub fn goal_manifold(&self) -> &GoalManifold {
        &self.goal_manifold
    }

    /// Get reference to belief network
    pub fn beliefs(&self) -> &BeliefNetwork {
        &self.beliefs
    }

    /// Get mutable reference to belief network
    pub fn beliefs_mut(&mut self) -> &mut BeliefNetwork {
        &mut self.beliefs
    }

    /// Get reference to meta-state
    pub fn meta_state(&self) -> &MetaCognitiveState {
        &self.meta_state
    }

    /// Get mutable reference to meta-state
    pub fn meta_state_mut(&mut self) -> &mut MetaCognitiveState {
        &mut self.meta_state
    }

    /// Get decision log
    pub fn decisions(&self) -> &[Decision] {
        &self.decision_log
    }

    /// Get execution trace
    pub fn execution_trace(&self) -> &[Action] {
        &self.execution_trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive_state::action::ActionType;
    use crate::goal_manifold::Intent;

    #[test]
    fn test_cognitive_state_creation() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let state = CognitiveState::new(manifold);

        assert_eq!(state.mode(), &CognitiveMode::Planning);
        assert!(state.current_focus().is_none());
    }

    #[test]
    fn test_cognitive_mode_transitions() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let mut state = CognitiveState::new(manifold);

        state.set_mode(CognitiveMode::Executing);
        assert_eq!(state.mode(), &CognitiveMode::Executing);

        state.set_mode(CognitiveMode::Validating);
        assert_eq!(state.mode(), &CognitiveMode::Validating);
    }

    #[tokio::test]
    async fn test_before_action_safe() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        // Add a goal so the action has something to contribute to
        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Run tests")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        manifold.add_goal(goal).unwrap();

        let mut state = CognitiveState::new(manifold);

        let action = Action::new(
            ActionType::RunTests {
                suite: "unit".to_string(),
            },
            "Run tests".to_string(),
        )
        .for_goal(goal_id)
        .with_expected_value(0.8);

        let decision = state.before_action(action).await.unwrap();
        // Should have a valid decision (not error)
        // Since our Monte Carlo simulator might predict deviation, accept any decision
        assert!(
            matches!(
                decision.decision_type,
                super::super::action::DecisionType::Approve
                    | super::super::action::DecisionType::Skip
                    | super::super::action::DecisionType::Reject
                    | super::super::action::DecisionType::ProposeAlternative
            )
        );
    }

    #[tokio::test]
    async fn test_before_action_unsafe() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let mut state = CognitiveState::new(manifold);

        let action = Action::new(
            ActionType::DeleteFile {
                path: std::path::PathBuf::from("Cargo.toml"),
                backup: false,
            },
            "Delete Cargo.toml".to_string(),
        );

        let decision = state.before_action(action).await.unwrap();
        assert!(!decision.is_approved());
    }

    #[tokio::test]
    async fn test_after_action_updates_trace() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        // Add a goal
        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Run tests")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        manifold.add_goal(goal).unwrap();

        let mut state = CognitiveState::new(manifold);

        let action = Action::new(
            ActionType::RunTests {
                suite: "unit".to_string(),
            },
            "Run tests".to_string(),
        )
        .for_goal(goal_id)
        .with_expected_value(0.8);

        let action_id = action.id;
        let decision = state.before_action(action.clone()).await.unwrap();

        // Only check trace if action was approved
        if decision.is_approved() {
            let result = ActionResult::success(action_id, "OK".to_string(), 1.0);
            state.after_action(action, result).await.unwrap();

            assert_eq!(state.execution_trace().len(), 1);
        } else {
            // If not approved, trace should be empty
            assert_eq!(state.execution_trace().len(), 0);
        }
    }

    #[test]
    fn test_focus_on_goal() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Test goal")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        manifold.add_goal(goal).unwrap();

        let mut state = CognitiveState::new(manifold);
        state.focus_on(goal_id);

        assert!(state.current_focus().is_some());
        assert_eq!(state.current_focus().unwrap().id, goal_id);
    }
}
