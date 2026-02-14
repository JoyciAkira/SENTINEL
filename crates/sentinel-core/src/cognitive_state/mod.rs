//! Cognitive State Module - Layer 3 of Sentinel Architecture
//!
//! This module implements the Cognitive State Machine, which gives Sentinel
//! meta-cognition: awareness of its own thinking process.
//!
//! # Core Concept
//!
//! The Cognitive State is the "working memory" of the agent - everything it knows
//! about the project, its goals, and its own thinking process.
//!
//! Think of it as the agent's consciousness:
//! - What am I trying to achieve? (goals)
//! - What do I believe to be true? (beliefs)
//! - What am I uncertain about? (uncertainties)
//! - Why am I doing this action? (rationale)
//! - Am I thinking correctly? (meta-cognition)
//!
//! # Architecture
//!
//! ```text
//! +-----------------------------------------------------------+
//! |                   Cognitive State                         |
//! +-----------------------------------------------------------+
//! |                                                            |
//! |  before_action() --> [Action Gate] --> approve/reject     |
//! |       |                                                    |
//! |       +--> Meta-cognitive check: Why?                     |
//! |       +--> Invariant verification: Safe?                  |
//! |       +--> Alignment prediction: Will deviate?            |
//! |       +--> Value of information: Worth it?                |
//! |                                                            |
//! |  after_action() --> [Learning] --> update beliefs         |
//! |       |                                                    |
//! |       +--> Update beliefs based on outcome                |
//! |       +--> Check alignment changed                        |
//! |       +--> Detect unexpected deviations                   |
//! |       +--> Store in episodic memory                       |
//! |                                                            |
//! +-----------------------------------------------------------+
//! ```

pub mod action;
pub mod belief;
pub mod decision;
pub mod meta_state;
pub mod state;

// Re-export main types
pub use action::{Action, ActionDecision, ActionResult, ActionType};
pub use belief::{Belief, BeliefNetwork, Uncertainty, UncertaintyType};
pub use decision::{Decision, Rationale};
pub use meta_state::MetaCognitiveState;
pub use state::{CognitiveMode, CognitiveState};
