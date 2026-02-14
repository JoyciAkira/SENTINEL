use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReliabilityConfig {
    pub thresholds: ReliabilityThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityEvaluation {
    pub healthy: bool,
    pub violations: Vec<String>,
}

impl Default for ReliabilityEvaluation {
    fn default() -> Self {
        Self {
            healthy: true,
            violations: Vec::new(),
        }
    }
}

pub fn load_reliability_config(path: &Path) -> ReliabilityConfig {
    let mut config = ReliabilityConfig::default();

    let content = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return config,
    };

    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(parsed) => parsed,
        Err(_) => return config,
    };

    if let Some(thresholds_value) = value
        .get("reliability")
        .and_then(|v| v.get("thresholds"))
        .or_else(|| value.get("reliability_thresholds"))
    {
        config.thresholds = thresholds_from_value(thresholds_value, &config.thresholds);
    }

    config
}

pub fn snapshot_from_signals(
    alignment_score: f64,
    alignment_confidence: f64,
    completion_percentage: f64,
    guardrail_allowed: bool,
) -> sentinel_core::ReliabilitySnapshot {
    let alignment_factor = if alignment_score <= 1.0 {
        alignment_score.clamp(0.0, 1.0)
    } else {
        (alignment_score / 100.0).clamp(0.0, 1.0)
    };
    let confidence_factor = alignment_confidence.clamp(0.0, 1.0);
    let completion_factor = completion_percentage.clamp(0.0, 1.0);

    let task_success_rate =
        (alignment_factor * 0.60 + confidence_factor * 0.20 + completion_factor * 0.20)
            .clamp(0.0, 1.0);
    let no_regression_rate = if guardrail_allowed {
        (alignment_factor * 0.70 + confidence_factor * 0.30).clamp(0.0, 1.0)
    } else {
        (alignment_factor * 0.50 + confidence_factor * 0.20).clamp(0.0, 1.0)
    };
    let rollback_rate = if guardrail_allowed {
        ((1.0 - task_success_rate) * 0.25).clamp(0.0, 1.0)
    } else {
        ((1.0 - task_success_rate) * 0.55 + 0.05).clamp(0.0, 1.0)
    };
    let invariant_violation_rate = if guardrail_allowed {
        ((1.0 - alignment_factor) * 0.35).clamp(0.0, 1.0)
    } else {
        ((1.0 - alignment_factor) * 0.65 + 0.05).clamp(0.0, 1.0)
    };
    let avg_time_to_recover_ms = (rollback_rate * 1200.0) + (invariant_violation_rate * 800.0);

    sentinel_core::ReliabilitySnapshot {
        task_success_rate,
        no_regression_rate,
        rollback_rate,
        avg_time_to_recover_ms,
        invariant_violation_rate,
    }
}

pub fn evaluate_snapshot(
    snapshot: &sentinel_core::ReliabilitySnapshot,
    thresholds: &ReliabilityThresholds,
) -> ReliabilityEvaluation {
    let mut violations = Vec::new();

    if snapshot.task_success_rate < thresholds.min_task_success_rate {
        violations.push(format!(
            "task_success_rate {:.1}% < {:.1}%",
            snapshot.task_success_rate * 100.0,
            thresholds.min_task_success_rate * 100.0
        ));
    }
    if snapshot.no_regression_rate < thresholds.min_no_regression_rate {
        violations.push(format!(
            "no_regression_rate {:.1}% < {:.1}%",
            snapshot.no_regression_rate * 100.0,
            thresholds.min_no_regression_rate * 100.0
        ));
    }
    if snapshot.rollback_rate > thresholds.max_rollback_rate {
        violations.push(format!(
            "rollback_rate {:.1}% > {:.1}%",
            snapshot.rollback_rate * 100.0,
            thresholds.max_rollback_rate * 100.0
        ));
    }
    if snapshot.invariant_violation_rate > thresholds.max_invariant_violation_rate {
        violations.push(format!(
            "invariant_violation_rate {:.1}% > {:.1}%",
            snapshot.invariant_violation_rate * 100.0,
            thresholds.max_invariant_violation_rate * 100.0
        ));
    }

    ReliabilityEvaluation {
        healthy: violations.is_empty(),
        violations,
    }
}

fn thresholds_from_value(
    value: &serde_json::Value,
    defaults: &ReliabilityThresholds,
) -> ReliabilityThresholds {
    let read = |key: &str, fallback: f64| -> f64 {
        value
            .get(key)
            .and_then(serde_json::Value::as_f64)
            .map(normalize_ratio)
            .unwrap_or(fallback)
    };

    ReliabilityThresholds {
        min_task_success_rate: read("min_task_success_rate", defaults.min_task_success_rate),
        min_no_regression_rate: read("min_no_regression_rate", defaults.min_no_regression_rate),
        max_rollback_rate: read("max_rollback_rate", defaults.max_rollback_rate),
        max_invariant_violation_rate: read(
            "max_invariant_violation_rate",
            defaults.max_invariant_violation_rate,
        ),
    }
}

fn normalize_ratio(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        return 0.0;
    }
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_snapshot, thresholds_from_value, ReliabilityThresholds};

    #[test]
    fn thresholds_accept_percentage_values() {
        let defaults = ReliabilityThresholds::default();
        let value = serde_json::json!({
            "min_task_success_rate": 95.0,
            "max_rollback_rate": 4.0
        });

        let parsed = thresholds_from_value(&value, &defaults);
        assert!((parsed.min_task_success_rate - 0.95).abs() < f64::EPSILON);
        assert!((parsed.max_rollback_rate - 0.04).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluation_reports_threshold_violations() {
        let thresholds = ReliabilityThresholds::default();
        let snapshot = sentinel_core::ReliabilitySnapshot {
            task_success_rate: 0.9,
            no_regression_rate: 0.9,
            rollback_rate: 0.2,
            avg_time_to_recover_ms: 100.0,
            invariant_violation_rate: 0.1,
        };

        let eval = evaluate_snapshot(&snapshot, &thresholds);
        assert!(!eval.healthy);
        assert!(eval.violations.len() >= 3);
    }
}
