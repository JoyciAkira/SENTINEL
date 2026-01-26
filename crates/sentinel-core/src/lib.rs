//! Sentinel Core - The cognitive engine for goal-aligned AI coding agents
//!
//! Sentinel Core provides the foundational data structures and algorithms for
//! maintaining perfect alignment between AI agent actions and project goals.
//!
//! # Architecture
//!
//! Sentinel is built on five layers:
//!
//! 1. **Goal Manifold** (`goal_manifold`): Immutable, cryptographically verified goal representation ✅
//! 2. **Alignment Field** (`alignment`): Continuous validation of goal alignment ✅
//! 3. **Cognitive State** (`cognitive_state`): Self-aware execution with meta-cognition ✅
//! 4. **Memory Manifold** (`memory`): Hierarchical infinite-context memory ✅
//! 5. **Meta-Learning** (`learning`): Cross-project learning and improvement 🚧
//!
//! # Quick Start
//!
//! ```
//! use sentinel_core::goal_manifold::{GoalManifold, Intent};
//! use sentinel_core::goal_manifold::goal::Goal;
//! use sentinel_core::goal_manifold::predicate::Predicate;
//! use sentinel_core::types::ProbabilityDistribution;
//!
//! // Define the root intent
//! let intent = Intent::new(
//!     "Build a REST API with user authentication",
//!     vec!["TypeScript", "PostgreSQL", "Test coverage >80%"]
//! );
//!
//! // Create the Goal Manifold
//! let mut manifold = GoalManifold::new(intent);
//!
//! // Add a goal
//! let goal = Goal::builder()
//!     .description("Implement JWT authentication")
//!     .add_success_criterion(Predicate::TestsPassing {
//!         suite: "auth".to_string(),
//!         min_coverage: 0.8,
//!     })
//!     .add_success_criterion(Predicate::FileExists("src/auth/jwt.ts".into()))
//!     .complexity(ProbabilityDistribution::normal(5.0, 1.5))
//!     .value_to_root(0.3)
//!     .build()
//!     .unwrap();
//!
//! manifold.add_goal(goal).unwrap();
//!
//! // Verify integrity
//! assert!(manifold.verify_integrity());
//!
//! // Check completion
//! println!("Progress: {:.1}%", manifold.completion_percentage() * 100.0);
//! ```
//!
//! # Features
//!
//! - **Immutable Goal Tracking**: Cryptographically verified goal manifolds
//! - **Formal Success Criteria**: Predicates that can be automatically verified
//! - **DAG Dependencies**: Sophisticated goal dependency management
//! - **Invariant Validation**: Hard constraints that must never be violated
//! - **Version History**: Complete audit trail of all changes
//!
//! # Design Principles
//!
//! 1. **Correctness over convenience**: We use Rust's type system to prevent invalid states
//! 2. **Immutability by default**: Changes create new versions, preserving history
//! 3. **Formal verification**: All success criteria are formally verifiable predicates
//! 4. **Performance**: Zero-copy operations where possible, efficient algorithms
//! 5. **Testability**: >90% test coverage with property-based testing

#![recursion_limit = "256"]
#![deny(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod alignment;
pub mod architect;
pub mod cognitive_state;
pub mod error;
pub mod external;
pub mod goal_manifold;
pub mod learning;
pub mod memory;
pub mod types;

// Re-export commonly used types for convenience
pub use alignment::{
    AlignmentField, AlignmentVector, MonteCarloSimulator, ProjectState, SimulationResult,
};
pub use cognitive_state::{Action, ActionDecision, CognitiveMode, CognitiveState};
pub use error::{Result, SentinelError};
pub use goal_manifold::{GoalManifold, Intent};
pub use learning::{
    CompletedProject, DeviationPattern, DeviationRisk, KnowledgeBase, LearningReport,
    PatternMiningEngine, Strategy, StrategySynthesizer, SuccessPattern,
};
pub use memory::{MemoryItem, MemoryManifold, MemoryType};
pub use types::{Blake3Hash, GoalStatus, ProbabilityDistribution, Timestamp};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod integration_tests {
    use super::*;
    use goal_manifold::goal::Goal;
    use goal_manifold::predicate::Predicate;

    #[test]
    fn test_end_to_end_workflow() {
        // Create a complete goal manifold workflow
        let intent = Intent::new(
            "Build a task management API",
            vec!["TypeScript", "PostgreSQL", "REST"],
        );

        let mut manifold = GoalManifold::new(intent);

        // Create goals with dependencies
        let setup_goal = Goal::builder()
            .description("Setup project structure")
            .add_success_criterion(Predicate::FileExists("package.json".into()))
            .add_success_criterion(Predicate::FileExists("tsconfig.json".into()))
            .value_to_root(0.05)
            .build()
            .unwrap();

        let db_goal = Goal::builder()
            .description("Setup database schema")
            .add_success_criterion(Predicate::FileExists("migrations/001_init.sql".into()))
            .add_dependency(setup_goal.id)
            .value_to_root(0.15)
            .build()
            .unwrap();

        let api_goal = Goal::builder()
            .description("Implement REST endpoints")
            .add_success_criterion(Predicate::TestsPassing {
                suite: "api".to_string(),
                min_coverage: 0.8,
            })
            .add_dependency(db_goal.id)
            .value_to_root(0.50)
            .build()
            .unwrap();

        let setup_id = setup_goal.id;
        let db_id = db_goal.id;
        let api_id = api_goal.id;

        // Add goals
        manifold.add_goal(setup_goal).unwrap();
        manifold.add_goal(db_goal).unwrap();
        manifold.add_goal(api_goal).unwrap();

        // Verify structure
        assert_eq!(manifold.goal_count(), 3);
        assert!(manifold.verify_integrity());

        // Simulate execution with proper state transitions
        let setup = manifold.get_goal_mut(&setup_id).unwrap();
        setup.mark_ready().unwrap();
        setup.start().unwrap();
        setup.begin_validation().unwrap();
        setup.complete().unwrap();

        // Check progress
        assert!((manifold.completion_percentage() - 0.33).abs() < 0.02);

        // DB goal should now be ready
        assert!(manifold.goal_dag.dependencies_satisfied(db_id));
    }

    #[test]
    fn test_invariant_system() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        // Add critical invariant
        let invariant = goal_manifold::Invariant::critical(
            "No TODO comments in production",
            Predicate::Not(Box::new(Predicate::Custom {
                code: "grep -r TODO src/".to_string(),
                language: goal_manifold::predicate::PredicateLanguage::Shell,
                description: "Check for TODO comments".to_string(),
            })),
        );

        manifold.add_invariant(invariant).unwrap();

        assert_eq!(manifold.invariants.len(), 1);
        assert!(manifold.verify_integrity());
    }

    #[test]
    fn test_version_history() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        // Initial version
        assert_eq!(manifold.current_version(), 1);

        // Add goal - creates version 2
        let goal1 = Goal::builder()
            .description("Goal 1")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();
        manifold.add_goal(goal1).unwrap();
        assert_eq!(manifold.current_version(), 2);

        // Add another goal - creates version 3
        let goal2 = Goal::builder()
            .description("Goal 2")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();
        manifold.add_goal(goal2).unwrap();
        assert_eq!(manifold.current_version(), 3);

        // Check version history
        let history = manifold.version_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
        assert_eq!(history[2].version, 3);
    }
}
pub mod evidence;
pub mod guardrail;
pub mod federation;
