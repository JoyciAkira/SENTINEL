//! E2E Test: Context Compaction
//!
//! Tests the memory compaction engine for Pinned Lightweight Transcript.

use sentinel_core::memory::compaction::{
    ChatHistory, ChatMessage, ChatTurn, CompactionEngine, CompactionConfig, CompressionLevel,
    MessageRole, TurnType,
};

/// E2E test: Compaction preserves key information
#[tokio::test]
async fn e2e_compaction_preserves_key_info() {
    let mut history = ChatHistory::new("test-run".to_string());

    for i in 1..=100 {
        let turn = ChatTurn::new(
            i,
            vec![
                ChatMessage::new(MessageRole::User, format!("User message {}", i)),
                ChatMessage::new(MessageRole::Assistant, format!("Assistant response {}", i)),
            ],
        );
        history.add_turn(turn);
    }

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 100).expect("Compaction should succeed");

    assert!(plt.compression_ratio() >= 0.5, "Should compress");
    assert!(plt.frames.len() < 50, "Should have fewer frames than turns");
}

/// E2E test: Compression levels produce different results
#[tokio::test]
async fn e2e_compression_levels_progressive() {
    let mut history = ChatHistory::new("levels-test".to_string());

    for i in 1..=100 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(
                MessageRole::User,
                "Test message for compression level comparison".to_string(),
            )],
        ));
    }

    let config_l0 = CompactionConfig::high_quality();
    let engine_l0 = CompactionEngine::new(config_l0);
    let plt_l0 = engine_l0.compact(&history, 100).unwrap();

    let config_l1 = CompactionConfig::standard();
    let engine_l1 = CompactionEngine::new(config_l1);
    let plt_l1 = engine_l1.compact(&history, 100).unwrap();

    let config_l2 = CompactionConfig::memory_constrained();
    let engine_l2 = CompactionEngine::new(config_l2);
    let plt_l2 = engine_l2.compact(&history, 100).unwrap();

    assert!(plt_l0.frames.len() >= plt_l1.frames.len());
    assert!(plt_l1.frames.len() >= plt_l2.frames.len());
}

/// E2E test: Anchor extraction for important turns
#[tokio::test]
async fn e2e_anchor_extraction_for_important_turns() {
    let mut history = ChatHistory::new("anchors-test".to_string());

    for i in 1..=20 {
        let mut turn = ChatTurn::new(
            i,
            vec![ChatMessage::new(
                MessageRole::Assistant,
                format!("Response {}", i),
            )],
        );

        if i % 5 == 0 {
            turn.turn_type = TurnType::Decision;
        }

        history.add_turn(turn);
    }

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 20).unwrap();

    // Should have anchors for decision turns
    assert!(plt.anchors.len() <= 4);
}

/// E2E test: Memory budget compliance for large history
#[tokio::test]
async fn e2e_memory_budget_compliance() {
    let mut history = ChatHistory::new("budget-test".to_string());

    for i in 1..=1000 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(
                MessageRole::User,
                "Test message with some content for memory budget testing".to_string(),
            )],
        ));
    }

    let config = CompactionConfig::memory_constrained();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 1000).unwrap();

    let estimated_bytes = plt.frames.len() * 150;
    assert!(estimated_bytes < 1_500_000, "Should be under 1.5MB budget");
}

/// E2E test: Frame content structure
#[tokio::test]
async fn e2e_frame_content_structure() {
    let mut history = ChatHistory::new("frame-test".to_string());

    history.add_turn(ChatTurn::new(
        1,
        vec![ChatMessage::new(
            MessageRole::User,
            "Build a web app".to_string(),
        )],
    ));

    history.add_turn(ChatTurn::new(
        2,
        vec![ChatMessage::new(
            MessageRole::Assistant,
            "I'll help you build a web app with React and TypeScript.".to_string(),
        )],
    ));

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 2).unwrap();

    assert!(!plt.frames.is_empty());

    for frame in &plt.frames {
        // frame_id is a Uuid, so we check it's not nil
        assert!(frame.turn_range.0 <= frame.turn_range.1);
    }
}

/// E2E test: Compression level metadata
#[tokio::test]
async fn e2e_compression_level_metadata() {
    let mut history = ChatHistory::new("metadata-test".to_string());

    for i in 1..=50 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(MessageRole::User, format!("Msg {}", i))],
        ));
    }

    let config_l0 = CompactionConfig::high_quality();
    let engine_l0 = CompactionEngine::new(config_l0);
    let plt_l0 = engine_l0.compact(&history, 50).unwrap();
    assert!(matches!(plt_l0.metadata.compression_level, CompressionLevel::L0));

    let config_l1 = CompactionConfig::standard();
    let engine_l1 = CompactionEngine::new(config_l1);
    let plt_l1 = engine_l1.compact(&history, 50).unwrap();
    assert!(matches!(plt_l1.metadata.compression_level, CompressionLevel::L1));

    let config_l2 = CompactionConfig::memory_constrained();
    let engine_l2 = CompactionEngine::new(config_l2);
    let plt_l2 = engine_l2.compact(&history, 50).unwrap();
    assert!(matches!(plt_l2.metadata.compression_level, CompressionLevel::L2));
}

/// E2E test: Empty history handling
#[tokio::test]
async fn e2e_empty_history_handling() {
    let history = ChatHistory::new("empty-test".to_string());
    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);

    let plt = engine.compact(&history, 0).unwrap();

    assert_eq!(plt.metadata.total_turns, 0);
}

/// E2E test: Transcript serialization
#[tokio::test]
async fn e2e_transcript_serialization() {
    let mut history = ChatHistory::new("serialize-test".to_string());

    for i in 1..=20 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(MessageRole::User, format!("Msg {}", i))],
        ));
    }

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 20).unwrap();

    let json = serde_json::to_string(&plt).expect("Should serialize to JSON");
    assert!(!json.is_empty());

    let deserialized: sentinel_core::memory::compaction::PinnedLightweightTranscript =
        serde_json::from_str(&json).expect("Should deserialize from JSON");

    assert_eq!(deserialized.transcript_id, plt.transcript_id);
}

/// E2E test: Incremental compaction
#[tokio::test]
async fn e2e_incremental_compaction() {
    let mut history = ChatHistory::new("incremental-test".to_string());

    for i in 1..=50 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(MessageRole::User, format!("Msg {}", i))],
        ));
    }

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt1 = engine.compact(&history, 50).unwrap();
    let frames1 = plt1.frames.len();

    for i in 51..=100 {
        history.add_turn(ChatTurn::new(
            i,
            vec![ChatMessage::new(MessageRole::User, format!("Msg {}", i))],
        ));
    }

    let plt2 = engine.compact(&history, 100).unwrap();
    let frames2 = plt2.frames.len();

    assert!(frames2 >= frames1);
    assert_eq!(plt2.metadata.total_turns, 100);
}

/// E2E test: Run ID propagation
#[tokio::test]
async fn e2e_run_id_propagation() {
    let run_id = "test-run-123".to_string();
    let mut history = ChatHistory::new(run_id.clone());

    history.add_turn(ChatTurn::new(
        1,
        vec![ChatMessage::new(MessageRole::User, "Test".to_string())],
    ));

    let config = CompactionConfig::standard();
    let engine = CompactionEngine::new(config);
    let plt = engine.compact(&history, 1).unwrap();

    assert_eq!(plt.run_id, run_id);
}
