//! Quality Maximizer Loop - Generate -> Critique -> Repair -> Verify
//!
//! This module implements the iterative quality improvement loop.

use crate::error::Result;
use crate::quality::{
    Artifact, QualityContext, QualityEvaluator, QualityReport,
};
use chrono::Utc;
use std::time::Duration;

/// Quality Maximizer - iterative quality improvement loop
pub struct QualityMaximizer {
    evaluator: QualityEvaluator,
    max_iterations: u32,
}

impl QualityMaximizer {
    /// Create a new quality maximizer
    pub fn new() -> Self {
        Self {
            evaluator: QualityEvaluator::new(),
            max_iterations: 3,
        }
    }

    /// Create with custom max iterations
    pub fn with_max_iterations(max_iterations: u32) -> Self {
        Self {
            evaluator: QualityEvaluator::new(),
            max_iterations,
        }
    }

    /// Execute the quality maximization loop
    ///
    /// This implements the generate -> critique -> repair -> verify cycle:
    /// 1. First iteration: evaluate existing artifact
    /// 2. If not converged: generate revision
    /// 3. Evaluate revision
    /// 4. Repeat until converged or max iterations reached
    pub async fn maximize_quality(
        &self,
        module_id: &str,
        artifact: &Artifact,
        context: &QualityContext,
    ) -> Result<QualityReport> {
        let mut current_artifact = artifact.clone();
        let mut iteration = 1;
        let mut best_report: Option<QualityReport> = None;

        loop {
            // Evaluate current artifact
            let mut report = self
                .evaluator
                .evaluate(module_id, &current_artifact, context)
                .await?;

            // Update iteration number in metadata
            report.metadata.iteration = iteration;

            // Track best report
            if best_report.is_none() || self.is_better_than(&report, best_report.as_ref().unwrap()) {
                best_report = Some(report.clone());
            }

            // Check convergence
            if self.is_converged(&report) {
                return Ok(report);
            }

            // Check max iterations
            if iteration >= self.max_iterations {
                // Return best report if current didn't converge
                if let Some(best) = best_report {
                    return Ok(best);
                }
                return Ok(report);
            }

            // Generate revision for next iteration
            current_artifact = self
                .generate_revision(&current_artifact, &report, context)
                .await?;

            iteration += 1;
        }
    }

    /// Generate a revised artifact based on quality feedback
    async fn generate_revision(
        &self,
        artifact: &Artifact,
        report: &QualityReport,
        context: &QualityContext,
    ) -> Result<Artifact> {
        // TODO: Implement actual revision generation using LLM
        // For now, return a placeholder
        let revised_content = self
            .generate_revision_prompt(artifact, report)
            .await
            .unwrap_or_else(|_| format!("// Revision {} of {}\n{}", report.metadata.iteration, artifact.uri, artifact.content));

        Artifact::new(artifact.artifact_type.clone(), revised_content, artifact.uri.clone())
    }

    /// Generate revision prompt (placeholder)
    async fn generate_revision_prompt(
        &self,
        artifact: &Artifact,
        report: &QualityReport,
    ) -> Result<String> {
        // Build critique message
        let mut critique = String::from("Quality evaluation found issues:\n");

        for score in &report.scores {
            if matches!(score.result, crate::quality::Verdict::Fail) {
                critique.push_str(&format!(
                    "- {:?}: {:.1}/{} (threshold: {:.1}) - FAILED\n",
                    score.metric, score.value, score.threshold, score.threshold
                ));
            }
        }

        // TODO: Send to LLM for revision
        // For now, return placeholder
        Ok(format!(
            "// Revision based on:\n{}\n// Original:\n{}",
            critique, artifact.content
        ))
    }

    /// Check if the quality report indicates convergence
    fn is_converged(&self, report: &QualityReport) -> bool {
        // Converged if:
        // - Overall verdict is Pass
        // - All hard gates pass
        // - No critical findings
        report.passes_hard_gates() && matches!(report.overall, crate::quality::QualityVerdict::Pass)
    }

    /// Check if report1 is better than report2
    fn is_better_than(&self, report1: &QualityReport, report2: &QualityReport) -> bool {
        // Compare by number of passed dimensions
        let passed1 = report1
            .scores
            .iter()
            .filter(|s| matches!(s.result, crate::quality::Verdict::Pass))
            .count();

        let passed2 = report2
            .scores
            .iter()
            .filter(|s| matches!(s.result, crate::quality::Verdict::Pass))
            .count();

        if passed1 != passed2 {
            return passed1 > passed2;
        }

        // Tie-breaker: overall average score
        let avg1 = report1.scores.iter().map(|s| s.value).sum::<f64>() / report1.scores.len() as f64;
        let avg2 = report2.scores.iter().map(|s| s.value).sum::<f64>() / report2.scores.len() as f64;

        avg1 > avg2
    }

    /// Get estimated time for next iteration
    pub fn estimate_iteration_time(&self, artifact: &Artifact) -> Duration {
        // Estimate based on artifact size
        let size_bytes = artifact.content.len() as u64;

        // Base time + size-dependent factor
        let base_ms = 500;
        let size_factor_ms = size_bytes / 100; // 1ms per 100 bytes

        Duration::from_millis(base_ms + size_factor_ms)
    }
}

impl Default for QualityMaximizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::{ArtifactType, QualityConfig};

    fn create_test_context() -> QualityContext {
        QualityContext {
            run_id: "test_run".to_string(),
            quality_config: QualityConfig {
                max_iterations: 3,
                enable_auto_repair: true,
                llm_provider: "anthropic".to_string(),
            },
            evaluation_suite: None,
        }
    }

    fn create_test_artifact(content: &str) -> Artifact {
        Artifact::new(
            ArtifactType::Code,
            content.to_string(),
            "file:///test.rs".to_string(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_maximizer_creates_report() {
        let maximizer = QualityMaximizer::new();
        let artifact = create_test_artifact("fn main() {}");
        let context = create_test_context();

        let report = maximizer
            .maximize_quality("test_module", &artifact, &context)
            .await
            .unwrap();

        assert_eq!(report.module_id, "test_module");
        assert_eq!(report.run_id, "test_run");
        assert!(!report.scores.is_empty());
    }

    #[test]
    fn test_convergence_check() {
        let maximizer = QualityMaximizer::new();

        // Passing report should converge
        let passing_scores = vec![
            crate::quality::DimensionScore::new(
                crate::quality::QualityMetric::Correctness,
                90.0,
                85.0,
                crate::quality::GateType::Hard,
            ),
            crate::quality::DimensionScore::new(
                crate::quality::QualityMetric::Reliability,
                85.0,
                80.0,
                crate::quality::GateType::Hard,
            ),
        ];

        let metadata = crate::quality::QualityMetadata {
            llm_provider: "test".to_string(),
            model: "test".to_string(),
            evaluated_at: Utc::now(),
            evaluation_duration_ms: 100,
            iteration: 1,
        };

        let passing_report = QualityReport::new(
            "run".to_string(),
            "mod".to_string(),
            passing_scores,
            metadata.clone(),
        );

        assert!(maximizer.is_converged(&passing_report));

        // Failing hard gate should not converge
        let failing_scores = vec![
            crate::quality::DimensionScore::new(
                crate::quality::QualityMetric::Correctness,
                75.0,
                85.0,
                crate::quality::GateType::Hard,
            ),
        ];

        let failing_report = QualityReport::new(
            "run".to_string(),
            "mod".to_string(),
            failing_scores,
            metadata.clone(),
        );

        assert!(!maximizer.is_converged(&failing_report));
    }

    #[test]
    fn test_iteration_time_estimation() {
        let maximizer = QualityMaximizer::new();

        let small_artifact = create_test_artifact("fn main() {}");
        let small_time = maximizer.estimate_iteration_time(&small_artifact);
        assert!(small_time.as_millis() < 1000);

        let large_content = "x".repeat(10000);
        let large_artifact = create_test_artifact(&large_content);
        let large_time = maximizer.estimate_iteration_time(&large_artifact);
        assert!(large_time > small_time);
    }

    #[tokio::test]
    async fn test_revision_generation() {
        let maximizer = QualityMaximizer::new();
        let artifact = create_test_artifact("fn main() {}");
        let context = create_test_context();

        // Create a failing report
        let scores = vec![crate::quality::DimensionScore::new(
            crate::quality::QualityMetric::Correctness,
            70.0,
            85.0,
            crate::quality::GateType::Hard,
        )];

        let metadata = crate::quality::QualityMetadata {
            llm_provider: "test".to_string(),
            model: "test".to_string(),
            evaluated_at: Utc::now(),
            evaluation_duration_ms: 100,
            iteration: 1,
        };

        let report = crate::quality::QualityReport::new("run".to_string(), "mod".to_string(), scores, metadata);

        let revision = maximizer
            .generate_revision(&artifact, &report, &context)
            .await
            .unwrap();

        // Revision should contain critique
        assert!(revision.content.contains("Revision"));
        assert!(revision.id != artifact.id); // New artifact created
    }
}
