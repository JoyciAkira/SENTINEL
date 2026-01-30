//! Core types for meta-learning

use crate::cognitive_state::action::Action;
use crate::goal_manifold::goal::Goal;
use crate::types::Timestamp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// A successfully completed project for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedProject {
    pub id: Uuid,
    pub root_goal: Goal,
    pub actions: Vec<RecordedAction>,
    pub deviations: Vec<DeviationEvent>,
    pub corrections: Vec<Correction>,
    pub completion_time: Duration,
    pub final_alignment_score: f64,
    pub metadata: ProjectMetadata,
}

/// An action with its context and outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedAction {
    pub action: Action,
    pub timestamp: Timestamp,
    pub alignment_score: f64, // 0.0-100.0
    pub goal_id: Uuid,
    pub outcome: ActionResult,
    pub context: ActionContext,
}

/// Context in which an action was taken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContext {
    pub current_goal: Uuid,
    pub state_snapshot: ProjectSnapshot,
    pub recent_actions: Vec<Uuid>, // IDs of recent actions
    pub alignment_trend: AlignmentTrend,
}

/// A deviation from optimal path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationEvent {
    pub id: Uuid,
    pub timestamp: Timestamp,
    pub triggering_action: Uuid,
    pub severity: DeviationSeverity,
    pub context: DeviationContext,
    pub symptoms: Vec<String>,
    pub root_cause: Option<String>,
    pub correction_applied: Option<Uuid>,
}

/// Correction applied to a deviation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub id: Uuid,
    pub deviation_id: Uuid,
    pub correction_actions: Vec<Action>,
    pub was_successful: bool,
    pub alignment_before: f64,
    pub alignment_after: f64,
    pub time_to_recover: Duration,
}

/// A successful pattern discovered from projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub action_sequence: Vec<ActionType>,
    pub applicable_to_goal_types: Vec<GoalType>,
    pub success_rate: f64, // 0.0-1.0
    pub support: usize,    // Number of projects
    pub preconditions: Vec<String>,
    pub expected_outcomes: Vec<String>,
    pub confidence: f64, // Confidence in this pattern
    pub learned_at: Timestamp,
}

/// A deviation pattern (anti-pattern)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationPattern {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub trigger_action_types: Vec<ActionType>,
    pub context_signatures: Vec<String>,
    pub symptom_patterns: Vec<String>,
    pub frequency: f64, // How often it occurs
    pub severity: DeviationSeverity,
}

/// Strategy synthesized from patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub recommended_approaches: Vec<SuccessPattern>,
    pub pitfalls_to_avoid: Vec<DeviationPattern>,
    pub estimated_completion_time: Duration,
    pub confidence: f64,   // 0.0-1.0
    pub rationale: String, // LLM-generated explanation
    pub generated_at: Timestamp,
}

/// Deviation risk prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationRisk {
    pub probability: f64, // 0.0-1.0
    pub similar_past_cases: Vec<DeviationCase>,
    pub risk_factors: Vec<RiskFactor>,
    pub recommended_precautions: Vec<Action>,
    pub confidence: f64,
}

/// A similar past deviation case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationCase {
    pub project_id: Uuid,
    pub action_id: Uuid,
    pub deviation: DeviationEvent,
    pub similarity_score: f64, // 0.0-1.0
}

/// A risk factor for deviation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor: String,
    pub impact: f64, // -1.0 to 1.0
    pub explanation: String,
}

/// Snapshot of project state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub files: Vec<FileSnapshot>,
    pub test_results: TestResults,
    pub goal_status: Vec<GoalStatusSnapshot>,
    pub timestamp: Timestamp,
}

/// Snapshot of a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub size: u64,
    pub language: String,
    pub last_modified: Timestamp,
}

/// Test results snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub coverage: f64,
}

/// Goal status snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalStatusSnapshot {
    pub goal_id: Uuid,
    pub status: String,
    pub completion_percentage: f64,
}

/// Action outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Success { alignment_improved: bool },
    Failed { error: String },
    Deviated { detected: bool, corrected: bool },
}

/// Action context types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentTrend {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

/// Deviation severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Action type for pattern mining
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    CreateFile { language: String },
    EditFile { language: String },
    DeleteFile,
    RunTests,
    RunCommand,
    CreateDirectory,
    Dependency,
    Custom { type_name: String },
    UpdateGoal,
    Query,
    ApplyPattern,
}

impl From<crate::cognitive_state::action::ActionType> for ActionType {
    fn from(other: crate::cognitive_state::action::ActionType) -> Self {
        match other {
            crate::cognitive_state::action::ActionType::CreateFile { path, .. } => {
                let language = path
                    .extension()
                    .map(|e| e.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".to_string());
                ActionType::CreateFile { language }
            }
            crate::cognitive_state::action::ActionType::EditFile { path, .. } => {
                let language = path
                    .extension()
                    .map(|e| e.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unknown".to_string());
                ActionType::EditFile { language }
            }
            crate::cognitive_state::action::ActionType::DeleteFile { .. } => ActionType::DeleteFile,
            crate::cognitive_state::action::ActionType::RunTests { .. } => ActionType::RunTests,
            crate::cognitive_state::action::ActionType::RunCommand { .. } => ActionType::RunCommand,
            crate::cognitive_state::action::ActionType::UpdateGoal { .. } => ActionType::UpdateGoal,
            crate::cognitive_state::action::ActionType::Custom { name, .. } => {
                ActionType::Custom { type_name: name }
            }
            crate::cognitive_state::action::ActionType::Query { .. } => ActionType::Query,
            crate::cognitive_state::action::ActionType::ApplyPattern { .. } => ActionType::ApplyPattern,
        }
    }
}


/// Goal type for pattern classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalType {
    FeatureImplementation,
    BugFix,
    Refactoring,
    Testing,
    Documentation,
    PerformanceOptimization,
    Security,
    Infrastructure,
    Database,
    Authentication,
    Payment,
    Api,
    Ui,
}

/// Deviation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationContext {
    pub state_snapshot: ProjectSnapshot,
    pub recent_alignment_scores: Vec<f64>,
    pub alignment_trend: AlignmentTrend,
    pub active_goals: Vec<Uuid>,
    pub resource_usage: ResourceUsage,
}

/// Resource usage snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub language: String,
    pub framework: Option<String>,
    pub total_lines_of_code: usize,
    pub total_actions: usize,
    pub total_deviations: usize,
    pub started_at: Timestamp,
    pub completed_at: Timestamp,
}

/// Learning report from meta-learning engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningReport {
    pub project_id: Uuid,
    pub timestamp: Timestamp,
    pub success_patterns_extracted: usize,
    pub deviation_patterns_extracted: usize,
    pub knowledge_base_size: usize,
    pub training_examples_added: usize,
    pub cross_patterns_discovered: usize,
    pub confidence_improvement: f64,
}
