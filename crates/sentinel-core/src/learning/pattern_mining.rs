//! Pattern Mining Engine - Layer 5
//!
//! Estrattore di pattern di successo e deviazioni dai progetti completati.

use crate::learning::types::{
    ActionType, CompletedProject, DeviationEvent, DeviationPattern, GoalType, RecordedAction,
    SuccessPattern,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PatternMiningEngine {
    pub success_patterns: HashMap<Uuid, SuccessPattern>,
    pub deviation_patterns: HashMap<Uuid, DeviationPattern>,
}

impl PatternMiningEngine {
    pub fn new() -> Self {
        Self {
            success_patterns: HashMap::new(),
            deviation_patterns: HashMap::new(),
        }
    }

    pub fn extract_success_patterns(&mut self, project: &CompletedProject) -> Vec<SuccessPattern> {
        let mut patterns = Vec::new();
        let sequences = self.extract_sequences(&project.actions.iter().collect::<Vec<_>>());

        for sequence in sequences {
            let pattern = SuccessPattern {
                id: Uuid::new_v4(),
                name: self.generate_pattern_name(&sequence),
                description: format!("Frequent sequence detected in project {}", project.id),
                action_sequence: sequence,
                applicable_to_goal_types: self.classify_goal_types(&project.root_goal),
                success_rate: project.final_alignment_score,
                support: 1,
                preconditions: Vec::new(),
                expected_outcomes: Vec::new(),
                confidence: 0.8,
                learned_at: chrono::Utc::now(),
            };
            patterns.push(pattern);
        }
        patterns
    }

    pub fn extract_deviation_patterns(
        &mut self,
        project: &CompletedProject,
    ) -> Vec<DeviationPattern> {
        let mut patterns = Vec::new();
        for deviation in &project.deviations {
            if let Some(recorded_action) = project
                .actions
                .iter()
                .find(|a| a.action.id == deviation.triggering_action)
            {
                let pattern = DeviationPattern {
                    id: Uuid::new_v4(),
                    name: format!(
                        "Deviation for {:?}",
                        ActionType::from(recorded_action.action.action_type.clone())
                    ),
                    description: deviation.symptoms.join(", "),
                    trigger_action_types: vec![ActionType::from(
                        recorded_action.action.action_type.clone(),
                    )],
                    context_signatures: self.extract_context_signatures(deviation),
                    symptom_patterns: deviation.symptoms.clone(),
                    frequency: 1.0,
                    severity: deviation.severity,
                };
                patterns.push(pattern);
            }
        }
        patterns
    }

    fn extract_sequences(&self, actions: &[&RecordedAction]) -> Vec<Vec<ActionType>> {
        let mut sequences = Vec::new();
        let mut by_goal: HashMap<Uuid, Vec<&RecordedAction>> = HashMap::new();

        for action in actions {
            by_goal.entry(action.goal_id).or_default().push(action);
        }

        for mut goal_actions in by_goal.into_values() {
            goal_actions.sort_by_key(|a| a.timestamp);
            let sequence: Vec<ActionType> = goal_actions
                .iter()
                .map(|a| ActionType::from(a.action.action_type.clone()))
                .collect();
            if !sequence.is_empty() {
                sequences.push(sequence);
            }
        }
        sequences
    }

    fn generate_pattern_name(&self, sequence: &[ActionType]) -> String {
        format!(
            "Pattern-{:?}",
            sequence.first().unwrap_or(&ActionType::RunCommand)
        )
    }

    fn classify_goal_types(&self, root_goal: &crate::goal_manifold::goal::Goal) -> Vec<GoalType> {
        let mut types = Vec::new();
        let desc = root_goal.description.to_lowercase();
        if desc.contains("api") {
            types.push(GoalType::Api);
        }
        if types.is_empty() {
            types.push(GoalType::FeatureImplementation);
        }
        types
    }

    fn extract_context_signatures(&self, _deviation: &DeviationEvent) -> Vec<String> {
        Vec::new()
    }

    pub fn update_patterns(&mut self, project: &CompletedProject) {
        let success = self.extract_success_patterns(project);
        for p in success {
            self.success_patterns.insert(p.id, p);
        }
    }
}

impl Default for PatternMiningEngine {
    fn default() -> Self {
        Self::new()
    }
}
