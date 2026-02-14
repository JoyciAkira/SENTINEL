//! # Compaction Policy
//!
//! Defines when and how compaction should be triggered.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for compaction triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Maximum number of turns before triggering compaction
    pub max_turns: u32,

    /// Maximum time duration before triggering compaction
    pub max_duration: Duration,

    /// Maximum memory usage before triggering compaction (in bytes)
    pub max_memory_bytes: usize,

    /// Enable event-based triggering (e.g., after errors, decisions)
    pub enable_event_triggers: bool,

    /// Target compression level
    pub target_compression_level: CompressionTarget,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_turns: 100,
            max_duration: Duration::minutes(30),
            max_memory_bytes: 10 * 1024 * 1024, // 10MB
            enable_event_triggers: true,
            target_compression_level: CompressionTarget::L1,
        }
    }
}

impl CompactionConfig {
    /// Create config for memory-constrained environments
    pub fn memory_constrained() -> Self {
        Self {
            max_turns: 50,
            max_duration: Duration::minutes(15),
            max_memory_bytes: 5 * 1024 * 1024, // 5MB
            enable_event_triggers: true,
            target_compression_level: CompressionTarget::L2,
        }
    }

    /// Create config for standard usage
    pub fn standard() -> Self {
        Self::default()
    }

    /// Create config for maximum quality (minimal compression)
    pub fn high_quality() -> Self {
        Self {
            max_turns: 200,
            max_duration: Duration::hours(1),
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            enable_event_triggers: false,
            target_compression_level: CompressionTarget::L0,
        }
    }
}

/// Target compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionTarget {
    /// No compression
    #[serde(rename = "L0")]
    L0,

    /// Standard compression (~10x)
    #[serde(rename = "L1")]
    L1,

    /// Aggressive compression (~40x)
    #[serde(rename = "L2")]
    L2,
}

impl CompressionTarget {
    /// Convert to compression level from frames module
    pub fn to_level(self) -> crate::memory::compaction::frames::CompressionLevel {
        match self {
            Self::L0 => crate::memory::compaction::frames::CompressionLevel::L0,
            Self::L1 => crate::memory::compaction::frames::CompressionLevel::L1,
            Self::L2 => crate::memory::compaction::frames::CompressionLevel::L2,
        }
    }
}

/// Compaction trigger reason
///
/// Describes why compaction was triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerReason {
    /// Triggered by turn count threshold
    TurnCount { current: u32, threshold: u32 },

    /// Triggered by time duration
    TimeElapsed {
        elapsed: Duration,
        threshold: Duration,
    },

    /// Triggered by memory usage
    MemoryUsage {
        current_bytes: usize,
        threshold_bytes: usize,
    },

    /// Triggered by a specific event
    Event { event_type: CompactionEvent },

    /// Manual trigger
    Manual,
}

/// Events that can trigger compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionEvent {
    /// Error occurred
    Error,

    /// Important decision made
    Decision,

    /// Milestone reached
    Milestone,

    /// Task completed
    TaskComplete,

    /// Context switch
    ContextSwitch,
}

/// Compaction trigger policy
///
/// Evaluates whether compaction should be triggered based on various conditions.
pub struct CompactionTrigger {
    config: CompactionConfig,
    last_compaction_turn: u32,
    last_compaction_time: DateTime<Utc>,
}

impl CompactionTrigger {
    /// Create new trigger policy from config
    pub fn from_config(config: &CompactionConfig) -> Self {
        Self {
            config: config.clone(),
            last_compaction_turn: 0,
            last_compaction_time: Utc::now(),
        }
    }

    /// Check if compaction should be triggered based on turn count
    pub fn should_trigger_turn_count(&self, current_turn: u32) -> bool {
        let turns_since_last = current_turn - self.last_compaction_turn;
        turns_since_last >= self.config.max_turns
    }

    /// Check if compaction should be triggered based on time elapsed
    pub fn should_trigger_time(&self, current_time: DateTime<Utc>) -> bool {
        let elapsed = current_time - self.last_compaction_time;
        elapsed > self.config.max_duration
    }

    /// Check if compaction should be triggered based on memory usage
    pub fn should_trigger_memory(&self, current_memory_bytes: usize) -> bool {
        current_memory_bytes >= self.config.max_memory_bytes
    }

    /// Check if an event should trigger compaction
    pub fn should_trigger_event(&self, event: CompactionEvent) -> bool {
        if !self.config.enable_event_triggers {
            return false;
        }

        // Only certain events trigger immediate compaction
        matches!(event, CompactionEvent::TaskComplete | CompactionEvent::ContextSwitch)
    }

    /// Check if any trigger condition is met
    pub fn should_trigger(
        &self,
        current_turn: u32,
        current_time: DateTime<Utc>,
        memory_bytes: usize,
    ) -> Option<TriggerReason> {
        // Check turn count
        if self.should_trigger_turn_count(current_turn) {
            return Some(TriggerReason::TurnCount {
                current: current_turn,
                threshold: self.config.max_turns,
            });
        }

        // Check time
        if self.should_trigger_time(current_time) {
            return Some(TriggerReason::TimeElapsed {
                elapsed: current_time - self.last_compaction_time,
                threshold: self.config.max_duration,
            });
        }

        // Check memory
        if self.should_trigger_memory(memory_bytes) {
            return Some(TriggerReason::MemoryUsage {
                current_bytes: memory_bytes,
                threshold_bytes: self.config.max_memory_bytes,
            });
        }

        None
    }

    /// Reset trigger state after compaction
    pub fn reset(&mut self, turn: u32, time: DateTime<Utc>) {
        self.last_compaction_turn = turn;
        self.last_compaction_time = time;
    }

    /// Get the target compression level
    pub fn target_level(&self) -> CompressionTarget {
        self.config.target_compression_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CompactionConfig::default();
        assert_eq!(config.max_turns, 100);
        assert_eq!(config.max_memory_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_memory_constrained_config() {
        let config = CompactionConfig::memory_constrained();
        assert_eq!(config.max_turns, 50);
        assert_eq!(config.max_memory_bytes, 5 * 1024 * 1024);
        assert_eq!(config.target_compression_level, CompressionTarget::L2);
    }

    #[test]
    fn test_turn_count_trigger() {
        let config = CompactionConfig::default();
        let trigger = CompactionTrigger::from_config(&config);

        assert!(!trigger.should_trigger_turn_count(50));
        assert!(trigger.should_trigger_turn_count(100));
        assert!(trigger.should_trigger_turn_count(150));
    }

    #[test]
    fn test_time_trigger() {
        let config = CompactionConfig {
            max_duration: Duration::minutes(30),
            ..Default::default()
        };
        let trigger = CompactionTrigger::from_config(&config);

        let now = Utc::now();
        let twenty_min_later = now + Duration::minutes(20);
        let forty_min_later = now + Duration::minutes(40);

        assert!(!trigger.should_trigger_time(twenty_min_later));
        assert!(trigger.should_trigger_time(forty_min_later));
    }

    #[test]
    fn test_memory_trigger() {
        let config = CompactionConfig {
            max_memory_bytes: 10 * 1024 * 1024,
            ..Default::default()
        };
        let trigger = CompactionTrigger::from_config(&config);

        assert!(!trigger.should_trigger_memory(5 * 1024 * 1024));
        assert!(trigger.should_trigger_memory(10 * 1024 * 1024));
        assert!(trigger.should_trigger_memory(15 * 1024 * 1024));
    }

    #[test]
    fn test_event_trigger() {
        let config = CompactionConfig {
            enable_event_triggers: true,
            ..Default::default()
        };
        let trigger = CompactionTrigger::from_config(&config);

        assert!(trigger.should_trigger_event(CompactionEvent::TaskComplete));
        assert!(trigger.should_trigger_event(CompactionEvent::ContextSwitch));
        assert!(!trigger.should_trigger_event(CompactionEvent::Error));
        assert!(!trigger.should_trigger_event(CompactionEvent::Decision));
    }

    #[test]
    fn test_event_triggers_disabled() {
        let config = CompactionConfig {
            enable_event_triggers: false,
            ..Default::default()
        };
        let trigger = CompactionTrigger::from_config(&config);

        assert!(!trigger.should_trigger_event(CompactionEvent::TaskComplete));
        assert!(!trigger.should_trigger_event(CompactionEvent::ContextSwitch));
    }

    #[test]
    fn test_reset() {
        let config = CompactionConfig::default();
        let mut trigger = CompactionTrigger::from_config(&config);

        trigger.reset(100, Utc::now());

        assert_eq!(trigger.last_compaction_turn, 100);
    }
}
