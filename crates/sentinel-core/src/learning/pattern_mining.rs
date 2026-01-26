//! Pattern Mining System - Extract patterns from completed projects
//!
//! This module implements frequent pattern mining to discover
//! success and deviation patterns from project history.

use super::types::*;
use crate::error::Result;
use uuid::Uuid;
use std::collections::{HashMap, HashSet};

/// Configuration for pattern mining
#[derive(Clone, Debug)]
pub struct MiningConfig {
    /// Minimum support for a pattern (frequency)
    pub min_support: usize,

    /// Minimum confidence for association rules
    pub min_confidence: f64,

    /// Maximum pattern length (sequence depth)
    pub max_length: usize,

    /// Minimum occurrences for deviation pattern
    pub min_deviation_occurrences: usize,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            min_support: 3,
            min_confidence: 0.7,
            max_length: 5,
            min_deviation_occurrences: 2,
        }
    }
}

/// Pattern Mining System
///
/// Discovers patterns from completed projects using frequent pattern mining.
#[derive(Clone, Debug)]
pub struct PatternMiningEngine {
    /// Mining configuration
    pub config: MiningConfig,

    /// Cached patterns
    success_patterns: HashMap<Uuid, SuccessPattern>,
    deviation_patterns: HashMap<Uuid, DeviationPattern>,
}

impl PatternMiningEngine {
    /// Create a new pattern miner
    pub fn new() -> Self {
        Self::with_config(MiningConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: MiningConfig) -> Self {
        Self {
            config,
            success_patterns: HashMap::new(),
            deviation_patterns: HashMap::new(),
        }
    }

    /// Extract success patterns from high-alignment actions
    pub fn extract_success_patterns(&mut self, project: &CompletedProject) -> Vec<SuccessPattern> {
        let high_alignment_actions: Vec<&RecordedAction> = project
            .actions
            .iter()
            .filter(|a| a.alignment_score > 80.0)
            .collect();

        if high_alignment_actions.len() < self.config.min_support {
            return Vec::new();
        }

        // Extract action sequences
        let sequences = self.extract_sequences(&high_alignment_actions);

        // Mine frequent patterns
        let frequent_patterns = self.mine_frequent_patterns(&sequences);

        // Convert to SuccessPattern objects
        let mut patterns = Vec::new();
        for (sequence, support) in frequent_patterns {
            let pattern =
                self.create_success_pattern(&sequence, support, project);

            patterns.push(pattern);
        }

        // Cache patterns
        for pattern in &patterns {
            self.success_patterns.insert(pattern.id, pattern.clone());
        }

        patterns
    }

    /// Extract deviation patterns from project deviations
    pub fn extract_deviation_patterns(
        &mut self,
        project: &CompletedProject,
    ) -> Vec<DeviationPattern> {
        let mut patterns = Vec::new();

        for deviation in &project.deviations {
            // Find action that triggered deviation
            let triggering_action = project
                .actions
                .iter()
                .find(|a| a.action.id == deviation.triggering_action);

            if let Some(recorded_action) = triggering_action {
                let pattern = DeviationPattern {
                    id: Uuid::new_v4(),
                    name: format!("Deviation for {:?}", ActionType::from(recorded_action.action.action_type.clone())),
                    description: deviation.root_cause.clone().unwrap_or_default(),
                    trigger_action_types: vec![ActionType::from(recorded_action.action.action_type.clone())],
                    context_signatures: self.extract_context_signatures(deviation),
                    symptom_patterns: deviation.symptoms.clone(),
                    frequency: 1.0,
                    severity: deviation.severity,
                };

                patterns.push(pattern);
            }
        }

        // Cache patterns
        for pattern in &patterns {
            self.deviation_patterns.insert(pattern.id, pattern.clone());
        }

        patterns
    }

    /// Extract sequences of actions
    fn extract_sequences(&self, actions: &[&RecordedAction]) -> Vec<Vec<ActionType>> {
        let mut sequences = Vec::new();

        // Group by goal ID
        let mut by_goal: HashMap<Uuid, Vec<&RecordedAction>> = HashMap::new();
        for action in actions {
            by_goal
                .entry(action.goal_id)
                .or_insert_with(Vec::new)
                .push(action);
        }

        // Create sequences for each goal
        for (_, mut goal_actions) in by_goal {
            // Sort by timestamp
            goal_actions.sort_by_key(|a| a.timestamp);

            // Extract action types
            let sequence: Vec<ActionType> =
                goal_actions.iter().map(|a| ActionType::from(a.action.action_type.clone())).collect();

            sequences.push(sequence);
        }

        sequences
    }

    /// Mine frequent patterns using FP-Growth-like approach (simplified)
    fn mine_frequent_patterns(&self, sequences: &[Vec<ActionType>]) -> Vec<(Vec<ActionType>, usize)> {
        let mut pattern_counts: HashMap<Vec<ActionType>, usize> = HashMap::new();

        // Count single items
        for sequence in sequences {
            for action_type in sequence {
                let key = vec![action_type.clone()];
                *pattern_counts.entry(key).or_insert(0) += 1;
            }
        }

        // Filter by minimum support
        let frequent_singles: HashSet<ActionType> = pattern_counts
            .iter()
            .filter(|(_, count)| **count >= self.config.min_support)
            .map(|(k, _)| k[0].clone())
            .collect();

        // Generate larger patterns (up to max_length)
        for length in 2..=self.config.max_length {
            for sequence in sequences {
                // Generate all subsequences of this length
                for i in 0..sequence.len().saturating_sub(length - 1) {
                    let subsequence: Vec<ActionType> = sequence[i..i + length].to_vec();

                    // Only if all items are frequent
                    if subsequence
                        .iter()
                        .all(|item| frequent_singles.contains(item))
                    {
                        *pattern_counts.entry(subsequence).or_insert(0) += 1;
                    }
                }
            }
        }

        // Filter and return
        let result: Vec<_> = pattern_counts
            .into_iter()
            .filter(|(_, count)| *count >= self.config.min_support)
            .collect();
        
        result
    }

    /// Create a success pattern from a sequence
    fn create_success_pattern(
        &self,
        sequence: &[ActionType],
        support: usize,
        project: &CompletedProject,
    ) -> SuccessPattern {
        let name = self.generate_pattern_name(sequence);
        let description = format!(
            "Pattern '{}' observed in {} successful actions",
            name, support
        );
        let goal_types = self.classify_goal_types(&project.root_goal);

        let mut success_rate = 0.0;
        let success_count = project
            .actions
            .iter()
            .filter(|a| a.alignment_score > 80.0)
            .count();

        if !project.actions.is_empty() {
            success_rate = success_count as f64 / project.actions.len() as f64;
        }

        SuccessPattern {
            id: Uuid::new_v4(),
            name,
            description,
            action_sequence: sequence.to_vec(),
            applicable_to_goal_types: goal_types,
            success_rate,
            support,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.8,
            learned_at: crate::types::now(),
        }
    }

    /// Extract context signatures from deviation
    fn extract_context_signatures(&self, _deviation: &DeviationEvent) -> Vec<String> {
        // Implementation stub
        Vec::new()
    }

    /// Generate a human-readable pattern name
    fn generate_pattern_name(&self, sequence: &[ActionType]) -> String {
        if sequence.is_empty() {
            return "Empty Pattern".to_string();
        }

        if sequence.len() == 1 {
            return format!("{:?}-First", sequence[0]);
        }

        if sequence.len() == 2 {
            return format!("{:?} then {:?}", sequence[0], sequence[1]);
        }

        format!("{:?}...{:?}", sequence[0], sequence.last().unwrap())
    }

    /// Classify goal types from root goal
    fn classify_goal_types(&self, root_goal: &crate::goal_manifold::goal::Goal) -> Vec<GoalType> {
        let description = root_goal.description.to_lowercase();
        let mut types = Vec::new();

        if description.contains("api") {
            types.push(GoalType::Api);
        }
        if description.contains("auth") {
            types.push(GoalType::Authentication);
        }
        if description.contains("test") {
            types.push(GoalType::Testing);
        }
        if description.contains("ui") {
            types.push(GoalType::Ui);
        }
        if description.contains("db") || description.contains("database") {
            types.push(GoalType::Database);
        }

        if types.is_empty() {
            types.push(GoalType::FeatureImplementation);
        }

        types
    }

    /// Update existing patterns with new data
    pub fn update_patterns(&mut self, project: &CompletedProject) {
        // Extract new patterns
        let new_success = self.extract_success_patterns(project);
        let new_deviation = self.extract_deviation_patterns(project);

        // Merge with existing
        for new_pattern in new_success {
            // Simplified merge logic: just overwrite if ID matches, but IDs are new
            // Real implementation would need a way to identify same patterns
            self.success_patterns.insert(new_pattern.id, new_pattern);
        }

        for new_pattern in new_deviation {
            self.deviation_patterns.insert(new_pattern.id, new_pattern);
        }
    }

    /// Get all success patterns
    pub fn get_success_patterns(&self) -> Vec<SuccessPattern> {
        self.success_patterns.values().cloned().collect()
    }

    /// Get all deviation patterns
    pub fn get_deviation_patterns(&self) -> Vec<DeviationPattern> {
        self.deviation_patterns.values().cloned().collect()
    }
}


impl Default for PatternMiningEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::now;
    use crate::goal_manifold::goal::Goal;
    use std::time::Duration;

    fn create_test_project() -> CompletedProject {
        let goal = Goal::builder()
            .description("Build REST API")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .value_to_root(1.0)
            .build()
            .unwrap();

        CompletedProject {
            id: Uuid::new_v4(),
            root_goal: goal,
            actions: vec![
                RecordedAction {
                    action: crate::cognitive_state::action::Action::new(
                        crate::cognitive_state::action::ActionType::RunTests { suite: "unit".to_string() },
                        "Run tests".to_string(),
                    ),
                    timestamp: now(),
                    alignment_score: 90.0,
                    goal_id: Uuid::new_v4(),
                    outcome: ActionResult::Success { alignment_improved: true },
                    context: ActionContext {
                        current_goal: Uuid::new_v4(),
                        state_snapshot: ProjectSnapshot {
                            files: vec![],
                            test_results: TestResults { passed: 0, failed: 0, skipped: 0, coverage: 0.0 },
                            goal_status: vec![],
                            timestamp: now(),
                        },
                        recent_actions: vec![],
                        alignment_trend: AlignmentTrend::Stable,
                    },
                },
                RecordedAction {
                    action: crate::cognitive_state::action::Action::new(
                        crate::cognitive_state::action::ActionType::RunTests { suite: "unit".to_string() },
                        "Run tests 2".to_string(),
                    ),
                    timestamp: now(),
                    alignment_score: 85.0,
                    goal_id: Uuid::new_v4(),
                    outcome: ActionResult::Success { alignment_improved: true },
                    context: ActionContext {
                        current_goal: Uuid::new_v4(),
                        state_snapshot: ProjectSnapshot {
                            files: vec![],
                            test_results: TestResults { passed: 0, failed: 0, skipped: 0, coverage: 0.0 },
                            goal_status: vec![],
                            timestamp: now(),
                        },
                        recent_actions: vec![],
                        alignment_trend: AlignmentTrend::Stable,
                    },
                },
                RecordedAction {
                    action: crate::cognitive_state::action::Action::new(
                        crate::cognitive_state::action::ActionType::RunTests { suite: "unit".to_string() },
                        "Run tests 3".to_string(),
                    ),
                    timestamp: now(),
                    alignment_score: 88.0,
                    goal_id: Uuid::new_v4(),
                    outcome: ActionResult::Success { alignment_improved: true },
                    context: ActionContext {
                        current_goal: Uuid::new_v4(),
                        state_snapshot: ProjectSnapshot {
                            files: vec![],
                            test_results: TestResults { passed: 0, failed: 0, skipped: 0, coverage: 0.0 },
                            goal_status: vec![],
                            timestamp: now(),
                        },
                        recent_actions: vec![],
                        alignment_trend: AlignmentTrend::Stable,
                    },
                },
            ],
            deviations: vec![],
            corrections: vec![],
            completion_time: Duration::from_secs(3600),
            final_alignment_score: 88.4,
            metadata: ProjectMetadata {
                language: "rust".to_string(),
                framework: None,
                total_lines_of_code: 100,
                total_actions: 3,
                total_deviations: 0,
                started_at: now(),
                completed_at: now(),
            },
        }
    }

    #[test]
    fn test_pattern_miner_creation() {
        let miner = PatternMiningEngine::new();
        assert_eq!(miner.get_success_patterns().len(), 0);
    }

    #[test]
    fn test_extract_success_patterns() {
        let mut miner = PatternMiningEngine::new();
        let project = create_test_project();

        let patterns = miner.extract_success_patterns(&project);

        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_mine_frequent_patterns() {
        let miner = PatternMiningEngine::new();
        let sequences = vec![
            vec![ActionType::RunTests, ActionType::CreateFile { language: "rust".to_string() }],
            vec![ActionType::RunTests, ActionType::CreateFile { language: "rust".to_string() }],
            vec![ActionType::RunTests],
        ];

        let patterns = miner.mine_frequent_patterns(&sequences);

        assert!(patterns.iter().any(|(seq, count)| seq.len() == 1 && seq[0] == ActionType::RunTests && *count == 3));
    }
}

