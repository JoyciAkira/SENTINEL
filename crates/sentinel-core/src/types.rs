//! Core types for Sentinel
//!
//! Questo modulo definisce i tipi fondamentali usati in Sentinel:
//! - Goal status
//! - Timestamps
//! - Hashes
//! - Distribution types (Confidence calibration)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Goal execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GoalStatus {
    Pending,
    Ready,
    InProgress,
    Validating,
    Completed,
    Blocked,
    Failed,
    Deprecated,
}

impl GoalStatus {
    pub fn can_transition_to(self, next: GoalStatus) -> bool {
        use GoalStatus::*;
        match (self, next) {
            (Pending, Ready) | (Pending, Blocked) | (Pending, Deprecated) => true,
            (Ready, InProgress) | (Ready, Blocked) | (Ready, Deprecated) => true,
            (InProgress, Validating) | (InProgress, Blocked) | (InProgress, Failed) => true,
            (Validating, Completed) | (Validating, Failed) | (Validating, InProgress) => true,
            (Blocked, Ready) | (Blocked, InProgress) | (Blocked, Deprecated) => true,
            (Failed, InProgress) | (Failed, Deprecated) => true,
            (Completed, _) | (Deprecated, _) => false,
            (s1, s2) if s1 == s2 => true,
            _ => false,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, GoalStatus::Completed | GoalStatus::Deprecated)
    }

    pub fn is_working(self) -> bool {
        matches!(self, GoalStatus::Ready | GoalStatus::InProgress | GoalStatus::Validating)
    }
}

impl fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Timestamp type alias
pub type Timestamp = DateTime<Utc>;

/// Create a timestamp for the current moment
pub fn now() -> Timestamp {
    Utc::now()
}

/// Blake3 hash wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    pub fn new(bytes: [u8; 32]) -> Self { Self(bytes) }
    pub fn empty() -> Self { Self([0u8; 32]) }
    pub fn to_hex(&self) -> String { hex::encode(self.0) }
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(Self(bytes))
    }
}

impl From<blake3::Hash> for Blake3Hash {
    fn from(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

/// Risultato di una valutazione di allineamento (Calibration Engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentReport {
    pub score: f64,
    pub confidence: f64,
    pub violations: Vec<AlignmentViolation>,
}

/// Una violazione rilevata da Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentViolation {
    pub id: uuid::Uuid,
    pub description: String,
    pub severity: crate::goal_manifold::InvariantSeverity,
    pub is_overridden: bool,
}

/// Registro di un feedback umano (Override)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanOverride {
    pub violation_id: uuid::Uuid,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Identità di un agente AI nel Social Manifold
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Agent {
    pub id: uuid::Uuid,
    pub name: String,
    pub capabilities: Vec<String>,
}

/// Stato di un blocco su un Goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalLock {
    pub agent_id: uuid::Uuid,
    pub locked_at: Timestamp,
    pub expires_at: Option<Timestamp>,
}

/// Probability distribution for metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbabilityDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub distribution_type: DistributionType,
}

impl ProbabilityDistribution {
    pub fn normal(mean: f64, std_dev: f64) -> Self {
        Self {
            mean,
            std_dev,
            min: mean - 2.0 * std_dev,
            max: mean + 2.0 * std_dev,
            distribution_type: DistributionType::Normal,
        }
    }

    pub fn uniform(min: f64, max: f64) -> Self {
        Self {
            mean: (min + max) / 2.0,
            std_dev: (max - min) / (2.0 * 3.0_f64.sqrt()),
            min,
            max,
            distribution_type: DistributionType::Uniform,
        }
    }

    pub fn point(value: f64) -> Self {
        Self {
            mean: value,
            std_dev: 0.0,
            min: value,
            max: value,
            distribution_type: DistributionType::Point,
        }
    }

    pub fn sample(&self) -> f64 {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        match self.distribution_type {
            DistributionType::Normal => {
                let u1: f64 = rng.gen();
                let u2: f64 = rng.gen();
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                self.mean + z0 * self.std_dev
            }
            DistributionType::Uniform => rng.gen_range(self.min..=self.max),
            DistributionType::Point => self.mean,
        }
    }

    pub fn confidence_interval(&self, confidence: f64) -> (f64, f64) {
        match self.distribution_type {
            DistributionType::Normal => {
                let z = if (confidence - 0.95).abs() < 0.01 { 1.96 } else { 1.0 };
                (self.mean - z * self.std_dev, self.mean + z * self.std_dev)
            }
            _ => (self.min, self.max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistributionType { Normal, Uniform, Point }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Comparison {
    #[serde(rename = "==")] Equal,
    #[serde(rename = "!=")] NotEqual,
    #[serde(rename = "<")] LessThan,
    #[serde(rename = "<=")] LessThanOrEqual,
    #[serde(rename = ">")] GreaterThan,
    #[serde(rename = ">=")] GreaterThanOrEqual,
}

impl Comparison {
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
