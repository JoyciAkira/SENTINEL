# sentinel-core

[![Crates.io](https://img.shields.io/crates/v/sentinel-core.svg)](https://crates.io/crates/sentinel-core)
[![Documentation](https://docs.rs/sentinel-core/badge.svg)](https://docs.rs/sentinel-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Core engine for Sentinel - the cognitive operating system for goal-aligned AI coding agents.

## Features

- **Immutable Goal Tracking**: Cryptographically verified goal manifolds with Blake3
- **Predictive Deviation Detection**: Monte Carlo simulation predicts deviations before they happen
- **Continuous Alignment Validation**: Real-time measurement of goal alignment (0-100 score)
- **Gradient Computation**: Mathematical gradients for intelligent auto-correction
- **Formal Success Criteria**: Predicates that can be automatically verified
- **DAG Dependencies**: Sophisticated goal dependency management with cycle prevention
- **Type Safety**: Rust's type system prevents invalid states at compile time
- **High Performance**: Sub-millisecond operations, <300ms full validation
- **Well Tested**: 76/76 tests passing with comprehensive coverage

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sentinel-core = "0.1.0"
```

## Quick Example

```rust
use sentinel_core::goal_manifold::{GoalManifold, Intent};
use sentinel_core::goal_manifold::goal::Goal;
use sentinel_core::goal_manifold::predicate::Predicate;
use sentinel_core::alignment::{AlignmentField, ProjectState};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> sentinel_core::error::Result<()> {
    // Layer 1: Create a goal manifold
    let intent = Intent::new(
        "Build authentication system",
        vec!["TypeScript", "JWT", "PostgreSQL"]
    );

    let mut manifold = GoalManifold::new(intent);

    // Add goals with formal success criteria
    let goal = Goal::builder()
        .description("Implement JWT token generation")
        .add_success_criterion(Predicate::TestsPassing {
            suite: "auth".to_string(),
            min_coverage: 0.8,
        })
        .value_to_root(0.4)
        .build()?;

    manifold.add_goal(goal)?;

    // Verify integrity
    assert!(manifold.verify_integrity());

    // Layer 2: Create alignment field for continuous validation
    let field = AlignmentField::new(manifold);
    let state = ProjectState::new(PathBuf::from("."));

    // Compute current alignment
    let alignment = field.compute_alignment(&state).await?;
    println!("Alignment score: {:.1}/100", alignment.score);
    println!("Status: {:?}", alignment.severity());

    // Predict future deviation
    let prediction = field.predict_alignment(&state).await?;
    if prediction.will_likely_deviate() {
        println!("⚠️  WARNING: Action likely to cause deviation!");
        println!("   Risk: {:?}", prediction.risk_level());
    }

    Ok(())
}
```

## Modules

- **goal_manifold**: Core goal manifold data structures (Layer 1)
  - `Goal`: Individual objectives with success criteria
  - `GoalManifold`: Immutable collection of goals with integrity verification
  - `predicate`: Formal predicates for success criteria
  - `dag`: Directed Acyclic Graph for goal dependencies

- **alignment**: Continuous validation system (Layer 2)
  - `AlignmentField`: Main alignment computation engine
  - `ProjectState`: Multi-dimensional state representation
  - `AlignmentVector`: Alignment scores with severity/trend analysis
  - `MonteCarloSimulator`: Predictive deviation detection

- **types**: Shared types
  - `GoalStatus`: Execution status with validated transitions
  - `Blake3Hash`: Type-safe cryptographic hashes
  - `ProbabilityDistribution`: Uncertainty representation

- **error**: Error types
  - `SentinelError`: Top-level error type
  - `GoalError`, `DagError`, `PredicateError`: Specific error types

## Design Principles

1. **Correctness over convenience**: Use Rust's type system to prevent bugs
2. **Immutability**: Changes create new versions, preserving history
3. **Formal verification**: All success criteria are formally verifiable
4. **Performance**: Efficient algorithms, zero-copy where possible
5. **Testability**: Comprehensive test suite with property-based testing

## Performance

On Apple M1 Pro:

**Layer 1 (Goal Manifold):**
- Goal evaluation: <1ms for 100 goals
- DAG operations: <10ms for 1000 nodes
- Integrity verification: <5ms (Blake3)
- Topological sort: <20ms for 1000 goals

**Layer 2 (Alignment Field):**
- Alignment computation: <5ms for 100 goals
- Gradient computation: <50ms (10 dimensions)
- Monte Carlo simulation: <200ms (1000 iterations)
- Full validation cycle: <300ms

## Safety

This crate enforces:

- No `unsafe` code
- Strict type checking
- Validated state transitions
- Comprehensive error handling
- Immutable by default

## License

MIT
