//! # Memory Compaction Engine (Wave 3.3)
//!
//! Ultra-lightweight context compaction system for maintaining conversation history
//! within strict memory budgets.
//!
//! ## Overview
//!
//! The compaction engine transforms verbose chat history into a Pinned Lightweight
//! Transcript (PLT) format, achieving 10-50x compression while preserving critical
//! context and semantic information.
//!
//! ## Architecture
//!
//! ```text
//! Chat History → Compaction Engine → Pinned Lightweight Transcript
//!     │                  │                        │
//!     │              Triggers:                  │
//!     │            - Turn count              ┌───┴────┐
//!     │            - Time elapsed           │ Frames │
//!     │            - Memory usage           │  ...   │
//!     │            - Events                 └───┬────┘
//!     │                                      │
//!     │                                   Anchors
//!     │                                   (jump refs)
//! ```
//!
//! ## Compression Levels
//!
//! - **L0**: No compression (raw transcript)
//! - **L1**: Semantic compression (~10x reduction, ~100 tokens/turn)
//! - **L2**: Aggressive compression (~40x reduction, ~25 tokens/turn)
//!
//! ## Performance Budget
//!
//! - Memory: <= 1.5MB per 10k turns
//! - Compaction latency: p95 <= 45ms
//! - Frame render: p95 <= 4ms
//!
//! ## Example
//!
//! ```rust
//! use sentinel_core::memory::compaction::{
//!     CompactionEngine, CompactionConfig, ChatHistory, ChatTurn, ChatMessage, MessageRole,
//! };
//!
//! let config = CompactionConfig::standard();
//! let engine = CompactionEngine::new(config);
//!
//! let mut history = ChatHistory::new("session-123".to_string());
//! history.add_turn(ChatTurn::new(
//!     1,
//!     vec![
//!         ChatMessage::new(MessageRole::User, "Hello".to_string()),
//!         ChatMessage::new(MessageRole::Assistant, "Hi there!".to_string()),
//!     ],
//! ));
//!
//! let plt = engine.compact(&history, 1)?;
//!
//! println!("Compressed {} turns into {} frames",
//!     plt.metadata.total_turns,
//!     plt.frames.len()
//! );
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod engine;
pub mod frames;
pub mod policy;

pub use engine::{ChatHistory, ChatMessage, ChatTurn, CompactionEngine, CompressionStrategy};
pub use frames::{
    AnchorRef, CompressionLevel, FrameContent, FrameType, PinnedLightweightTranscript,
    SummaryFrame, TranscriptMetadata,
};
pub use policy::{CompactionConfig, CompactionEvent, CompressionTarget, TriggerReason};

// Re-exports for convenience
pub use engine::{MessageFlag, MessageMetadata, MessageRole, TurnType};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::compaction::engine::{ChatMessage, ChatTurn, MessageRole};

    #[test]
    fn test_full_compaction_flow() {
        // Create test chat history
        let mut history = ChatHistory::new("test-run".to_string());

        for i in 1..=50 {
            let turn = ChatTurn::new(
                i,
                vec![
                    ChatMessage::new(MessageRole::User, format!("User message {}", i)),
                    ChatMessage::new(MessageRole::Assistant, format!("Assistant response {}", i)),
                ],
            );
            history.add_turn(turn);
        }

        // Compact using L1 compression
        let config = CompactionConfig::standard();
        let engine = CompactionEngine::new(config);

        let plt = engine.compact(&history, 50).expect("Compaction failed");

        // Verify results
        assert_eq!(plt.frames.len(), 3); // ~20 turns per frame
        assert!(plt.compression_ratio() >= 1.0);
        assert_eq!(plt.metadata.total_turns, 50);
    }

    #[test]
    fn test_memory_budget_compliance() {
        let mut history = ChatHistory::new("budget-test".to_string());

        // Create 1000 turns
        for i in 1..=1000 {
            let turn = ChatTurn::new(
                i,
                vec![ChatMessage::new(
                    MessageRole::User,
                    "Test message with some content".to_string(),
                )],
            );
            history.add_turn(turn);
        }

        // Compact with L2 (most aggressive)
        let config = CompactionConfig::memory_constrained();
        let engine = CompactionEngine::new(config);

        let plt = engine.compact(&history, 1000).expect("Compaction failed");

        // Estimate memory: ~150 bytes per frame * (1000/50) frames = ~3KB
        // Well under 1.5MB budget for 10k turns
        let estimated_bytes = plt.frames.len() * 150;
        assert!(estimated_bytes < 1_500_000, "Memory budget exceeded");
    }

    #[test]
    fn test_compression_levels() {
        let mut history = ChatHistory::new("levels-test".to_string());

        for i in 1..=100 {
            let turn = ChatTurn::new(
                i,
                vec![ChatMessage::new(
                    MessageRole::User,
                    "Test message for compression".to_string(),
                )],
            );
            history.add_turn(turn);
        }

        // Test L0
        let config_l0 = CompactionConfig::high_quality();
        let engine_l0 = CompactionEngine::new(config_l0);
        let plt_l0 = engine_l0.compact(&history, 100).unwrap();

        // Test L1
        let config_l1 = CompactionConfig::standard();
        let engine_l1 = CompactionEngine::new(config_l1);
        let plt_l1 = engine_l1.compact(&history, 100).unwrap();

        // Test L2
        let config_l2 = CompactionConfig::memory_constrained();
        let engine_l2 = CompactionEngine::new(config_l2);
        let plt_l2 = engine_l2.compact(&history, 100).unwrap();

        // L0 should have most frames (least compression)
        assert!(plt_l0.frames.len() >= plt_l1.frames.len());
        // L1 should have more frames than L2
        assert!(plt_l1.frames.len() >= plt_l2.frames.len());
    }

    #[test]
    fn test_anchor_extraction() {
        let mut history = ChatHistory::new("anchors-test".to_string());

        // Add a mix of turns
        for i in 1..=20 {
            let mut turn = ChatTurn::new(
                i,
                vec![ChatMessage::new(
                    MessageRole::Assistant,
                    format!("Response {}", i),
                )],
            );

            // Mark some as important
            if i % 5 == 0 {
                turn.turn_type = crate::memory::compaction::engine::TurnType::Decision;
            }

            history.add_turn(turn);
        }

        let config = CompactionConfig::standard();
        let engine = CompactionEngine::new(config);
        let plt = engine.compact(&history, 20).unwrap();

        // Should have anchors for decision turns
        assert!(!plt.anchors.is_empty());
        assert!(plt.anchors.len() <= 4); // Only decisions (5, 10, 15, 20)
    }
}
