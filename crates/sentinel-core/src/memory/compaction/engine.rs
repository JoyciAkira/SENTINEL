//! # Compaction Engine
//!
//! Core engine for compacting chat history into Pinned Lightweight Transcript format.

use crate::memory::compaction::frames::{
    AnchorRef, CompressionLevel, FrameContent, FrameType, PinnedLightweightTranscript, SummaryFrame,
};
use crate::memory::compaction::policy::{CompactionConfig, CompactionTrigger as TriggerPolicy};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// Compression strategy for different compression levels
pub struct CompressionStrategy {
    /// Maximum summary length in characters
    max_summary_length: usize,

    /// Maximum key points per frame
    max_key_points: usize,

    /// Whether to preserve outcome information
    preserve_outcomes: bool,

    /// Maximum turns per frame
    max_turns_per_frame: u32,
}

impl CompressionStrategy {
    /// Create strategy for L0 (no compression)
    pub fn l0() -> Self {
        Self {
            max_summary_length: 500,
            max_key_points: 10,
            preserve_outcomes: true,
            max_turns_per_frame: 5,
        }
    }

    /// Create strategy for L1 (standard compression)
    pub fn l1() -> Self {
        Self {
            max_summary_length: 100,
            max_key_points: 5,
            preserve_outcomes: true,
            max_turns_per_frame: 20,
        }
    }

    /// Create strategy for L2 (aggressive compression)
    pub fn l2() -> Self {
        Self {
            max_summary_length: 50,
            max_key_points: 3,
            preserve_outcomes: false,
            max_turns_per_frame: 50,
        }
    }

    /// Create strategy from compression level
    pub fn from_level(level: CompressionLevel) -> Self {
        match level {
            CompressionLevel::L0 => Self::l0(),
            CompressionLevel::L1 => Self::l1(),
            CompressionLevel::L2 => Self::l2(),
        }
    }
}

/// Compaction Engine
///
/// Compacts chat history into Pinned Lightweight Transcript format.
pub struct CompactionEngine {
    trigger_policy: TriggerPolicy,
    compression_strategy: CompressionStrategy,
}

impl CompactionEngine {
    /// Create new compaction engine
    pub fn new(config: CompactionConfig) -> Self {
        let trigger_policy = TriggerPolicy::from_config(&config);
        let compression_strategy = CompressionStrategy::from_level(config.target_compression_level.to_level());

        Self {
            trigger_policy,
            compression_strategy,
        }
    }

    /// Compact chat history into PLT
    pub fn compact(
        &self,
        chat_history: &ChatHistory,
        current_turn: u32,
    ) -> Result<PinnedLightweightTranscript> {
        let mut plt = PinnedLightweightTranscript::new(chat_history.run_id.clone());

        // Group turns into frames based on compression strategy
        let frame_groups = self.group_turns_into_frames(chat_history, current_turn)?;

        // Create summary frames from each group
        for (turn_range, turns) in frame_groups {
            let frame = self.create_frame_from_turns(&turns, turn_range)?;
            plt.add_frame(frame);
        }

        // Extract high-relevance anchors
        let anchors = self.extract_anchors(chat_history, current_turn)?;
        for anchor in anchors {
            plt.add_anchor(anchor);
        }

        // Update metadata
        plt.set_compression_level(self.trigger_policy.target_level().to_level());
        plt.metadata.update_compression_ratio(
            chat_history.estimated_tokens(),
            plt.total_tokens(),
        );

        Ok(plt)
    }

    /// Group turns into frames based on compression strategy
    fn group_turns_into_frames(
        &self,
        history: &ChatHistory,
        current_turn: u32,
    ) -> Result<Vec<((u32, u32), Vec<ChatTurn>)>> {
        let mut groups = Vec::new();
        let mut current_group_start = 0;
        let mut current_group: Vec<ChatTurn> = Vec::new();

        for turn in &history.turns {
            // Skip turns beyond current_turn
            if turn.turn_number > current_turn {
                break;
            }

            current_group.push(turn.clone());

            // Check if we should end the current frame
            let group_size = (turn.turn_number - current_group_start) as usize;
            if group_size >= self.compression_strategy.max_turns_per_frame as usize
                || turn.turn_number % self.compression_strategy.max_turns_per_frame == 0
            {
                let turn_range = (current_group_start, turn.turn_number);
                groups.push((turn_range, current_group.clone()));
                current_group = Vec::new();
                current_group_start = turn.turn_number + 1;
            }
        }

        // Add remaining turns as final frame
        if !current_group.is_empty() {
            let last_turn = current_group.last().unwrap().turn_number;
            let turn_range = (current_group_start, last_turn);
            groups.push((turn_range, current_group));
        }

        Ok(groups)
    }

    /// Create a summary frame from a group of turns
    fn create_frame_from_turns(&self, turns: &[ChatTurn], turn_range: (u32, u32)) -> Result<SummaryFrame> {
        // Determine frame type based on content
        let frame_type = self.classify_frame_type(turns);

        // Generate summary
        let summary = self.generate_summary(turns, self.compression_strategy.max_summary_length);

        // Extract key points
        let key_points = self.extract_key_points(turns, self.compression_strategy.max_key_points);

        // Extract outcome if applicable
        let outcome = if self.compression_strategy.preserve_outcomes {
            self.extract_outcome(turns)
        } else {
            None
        };

        let content = FrameContent {
            summary,
            key_points,
            outcome,
        };

        Ok(SummaryFrame::new(frame_type, content, turn_range))
    }

    /// Classify the frame type based on turn content
    fn classify_frame_type(&self, turns: &[ChatTurn]) -> FrameType {
        let has_errors = turns.iter().any(|t| t.is_error());
        let has_decisions = turns.iter().any(|t| t.is_decision());
        let has_milestones = turns.iter().any(|t| t.is_milestone());

        if has_errors {
            FrameType::Error
        } else if has_decisions {
            FrameType::Decision
        } else if has_milestones {
            FrameType::Milestone
        } else {
            FrameType::Summary
        }
    }

    /// Generate a summary from turns
    fn generate_summary(&self, turns: &[ChatTurn], max_length: usize) -> String {
        if turns.is_empty() {
            return String::new();
        }

        // Simple approach: concatenate and truncate
        let mut summary = String::new();

        for turn in turns.iter().take(3) {
            // Get first message content
            if let Some(msg) = turn.messages.first() {
                let content = &msg.content;
                // Take first sentence or up to max_length
                let truncated = if content.len() > max_length {
                    let mut s = content.chars().take(max_length).collect::<String>();
                    if let Some(pos) = s.rfind(' ') {
                        s.truncate(pos);
                    }
                    s + "..."
                } else {
                    content.clone()
                };
                summary.push_str(&truncated);
                summary.push(' ');
            }
        }

        // Trim to max length
        if summary.len() > max_length {
            summary.truncate(max_length);
            if let Some(pos) = summary.rfind(' ') {
                summary.truncate(pos);
            }
            summary.push_str("...");
        }

        summary.trim().to_string()
    }

    /// Extract key points from turns
    fn extract_key_points(&self, turns: &[ChatTurn], max_points: usize) -> Vec<String> {
        let mut key_points = Vec::new();

        for turn in turns.iter().take(max_points * 2) {
            for msg in &turn.messages {
                // Simple heuristic: extract sentences with keywords
                let content = &msg.content.to_lowercase();
                let keywords = ["implemented", "fixed", "added", "created", "decided", "error", "issue"];

                for keyword in &keywords {
                    if content.contains(keyword) && key_points.len() < max_points {
                        let point = msg.content
                            .chars()
                            .take(100)
                            .collect::<String>()
                            .trim()
                            .to_string();
                        if !point.is_empty() && !key_points.contains(&point) {
                            key_points.push(point);
                        }
                    }
                }

                if key_points.len() >= max_points {
                    break;
                }
            }

            if key_points.len() >= max_points {
                break;
            }
        }

        key_points
    }

    /// Extract outcome from turns
    fn extract_outcome(&self, turns: &[ChatTurn]) -> Option<String> {
        // Look for the last substantive message
        for turn in turns.iter().rev() {
            for msg in turn.messages.iter().rev() {
                if msg.content.len() > 20 && msg.content.len() < 200 {
                    // Check if it looks like a conclusion
                    let content = msg.content.to_lowercase();
                    if content.contains("success")
                        || content.contains("complete")
                        || content.contains("finished")
                        || content.contains("done")
                    {
                        return Some(msg.content.clone());
                    }
                }
            }
        }
        None
    }

    /// Extract high-relevance anchors from history
    fn extract_anchors(&self, history: &ChatHistory, current_turn: u32) -> Result<Vec<AnchorRef>> {
        let mut anchors = Vec::new();

        for turn in &history.turns {
            if turn.turn_number > current_turn {
                continue;
            }

            // High relevance for errors, decisions, milestones
            let relevance = if turn.is_error() {
                0.95
            } else if turn.is_decision() {
                0.90
            } else if turn.is_milestone() {
                0.85
            } else {
                continue; // Skip non-important turns
            };

            let anchor = AnchorRef::new(
                turn.turn_number,
                relevance,
                format!("intent://local/turn/{}", turn.turn_number),
            );
            anchors.push(anchor);
        }

        Ok(anchors)
    }
}

/// Chat history representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistory {
    pub run_id: String,
    pub turns: Vec<ChatTurn>,
}

impl ChatHistory {
    /// Create new chat history
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            turns: Vec::new(),
        }
    }

    /// Add a turn to history
    pub fn add_turn(&mut self, turn: ChatTurn) {
        self.turns.push(turn);
    }

    /// Estimate token count for entire history
    pub fn estimated_tokens(&self) -> u32 {
        self.turns
            .iter()
            .map(|t| t.estimated_tokens())
            .sum()
    }
}

/// A single chat turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurn {
    pub turn_number: u32,
    pub timestamp: DateTime<Utc>,
    pub messages: Vec<ChatMessage>,
    pub turn_type: TurnType,
}

impl ChatTurn {
    /// Create new chat turn
    pub fn new(turn_number: u32, messages: Vec<ChatMessage>) -> Self {
        Self {
            turn_number,
            timestamp: Utc::now(),
            messages,
            turn_type: TurnType::Standard,
        }
    }

    /// Check if this turn contains an error
    pub fn is_error(&self) -> bool {
        self.turn_type == TurnType::Error
            || self.messages.iter().any(|m| m.is_error())
    }

    /// Check if this turn contains a decision
    pub fn is_decision(&self) -> bool {
        self.turn_type == TurnType::Decision
            || self.messages.iter().any(|m| m.is_decision())
    }

    /// Check if this turn is a milestone
    pub fn is_milestone(&self) -> bool {
        self.turn_type == TurnType::Milestone
            || self.messages.iter().any(|m| m.is_milestone())
    }

    /// Estimate token count for this turn
    pub fn estimated_tokens(&self) -> u32 {
        self.messages
            .iter()
            .map(|m| m.estimated_tokens())
            .sum::<u32>()
    }
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub metadata: MessageMetadata,
}

impl ChatMessage {
    /// Create new message
    pub fn new(role: MessageRole, content: String) -> Self {
        Self {
            role,
            content,
            metadata: MessageMetadata::default(),
        }
    }

    /// Check if this message is an error
    pub fn is_error(&self) -> bool {
        self.content.to_lowercase().contains("error")
            || self.metadata.message_flags.contains(&MessageFlag::Error)
    }

    /// Check if this message is a decision
    pub fn is_decision(&self) -> bool {
        self.metadata.message_flags.contains(&MessageFlag::Decision)
    }

    /// Check if this message is a milestone
    pub fn is_milestone(&self) -> bool {
        self.metadata.message_flags.contains(&MessageFlag::Milestone)
    }

    /// Estimate token count (~4 chars per token)
    pub fn estimated_tokens(&self) -> u32 {
        (self.content.len() / 4) as u32
    }
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMetadata {
    pub message_flags: Vec<MessageFlag>,
    pub additional_data: HashMap<String, String>,
}

/// Message flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageFlag {
    Error,
    Decision,
    Milestone,
    Code,
    ToolUse,
}

/// Turn type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnType {
    Standard,
    Error,
    Decision,
    Milestone,
}

impl Default for TurnType {
    fn default() -> Self {
        Self::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compaction_engine_creation() {
        let config = CompactionConfig::default();
        let engine = CompactionEngine::new(config);
        assert_eq!(engine.compression_strategy.max_key_points, 5);
    }

    #[test]
    fn test_chat_history() {
        let mut history = ChatHistory::new("test-run".to_string());
        assert_eq!(history.turns.len(), 0);

        let turn = ChatTurn::new(
            1,
            vec![ChatMessage::new(MessageRole::User, "Hello".to_string())],
        );
        history.add_turn(turn);

        assert_eq!(history.turns.len(), 1);
    }

    #[test]
    fn test_chat_turn_flags() {
        let mut turn = ChatTurn::new(
            1,
            vec![ChatMessage::new(MessageRole::Assistant, "Fixed the bug".to_string())],
        );

        assert!(!turn.is_error());
        assert!(!turn.is_decision());

        turn.turn_type = TurnType::Error;
        assert!(turn.is_error());
    }

    #[test]
    fn test_message_token_estimation() {
        let msg = ChatMessage::new(MessageRole::User, "Hello world".to_string());
        // ~11 chars / 4 = 2.75 -> 2 tokens (integer division)
        assert_eq!(msg.estimated_tokens(), 2);
    }

    #[test]
    fn test_compaction_strategy_levels() {
        let l0 = CompressionStrategy::l0();
        assert_eq!(l0.max_key_points, 10);
        assert!(l0.preserve_outcomes);

        let l1 = CompressionStrategy::l1();
        assert_eq!(l1.max_key_points, 5);
        assert!(l1.preserve_outcomes);

        let l2 = CompressionStrategy::l2();
        assert_eq!(l2.max_key_points, 3);
        assert!(!l2.preserve_outcomes);
    }
}
