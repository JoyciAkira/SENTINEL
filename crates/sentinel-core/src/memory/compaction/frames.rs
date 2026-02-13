//! # Pinned Lightweight Transcript Frames
//!
//! Lightweight, compressed representation of chat history for ultra-low memory footprint.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pinned Lightweight Transcript (PLT)
///
/// A compressed, structured representation of chat history designed for minimal
/// memory footprint while preserving critical context.
///
/// # Memory Budget
/// - Target: <= 1.5MB per 10k turns
/// - Typical compression ratio: 10-50x depending on compression level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedLightweightTranscript {
    /// Unique identifier for this transcript
    pub transcript_id: Uuid,

    /// Run/session identifier
    pub run_id: String,

    /// Compressed summary frames
    pub frames: Vec<SummaryFrame>,

    /// Anchor references for jumping to original context
    pub anchors: Vec<AnchorRef>,

    /// Metadata about the transcript
    pub metadata: TranscriptMetadata,
}

impl PinnedLightweightTranscript {
    /// Create a new empty transcript
    pub fn new(run_id: String) -> Self {
        Self {
            transcript_id: Uuid::new_v4(),
            run_id,
            frames: Vec::new(),
            anchors: Vec::new(),
            metadata: TranscriptMetadata::new(),
        }
    }

    /// Add a frame to the transcript
    pub fn add_frame(&mut self, frame: SummaryFrame) {
        self.frames.push(frame);
        // Use the maximum turn number from all frames to avoid double-counting
        self.metadata.total_turns = self.frames.iter().map(|f| f.turn_range.1).max().unwrap_or(0);
        self.metadata.last_updated = Utc::now();
    }

    /// Add an anchor reference
    pub fn add_anchor(&mut self, anchor: AnchorRef) {
        self.anchors.push(anchor);
    }

    /// Calculate total token estimate
    pub fn total_tokens(&self) -> u32 {
        self.frames.iter().map(|f| f.token_estimate).sum()
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        self.metadata.compressed_ratio
    }

    /// Update compression level
    pub fn set_compression_level(&mut self, level: CompressionLevel) {
        self.metadata.compression_level = level;
    }
}

/// A summary frame representing a compressed segment of chat history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryFrame {
    /// Unique frame identifier
    pub frame_id: Uuid,

    /// Type of frame
    pub frame_type: FrameType,

    /// When this frame was created
    pub timestamp: DateTime<Utc>,

    /// Frame content
    pub content: FrameContent,

    /// Range of turns this frame covers (start, end) inclusive
    pub turn_range: (u32, u32),

    /// Estimated token count for this frame
    pub token_estimate: u32,
}

impl SummaryFrame {
    /// Create a new summary frame
    pub fn new(
        frame_type: FrameType,
        content: FrameContent,
        turn_range: (u32, u32),
    ) -> Self {
        let token_estimate = Self::estimate_tokens(&content);
        Self {
            frame_id: Uuid::new_v4(),
            frame_type,
            timestamp: Utc::now(),
            content,
            turn_range,
            token_estimate,
        }
    }

    /// Estimate token count for frame content
    fn estimate_tokens(content: &FrameContent) -> u32 {
        // Rough estimate: ~4 chars per token
        let summary_tokens = content.summary.len() as u32 / 4;
        let key_points_tokens: u32 = content.key_points.iter()
            .map(|p| p.len() as u32 / 4)
            .sum();
        let outcome_tokens = content.outcome.as_ref()
            .map(|o| o.len() as u32 / 4)
            .unwrap_or(0);

        summary_tokens + key_points_tokens + outcome_tokens
    }

    /// Check if frame contains a specific turn
    pub fn contains_turn(&self, turn: u32) -> bool {
        turn >= self.turn_range.0 && turn <= self.turn_range.1
    }
}

/// Frame type categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameType {
    /// Standard summary frame
    #[serde(rename = "summary")]
    Summary,

    /// Milestone frame (important achievement)
    #[serde(rename = "milestone")]
    Milestone,

    /// Error frame (captures failures and fixes)
    #[serde(rename = "error")]
    Error,

    /// Decision frame (important architectural decision)
    #[serde(rename = "decision")]
    Decision,
}

/// Compressed frame content
///
/// Designed for minimal token usage while preserving essential information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameContent {
    /// Brief summary (1-2 sentences max, ~30-50 tokens)
    pub summary: String,

    /// Key points (3-5 bullets max, ~50-80 tokens)
    pub key_points: Vec<String>,

    /// Final outcome if completed (optional, ~20-40 tokens)
    pub outcome: Option<String>,
}

impl FrameContent {
    /// Create new frame content
    pub fn new(summary: String, key_points: Vec<String>) -> Self {
        Self {
            summary,
            key_points,
            outcome: None,
        }
    }

    /// Create with outcome
    pub fn with_outcome(mut self, outcome: String) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Validate content constraints
    pub fn is_valid(&self) -> bool {
        // Summary should be 1-2 sentences
        let sentence_count = self.summary.matches('.').count() + self.summary.matches('!').count();
        if sentence_count > 2 {
            return false;
        }

        // Key points should be 3-5 bullets
        if self.key_points.len() > 5 || self.key_points.is_empty() {
            return false;
        }

        true
    }
}

/// Anchor reference for jumping to original context
///
/// Provides lightweight references to important turns that can be expanded on demand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorRef {
    /// Unique anchor identifier
    pub anchor_id: Uuid,

    /// Turn number this anchor references
    pub turn_number: u32,

    /// Relevance score (0.0-1.0)
    pub relevance_score: f64,

    /// URI for jumping to original context
    pub jump_uri: String,

    /// Optional brief label for the anchor
    pub label: Option<String>,
}

impl AnchorRef {
    /// Create a new anchor reference
    pub fn new(turn_number: u32, relevance_score: f64, jump_uri: String) -> Self {
        Self {
            anchor_id: Uuid::new_v4(),
            turn_number,
            relevance_score: relevance_score.clamp(0.0, 1.0),
            jump_uri,
            label: None,
        }
    }

    /// Create with label
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

/// Transcript metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    /// Total number of turns covered
    pub total_turns: u32,

    /// Compression ratio (original_tokens / compressed_tokens)
    pub compressed_ratio: f64,

    /// Last update timestamp
    pub last_updated: DateTime<Utc>,

    /// Current compression level
    pub compression_level: CompressionLevel,
}

impl TranscriptMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self {
            total_turns: 0,
            compressed_ratio: 1.0,
            last_updated: Utc::now(),
            compression_level: CompressionLevel::L0,
        }
    }

    /// Update compression ratio
    pub fn update_compression_ratio(&mut self, original_tokens: u32, compressed_tokens: u32) {
        if compressed_tokens > 0 {
            self.compressed_ratio = original_tokens as f64 / compressed_tokens as f64;
        }
    }
}

impl Default for TranscriptMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression level
///
/// Defines how aggressively the transcript is compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// No compression (raw transcript)
    #[serde(rename = "L0")]
    L0,

    /// Semantic compression (summarized, ~10x reduction)
    #[serde(rename = "L1")]
    L1,

    /// Aggressive compression (key facts only, ~30-50x reduction)
    #[serde(rename = "L2")]
    L2,
}

impl CompressionLevel {
    /// Get estimated compression ratio for this level
    pub fn estimated_ratio(&self) -> f64 {
        match self {
            Self::L0 => 1.0,
            Self::L1 => 10.0,
            Self::L2 => 40.0,
        }
    }

    /// Check if this level meets memory budget
    pub fn meets_memory_budget(&self, original_tokens: u32, max_tokens: u32) -> bool {
        let estimated = (original_tokens as f64 / self.estimated_ratio()) as u32;
        estimated <= max_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plt_creation() {
        let plt = PinnedLightweightTranscript::new("test-run".to_string());
        assert_eq!(plt.run_id, "test-run");
        assert_eq!(plt.frames.len(), 0);
        assert_eq!(plt.anchors.len(), 0);
    }

    #[test]
    fn test_summary_frame_creation() {
        let content = FrameContent::new(
            "Implemented authentication".to_string(),
            vec![
                "Added JWT token validation".to_string(),
                "Created login endpoint".to_string(),
            ],
        );

        let frame = SummaryFrame::new(FrameType::Summary, content, (1, 5));

        assert_eq!(frame.turn_range, (1, 5));
        assert!(frame.token_estimate > 0);
    }

    #[test]
    fn test_frame_content_validation() {
        let valid = FrameContent::new(
            "Test summary".to_string(),
            vec!["Point 1".to_string(), "Point 2".to_string()],
        );
        assert!(valid.is_valid());

        // Too many key points
        let invalid = FrameContent::new(
            "Test".to_string(),
            vec!["A".to_string(); 6],
        );
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_anchor_ref_creation() {
        let anchor = AnchorRef::new(
            42,
            0.95,
            "intent://local/turn/42".to_string(),
        );

        assert_eq!(anchor.turn_number, 42);
        assert_eq!(anchor.relevance_score, 0.95);
        assert_eq!(anchor.jump_uri, "intent://local/turn/42");
    }

    #[test]
    fn test_compression_levels() {
        assert_eq!(CompressionLevel::L0.estimated_ratio(), 1.0);
        assert_eq!(CompressionLevel::L1.estimated_ratio(), 10.0);
        assert_eq!(CompressionLevel::L2.estimated_ratio(), 40.0);
    }

    #[test]
    fn test_frame_contains_turn() {
        let content = FrameContent::new("Test".to_string(), vec!["Point".to_string()]);
        let frame = SummaryFrame::new(FrameType::Summary, content, (10, 20));

        assert!(frame.contains_turn(10));
        assert!(frame.contains_turn(15));
        assert!(frame.contains_turn(20));
        assert!(!frame.contains_turn(9));
        assert!(!frame.contains_turn(21));
    }

    #[test]
    fn test_plt_add_frame() {
        let mut plt = PinnedLightweightTranscript::new("test".to_string());
        let content = FrameContent::new("Test".to_string(), vec!["Point".to_string()]);
        let frame = SummaryFrame::new(FrameType::Summary, content, (1, 5));

        plt.add_frame(frame);

        assert_eq!(plt.frames.len(), 1);
        assert_eq!(plt.metadata.total_turns, 5);
    }
}
