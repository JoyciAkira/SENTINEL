//! Telemetry events for onboarding tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A complete telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event: OnboardingEvent,
}

/// Onboarding event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OnboardingEvent {
    /// First time running Sentinel
    FirstRun {
        version: String,
    },

    /// Completed an onboarding milestone
    MilestoneCompleted {
        milestone: OnboardingMilestone,
        duration_seconds: Option<u64>,
    },

    /// Used a specific feature
    FeatureUsed {
        feature: String,
        context: serde_json::Value,
    },

    /// Encountered an error
    Error {
        error_type: String,
        message: String,
        context: serde_json::Value,
    },

    /// Screen/page viewed
    ScreenView {
        screen: String,
        duration_seconds: Option<u64>,
    },

    /// Command executed
    CommandExecuted {
        command: String,
        args: Vec<String>,
        success: bool,
    },

    /// Blueprint applied
    BlueprintApplied {
        blueprint_id: String,
        blueprint_name: String,
    },

    /// Goal created
    GoalCreated {
        goal_type: String,
        has_subgoals: bool,
    },

    /// Goal completed
    GoalCompleted {
        goal_id: Uuid,
        duration_seconds: u64,
    },

    /// Alignment checked
    AlignmentChecked {
        score: f64,
        threshold: f64,
    },

    /// Help requested
    HelpRequested {
        topic: String,
        source: String,
    },

    /// Custom event
    Custom {
        name: String,
        data: serde_json::Value,
    },

    /// Generated revision prompt for quality improvement
    RevisionPromptGenerated {
        prompt: String,
        llm_provider: String,
        model: String,
    },
}


/// Onboarding milestones
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OnboardingMilestone {
    /// Completed first run setup
    FirstRunComplete,

    /// Created first goal
    FirstGoalCreated,

    /// Completed first goal
    FirstGoalCompleted,

    /// Used the TUI interface
    TuiUsed,

    /// Used the VS Code extension
    VsCodeExtensionUsed,

    /// Applied a blueprint
    BlueprintApplied,

    /// Ran alignment check
    AlignmentChecked,

    /// Reviewed goals
    GoalsReviewed,

    /// Enabled telemetry (meta!)
    TelemetryEnabled,

    /// Read documentation
    DocumentationRead,

    /// Ran first test
    FirstTestRun,
}

impl OnboardingMilestone {
    /// Get all defined milestones
    pub fn all_milestones() -> Vec<Self> {
        vec![
            Self::FirstRunComplete,
            Self::FirstGoalCreated,
            Self::FirstGoalCompleted,
            Self::TuiUsed,
            Self::VsCodeExtensionUsed,
            Self::BlueprintApplied,
            Self::AlignmentChecked,
            Self::GoalsReviewed,
            Self::TelemetryEnabled,
            Self::DocumentationRead,
            Self::FirstTestRun,
        ]
    }

    /// Get the milestone display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FirstRunComplete => "First Run Complete",
            Self::FirstGoalCreated => "First Goal Created",
            Self::FirstGoalCompleted => "First Goal Completed",
            Self::TuiUsed => "TUI Interface Used",
            Self::VsCodeExtensionUsed => "VS Code Extension Used",
            Self::BlueprintApplied => "Blueprint Applied",
            Self::AlignmentChecked => "Alignment Checked",
            Self::GoalsReviewed => "Goals Reviewed",
            Self::TelemetryEnabled => "Telemetry Enabled",
            Self::DocumentationRead => "Documentation Read",
            Self::FirstTestRun => "First Test Run",
        }
    }

    /// Get the milestone description
    pub fn description(&self) -> &'static str {
        match self {
            Self::FirstRunComplete => "Completed the initial Sentinel setup",
            Self::FirstGoalCreated => "Created your first goal in the manifold",
            Self::FirstGoalCompleted => "Successfully completed your first goal",
            Self::TuiUsed => "Used the Terminal User Interface",
            Self::VsCodeExtensionUsed => "Connected via VS Code extension",
            Self::BlueprintApplied => "Applied a quickstart blueprint",
            Self::AlignmentChecked => "Checked alignment score",
            Self::GoalsReviewed => "Reviewed goal progress",
            Self::TelemetryEnabled => "Opted in to telemetry",
            Self::DocumentationRead => "Read the help documentation",
            Self::FirstTestRun => "Ran the test suite",
        }
    }

    /// Get the milestone category
    pub fn category(&self) -> MilestoneCategory {
        match self {
            Self::FirstRunComplete => MilestoneCategory::Setup,
            Self::FirstGoalCreated => MilestoneCategory::Core,
            Self::FirstGoalCompleted => MilestoneCategory::Core,
            Self::TuiUsed => MilestoneCategory::Interface,
            Self::VsCodeExtensionUsed => MilestoneCategory::Interface,
            Self::BlueprintApplied => MilestoneCategory::Advanced,
            Self::AlignmentChecked => MilestoneCategory::Core,
            Self::GoalsReviewed => MilestoneCategory::Core,
            Self::TelemetryEnabled => MilestoneCategory::Setup,
            Self::DocumentationRead => MilestoneCategory::Learning,
            Self::FirstTestRun => MilestoneCategory::Advanced,
        }
    }
}

/// Milestone category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MilestoneCategory {
    Setup,
    Core,
    Interface,
    Advanced,
    Learning,
}

/// Onboarding session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingSession {
    pub session_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub events: Vec<TelemetryEvent>,
    pub milestones_achieved: Vec<OnboardingMilestone>,
}

impl OnboardingSession {
    pub fn duration_seconds(&self) -> Option<i64> {
        self.end_time.map(|end| {
            end.signed_duration_since(self.start_time)
                .num_seconds()
        })
    }

    pub fn is_complete(&self) -> bool {
        self.end_time.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestone_display_names() {
        assert_eq!(OnboardingMilestone::FirstRunComplete.display_name(), "First Run Complete");
        assert_eq!(OnboardingMilestone::FirstGoalCreated.display_name(), "First Goal Created");
    }

    #[test]
    fn test_milestone_categories() {
        assert_eq!(OnboardingMilestone::FirstRunComplete.category(), MilestoneCategory::Setup);
        assert_eq!(OnboardingMilestone::TuiUsed.category(), MilestoneCategory::Interface);
    }

    #[test]
    fn test_all_milestones_defined() {
        let milestones = OnboardingMilestone::all_milestones();
        assert_eq!(milestones.len(), 11);
    }
}
