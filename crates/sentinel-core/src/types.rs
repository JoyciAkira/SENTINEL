//! Core types for Sentinel
//!
//! This module defines fundamental types used throughout Sentinel:
//! - Goal status
//! - Timestamps
//! - Hashes
//! - Distribution types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Goal execution status
///
/// Represents the current state of a goal in its lifecycle.
/// State transitions are validated to ensure integrity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalStatus {
    /// Goal is waiting for dependencies
    Pending,

    /// Dependencies satisfied, ready to start
    Ready,

    /// Currently being worked on
    InProgress,

    /// Running validation tests
    Validating,

    /// Successfully completed
    Completed,

    /// Blocked by external factors
    Blocked,

    /// Failed after retries
    Failed,

    /// No longer relevant to root goal
    Deprecated,
}

impl GoalStatus {
    /// Check if a state transition is valid
    ///
    /// # Examples
    ///
    /// ```
    /// use sentinel_core::types::GoalStatus;
    ///
    /// assert!(GoalStatus::Pending.can_transition_to(GoalStatus::Ready));
    /// assert!(!GoalStatus::Completed.can_transition_to(GoalStatus::Pending));
    /// ```
    pub fn can_transition_to(self, next: GoalStatus) -> bool {
        use GoalStatus::*;

        match (self, next) {
            // From Pending
            (Pending, Ready) => true,
            (Pending, Blocked) => true,
            (Pending, Deprecated) => true,

            // From Ready
            (Ready, InProgress) => true,
            (Ready, Blocked) => true,
            (Ready, Deprecated) => true,

            // From InProgress
            (InProgress, Validating) => true,
            (InProgress, Blocked) => true,
            (InProgress, Failed) => true,

            // From Validating
            (Validating, Completed) => true,
            (Validating, Failed) => true,
            (Validating, InProgress) => true, // Retry after validation failure

            // From Blocked
            (Blocked, Ready) => true,
            (Blocked, InProgress) => true,
            (Blocked, Deprecated) => true,

            // From Failed
            (Failed, InProgress) => true, // Retry
            (Failed, Deprecated) => true,

            // Completed and Deprecated are terminal states
            (Completed, _) => false,
            (Deprecated, _) => false,

            // Same state is always valid
            (s1, s2) if s1 == s2 => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(self) -> bool {
        matches!(self, GoalStatus::Completed | GoalStatus::Deprecated)
    }

    /// Check if this is a working state (Ready, InProgress, Validating)
    pub fn is_working(self) -> bool {
        matches!(
            self,
            GoalStatus::Ready | GoalStatus::InProgress | GoalStatus::Validating
        )
    }
}

impl fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GoalStatus::Pending => write!(f, "pending"),
            GoalStatus::Ready => write!(f, "ready"),
            GoalStatus::InProgress => write!(f, "in_progress"),
            GoalStatus::Validating => write!(f, "validating"),
            GoalStatus::Completed => write!(f, "completed"),
            GoalStatus::Blocked => write!(f, "blocked"),
            GoalStatus::Failed => write!(f, "failed"),
            GoalStatus::Deprecated => write!(f, "deprecated"),
        }
    }
}

/// Timestamp type alias for consistency
pub type Timestamp = DateTime<Utc>;

/// Create a timestamp for the current moment
pub fn now() -> Timestamp {
    Utc::now()
}

/// Blake3 hash wrapper for type safety
///
/// Provides a type-safe wrapper around Blake3 hashes to prevent
/// mixing different hash types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Create a new Blake3Hash from raw bytes
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string for display
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

impl fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blake3:{}", self.to_hex())
    }
}

impl From<blake3::Hash> for Blake3Hash {
    fn from(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

/// Probability distribution for complexity estimates
///
/// Instead of single-point estimates, we use distributions to capture uncertainty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbabilityDistribution {
    /// Mean (μ)
    pub mean: f64,

    /// Standard deviation (σ)
    pub std_dev: f64,

    /// Minimum value (p5)
    pub min: f64,

    /// Maximum value (p95)
    pub max: f64,

    /// Distribution type
    pub distribution_type: DistributionType,
}

impl ProbabilityDistribution {
    /// Create a normal distribution
    ///
    /// # Examples
    ///
    /// ```
    /// use sentinel_core::types::ProbabilityDistribution;
    ///
    /// let dist = ProbabilityDistribution::normal(5.0, 1.5);
    /// assert_eq!(dist.mean, 5.0);
    /// ```
    pub fn normal(mean: f64, std_dev: f64) -> Self {
        Self {
            mean,
            std_dev,
            min: mean - 2.0 * std_dev, // ~p5
            max: mean + 2.0 * std_dev, // ~p95
            distribution_type: DistributionType::Normal,
        }
    }

    /// Create a uniform distribution
    pub fn uniform(min: f64, max: f64) -> Self {
        let mean = (min + max) / 2.0;
        let std_dev = (max - min) / (2.0 * 3.0_f64.sqrt());

        Self {
            mean,
            std_dev,
            min,
            max,
            distribution_type: DistributionType::Uniform,
        }
    }

    /// Create a point estimate (zero variance)
    pub fn point(value: f64) -> Self {
        Self {
            mean: value,
            std_dev: 0.0,
            min: value,
            max: value,
            distribution_type: DistributionType::Point,
        }
    }

    /// Sample from the distribution (using Box-Muller for normal)
    pub fn sample(&self) -> f64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        match self.distribution_type {
            DistributionType::Normal => {
                // Box-Muller transform
                let u1: f64 = rng.gen();
                let u2: f64 = rng.gen();
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                self.mean + z0 * self.std_dev
            }
            DistributionType::Uniform => rng.gen_range(self.min..=self.max),
            DistributionType::Point => self.mean,
        }
    }

    /// Get confidence interval [lower, upper]
    pub fn confidence_interval(&self, confidence: f64) -> (f64, f64) {
        match self.distribution_type {
            DistributionType::Normal => {
                // Z-score for confidence level
                let z = match confidence {
                    c if (c - 0.68).abs() < 0.01 => 1.0,  // 68%
                    c if (c - 0.95).abs() < 0.01 => 1.96, // 95%
                    c if (c - 0.99).abs() < 0.01 => 2.58, // 99%
                    _ => 1.96,                            // default to 95%
                };
                (self.mean - z * self.std_dev, self.mean + z * self.std_dev)
            }
            DistributionType::Uniform => (self.min, self.max),
            DistributionType::Point => (self.mean, self.mean),
        }
    }
}

/// Type of probability distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistributionType {
    Normal,
    Uniform,
    Point,
}

/// Comparison operator for predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Comparison {
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = "<=")]
    LessThanOrEqual,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
}

impl Comparison {
    /// Evaluate the comparison
    pub fn evaluate(&self, left: f64, right: f64) -> bool {
        match self {
            Comparison::Equal => (left - right).abs() < f64::EPSILON,
            Comparison::NotEqual => (left - right).abs() >= f64::EPSILON,
            Comparison::LessThan => left < right,
            Comparison::LessThanOrEqual => left <= right,
            Comparison::GreaterThan => left > right,
            Comparison::GreaterThanOrEqual => left >= right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_status_transitions() {
        assert!(GoalStatus::Pending.can_transition_to(GoalStatus::Ready));
        assert!(GoalStatus::Ready.can_transition_to(GoalStatus::InProgress));
        assert!(GoalStatus::InProgress.can_transition_to(GoalStatus::Validating));
        assert!(GoalStatus::Validating.can_transition_to(GoalStatus::Completed));

        // Invalid transitions
        assert!(!GoalStatus::Completed.can_transition_to(GoalStatus::Pending));
        assert!(!GoalStatus::Deprecated.can_transition_to(GoalStatus::Ready));
    }

    #[test]
    fn test_goal_status_terminal() {
        assert!(GoalStatus::Completed.is_terminal());
        assert!(GoalStatus::Deprecated.is_terminal());
        assert!(!GoalStatus::InProgress.is_terminal());
    }

    #[test]
    fn test_blake3_hash_display() {
        let hash = Blake3Hash::new([0u8; 32]);
        assert!(hash.to_string().starts_with("blake3:"));
    }

    #[test]
    fn test_probability_distribution_normal() {
        let dist = ProbabilityDistribution::normal(5.0, 1.5);
        assert_eq!(dist.mean, 5.0);
        assert_eq!(dist.std_dev, 1.5);
    }

    #[test]
    fn test_probability_distribution_confidence_interval() {
        let dist = ProbabilityDistribution::normal(5.0, 1.0);
        let (lower, upper) = dist.confidence_interval(0.95);

        // 95% CI should be approximately mean ± 1.96*std_dev
        assert!((lower - 3.04).abs() < 0.01);
        assert!((upper - 6.96).abs() < 0.01);
    }

    #[test]
    fn test_comparison_evaluate() {
        assert!(Comparison::Equal.evaluate(1.0, 1.0));
        assert!(Comparison::LessThan.evaluate(1.0, 2.0));
        assert!(Comparison::GreaterThan.evaluate(2.0, 1.0));
        assert!(!Comparison::Equal.evaluate(1.0, 2.0));
    }
}
