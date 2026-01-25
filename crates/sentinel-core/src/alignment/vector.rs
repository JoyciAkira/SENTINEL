//! Alignment Vector - Result of alignment computation
//!
//! This module defines the AlignmentVector type, which represents
//! the result of computing alignment between current state and goals.

use serde::{Deserialize, Serialize};

/// Result of alignment computation
///
/// The AlignmentVector tells us:
/// 1. WHERE we are (current alignment score)
/// 2. WHICH DIRECTION to go (gradient)
/// 3. HOW FAR we are from optimal (deviation magnitude)
/// 4. WHAT MIGHT HAPPEN (probabilistic future)
///
/// # Examples
///
/// ```
/// use sentinel_core::alignment::vector::AlignmentVector;
/// use sentinel_core::alignment::vector::Vector;
///
/// let alignment = AlignmentVector {
///     score: 85.0,
///     goal_contribution: Vector::new(vec![0.5, 0.3, 0.2]),
///     deviation_magnitude: 0.15,
///     entropy_gradient: -0.05,
///     confidence: 0.92,
/// };
///
/// assert!(alignment.score > 80.0); // Good alignment
/// assert!(alignment.is_well_aligned());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentVector {
    /// Overall alignment score (0-100)
    ///
    /// Interpretation:
    /// - 90-100: Excellent - strongly aligned
    /// - 75-90: Good - aligned
    /// - 60-75: Acceptable - somewhat aligned
    /// - 40-60: Concerning - may be off-track
    /// - 20-40: Deviation - correction needed
    /// - 0-20: Critical - severe deviation
    pub score: f64,

    /// Gradient vector pointing toward goal
    ///
    /// This is the direction of steepest ascent in the alignment field.
    /// Following this direction increases alignment fastest.
    pub goal_contribution: Vector,

    /// Distance from optimal path (0.0-1.0)
    ///
    /// 0.0 = on optimal path
    /// 1.0 = maximally deviated
    pub deviation_magnitude: f64,

    /// Rate of change of uncertainty
    ///
    /// Negative = uncertainty decreasing (good)
    /// Positive = uncertainty increasing (bad)
    pub entropy_gradient: f64,

    /// Confidence in this alignment assessment (0.0-1.0)
    pub confidence: f64,
}

impl AlignmentVector {
    /// Create a new alignment vector
    pub fn new(score: f64) -> Self {
        Self {
            score,
            goal_contribution: Vector::zero(1),
            deviation_magnitude: 0.0,
            entropy_gradient: 0.0,
            confidence: 1.0,
        }
    }

    /// Check if alignment is excellent (>= 90)
    pub fn is_excellent(&self) -> bool {
        self.score >= 90.0
    }

    /// Check if alignment is good (>= 75)
    pub fn is_good(&self) -> bool {
        self.score >= 75.0
    }

    /// Check if alignment is acceptable (>= 60)
    pub fn is_acceptable(&self) -> bool {
        self.score >= 60.0
    }

    /// Check if deviation detected (< 60)
    pub fn is_deviating(&self) -> bool {
        self.score < 60.0
    }

    /// Check if critical deviation (< 20)
    pub fn is_critical(&self) -> bool {
        self.score < 20.0
    }

    /// Check if well aligned (>= 75)
    pub fn is_well_aligned(&self) -> bool {
        self.is_good()
    }

    /// Get severity level
    pub fn severity(&self) -> AlignmentSeverity {
        match self.score {
            s if s >= 90.0 => AlignmentSeverity::Excellent,
            s if s >= 75.0 => AlignmentSeverity::Good,
            s if s >= 60.0 => AlignmentSeverity::Acceptable,
            s if s >= 40.0 => AlignmentSeverity::Concerning,
            s if s >= 20.0 => AlignmentSeverity::Deviation,
            _ => AlignmentSeverity::Critical,
        }
    }

    /// Get trend based on entropy gradient
    pub fn trend(&self) -> AlignmentTrend {
        if self.entropy_gradient.abs() < 0.01 {
            AlignmentTrend::Stable
        } else if self.entropy_gradient < 0.0 {
            AlignmentTrend::Improving
        } else {
            AlignmentTrend::Degrading
        }
    }
}

/// Severity of alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignmentSeverity {
    /// 90-100: Excellent alignment
    Excellent,

    /// 75-90: Good alignment
    Good,

    /// 60-75: Acceptable alignment
    Acceptable,

    /// 40-60: Concerning - may be off-track
    Concerning,

    /// 20-40: Deviation - correction needed
    Deviation,

    /// 0-20: Critical deviation
    Critical,
}

/// Trend of alignment over time
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlignmentTrend {
    /// Alignment improving (entropy decreasing)
    Improving,

    /// Alignment stable
    Stable,

    /// Alignment degrading (entropy increasing)
    Degrading,
}

/// Mathematical vector for gradient computations
///
/// Represents a direction and magnitude in state space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector {
    /// Vector components
    pub components: Vec<f64>,
}

impl Vector {
    /// Create a new vector
    pub fn new(components: Vec<f64>) -> Self {
        Self { components }
    }

    /// Create a zero vector of given dimension
    pub fn zero(dimension: usize) -> Self {
        Self {
            components: vec![0.0; dimension],
        }
    }

    /// Get the dimension of this vector
    pub fn dimension(&self) -> usize {
        self.components.len()
    }

    /// Compute magnitude (L2 norm)
    pub fn magnitude(&self) -> f64 {
        self.components.iter().map(|&x| x * x).sum::<f64>().sqrt()
    }

    /// Normalize to unit vector
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag < f64::EPSILON {
            return Self::zero(self.dimension());
        }

        Self {
            components: self.components.iter().map(|&x| x / mag).collect(),
        }
    }

    /// Dot product with another vector
    pub fn dot(&self, other: &Vector) -> f64 {
        assert_eq!(
            self.dimension(),
            other.dimension(),
            "Vectors must have same dimension"
        );

        self.components
            .iter()
            .zip(&other.components)
            .map(|(&a, &b)| a * b)
            .sum()
    }

    /// Add two vectors
    pub fn add(&self, other: &Vector) -> Self {
        assert_eq!(self.dimension(), other.dimension());

        Self {
            components: self
                .components
                .iter()
                .zip(&other.components)
                .map(|(&a, &b)| a + b)
                .collect(),
        }
    }

    /// Scalar multiplication
    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            components: self.components.iter().map(|&x| x * scalar).collect(),
        }
    }

    /// Get component at index
    pub fn get(&self, index: usize) -> Option<f64> {
        self.components.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_vector_severity() {
        let excellent = AlignmentVector::new(95.0);
        assert!(excellent.is_excellent());
        assert_eq!(excellent.severity(), AlignmentSeverity::Excellent);

        let good = AlignmentVector::new(80.0);
        assert!(good.is_good());
        assert!(!good.is_excellent());
        assert_eq!(good.severity(), AlignmentSeverity::Good);

        let deviating = AlignmentVector::new(50.0);
        assert!(deviating.is_deviating());
        assert_eq!(deviating.severity(), AlignmentSeverity::Concerning);

        let critical = AlignmentVector::new(15.0);
        assert!(critical.is_critical());
        assert_eq!(critical.severity(), AlignmentSeverity::Critical);
    }

    #[test]
    fn test_alignment_trend() {
        let mut alignment = AlignmentVector::new(80.0);

        alignment.entropy_gradient = -0.1;
        assert_eq!(alignment.trend(), AlignmentTrend::Improving);

        alignment.entropy_gradient = 0.1;
        assert_eq!(alignment.trend(), AlignmentTrend::Degrading);

        alignment.entropy_gradient = 0.0;
        assert_eq!(alignment.trend(), AlignmentTrend::Stable);
    }

    #[test]
    fn test_vector_operations() {
        let v1 = Vector::new(vec![1.0, 2.0, 3.0]);
        let v2 = Vector::new(vec![4.0, 5.0, 6.0]);

        // Magnitude
        assert_eq!(v1.magnitude(), (14.0_f64).sqrt());

        // Dot product
        assert_eq!(v1.dot(&v2), 32.0);

        // Add
        let v3 = v1.add(&v2);
        assert_eq!(v3.components, vec![5.0, 7.0, 9.0]);

        // Scale
        let v4 = v1.scale(2.0);
        assert_eq!(v4.components, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_vector_normalize() {
        let v = Vector::new(vec![3.0, 4.0]);
        let normalized = v.normalize();

        assert!((normalized.magnitude() - 1.0).abs() < f64::EPSILON);
        assert!((normalized.components[0] - 0.6).abs() < 0.001);
        assert!((normalized.components[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_zero_vector() {
        let v = Vector::zero(5);
        assert_eq!(v.dimension(), 5);
        assert_eq!(v.magnitude(), 0.0);
    }
}
