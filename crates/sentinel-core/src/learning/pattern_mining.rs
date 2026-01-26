//! Pattern Mining Engine
//!
//! Extracts frequent action patterns from completed projects
//! using FP-Growth algorithm.

use crate::cognitive_state::Action;
use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use crate::learning::types::*;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

/// Pattern Mining Engine
///
/// Extracts frequent sequential patterns from completed projects.
/// Uses FP-Growth algorithm for efficient frequent pattern mining.
pub struct PatternMiningEngine {
    min_support: f64,
    min_confidence: f64,
}

impl PatternMiningEngine {
    /// Create a new pattern mining engine
    pub fn new(min_support: f64, min_confidence: f64) -> Self {
        Self {
            min_support,
            min_confidence,
        }
    }

    /// Extract success patterns from a completed project
    pub async fn extract_patterns(
        &self,
        project: &CompletedProject,
    ) -> Result<Vec<SuccessPattern>> {
        // Filter high-alignment actions
        let high_alignment_actions = self.filter_high_alignment_actions(project);

        // Extract action sequences
        let action_sequences = self.extract_action_sequences(&high_alignment_actions);

        // Mine frequent patterns using FP-Growth
        let frequent_patterns =
            self.frequent_pattern_mining(&action_sequences, self.min_support)?;

        // Classify patterns by goal type
        let classified_patterns = self.classify_patterns(&frequent_patterns, project)?;

        Ok(classified_patterns)
    }

    /// Extract deviation patterns from a project
    pub fn extract_deviation_patterns(&self, project: &CompletedProject) -> Vec<DeviationPattern> {
        project
            .deviations
            .iter()
            .map(|deviation| {
                let trigger_action = project
                    .actions
                    .iter()
                    .find(|a| a.action.id() == deviation.triggering_action);

                DeviationPattern {
                    id: Uuid::new_v4(),
                    name: format!(
                        "Deviation: {}",
                        deviation.root_cause.as_deref().unwrap_or("Unknown cause")
                    ),
                    description: deviation.symptoms.join(", "),
                    trigger_action_types: trigger_action
                        .map(|a| vec![ActionType::from(&a.action)])
                        .unwrap_or_default(),
                    context_signatures: self.extract_context_signatures(deviation),
                    symptom_patterns: deviation.symptoms.clone(),
                    frequency: 1.0, // Will be aggregated across projects
                    severity: deviation.severity,
                }
            })
            .collect()
    }

    /// Filter actions with high alignment scores
    fn filter_high_alignment_actions<'a>(
        &self,
        project: &'a CompletedProject,
    ) -> Vec<&'a RecordedAction> {
        project
            .actions
            .iter()
            .filter(|a| a.alignment_score >= 80.0)
            .collect()
    }

    /// Extract action sequences grouped by goal
    fn extract_action_sequences(
        &self,
        actions: &[&RecordedAction],
    ) -> Vec<(Uuid, Vec<ActionType>)> {
        let mut sequences: HashMap<Uuid, Vec<ActionType>> = HashMap::new();

        for action in actions {
            sequences
                .entry(action.goal_id)
                .or_insert_with(Vec::new)
                .push(ActionType::from(&action.action));
        }

        sequences.into_iter().collect()
    }

    /// FP-Growth algorithm for frequent pattern mining
    fn frequent_pattern_mining(
        &self,
        sequences: &[(Uuid, Vec<ActionType>)],
        min_support: f64,
    ) -> Result<Vec<(Vec<ActionType>, usize)>> {
        let total_sequences = sequences.len();
        if total_sequences == 0 {
            return Ok(Vec::new());
        }

        // Count support for each action type
        let mut support_counts: HashMap<ActionType, usize> = HashMap::new();

        for (_, sequence) in sequences {
            for action_type in sequence {
                *support_counts.entry(action_type.clone()).or_insert(0) += 1;
            }
        }

        // Filter by minimum support
        let min_count = (total_sequences as f64 * min_support) as usize;
        let frequent_items: Vec<ActionType> = support_counts
            .into_iter()
            .filter(|(_, count)| *count >= min_count)
            .map(|(action, _)| action)
            .collect();

        if frequent_items.is_empty() {
            return Ok(Vec::new());
        }

        // Mine frequent patterns
        let mut patterns = Vec::new();

        // Single-item patterns
        for action in &frequent_items {
            let support = sequences
                .iter()
                .filter(|(_, seq)| seq.contains(action))
                .count();
            if support >= min_count {
                patterns.push((vec![action.clone()], support));
            }
        }

        // Two-item patterns
        for i in 0..frequent_items.len() {
            for j in (i + 1)..frequent_items.len() {
                let pattern = vec![frequent_items[i].clone(), frequent_items[j].clone()];
                let support = sequences
                    .iter()
                    .filter(|(_, seq)| self.subsequence_in_sequence(&pattern, seq))
                    .count();

                if support >= min_count {
                    patterns.push((pattern, support));
                }
            }
        }

        // Three-item patterns (if enough sequences)
        if total_sequences >= 20 {
            for i in 0..frequent_items.len() {
                for j in (i + 1)..frequent_items.len() {
                    for k in (j + 1)..frequent_items.len() {
                        let pattern = vec![
                            frequent_items[i].clone(),
                            frequent_items[j].clone(),
                            frequent_items[k].clone(),
                        ];
                        let support = sequences
                            .iter()
                            .filter(|(_, seq)| self.subsequence_in_sequence(&pattern, seq))
                            .count();

                        if support >= min_count {
                            patterns.push((pattern, support));
                        }
                    }
                }
            }
        }

        Ok(patterns)
    }

    /// Check if pattern is a subsequence of sequence
    fn subsequence_in_sequence(&self, pattern: &[ActionType], sequence: &[ActionType]) -> bool {
        if pattern.is_empty() {
            return true;
        }

        let mut pattern_idx = 0;
        for action in sequence {
            if pattern_idx < pattern.len() && action == &pattern[pattern_idx] {
                pattern_idx += 1;
            }
        }

        pattern_idx == pattern.len()
    }

    /// Classify patterns by goal type
    fn classify_patterns(
        &self,
        patterns: &[(Vec<ActionType>, usize)],
        project: &CompletedProject,
    ) -> Result<Vec<SuccessPattern>> {
        let goal_type = self.classify_goal(&project.root_goal);

        patterns
            .iter()
            .map(|(action_sequence, support)| {
                Ok(SuccessPattern {
                    id: Uuid::new_v4(),
                    name: self.generate_pattern_name(action_sequence, &goal_type),
                    description: format!(
                        "Sequence of {} actions with {}% support",
                        action_sequence.len(),
                        (support * 100 / project.actions.len())
                    ),
                    action_sequence: action_sequence.clone(),
                    applicable_to_goal_types: vec![goal_type.clone()],
                    success_rate: 0.8 + (*support as f64 / project.actions.len() as f64) * 0.2,
                    support: *support,
                    preconditions: vec![],
                    expected_outcomes: vec![
                        "High alignment score".to_string(),
                        "Goal completion".to_string(),
                    ],
                    confidence: (*support as f64 / project.actions.len() as f64),
                    learned_at: crate::types::now(),
                })
            })
            .collect()
    }

    /// Classify goal by type
    fn classify_goal(&self, goal: &Goal) -> GoalType {
        let description = goal.description.to_lowercase();

        if description.contains("bug") || description.contains("fix") {
            GoalType::BugFix
        } else if description.contains("test") {
            GoalType::Testing
        } else if description.contains("refactor") {
            GoalType::Refactoring
        } else if description.contains("auth") {
            GoalType::Authentication
        } else if description.contains("api") {
            GoalType::Api
        } else if description.contains("database") || description.contains("db") {
            GoalType::Database
        } else if description.contains("performance") || description.contains("optimize") {
            GoalType::PerformanceOptimization
        } else if description.contains("security") {
            GoalType::Security
        } else if description.contains("payment") || description.contains("billing") {
            GoalType::Payment
        } else if description.contains("ui") || description.contains("interface") {
            GoalType::Ui
        } else if description.contains("infrastructure") || description.contains("deploy") {
            GoalType::Infrastructure
        } else if description.contains("document") {
            GoalType::Documentation
        } else {
            GoalType::FeatureImplementation
        }
    }

    /// Generate a human-readable pattern name
    fn generate_pattern_name(
        &self,
        action_sequence: &[ActionType],
        goal_type: &GoalType,
    ) -> String {
        if action_sequence.is_empty() {
            return format!("Empty Pattern ({:?})", goal_type);
        }

        let actions_str = action_sequence
            .iter()
            .take(3)
            .map(|a| match a {
                ActionType::CreateFile { .. } => "Create",
                ActionType::EditFile { .. } => "Edit",
                ActionType::RunTests => "Test",
                ActionType::RunCommand => "Command",
                _ => "Action",
            })
            .collect::<Vec<_>>()
            .join(" → ");

        format!(
            "{}: {}{}",
            self.goal_type_name(goal_type),
            actions_str,
            if action_sequence.len() > 3 {
                " → ..."
            } else {
                ""
            }
        )
    }

    /// Get human-readable goal type name
    fn goal_type_name(&self, goal_type: &GoalType) -> String {
        match goal_type {
            GoalType::FeatureImplementation => "Feature",
            GoalType::BugFix => "BugFix",
            GoalType::Refactoring => "Refactor",
            GoalType::Testing => "Test",
            GoalType::Documentation => "Doc",
            GoalType::PerformanceOptimization => "Perf",
            GoalType::Security => "Security",
            GoalType::Infrastructure => "Infra",
            GoalType::Database => "DB",
            GoalType::Authentication => "Auth",
            GoalType::Payment => "Payment",
            GoalType::Api => "API",
            GoalType::Ui => "UI",
        }
        .to_string()
    }

    /// Extract context signatures for deviation
    fn extract_context_signatures(&self, deviation: &DeviationEvent) -> Vec<String> {
        let mut signatures = Vec::new();

        // Extract alignment trend signature
        signatures.push(format!(
            "alignment_trend:{:?}",
            deviation.context.alignment_trend
        ));

        // Extract active goals count
        signatures.push(format!(
            "active_goals:{}",
            deviation.context.active_goals.len()
        ));

        // Extract recent alignment scores
        if !deviation.context.recent_alignment_scores.is_empty() {
            let avg: f64 = deviation
                .context
                .recent_alignment_scores
                .iter()
                .sum::<f64>()
                / deviation.context.recent_alignment_scores.len() as f64;
            signatures.push(format!("avg_alignment:{:.1}", avg));
        }

        signatures
    }
}

impl Default for PatternMiningEngine {
    fn default() -> Self {
        Self::new(0.3, 0.7)
    }
}

// Implement conversions
impl From<&Action> for ActionType {
    fn from(action: &Action) -> Self {
        match &action.action_type {
            crate::cognitive_state::ActionType::CreateFile { path, .. } => {
                let extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("unknown");
                ActionType::CreateFile {
                    language: extension.to_string(),
                }
            }
            crate::cognitive_state::ActionType::EditFile { path, .. } => {
                let extension = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("unknown");
                ActionType::EditFile {
                    language: extension.to_string(),
                }
            }
            crate::cognitive_state::ActionType::DeleteFile { .. } => ActionType::DeleteFile,
            crate::cognitive_state::ActionType::RunTests { .. } => ActionType::RunTests,
            crate::cognitive_state::ActionType::RunCommand { .. } => ActionType::RunCommand,
            _ => ActionType::Custom {
                type_name: format!("{:?}", action.action_type),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_mining_engine_creation() {
        let engine = PatternMiningEngine::new(0.3, 0.7);
        assert_eq!(engine.min_support, 0.3);
        assert_eq!(engine.min_confidence, 0.7);
    }

    #[test]
    fn test_default_engine() {
        let engine = PatternMiningEngine::default();
        assert_eq!(engine.min_support, 0.3);
        assert_eq!(engine.min_confidence, 0.7);
    }

    #[test]
    fn test_subsequence_in_sequence() {
        let engine = PatternMiningEngine::default();

        let pattern = vec![ActionType::RunTests, ActionType::RunCommand];
        let sequence = vec![
            ActionType::CreateFile {
                language: "rust".to_string(),
            },
            ActionType::RunTests,
            ActionType::RunCommand,
        ];

        assert!(engine.subsequence_in_sequence(&pattern, &sequence));
    }

    #[test]
    fn test_goal_classification() {
        let engine = PatternMiningEngine::default();

        let mut goal = crate::goal_manifold::goal::Goal::builder()
            .description("Fix authentication bug")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .value_to_root(1.0)
            .build()
            .unwrap();

        let goal_type = engine.classify_goal(&goal);
        assert_eq!(goal_type, GoalType::BugFix);
    }

    #[tokio::test]
    async fn test_extract_patterns_empty_project() {
        let engine = PatternMiningEngine::default();
        let project = CompletedProject {
            id: Uuid::new_v4(),
            root_goal: crate::goal_manifold::goal::Goal::builder()
                .description("Test")
                .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
                .value_to_root(1.0)
                .build()
                .unwrap(),
            actions: Vec::new(),
            deviations: Vec::new(),
            corrections: Vec::new(),
            completion_time: Duration::from_secs(0),
            final_alignment_score: 100.0,
            metadata: ProjectMetadata {
                language: "rust".to_string(),
                framework: None,
                total_lines_of_code: 0,
                total_actions: 0,
                total_deviations: 0,
                started_at: crate::types::now(),
                completed_at: crate::types::now(),
            },
        };

        let patterns = engine.extract_patterns(&project).await.unwrap();
        assert!(patterns.is_empty());
    }
}
