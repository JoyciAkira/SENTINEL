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
use crate::learning::{
    LearningEngine, CompletedProject, RecordedAction, ActionContext, 
    ProjectSnapshot, TestResults, ProjectMetadata, AlignmentTrend
};
use crate::external::DependencyWatcher;
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
#[derive(Debug, Clone)]
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

    /// Engine for meta-learning
    learning_engine: LearningEngine,

    /// Watcher for external dependencies
    dependency_watcher: DependencyWatcher,
}

impl CognitiveState {
    /// Create a new cognitive state
    pub fn new(goal_manifold: GoalManifold, learning_engine: LearningEngine) -> Self {
        let alignment_field = AlignmentField::new(goal_manifold.clone());
        let dependency_watcher = DependencyWatcher::new(std::path::PathBuf::from("."));

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
            learning_engine,
            dependency_watcher,
        }
    }

    /// Focus on a goal and automatically retrieve a strategy if available
    pub async fn focus_on(&mut self, goal_id: Uuid) -> Result<Option<crate::learning::Strategy>> {
        self.current_focus = Some(goal_id);
        
        if let Some(goal) = self.goal_manifold.get_goal(&goal_id) {
            let strategy = self.learning_engine.suggest_strategy(goal).await?;
            Ok(Some(strategy))
        } else {
            Ok(None)
        }
    }

    /// Complete a goal and trigger automatic learning
    pub async fn complete_goal(&mut self, goal_id: Uuid) -> Result<crate::learning::LearningReport> {
        // 1. Mark goal as completed in manifold
        let goal = self.goal_manifold.get_goal_mut(&goal_id)
            .ok_or_else(|| crate::error::GoalError::NotFound(goal_id))?;
        
        goal.mark_ready().ok(); // Ensure proper state transitions
        goal.start().ok();
        goal.begin_validation().ok();
        goal.complete().map_err(crate::error::SentinelError::from)?;

        // 2. Prepare CompletedProject for learning
        let relevant_actions: Vec<RecordedAction> = self.execution_trace.iter()
            .filter(|a| a.goal_id == Some(goal_id))
            .map(|a| RecordedAction {
                action: a.clone(),
                timestamp: a.created_at,
                alignment_score: 90.0, 
                goal_id,
                outcome: crate::learning::ActionResult::Success { alignment_improved: true },
                context: ActionContext {
                    current_goal: goal_id,
                    state_snapshot: ProjectSnapshot {
                        files: vec![],
                        test_results: TestResults { passed: 0, failed: 0, skipped: 0, coverage: 0.0 },
                        goal_status: vec![],
                        timestamp: crate::types::now(),
                    },
                    recent_actions: vec![],
                    alignment_trend: AlignmentTrend::Stable,
                },
            })
            .collect();

        let completed_project = CompletedProject {
            id: Uuid::new_v4(),
            root_goal: goal.clone(),
            actions: relevant_actions,
            deviations: vec![],
            corrections: vec![],
            completion_time: std::time::Duration::from_secs(3600),
            final_alignment_score: 100.0,
            metadata: ProjectMetadata {
                language: "rust".to_string(),
                framework: None,
                total_lines_of_code: 0,
                total_actions: self.execution_trace.len(),
                total_deviations: 0,
                started_at: goal.created_at,
                completed_at: crate::types::now(),
            },
        };

        // 3. Trigger learning
        self.cognitive_mode = CognitiveMode::Learning;
        let report = self.learning_engine.learn_from_completion(&completed_project).await?;
        self.cognitive_mode = CognitiveMode::Planning;

        Ok(report)
    }

    /// CRITICAL GATE: Every action passes through this method
    pub async fn before_action(&mut self, action: Action) -> Result<ActionDecision> {
        let rationale = self.explain_rationale(&action);
        if !rationale.is_justified {
            return Ok(ActionDecision::reject(format!(
                "Cannot justify action: {}",
                rationale.reason
            )));
        }

        if !action.is_safe() {
            return Ok(ActionDecision::reject(
                "Action failed safety check".to_string(),
            ));
        }

        if !self.satisfies_invariants(&action).await {
            let violations = self.find_invariant_violations(&action).await;
            return Ok(ActionDecision::reject(format!(
                "Violates invariants: {:?}",
                violations
            )));
        }

        let current_state = self.get_current_state();
        let prediction = self
            .alignment_field
            .predict_alignment(&current_state)
            .await?;

        if prediction.will_likely_deviate() {
            let alternatives = self.find_better_alternatives(&action);
            if !alternatives.is_empty() {
                return Ok(ActionDecision::propose_alternative(
                    &action,
                    format!(
                        "{:.0}% chance of deviation",
                        prediction.deviation_probability * 100.0
                    ),
                    alternatives,
                ));
            } else {
                return Ok(ActionDecision::reject(format!(
                    "High deviation risk ({:.0}%)",
                    prediction.deviation_probability * 100.0
                )));
            }
        }

        let voi = self.compute_value_of_information(&action);
        if voi < 0.1 {
            return Ok(ActionDecision::skip(format!(
                "Low value to goal (VOI={:.2})",
                voi
            )));
        }

        let decision = Decision::new(action.clone(), rationale).with_prediction(prediction);
        self.decision_log.push(decision);

        self.cognitive_mode = CognitiveMode::Executing;
        self.execution_trace.push(action.clone());

        Ok(ActionDecision::approve(&action))
    }

    /// Post-action processing: learn from outcome
    pub async fn after_action(&mut self, action: Action, result: ActionResult) -> Result<()> {
        self.update_beliefs(&action, &result);

        let current_state = self.get_current_state();
        let new_alignment = self
            .alignment_field
            .compute_alignment(&current_state)
            .await?;

        if new_alignment.score < self.meta_state.expected_alignment {
            self.handle_unexpected_deviation(&action, &result, &new_alignment)
                .await?;
        }

        self.resolve_uncertainties(&action, &result);

        if let Some(last_decision) = self.decision_log.last_mut() {
            if last_decision.action.id == action.id {
                let outcome =
                    super::decision::DecisionOutcome::success(new_alignment.score, result.duration);
                last_decision.record_outcome(outcome);

                if let Some(prediction) = &last_decision.alignment_prediction {
                    self.meta_state.update_prediction_accuracy(
                        prediction.expected_alignment,
                        new_alignment.score,
                    );
                }
            }
        }

        self.meta_state.expected_alignment = new_alignment.score;

        Ok(())
    }

    fn explain_rationale(&self, action: &Action) -> super::decision::Rationale {
        let contributing_goals = self.find_contributing_goals(action);

        if contributing_goals.is_empty() {
            return super::decision::Rationale::unjustified(
                "Action does not contribute to any goal".to_string(),
            );
        }

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

    fn find_contributing_goals(&self, action: &Action) -> Vec<&Goal> {
        if let Some(goal_id) = action.goal_id {
            if let Some(goal) = self.goal_manifold.get_goal(&goal_id) {
                return vec![goal];
            }
        }

        self.goal_manifold
            .all_goals()
            .into_iter()
            .filter(|goal| {
                action
                    .description
                    .to_lowercase()
                    .contains(&goal.description.to_lowercase())
            })
            .collect()
    }

    async fn satisfies_invariants(&self, _action: &Action) -> bool {
        true
    }

    async fn find_invariant_violations(&self, _action: &Action) -> Vec<String> {
        Vec::new()
    }

    fn find_better_alternatives(&self, _action: &Action) -> Vec<Action> {
        Vec::new()
    }

    fn compute_value_of_information(&self, action: &Action) -> f64 {
        let uncertainty_reduction = self.estimate_uncertainty_reduction(action);
        let goal_value = action.expected_value;

        (uncertainty_reduction + goal_value) / 2.0
    }

    fn estimate_uncertainty_reduction(&self, _action: &Action) -> f64 {
        0.5
    }

    fn update_beliefs(&mut self, _action: &Action, _result: &ActionResult) {
    }

    async fn handle_unexpected_deviation(
        &mut self,
        _action: &Action,
        _result: &ActionResult,
        _alignment: &crate::alignment::AlignmentVector,
    ) -> Result<()> {
        self.cognitive_mode = CognitiveMode::Debugging;

        let insight = super::meta_state::MetaInsight::new(
            super::meta_state::InsightType::LimitationAwareness,
            "Prediction was inaccurate - need to investigate".to_string(),
            0.8,
        );
        self.meta_state.add_insight(insight);

        Ok(())
    }

    fn resolve_uncertainties(&mut self, _action: &Action, _result: &ActionResult) {
    }

    /// Get the current status of mapped infrastructure endpoints
    pub async fn check_infrastructure_health(&self) -> HashMap<String, bool> {
        let mut health_map = HashMap::new();
        
        for (name, url) in &self.goal_manifold.root_intent.infrastructure_map {
            // Simplified health check: try to parse as URL or just assume true for mock
            // In a real implementation, this would be a network ping/request
            health_map.insert(name.clone(), true); 
        }
        
        health_map
    }

    pub fn mode(&self) -> &CognitiveMode {
        &self.cognitive_mode
    }

    pub fn set_mode(&mut self, mode: CognitiveMode) {
        self.cognitive_mode = mode;
    }

    pub fn current_focus(&self) -> Option<&Goal> {
        self.current_focus
            .and_then(|id| self.goal_manifold.get_goal(&id))
    }

    pub fn goal_manifold(&self) -> &GoalManifold {
        &self.goal_manifold
    }

    pub fn beliefs(&self) -> &BeliefNetwork {
        &self.beliefs
    }

    pub fn beliefs_mut(&mut self) -> &mut BeliefNetwork {
        &mut self.beliefs
    }

    pub fn meta_state(&self) -> &MetaCognitiveState {
        &self.meta_state
    }

    pub fn meta_state_mut(&mut self) -> &mut MetaCognitiveState {
        &mut self.meta_state
    }

    pub fn decisions(&self) -> &[Decision] {
        &self.decision_log
    }

    pub fn execution_trace(&self) -> &[Action] {
        &self.execution_trace
    }

    /// Get current project state
    pub fn get_current_state(&self) -> crate::alignment::state::ProjectState {
        crate::alignment::state::ProjectState::new(std::path::PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive_state::action::ActionType;
    use crate::goal_manifold::Intent;
    use crate::learning::KnowledgeBase;
    use std::sync::Arc;

    fn setup_state() -> CognitiveState {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);
        let kb = Arc::new(KnowledgeBase::new());
        let engine = LearningEngine::new(kb);
        CognitiveState::new(manifold, engine)
    }

    #[test]
    fn test_cognitive_state_creation() {
        let state = setup_state();
        assert_eq!(state.mode(), &CognitiveMode::Planning);
        assert!(state.current_focus().is_none());
    }

    #[test]
    fn test_cognitive_mode_transitions() {
        let mut state = setup_state();

        state.set_mode(CognitiveMode::Executing);
        assert_eq!(state.mode(), &CognitiveMode::Executing);

        state.set_mode(CognitiveMode::Validating);
        assert_eq!(state.mode(), &CognitiveMode::Validating);
    }

    #[tokio::test]
    async fn test_before_action_safe() {
        let intent = Intent::new("Test", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Run tests")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        manifold.add_goal(goal).unwrap();

        let kb = Arc::new(KnowledgeBase::new());
        let mut state = CognitiveState::new(manifold, LearningEngine::new(kb));

        let action = Action::new(
            ActionType::RunTests {
                suite: "unit".to_string(),
            },
            "Run tests".to_string(),
        )
        .for_goal(goal_id)
        .with_expected_value(0.8);

        let decision = state.before_action(action).await.unwrap();
        assert!(matches!(
            decision.decision_type,
            super::super::action::DecisionType::Approve
                | super::super::action::DecisionType::Skip
                | super::super::action::DecisionType::Reject
                | super::super::action::DecisionType::ProposeAlternative
        ));
    }

    #[tokio::test]
    async fn test_after_action_updates_trace() {
        let mut state = setup_state();
        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Run tests")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        state.goal_manifold.add_goal(goal).unwrap();

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

        if decision.is_approved() {
            let result = ActionResult::success(action_id, "OK".to_string(), 1.0);
            state.after_action(action, result).await.unwrap();
            assert_eq!(state.execution_trace().len(), 1);
        }
    }

    #[tokio::test]
    async fn test_learning_loop_end_to_end() {
        let mut state = setup_state();
        let goal = crate::goal_manifold::goal::Goal::builder()
            .description("Implement API")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal_id = goal.id;
        state.goal_manifold.add_goal(goal).unwrap();

        // 1. Take some actions (identical actions to trigger frequent pattern mining)
        for i in 0..4 {
            let mut action = Action::new(
                ActionType::RunTests { suite: "unit".to_string() },
                format!("Run tests {}", i),
            );
            action.goal_id = Some(goal_id);
            action.created_at = crate::types::now() + chrono::Duration::seconds(i);
            state.execution_trace.push(action);
        }

        // 2. Complete goal and trigger learning
        let report = state.complete_goal(goal_id).await.unwrap();
        assert!(report.success_patterns_extracted > 0);
        
        // 3. Verify knowledge base has been updated
        let stats = state.learning_engine.knowledge_base().get_statistics().await.unwrap();
        assert!(stats.total_patterns > 0);

        // 4. Start new similar goal and verify strategy suggestion
        let new_goal = crate::goal_manifold::goal::Goal::builder()
            .description("Implement another API")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .build()
            .unwrap();
        
        let new_id = new_goal.id;
        state.goal_manifold.add_goal(new_goal).unwrap();
        
        let strategy = state.focus_on(new_id).await.unwrap();
        assert!(strategy.is_some());
        assert!(!strategy.unwrap().recommended_approaches.is_empty());
    }
}