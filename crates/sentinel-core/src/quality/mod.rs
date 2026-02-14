//! Quality Loop Module - Cross-LLM quality maximization
//!
//! This module implements the Quality Maximizer Loop that ensures generated
//! artifacts meet quality standards through iterative refinement.

pub mod evaluation;
pub mod loop_impl;
pub mod rubric;

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use evaluation::{QualityEvaluation, QualityEvaluator};
pub use loop_impl::QualityMaximizer;
pub use rubric::QualityRubric;

/// Quality Report v1.0 - AC6 compliant
///
/// This schema is designed to integrate with the execution contract defined in AC6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub schema_version: String,
    /// Quality report ID - qr_<uuid>
    #[serde(rename = "quality_report_id")]
    pub report_id: String,
    /// Global execution correlation key
    #[serde(rename = "run_id")]
    pub run_id: String,
    /// Per-module correlation key
    #[serde(rename = "module_id")]
    pub module_id: String,
    pub scores: Vec<DimensionScore>,
    pub overall: QualityVerdict,
    /// AC6 linkage: artifact IDs produced by this quality evaluation
    #[serde(rename = "linked_artifact_ids")]
    pub linked_artifact_ids: Vec<String>,
    pub metadata: QualityMetadata,
}

impl QualityReport {
    /// Create a new quality report
    pub fn new(
        run_id: String,
        module_id: String,
        scores: Vec<DimensionScore>,
        metadata: QualityMetadata,
    ) -> Self {
        let overall = Self::compute_overall_verdict(&scores);

        Self {
            schema_version: "1.0".to_string(),
            report_id: format!("qr_{}", uuid::Uuid::new_v4()),
            run_id,
            module_id,
            scores,
            overall,
            linked_artifact_ids: Vec::new(),
            metadata,
        }
    }

    /// Compute overall verdict from dimension scores
    fn compute_overall_verdict(scores: &[DimensionScore]) -> QualityVerdict {
        // Fail if any hard gate failed
        let hard_gate_failed = scores.iter().any(|s| {
            matches!(s.gate, GateType::Hard) && matches!(s.result, Verdict::Fail)
        });

        if hard_gate_failed {
            QualityVerdict::Fail
        } else {
            QualityVerdict::Pass
        }
    }

    /// Check if the report passes all hard gates (AC6 G4: Quality Acceptance)
    pub fn passes_hard_gates(&self) -> bool {
        matches!(self.overall, QualityVerdict::Pass)
    }

    /// Get all failed dimensions
    pub fn failed_dimensions(&self) -> Vec<&QualityMetric> {
        self.scores
            .iter()
            .filter(|s| matches!(s.result, Verdict::Fail))
            .map(|s| &s.metric)
            .collect()
    }
}

/// Score for a single quality dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub metric: QualityMetric,
    /// Actual score value (0-100)
    pub value: f64,
    /// Required threshold
    pub threshold: f64,
    pub gate: GateType,
    pub result: Verdict,
}

impl DimensionScore {
    /// Create a new dimension score
    pub fn new(metric: QualityMetric, value: f64, threshold: f64, gate: GateType) -> Self {
        let result = if value >= threshold {
            Verdict::Pass
        } else {
            Verdict::Fail
        };

        Self {
            metric,
            value,
            threshold,
            gate,
            result,
        }
    }
}

/// Quality metrics measured by the rubric
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QualityMetric {
    /// Correctness: Does the artifact meet functional requirements?
    Correctness,
    /// Reliability: Does it work consistently under expected conditions?
    Reliability,
    /// Maintainability: Is it easy to understand and modify?
    Maintainability,
    /// Security: Does it follow security best practices?
    Security,
    /// UX/DevEx: Is the user/developer experience good?
    #[serde(rename = "UXDevEx")]
    UXDevEx,
}

impl QualityMetric {
    /// Get default threshold for this metric
    pub fn default_threshold(&self) -> f64 {
        match self {
            QualityMetric::Correctness => 85.0,
            QualityMetric::Reliability => 80.0,
            QualityMetric::Maintainability => 70.0,
            QualityMetric::Security => 90.0,
            QualityMetric::UXDevEx => 75.0,
        }
    }

    /// Get default gate type for this metric
    pub fn default_gate(&self) -> GateType {
        match self {
            QualityMetric::Correctness => GateType::Hard,
            QualityMetric::Reliability => GateType::Hard,
            QualityMetric::Maintainability => GateType::Soft,
            QualityMetric::Security => GateType::Hard,
            QualityMetric::UXDevEx => GateType::Soft,
        }
    }
}

/// Pass/Fail verdict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "fail")]
    Fail,
}

/// Hard or soft gate type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GateType {
    #[serde(rename = "hard")]
    Hard,
    #[serde(rename = "soft")]
    Soft,
}

/// Overall quality verdict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QualityVerdict {
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "fail")]
    Fail,
}

/// Metadata about quality evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetadata {
    /// LLM provider used (e.g., "anthropic", "openai")
    pub llm_provider: String,
    /// Model name (e.g., "claude-3-5-sonnet", "gpt-4")
    pub model: String,
    /// When evaluation was performed
    pub evaluated_at: DateTime<Utc>,
    /// Evaluation duration in milliseconds
    pub evaluation_duration_ms: u64,
    /// Loop iteration number (1-3)
    pub iteration: u32,
}

/// Generated artifact being evaluated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// AC6 linkage: immutable artifact ID
    #[serde(rename = "artifact_id")]
    pub id: String,
    /// Artifact type
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
    /// Artifact content (can be large)
    pub content: String,
    /// URI for accessing the artifact
    pub uri: String,
    /// SHA256 hash of content
    pub hash: String,
}

impl Artifact {
    /// Create a new artifact
    pub fn new(
        artifact_type: ArtifactType,
        content: String,
        uri: String,
    ) -> Result<Self> {
        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        Ok(Self {
            id: format!("art_{}", uuid::Uuid::new_v4()),
            artifact_type,
            content,
            uri,
            hash,
        })
    }
}

/// Type of artifact
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    /// Plan/architecture document
    Plan,
    /// Source code
    Code,
    /// Test code
    Test,
    /// Quality evaluation
    Evaluation,
    /// Final delivery package
    Delivery,
}

/// Context for quality evaluation
#[derive(Debug, Clone)]
pub struct QualityContext {
    /// AC6 linkage: run ID
    pub run_id: String,
    pub quality_config: QualityConfig,
    pub evaluation_suite: Option<EvaluationSuite>,
}

/// Quality configuration
#[derive(Debug, Clone)]
pub struct QualityConfig {
    /// Maximum iterations in the quality loop
    pub max_iterations: u32,
    /// Whether to enable auto-repair
    pub enable_auto_repair: bool,
    /// LLM provider to use for critique/repair
    pub llm_provider: String,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            max_iterations: 3,
            enable_auto_repair: true,
            llm_provider: "anthropic".to_string(),
        }
    }
}

/// Evaluation suite for running tests
#[derive(Debug, Clone)]
pub struct EvaluationSuite {
    /// Test command to run
    pub test_command: String,
    /// Minimum required coverage
    pub min_coverage: Option<f64>,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_report_computation() {
        let scores = vec![
            DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard),
            DimensionScore::new(QualityMetric::Reliability, 82.0, 80.0, GateType::Hard),
            DimensionScore::new(QualityMetric::Maintainability, 65.0, 70.0, GateType::Soft),
        ];

        let metadata = QualityMetadata {
            llm_provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            evaluated_at: Utc::now(),
            evaluation_duration_ms: 1500,
            iteration: 1,
        };

        let report = QualityReport::new(
            "run_test".to_string(),
            "mod_test".to_string(),
            scores,
            metadata,
        );

        // Should pass even though maintainability (soft gate) failed
        assert_eq!(report.overall, QualityVerdict::Pass);
        assert!(report.passes_hard_gates());
    }

    #[test]
    fn test_quality_report_hard_gate_failure() {
        let scores = vec![
            DimensionScore::new(QualityMetric::Correctness, 80.0, 85.0, GateType::Hard),
            DimensionScore::new(QualityMetric::Reliability, 85.0, 80.0, GateType::Hard),
        ];

        let metadata = QualityMetadata {
            llm_provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            evaluated_at: Utc::now(),
            evaluation_duration_ms: 1500,
            iteration: 1,
        };

        let report = QualityReport::new(
            "run_test".to_string(),
            "mod_test".to_string(),
            scores,
            metadata,
        );

        // Should fail because correctness (hard gate) failed
        assert_eq!(report.overall, QualityVerdict::Fail);
        assert!(!report.passes_hard_gates());
    }

    #[test]
    fn test_dimension_score_verdict() {
        let pass = DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard);
        assert_eq!(pass.result, Verdict::Pass);

        let fail = DimensionScore::new(QualityMetric::Correctness, 80.0, 85.0, GateType::Hard);
        assert_eq!(fail.result, Verdict::Fail);
    }

    #[test]
    fn test_artifact_creation() {
        let artifact = Artifact::new(
            ArtifactType::Code,
            "fn main() {}".to_string(),
            "file:///src/main.rs".to_string(),
        )
        .unwrap();

        assert!(artifact.id.starts_with("art_"));
        assert_eq!(artifact.artifact_type, ArtifactType::Code);
        assert!(!artifact.hash.is_empty());
    }
}
