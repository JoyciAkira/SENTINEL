//! Meta-Learning Engine
//!
//! This module implements Sentinel's self-improving capabilities.
//! It learns from completed projects and uses that knowledge to improve
//! future performance.

pub mod knowledge_base;
pub mod pattern_mining;
pub mod types;

// Re-exports
pub use knowledge_base::KnowledgeBase;
pub use pattern_mining::PatternMiningEngine;
pub use types::*;
