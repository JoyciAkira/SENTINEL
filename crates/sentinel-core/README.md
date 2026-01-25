# sentinel-core

[![Crates.io](https://img.shields.io/crates/v/sentinel-core.svg)](https://crates.io/crates/sentinel-core)
[![Documentation](https://docs.rs/sentinel-core/badge.svg)](https://docs.rs/sentinel-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Core engine for Sentinel - the cognitive operating system for goal-aligned AI coding agents.

## Features

- **Immutable Goal Tracking**: Cryptographically verified goal manifolds with Blake3
- **Formal Success Criteria**: Predicates that can be automatically verified
- **DAG Dependencies**: Sophisticated goal dependency management with cycle prevention
- **Type Safety**: Rust's type system prevents invalid states at compile time
- **High Performance**: Zero-copy operations, efficient algorithms
- **Well Tested**: 43/43 tests passing with comprehensive coverage

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

// Create a goal manifold
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
```

## Modules

- **goal_manifold**: Core goal manifold data structures
  - `Goal`: Individual objectives with success criteria
  - `GoalManifold`: Immutable collection of goals with integrity verification
  - `predicate`: Formal predicates for success criteria
  - `dag`: Directed Acyclic Graph for goal dependencies

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

- Goal evaluation: <1ms for 100 goals
- DAG operations: <10ms for 1000 nodes
- Integrity verification: <5ms (Blake3)
- Topological sort: <20ms for 1000 goals

## Safety

This crate enforces:

- No `unsafe` code
- Strict type checking
- Validated state transitions
- Comprehensive error handling
- Immutable by default

## License

MIT
