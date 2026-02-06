//! Execution contract types shared across Sentinel runtime layers.

/// Explicit execution contract:
/// - where we are
/// - where we must go
/// - how we will get there
/// - why this execution is justified
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionNorthStar {
    pub where_we_are: String,
    pub where_we_must_go: String,
    pub how: String,
    pub why: String,
    pub constraints: Vec<String>,
}

impl ExecutionNorthStar {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.where_we_are.trim().is_empty() {
            return Err("where_we_are is empty");
        }
        if self.where_we_must_go.trim().is_empty() {
            return Err("where_we_must_go is empty");
        }
        if self.how.trim().is_empty() {
            return Err("how is empty");
        }
        if self.why.trim().is_empty() {
            return Err("why is empty");
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
    use super::{ExecutionNorthStar, ReliabilitySnapshot};

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
}
