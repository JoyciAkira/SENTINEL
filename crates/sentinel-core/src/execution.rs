//! Execution contract types shared across Sentinel runtime layers.

use crate::quality::{Artifact, QualityContext, QualityMaximizer, QualityReport};
use crate::error::{Result, SentinelError};

/// Explicit execution contract:
/// - where we are
/// - where we must go
/// - how we will get there
/// - why this execution is justified

/// Execution status (AC6 state machine transitions)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    #[serde(rename = "INTENT_CAPTURED")]
    IntentCaptured,
    #[serde(rename = "PLAN_COMPILED")]
    PlanCompiled,
    #[serde(rename = "READY")]
    Ready,
    #[serde(rename = "EXECUTING")]
    Executing,
    #[serde(rename = "QUALITY_EVALUATING")]
    QualityEvaluating,
    #[serde(rename = "REVISION_REQUIRED")]
    RevisionRequired,
    #[serde(rename = "VALIDATED")]
    Validated,
    #[serde(rename = "DELIVERED")]
    Delivered,
    #[serde(rename = "FAILED")]
    Failed,
}

/// Result of an execution with quality gate
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub quality_report: Option<QualityReport>,
    pub artifacts: Vec<Artifact>,
    pub error_message: Option<String>,
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Ready,
            quality_report: None,
            artifacts: Vec::new(),
            error_message: None,
        }
    }
}

/// Execution engine with quality gate integration (AC6 compliant)
pub struct ExecutionEngine {
    quality_maximizer: QualityMaximizer,
    quality_context: QualityContext,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(run_id: String) -> Self {
        let quality_context = QualityContext {
            run_id,
            quality_config: crate::quality::QualityConfig::default(),
            evaluation_suite: None,
        };

        Self {
            quality_maximizer: QualityMaximizer::new(),
            quality_context,
        }
    }

    /// Execute with quality gate (AC6: EXECUTING -> QUALITY_EVALUATING -> VALIDATED/REVISION_REQUIRED)
    pub async fn execute_with_quality_gate(
        &self,
        module_id: &str,
        artifact: &Artifact,
    ) -> std::result::Result<ExecutionResult, SentinelError> {
        // 1. Pre-execution quality check (QUALITY_EVALUATING state)
        let quality_report = self
            .quality_maximizer
            .maximize_quality(module_id, artifact, &self.quality_context)
            .await?;

        // 2. Check hard gates (AC6 G4: Quality Acceptance)
        if !quality_report.passes_hard_gates() {
            return Ok(ExecutionResult {
                status: ExecutionStatus::RevisionRequired,
                quality_report: Some(quality_report),
                ..Default::default()
            });
        }

        // 3. Execute if passed
        let execution_artifacts = self.execute_module(module_id).await?;

        // 4. Post-execution validation
        let mut result = ExecutionResult {
            status: ExecutionStatus::Validated,
            quality_report: Some(quality_report),
            artifacts: execution_artifacts,
            ..Default::default()
        };

        // Link quality report to artifacts
        if let Some(ref report) = result.quality_report {
            for artifact in &mut result.artifacts {
                // AC6 linkage: artifact produced after quality evaluation
                // In full implementation, would add quality_report_id to artifact metadata
            }
        }

        Ok(result)
    }

    /// Execute a module (placeholder)
    async fn execute_module(&self, _module_id: &str) -> std::result::Result<Vec<Artifact>, SentinelError> {
        // TODO: Implement actual module execution
        // For now, return empty artifacts
        Ok(Vec::new())
    }

    /// Check if hard gates pass
    fn passes_hard_gates(&self, report: &QualityReport) -> bool {
        report.passes_hard_gates()
    }

    /// Validate output (placeholder)
    async fn validate_output(&self, _result: &ExecutionResult, _report: &QualityReport) -> std::result::Result<QualityReport, SentinelError> {
        // TODO: Implement post-execution validation
        // For now, return a dummy report
        Err(crate::SentinelError::NotImplemented(
            "Post-execution validation not yet implemented".to_string(),
        ))
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionNorthStar {
    pub where_we_are: String,
    pub where_we_must_go: String,
    pub how: String,
    pub why: String,
    pub constraints: Vec<String>,
}

impl ExecutionNorthStar {
    pub fn validate(&self) -> Result<()> {
        if self.where_we_are.trim().is_empty() {
            return Err(SentinelError::InvalidInput("where_we_are is empty".to_string()));
        }
        if self.where_we_must_go.trim().is_empty() {
            return Err(SentinelError::InvalidInput("where_we_must_go is empty".to_string()));
        }
        if self.how.trim().is_empty() {
            return Err(SentinelError::InvalidInput("how is empty".to_string()));
        }
        if self.why.trim().is_empty() {
            return Err(SentinelError::InvalidInput("why is empty".to_string()));
        }
        Ok(())
    }
}

/// Reliability KPIs for a single execution/reporting window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReliabilitySnapshot {
    pub task_success_rate: f64,
    pub no_regression_rate: f64,
    pub rollback_rate: f64,
    pub avg_time_to_recover_ms: f64,
    pub invariant_violation_rate: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReliabilityThresholds {
    pub min_task_success_rate: f64,
    pub min_no_regression_rate: f64,
    pub max_rollback_rate: f64,
    pub max_invariant_violation_rate: f64,
}

impl Default for ReliabilityThresholds {
    fn default() -> Self {
        Self {
            min_task_success_rate: 0.95,
            min_no_regression_rate: 0.95,
            max_rollback_rate: 0.05,
            max_invariant_violation_rate: 0.02,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReliabilityEvaluation {
    pub healthy: bool,
    pub violations: Vec<String>,
}

impl ReliabilitySnapshot {
    pub fn from_counts(
        total_tasks: u64,
        successful_tasks: u64,
        regression_incidents: u64,
        rollbacks: u64,
        recovery_events: u64,
        total_recovery_ms: u64,
        invariant_violations: u64,
    ) -> Self {
        let success_rate = ratio(successful_tasks, total_tasks);
        let no_regression_rate = ratio(
            total_tasks.saturating_sub(regression_incidents),
            total_tasks,
        );
        let rollback_rate = ratio(rollbacks, total_tasks);
        let avg_time_to_recover_ms = if recovery_events == 0 {
            0.0
        } else {
            total_recovery_ms as f64 / recovery_events as f64
        };
        let invariant_violation_rate = ratio(invariant_violations, total_tasks);

        Self {
            task_success_rate: success_rate,
            no_regression_rate,
            rollback_rate,
            avg_time_to_recover_ms,
            invariant_violation_rate,
        }
    }

    pub fn evaluate(&self, thresholds: &ReliabilityThresholds) -> ReliabilityEvaluation {
        let mut violations = Vec::new();

        if self.task_success_rate < thresholds.min_task_success_rate {
            violations.push(format!(
                "task_success_rate {:.1}% < {:.1}%",
                self.task_success_rate * 100.0,
                thresholds.min_task_success_rate * 100.0
            ));
        }
        if self.no_regression_rate < thresholds.min_no_regression_rate {
            violations.push(format!(
                "no_regression_rate {:.1}% < {:.1}%",
                self.no_regression_rate * 100.0,
                thresholds.min_no_regression_rate * 100.0
            ));
        }
        if self.rollback_rate > thresholds.max_rollback_rate {
            violations.push(format!(
                "rollback_rate {:.1}% > {:.1}%",
                self.rollback_rate * 100.0,
                thresholds.max_rollback_rate * 100.0
            ));
        }
        if self.invariant_violation_rate > thresholds.max_invariant_violation_rate {
            violations.push(format!(
                "invariant_violation_rate {:.1}% > {:.1}%",
                self.invariant_violation_rate * 100.0,
                thresholds.max_invariant_violation_rate * 100.0
            ));
        }

        ReliabilityEvaluation {
            healthy: violations.is_empty(),
            violations,
        }
    }
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionNorthStar, ReliabilitySnapshot, ReliabilityThresholds};

    #[test]
    fn north_star_validation_requires_all_axes() {
        let valid = ExecutionNorthStar {
            where_we_are: "Current repo state analyzed".to_string(),
            where_we_must_go: "Deliver aligned feature".to_string(),
            how: "Validated hierarchical plan".to_string(),
            why: "Increase alignment while preserving invariants".to_string(),
            constraints: vec![],
        };
        assert!(valid.validate().is_ok());

        let invalid = ExecutionNorthStar {
            how: String::new(),
            ..valid
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn reliability_snapshot_computes_stable_ratios() {
        let snapshot = ReliabilitySnapshot::from_counts(10, 9, 1, 1, 2, 600, 1);
        assert!((snapshot.task_success_rate - 0.9).abs() < f64::EPSILON);
        assert!((snapshot.no_regression_rate - 0.9).abs() < f64::EPSILON);
        assert!((snapshot.rollback_rate - 0.1).abs() < f64::EPSILON);
        assert!((snapshot.avg_time_to_recover_ms - 300.0).abs() < f64::EPSILON);
        assert!((snapshot.invariant_violation_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn reliability_evaluation_detects_violations() {
        let snapshot = ReliabilitySnapshot::from_counts(10, 8, 2, 2, 1, 1000, 1);
        let evaluation = snapshot.evaluate(&ReliabilityThresholds::default());
        assert!(!evaluation.healthy);
        assert!(!evaluation.violations.is_empty());
    }
}
