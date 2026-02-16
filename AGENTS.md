# Agent Guidelines for SENTINEL

This file provides guidelines for AI coding agents working on the SENTINEL project.

## Project Overview

SENTINEL is a goal-aligned AI coding agent system with a Rust core, TypeScript SDKs, and Python SDK. It implements a 10-layer cognitive architecture for preventing goal drift in AI agents.

## Build Commands

### Rust (Primary)
```bash
# Build entire workspace
cargo build

# Build in release mode
cargo build --release

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p sentinel-core
cargo test -p sentinel-cli

# Run a specific test
cargo test test_name_here

# Check code without building
cargo check

# Run with all features
cargo test --all-features

# Format code (uses rustfmt)
cargo fmt

# Lint with Clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### TypeScript (SDKs and VSCode Extension)
```bash
# SDK (sdks/typescript/)
cd sdks/typescript
npm run build      # Compile TypeScript
npm run test       # Run Jest tests
npm run lint       # Run ESLint
npm run format     # Run Prettier

# VSCode Extension (integrations/vscode/)
cd integrations/vscode
npm run build              # Build extension
npm run compile            # Compile with esbuild
npm run watch              # Watch mode
npm run quality:webview    # Full quality check
```

### Python (SDK)
```bash
cd sdks/python
pip install -e ".[dev]"

# Run tests
pytest
pytest tests/test_specific.py

# Code quality
black sentinel_sdk tests
isort sentinel_sdk tests
ruff check sentinel_sdk tests
mypy sentinel_sdk
```

## Code Style Guidelines

### Rust

#### Formatting
- **Indentation**: 4 spaces (default rustfmt)
- **Line length**: 100 characters maximum
- **Trailing commas**: Always use trailing commas in multi-line structures
- Run `cargo fmt` before committing

#### Naming Conventions
- **Types**: PascalCase (`GoalManifold`, `SentinelError`)
- **Functions/Methods**: snake_case (`compute_hash`, `add_goal`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_RETRY_COUNT`)
- **Modules**: snake_case (`goal_manifold`, `cognitive_state`)
- **Generic parameters**: Single uppercase letters (`T`, `K`, `V`)

#### Imports
```rust
// Group imports: std, external, internal (alphabetically within groups)
use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::types::{Blake3Hash, Timestamp};
```

#### Error Handling
- Use `thiserror` for error enums with structured variants
- Use `anyhow` for quick error propagation
- Always use `?` operator for error propagation
- Provide context with `.context()` from anyhow
```rust
pub fn add_goal(&mut self, goal: Goal) -> Result<()> {
    goal.validate().context("Goal validation failed")?;
    // ...
}
```

#### Types
- Use strong types over primitives (e.g., `Blake3Hash` instead of `[u8; 32]`)
- Use `#[derive(Debug, Clone)]` for most structs
- Use `#[serde(rename_all = "snake_case")]` for JSON compatibility
- Prefer `impl Into<String>` for string parameters

#### Documentation
- All public items must have doc comments (`///`)
- Include examples in doc comments
- Document panics, errors, and safety invariants
```rust
/// Create a new Goal Manifold
///
/// # Examples
///
/// ```
/// use sentinel_core::goal_manifold::{GoalManifold, Intent};
///
/// let manifold = GoalManifold::new(Intent::new("Build API", vec!["TypeScript"]));
/// ```
pub fn new(root_intent: Intent) -> Self { ... }
```

#### Testing
- Tests go in `#[cfg(test)]` module at the bottom of each file
- Name tests descriptively: `test_<what>_<condition>_<expected>`
- Use `assert!`, `assert_eq!`, `assert_ne!` appropriately
- Test both success and error cases
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifold_integrity_hash_changes_on_modification() {
        let mut manifold = create_test_manifold();
        let initial_hash = manifold.integrity_hash;
        manifold.add_goal(test_goal()).unwrap();
        assert_ne!(manifold.integrity_hash, initial_hash);
    }
}
```

### TypeScript

#### Formatting
- **Indentation**: 2 spaces
- **Line length**: 100 characters
- **Quotes**: Double quotes for strings
- **Semicolons**: Required

#### Naming Conventions
- **Types/Interfaces**: PascalCase (`SentinelClient`, `GoalConfig`)
- **Functions/Variables**: camelCase (`computeScore`, `goalCount`)
- **Constants**: UPPER_SNAKE_CASE or camelCase for module-level
- **Private members**: Leading underscore (`_internalMethod`)

#### Types
- Always use strict TypeScript (`strict: true` in tsconfig)
- Prefer interfaces over type aliases for objects
- Use explicit return types on public functions
- Avoid `any` - use `unknown` if type is truly unknown

#### Error Handling
- Use custom error classes extending Error
- Include context in error messages
- Use try/catch with specific error types

### Python

#### Formatting
- **Line length**: 100 characters (configured in pyproject.toml)
- Use Black formatter
- Use isort for import sorting (black profile)

#### Naming Conventions
- **Classes**: PascalCase (`SentinelClient`)
- **Functions/Variables**: snake_case (`compute_score`)
- **Constants**: UPPER_SNAKE_CASE
- **Private**: Leading underscore (`_internal_method`)

#### Type Hints
- Use type hints everywhere (mypy strict mode enabled)
- Use `from __future__ import annotations` for Python 3.9+
- Use `Optional[Type]` or `Type | None` (Python 3.10+)

#### Error Handling
- Use custom exception classes
- Chain exceptions with `raise ... from ...`
- Use context managers for resources

## Project Structure

```
sentinel/
├── crates/
│   ├── sentinel-core/       # Core library (Goal Manifold, Alignment, etc.)
│   ├── sentinel-cli/        # CLI application
│   ├── sentinel-agent-native/  # Native agent implementation
│   └── sentinel-sandbox/    # Sandbox for code execution
├── sdks/
│   ├── typescript/          # TypeScript SDK
│   └── python/              # Python SDK
├── integrations/
│   └── vscode/              # VSCode extension
└── docs/                    # Documentation
```

## Critical Invariants

1. **Goal Manifold Integrity**: All changes must update the Blake3 hash and version history
2. **Immutability**: Root intent is append-only; modifications create new versions
3. **Safety**: Use `#![deny(unsafe_code)]` - no unsafe blocks allowed
4. **Test Coverage**: Target >90% test coverage for new code
5. **Documentation**: All public APIs must be documented

## Common Patterns

### Builder Pattern (Rust)
```rust
let goal = Goal::builder()
    .description("Implement auth")
    .add_success_criterion(Predicate::FileExists("auth.rs".into()))
    .value_to_root(0.3)
    .build()?;
```

### Error Propagation
```rust
use crate::error::{Result, ResultExt};

operation().context("Operation failed")?;
```

### State Transitions
```rust
// Always use state machine methods for transitions
goal.mark_ready()?;
goal.start()?;
goal.complete()?;
```

## Pre-commit Checklist

- [ ] Run `cargo fmt` and `cargo clippy`
- [ ] All tests pass (`cargo test --all`)
- [ ] New code has tests
- [ ] Public APIs are documented
- [ ] No compiler warnings
- [ ] CHANGELOG.md updated for user-facing changes

## Resources

- Architecture: See `CLAUDE.md` for detailed specification
- Protocol: See `PROTOCOL.md` for API specifications
- Design Decisions: See `adr/` directory
