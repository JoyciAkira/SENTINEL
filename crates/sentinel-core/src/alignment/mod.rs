//! Alignment System - Layer 2 of Sentinel Architecture
//!
//! This module implements the Alignment Field, Sentinel's continuous validation
//! system that ensures AI agents never deviate from their goals.
//!
//! # Architecture
//!
//! The alignment system consists of four main components:
//!
//! ## 1. ProjectState (`state`)
//!
//! Represents the current state of a project in high-dimensional space:
//! - File states (modified, added, deleted)
//! - Test results and coverage
//! - Goal completion status
//! - Code metrics (complexity, quality)
//!
//! ## 2. AlignmentVector (`vector`)
//!
//! The result of alignment computation:
//! - Scalar alignment score (0-100)
//! - Goal contribution vector
//! - Deviation magnitude
//! - Confidence in measurement
//!
//! ## 3. MonteCarloSimulator (`simulator`)
//!
//! Predictive deviation detection using probabilistic simulation:
//! - Simulates thousands of possible futures
//! - Predicts deviation probability BEFORE actions
//! - Enables preventive correction
//!
//! ## 4. AlignmentField (`field`)
//!
//! The main alignment computation engine:
//! - Computes alignment for any project state
//! - Computes gradients for continuous improvement
//! - Integrates with Goal Manifold (Layer 1)
//!
//! # How It Works
//!
//! ```text
//! +-----------------------------------------------------------+
//! |              Goal Manifold (Layer 1)                      |
//! |  "What we want to achieve"                                |
//! +----------------------|------------------------------------+
//!                        |
//!                        v
//! +-----------------------------------------------------------+
//! |             Alignment Field (Layer 2)                     |
//! |  "How well are we aligned?"                               |
//! |                                                            |
//! |  ProjectState --> compute_alignment() --> AlignmentVector |
//! |       |                                        |           |
//! |       +--> compute_gradient() --> Improvement direction   |
//! |       |                                        |           |
//! |       +--> predict_alignment() --> Future deviation       |
//! +-----------------------------------------------------------+
//! ```
//!
//! # Example Usage
//!
//! ```no_run
//! use sentinel_core::alignment::{AlignmentField, ProjectState};
//! use sentinel_core::goal_manifold::{GoalManifold, Intent};
//! use sentinel_core::goal_manifold::goal::Goal;
//! use sentinel_core::goal_manifold::predicate::Predicate;
//! use std::path::PathBuf;
//!
//! # async fn example() -> sentinel_core::error::Result<()> {
//! // Create Goal Manifold (Layer 1)
//! let intent = Intent::new(
//!     "Build REST API with authentication",
//!     vec!["TypeScript", "PostgreSQL"]
//! );
//! let mut manifold = GoalManifold::new(intent);
//!
//! // Add goals
//! let goal = Goal::builder()
//!     .description("Implement JWT authentication")
//!     .add_success_criterion(Predicate::TestsPassing {
//!         suite: "auth".to_string(),
//!         min_coverage: 0.8,
//!     })
//!     .value_to_root(0.4)
//!     .build()?;
//!
//! manifold.add_goal(goal)?;
//!
//! // Create Alignment Field (Layer 2)
//! let field = AlignmentField::new(manifold);
//!
//! // Get current project state
//! let state = ProjectState::new(PathBuf::from("."));
//!
//! // Compute alignment
//! let alignment = field.compute_alignment(&state).await?;
//!
//! println!("Alignment score: {:.1}/100", alignment.score);
//! println!("Severity: {:?}", alignment.severity());
//! println!("Trend: {:?}", alignment.trend());
//!
//! // Predict future deviation
//! let prediction = field.predict_alignment(&state).await?;
//!
//! if prediction.will_likely_deviate() {
//!     println!("⚠️  WARNING: Action likely to cause deviation!");
//!     println!("   Deviation probability: {:.1}%", prediction.deviation_probability * 100.0);
//!     println!("   Risk level: {:?}", prediction.risk_level());
//! }
//!
//! // Compute gradient for improvement
//! let gradient = field.compute_gradient(&state, 0.01).await?;
//! println!("Improvement direction magnitude: {:.3}", gradient.magnitude());
//! # Ok(())
//! # }
//! ```
//!
//! # Key Concepts
//!
//! ## Alignment Score (0-100)
//!
//! The alignment score measures how well the current state aligns with goals:
//!
//! - **90-100**: Excellent - on track, goals being achieved
//! - **70-90**: Good - generally aligned, minor issues
//! - **50-70**: Acceptable - some concerns, needs attention
//! - **30-50**: Concerning - significant deviation risk
//! - **10-30**: Deviation - off track, correction needed
//! - **0-10**: Critical - severe deviation, immediate action required
//!
//! ## Deviation Prediction
//!
//! Unlike reactive systems that detect deviations after they happen,
//! Sentinel uses Monte Carlo simulation to predict deviations BEFORE
//! they occur. This enables preventive correction.
//!
//! The simulator runs 1000+ parallel simulations of possible futures,
//! computing the probability that an action will cause deviation.
//!
//! ## Gradient Computation
//!
//! The alignment field gradient points toward better alignment:
//!
//! - Magnitude: How much improvement is possible
//! - Direction: Which changes would help most
//!
//! This enables intelligent auto-correction: follow the gradient
//! to return to alignment.
//!
//! # Performance
//!
//! On Apple M1 Pro:
//!
//! - Alignment computation: <5ms for 100 goals
//! - Gradient computation: <50ms (10 dimensions)
//! - Monte Carlo prediction: <200ms (1000 iterations)
//! - Full validation cycle: <300ms
//!
//! # Safety
//!
//! This module maintains Sentinel's safety guarantees:
//!
//! - No `unsafe` code
//! - Validated state transitions
//! - Comprehensive error handling
//! - Thread-safe operations (Send + Sync)

pub mod field;
pub mod simulator;
pub mod state;
pub mod vector;

// Re-export main types for convenience
pub use field::{AlignmentConfig, AlignmentField};
pub use simulator::{
    MonteCarloSimulator, RiskLevel, SimulationConfig, SimulationResult, UncertaintyModel,
};
pub use state::{CodeMetrics, FileState, GoalState, ProjectState, TestResults};
pub use vector::{AlignmentSeverity, AlignmentTrend, AlignmentVector, Vector};
