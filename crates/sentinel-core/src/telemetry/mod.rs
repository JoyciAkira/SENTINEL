//! # Onboarding Telemetry (Wave 3.8)
//!
//! Telemetry system for tracking user onboarding and first-run experience.
//!
//! ## Overview
//!
//! The telemetry system captures:
//! - Time to first successful goal
//! - Feature adoption milestones
//! - Error rates during onboarding
//! - User engagement patterns
//!
//! ## Data Collected
//!
//! All telemetry is **opt-in** and **local-first**:
//! - No data leaves the user's machine without explicit consent
//! - All events are stored locally in `~/.sentinel/telemetry/`
//! - Users can export/delete their data at any time
//!
//! ## Usage
//!
//! ```rust
//! use sentinel_core::telemetry::{TelemetryClient, OnboardingEvent};
//! # #[tokio::main]
//! # async fn example() -> sentinel_core::Result<()> {
//! let client = TelemetryClient::new().await?;
//!
//! // Track an onboarding event
//! client.track(OnboardingEvent::FirstRun {
//!     version: "1.0.0".to_string(),
//! }).await?;
//!
//! // Generate a report
//! let report = client.generate_onboarding_report().await?;
//! # Ok(())
//! # }
//! ```

mod events;
mod storage;

pub use events::{
    OnboardingEvent, OnboardingMilestone, OnboardingSession, TelemetryEvent,
};
pub use storage::{TelemetryStorage, TelemetryStore};

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Telemetry client for tracking onboarding events
pub struct TelemetryClient {
    storage: TelemetryStorage,
    user_id: Uuid,
    session_id: Uuid,
    opt_in: bool,
}

impl TelemetryClient {
    /// Create a new telemetry client
    pub async fn new() -> Result<Self> {
        let storage = TelemetryStorage::new().await?;

        // Load or generate user ID
        let user_id = storage.get_or_create_user_id().await?;

        // Generate new session ID
        let session_id = Uuid::new_v4();

        // Check opt-in status
        let opt_in = storage.get_opt_in_status().await?;

        Ok(Self {
            storage,
            user_id,
            session_id,
            opt_in,
        })
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.opt_in
    }

    /// Set opt-in status
    pub async fn set_opt_in(&mut self, opt_in: bool) -> Result<()> {
        self.opt_in = opt_in;
        self.storage.set_opt_in_status(opt_in).await
    }

    /// Track a telemetry event
    pub async fn track(&self, event: OnboardingEvent) -> Result<()> {
        if !self.opt_in {
            return Ok(());
        }

        let telemetry_event = TelemetryEvent {
            id: Uuid::new_v4(),
            user_id: self.user_id,
            session_id: self.session_id,
            timestamp: Utc::now(),
            event: event.clone(),
        };

        self.storage.record_event(telemetry_event).await
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Generate an onboarding progress report
    pub async fn generate_onboarding_report(&self) -> Result<OnboardingReport> {
        let events = self.storage.get_all_events().await?;

        let mut completed_milestones = Vec::new();
        let mut first_run: Option<DateTime<Utc>> = None;
        let mut last_activity: Option<DateTime<Utc>> = None;
        let mut total_events = 0;
        let mut error_count = 0;
        let mut feature_usage: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for event in &events {
            total_events += 1;

            // Track first run
            if matches!(event.event, OnboardingEvent::FirstRun { .. }) {
                if first_run.is_none() {
                    first_run = Some(event.timestamp);
                }
            }

            // Track last activity
            last_activity = Some(event.timestamp);

            // Track milestones
            if let OnboardingEvent::MilestoneCompleted { milestone, .. } = &event.event {
                completed_milestones.push(milestone.clone());
            }

            // Track errors
            if matches!(event.event, OnboardingEvent::Error { .. }) {
                error_count += 1;
            }

            // Track feature usage
            if let OnboardingEvent::FeatureUsed { feature, .. } = &event.event {
                *feature_usage.entry(feature.clone()).or_insert(0) += 1;
            }
        }

        // Calculate onboarding progress percentage
        let all_milestones = OnboardingMilestone::all_milestones();
        let progress_percent = if all_milestones.is_empty() {
            100.0
        } else {
            (completed_milestones.len() as f64 / all_milestones.len() as f64) * 100.0
        };

        // Calculate time to first value
        let time_to_first_value = if let Some(first) = first_run {
            if let Some(first_goal) = completed_milestones
                .iter()
                .find(|m| matches!(m, OnboardingMilestone::FirstGoalCompleted))
            {
                // Find when this milestone was achieved
                events
                    .iter()
                    .find(|e| {
                        matches!(e.event, OnboardingEvent::MilestoneCompleted { .. })
                            && e.timestamp >= first
                    })
                    .map(|e| e.timestamp.signed_duration_since(first).num_seconds())
            } else {
                None
            }
        } else {
            None
        };

        Ok(OnboardingReport {
            user_id: self.user_id,
            progress_percent,
            completed_milestones,
            total_milestones: all_milestones.len(),
            first_run,
            last_activity,
            total_events,
            error_count,
            time_to_first_value_seconds: time_to_first_value,
            feature_usage,
        })
    }

    /// Export telemetry data as JSON
    pub async fn export_data(&self) -> Result<String> {
        let events = self.storage.get_all_events().await?;
        serde_json::to_string_pretty(&events).map_err(Into::into)
    }

    /// Delete all telemetry data
    pub async fn delete_all_data(&self) -> Result<()> {
        self.storage.clear_all().await
    }
}

impl Default for TelemetryClient {
    fn default() -> Self {
        // Create a minimal client with placeholder storage.
        // Use `TelemetryClient::new()` for a fully initialized client.
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        let base_path = PathBuf::from(home)
            .join(".sentinel")
            .join("telemetry");

        Self {
            storage: TelemetryStorage {
                base_path,
            },
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            opt_in: false,
        }
    }
}

/// Onboarding progress report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingReport {
    pub user_id: Uuid,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Completed milestones
    pub completed_milestones: Vec<OnboardingMilestone>,
    /// Total number of milestones
    pub total_milestones: usize,
    /// First run timestamp
    pub first_run: Option<DateTime<Utc>>,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
    /// Total events recorded
    pub total_events: usize,
    /// Number of errors encountered
    pub error_count: usize,
    /// Time from first run to first completed goal (seconds)
    pub time_to_first_value_seconds: Option<i64>,
    /// Feature usage counts
    pub feature_usage: std::collections::HashMap<String, usize>,
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Whether telemetry is enabled
    pub enabled: bool,
    /// Directory for storing telemetry data
    pub storage_path: PathBuf,
    /// Maximum events to store per user
    pub max_events: usize,
    /// Whether to include sensitive data in events
    pub include_sensitive_data: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());

        let storage_path = PathBuf::from(home)
            .join(".sentinel")
            .join("telemetry");

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&storage_path).ok();

        Self {
            enabled: false,
            storage_path,
            max_events: 10_000,
            include_sensitive_data: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to clean up test data
    async fn cleanup_test_data() {
        if let Ok(home) = std::env::var("HOME") {
            let telemetry_dir = std::path::PathBuf::from(home)
                .join(".sentinel")
                .join("telemetry");

            // Clean up config.json to reset opt-in state
            let config_path = telemetry_dir.join("config.json");
            let _ = tokio::fs::remove_file(&config_path).await;

            // Clean up events.jsonl
            let events_path = telemetry_dir.join("events.jsonl");
            let _ = tokio::fs::remove_file(&events_path).await;
        }
    }

    #[tokio::test]
    async fn test_telemetry_client_creation() {
        cleanup_test_data().await;
        let client = TelemetryClient::new().await.unwrap();
        assert!(!client.is_enabled()); // Default is opt-out
    }

    #[tokio::test]
    async fn test_opt_in_toggle() {
        cleanup_test_data().await;
        let mut client = TelemetryClient::new().await.unwrap();

        assert!(!client.is_enabled());
        client.set_opt_in(true).await.unwrap();
        assert!(client.is_enabled());
        client.set_opt_in(false).await.unwrap();
        assert!(!client.is_enabled());
    }

    #[tokio::test]
    async fn test_track_event_when_disabled() {
        cleanup_test_data().await;
        let client = TelemetryClient::new().await.unwrap();

        // Should not error even when disabled
        let result = client.track(OnboardingEvent::FirstRun {
            version: "test".to_string(),
        }).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_onboarding_report() {
        cleanup_test_data().await;
        let mut client = TelemetryClient::new().await.unwrap();
        client.set_opt_in(true).await.unwrap();

        // Clear any existing events
        client.delete_all_data().await.unwrap();

        // Track some events
        client.track(OnboardingEvent::FirstRun {
            version: "1.0.0".to_string(),
        }).await.unwrap();

        client.track(OnboardingEvent::FeatureUsed {
            feature: "init".to_string(),
            context: serde_json::json!({}),
        }).await.unwrap();

        let report = client.generate_onboarding_report().await.unwrap();
        assert_eq!(report.user_id, client.user_id);
        assert!(report.first_run.is_some());
        assert_eq!(report.total_events, 2);
    }
}
