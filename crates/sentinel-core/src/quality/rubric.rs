//! Quality Rubric - Scoring definitions for quality evaluation
//!
//! This module defines the rubric v1 with 5 quality dimensions.

use crate::quality::{DimensionScore, GateType};
use serde::{Deserialize, Serialize};

// Re-export from parent module to avoid circular dependency
pub use super::{QualityMetric, QualityReport};

/// Quality rubric for scoring artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRubric {
    pub version: String,
    pub dimensions: Vec<RubricDimension>,
}

impl QualityRubric {
    /// Create rubric v1 with standard 5 dimensions
    pub fn v1() -> Self {
        Self {
            version: "1.0".to_string(),
            dimensions: vec![
                RubricDimension {
                    metric: QualityMetric::Correctness,
                    description: "Functional correctness - does it meet requirements?".to_string(),
                    weight: 0.30,
                    default_threshold: 85.0,
                    default_gate: GateType::Hard,
                    evaluation_criteria: vec![
                        "All acceptance criteria met".to_string(),
                        "No functional bugs".to_string(),
                        "Edge cases handled".to_string(),
                    ],
                },
                RubricDimension {
                    metric: QualityMetric::Reliability,
                    description: "Consistent behavior under expected conditions".to_string(),
                    weight: 0.25,
                    default_threshold: 80.0,
                    default_gate: GateType::Hard,
                    evaluation_criteria: vec![
                        "Error handling present".to_string(),
                        "Graceful degradation".to_string(),
                        "Idempotent where appropriate".to_string(),
                    ],
                },
                RubricDimension {
                    metric: QualityMetric::Maintainability,
                    description: "Ease of understanding and modification".to_string(),
                    weight: 0.20,
                    default_threshold: 70.0,
                    default_gate: GateType::Soft,
                    evaluation_criteria: vec![
                        "Clear naming".to_string(),
                        "Reasonable complexity".to_string(),
                        "Documentation adequate".to_string(),
                    ],
                },
                RubricDimension {
                    metric: QualityMetric::Security,
                    description: "Security best practices followed".to_string(),
                    weight: 0.15,
                    default_threshold: 90.0,
                    default_gate: GateType::Hard,
                    evaluation_criteria: vec![
                        "No hardcoded secrets".to_string(),
                        "Input validation present".to_string(),
                        "Safe defaults used".to_string(),
                    ],
                },
                RubricDimension {
                    metric: QualityMetric::UXDevEx,
                    description: "User/Developer experience quality".to_string(),
                    weight: 0.10,
                    default_threshold: 75.0,
                    default_gate: GateType::Soft,
                    evaluation_criteria: vec![
                        "Clear error messages".to_string(),
                        "Helpful logging".to_string(),
                        "Intuitive API design".to_string(),
                    ],
                },
            ],
        }
    }

    /// Get dimension by metric
    pub fn get_dimension(&self, metric: &QualityMetric) -> Option<&RubricDimension> {
        self.dimensions.iter().find(|d| &d.metric == metric)
    }

    /// Score all dimensions for an artifact
    pub fn score_all(&self, scores: &[(QualityMetric, f64)]) -> Vec<DimensionScore> {
        self.dimensions
            .iter()
            .map(|dim| {
                let value = scores
                    .iter()
                    .find(|(m, _)| m == &dim.metric)
                    .map(|(_, v)| *v)
                    .unwrap_or(0.0);

                DimensionScore::new(dim.metric.clone(), value, dim.default_threshold, dim.default_gate)
            })
            .collect()
    }

    /// Compute weighted overall score
    pub fn compute_weighted_score(&self, scores: &[DimensionScore]) -> f64 {
        let mut total = 0.0;
        let mut weight_sum = 0.0;

        for dim in &self.dimensions {
            if let Some(score) = scores.iter().find(|s| s.metric == dim.metric) {
                total += score.value * dim.weight;
                weight_sum += dim.weight;
            }
        }

        if weight_sum > 0.0 {
            total / weight_sum
        } else {
            0.0
        }
    }
}

/// A single dimension in the quality rubric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricDimension {
    pub metric: QualityMetric,
    pub description: String,
    /// Weight for computing overall score (0-1)
    pub weight: f64,
    pub default_threshold: f64,
    pub default_gate: GateType,
    pub evaluation_criteria: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rubric_v1_structure() {
        let rubric = QualityRubric::v1();

        assert_eq!(rubric.version, "1.0");
        assert_eq!(rubric.dimensions.len(), 5);

        // Check weights sum to approximately 1.0
        let total_weight: f64 = rubric.dimensions.iter().map(|d| d.weight).sum();
        assert!((total_weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_get_dimension() {
        let rubric = QualityRubric::v1();

        let correctness = rubric.get_dimension(&QualityMetric::Correctness);
        assert!(correctness.is_some());
        assert_eq!(correctness.unwrap().default_threshold, 85.0);
        assert_eq!(correctness.unwrap().default_gate, GateType::Hard);
    }

    #[test]
    fn test_score_all_dimensions() {
        let rubric = QualityRubric::v1();

        let input_scores = vec![
            (QualityMetric::Correctness, 90.0),
            (QualityMetric::Reliability, 85.0),
            (QualityMetric::Maintainability, 75.0),
            (QualityMetric::Security, 92.0),
            (QualityMetric::UXDevEx, 80.0),
        ];

        let dimension_scores = rubric.score_all(&input_scores);

        assert_eq!(dimension_scores.len(), 5);

        // Check correctness passed hard gate
        let correctness = dimension_scores
            .iter()
            .find(|s| s.metric == QualityMetric::Correctness)
            .unwrap();
        assert!(correctness.value >= correctness.threshold);
    }

    #[test]
    fn test_weighted_score_computation() {
        let rubric = QualityRubric::v1();

        let scores = vec![
            DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard),
            DimensionScore::new(QualityMetric::Reliability, 80.0, 80.0, GateType::Hard),
            DimensionScore::new(QualityMetric::Maintainability, 70.0, 70.0, GateType::Soft),
            DimensionScore::new(QualityMetric::Security, 90.0, 90.0, GateType::Hard),
            DimensionScore::new(QualityMetric::UXDevEx, 75.0, 75.0, GateType::Soft),
        ];

        let weighted = rubric.compute_weighted_score(&scores);

        // Weighted score should be between min and max
        assert!(weighted >= 70.0 && weighted <= 90.0);
    }
}
