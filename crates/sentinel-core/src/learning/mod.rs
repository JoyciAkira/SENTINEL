//! Meta-Learning Engine
//!
//! This module implements Sentinel's self-improving capabilities.
//! It learns from completed projects and uses that knowledge to improve
//! future performance.

pub mod classifier;
pub mod knowledge_base;
pub mod pattern_mining;
pub mod strategy;
pub mod types;

// Re-exports
pub use classifier::DeviationClassifier;
pub use knowledge_base::KnowledgeBase;
pub use pattern_mining::PatternMiningEngine;
pub use strategy::StrategySynthesizer;
pub use types::*;
