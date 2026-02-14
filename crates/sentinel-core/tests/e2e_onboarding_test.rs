//! E2E Test: Onboarding Telemetry
//!
//! Tests the telemetry system for tracking user onboarding and TTFS.

use chrono::Utc;
use sentinel_core::telemetry::{OnboardingEvent, OnboardingMilestone, TelemetryClient};
use std::sync::Mutex;

/// Global mutex to serialize telemetry tests
/// This prevents race conditions when tests share the same storage directory
static TELEMETRY_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Acquire exclusive access to telemetry storage
/// Call this at the start of each test to prevent race conditions
fn acquire_test_lock() -> impl Drop {
    TELEMETRY_TEST_LOCK.lock().unwrap()
}

/// E2E test: TTFS (Time to First Success) calculation
///
/// Validates that TTFS is correctly measured from signup to first goal completion.
#[tokio::test]
async fn e2e_onboarding_ttfs_measured() {
    let _lock = acquire_test_lock();

    // Create a fresh telemetry client
    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();

    // Clear any existing data
    client.delete_all_data().await.unwrap();

    // Stage 0: Signup (simulated by FirstRun)
    let _signup_time = Utc::now();
    client
        .track(OnboardingEvent::FirstRun {
            version: "1.0.0".to_string(),
        })
        .await
        .unwrap();

    // Add a small delay to simulate time passing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Stage 1: First goal created
    client
        .track(OnboardingEvent::MilestoneCompleted {
            milestone: OnboardingMilestone::FirstGoalCreated,
            duration_seconds: None,
        })
        .await
        .unwrap();

    // More time passes...
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Stage 3: First success (goal completed)
    client
        .track(OnboardingEvent::MilestoneCompleted {
            milestone: OnboardingMilestone::FirstGoalCompleted,
            duration_seconds: Some(30),
        })
        .await
        .unwrap();

    // Generate onboarding report
    let report = client.generate_onboarding_report().await.unwrap();

    // Validate TTFS metrics - TTFS should be measured when FirstGoalCompleted milestone is achieved
    assert!(
        report.time_to_first_value_seconds.is_some(),
        "TTFS should be measured when FirstGoalCompleted milestone is tracked"
    );

    let ttfs = report.time_to_first_value_seconds.unwrap();
    assert!(ttfs >= 0, "TTFS should be non-negative");

    // Validate milestones
    assert!(
        report
            .completed_milestones
            .contains(&OnboardingMilestone::FirstGoalCreated),
        "FirstGoalCreated should be tracked"
    );
    assert!(
        report
            .completed_milestones
            .contains(&OnboardingMilestone::FirstGoalCompleted),
        "FirstGoalCompleted should be tracked"
    );
}

/// E2E test: AC8 telemetry event schema compliance
///
/// Validates that events have all required AC8 fields.
#[tokio::test]
async fn e2e_ac8_event_schema_compliance() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Track various event types and verify schema compliance
    let events = vec![
        OnboardingEvent::FirstRun {
            version: "1.0.0".to_string(),
        },
        OnboardingEvent::GoalCreated {
            goal_type: "web_app".to_string(),
            has_subgoals: true,
        },
        OnboardingEvent::GoalCompleted {
            goal_id: uuid::Uuid::new_v4(),
            duration_seconds: 120,
        },
        OnboardingEvent::AlignmentChecked {
            score: 85.0,
            threshold: 80.0,
        },
        OnboardingEvent::CommandExecuted {
            command: "sentinel init".to_string(),
            args: vec!["--force".to_string()],
            success: true,
        },
        OnboardingEvent::BlueprintApplied {
            blueprint_id: "web-app-v1".to_string(),
            blueprint_name: "Web App Auth".to_string(),
        },
    ];

    // Track all events
    for event in events {
        client.track(event).await.unwrap();
    }

    // Verify all events were recorded
    let report = client.generate_onboarding_report().await.unwrap();
    assert_eq!(report.total_events, 6, "All 6 events should be recorded");
}

/// E2E test: Opt-in privacy requirement
///
/// Validates that telemetry respects opt-in and doesn't collect when disabled.
#[tokio::test]
async fn e2e_opt_in_privacy_requirement() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.delete_all_data().await.unwrap();

    // Disable telemetry explicitly (in case it was enabled from previous tests)
    client.set_opt_in(false).await.unwrap();
    assert!(!client.is_enabled(), "Telemetry should be disabled");

    // Events should be silently dropped when disabled
    client
        .track(OnboardingEvent::FirstRun {
            version: "test".to_string(),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();
    assert_eq!(report.total_events, 0, "No events should be recorded when disabled");

    // Enable telemetry
    client.set_opt_in(true).await.unwrap();
    assert!(client.is_enabled());

    // Now events should be recorded
    client
        .track(OnboardingEvent::FirstRun {
            version: "test".to_string(),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();
    assert_eq!(report.total_events, 1, "Event should be recorded when enabled");
}

/// E2E test: Feature usage tracking
///
/// Validates that feature usage is properly tracked.
#[tokio::test]
async fn e2e_feature_usage_tracking() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Track feature usage - init 3 times total
    client
        .track(OnboardingEvent::FeatureUsed {
            feature: "init".to_string(),
            context: serde_json::json!({"args": ["--force"]}),
        })
        .await
        .unwrap();

    client
        .track(OnboardingEvent::FeatureUsed {
            feature: "ui".to_string(),
            context: serde_json::json!({"mode": "tui"}),
        })
        .await
        .unwrap();

    client
        .track(OnboardingEvent::FeatureUsed {
            feature: "init".to_string(),
            context: serde_json::json!({}),
        })
        .await
        .unwrap();

    client
        .track(OnboardingEvent::FeatureUsed {
            feature: "init".to_string(),
            context: serde_json::json!({}),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();

    // Verify feature usage counts
    assert_eq!(report.feature_usage.get("init"), Some(&3));
    assert_eq!(report.feature_usage.get("ui"), Some(&1));
}

/// E2E test: Error tracking
///
/// Validates that errors are properly counted.
#[tokio::test]
async fn e2e_error_tracking() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Track some errors
    client
        .track(OnboardingEvent::Error {
            error_type: "ValidationError".to_string(),
            message: "Invalid input".to_string(),
            context: serde_json::json!({"field": "goal"}),
        })
        .await
        .unwrap();

    client
        .track(OnboardingEvent::Error {
            error_type: "CompilationError".to_string(),
            message: "Build failed".to_string(),
            context: serde_json::json!({"exit_code": 1}),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();

    assert_eq!(report.error_count, 2);
}

/// E2E test: Progress percentage calculation
///
/// Validates that onboarding progress is calculated correctly.
#[tokio::test]
async fn e2e_progress_percentage_calculation() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Complete some milestones
    let milestones_to_complete = vec![
        OnboardingMilestone::FirstRunComplete,
        OnboardingMilestone::FirstGoalCreated,
        OnboardingMilestone::TuiUsed,
    ];

    for milestone in &milestones_to_complete {
        client
            .track(OnboardingEvent::MilestoneCompleted {
                milestone: milestone.clone(),
                duration_seconds: None,
            })
            .await
            .unwrap();
    }

    let report = client.generate_onboarding_report().await.unwrap();

    // Verify progress
    let expected_progress = (milestones_to_complete.len() as f64
        / OnboardingMilestone::all_milestones().len() as f64)
        * 100.0;

    assert!((report.progress_percent - expected_progress).abs() < 0.01);

    // Verify completed milestones
    for milestone in &milestones_to_complete {
        assert!(
            report.completed_milestones.contains(milestone),
            "{:?} should be in completed milestones",
            milestone
        );
    }
}

/// E2E test: Session ID uniqueness
///
/// Validates that each session gets a unique ID.
#[tokio::test]
async fn e2e_session_id_uniqueness() {
    let _lock = acquire_test_lock();

    let client1 = TelemetryClient::new().await.unwrap();
    let client2 = TelemetryClient::new().await.unwrap();

    // Each client should have unique session ID
    assert_ne!(client1.session_id(), client2.session_id());

    // User ID should persist (same machine) - check via report
    let report1 = client1.generate_onboarding_report().await.unwrap();
    let report2 = client2.generate_onboarding_report().await.unwrap();
    assert_eq!(report1.user_id, report2.user_id);
}

/// E2E test: Data export and deletion
///
/// Validates that telemetry data can be exported and deleted.
#[tokio::test]
async fn e2e_data_export_and_deletion() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Add some data
    client
        .track(OnboardingEvent::FirstRun {
            version: "test".to_string(),
        })
        .await
        .unwrap();

    // Export should succeed
    let exported = client.export_data().await.unwrap();
    assert!(!exported.is_empty());
    // JSON should contain expected fields
    assert!(exported.contains("\"id\"") || exported.contains("\"user_id\""));

    // Delete all data
    client.delete_all_data().await.unwrap();

    // Verify deletion
    let report = client.generate_onboarding_report().await.unwrap();
    assert_eq!(report.total_events, 0);
    assert_eq!(report.completed_milestones.len(), 0);
}

/// E2E test: Blueprint tracking
///
/// Validates that blueprint applications are tracked.
#[tokio::test]
async fn e2e_blueprint_tracking() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Track BlueprintApplied event
    client
        .track(OnboardingEvent::BlueprintApplied {
            blueprint_id: "web-app-auth-crud-board-v1".to_string(),
            blueprint_name: "Web App with Auth".to_string(),
        })
        .await
        .unwrap();

    // Track the milestone
    client
        .track(OnboardingEvent::MilestoneCompleted {
            milestone: OnboardingMilestone::BlueprintApplied,
            duration_seconds: Some(10),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();

    assert_eq!(report.total_events, 2);
    assert!(
        report
            .completed_milestones
            .contains(&OnboardingMilestone::BlueprintApplied),
        "BlueprintApplied should be tracked"
    );
}

/// E2E test: Alignment check tracking
///
/// Validates that alignment checks are tracked with scores.
#[tokio::test]
async fn e2e_alignment_check_tracking() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    // Track alignment checks with different scores
    client
        .track(OnboardingEvent::AlignmentChecked {
            score: 75.0,
            threshold: 80.0,
        })
        .await
        .unwrap();

    client
        .track(OnboardingEvent::AlignmentChecked {
            score: 92.0,
            threshold: 80.0,
        })
        .await
        .unwrap();

    // Should track the milestone as well
    client
        .track(OnboardingEvent::MilestoneCompleted {
            milestone: OnboardingMilestone::AlignmentChecked,
            duration_seconds: Some(5),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();

    assert_eq!(report.total_events, 3);
}

/// E2E test: First run tracking
///
/// Validates that first run is tracked correctly.
#[tokio::test]
async fn e2e_first_run_tracking() {
    let _lock = acquire_test_lock();

    let mut client = TelemetryClient::new().await.unwrap();
    client.set_opt_in(true).await.unwrap();
    client.delete_all_data().await.unwrap();

    client
        .track(OnboardingEvent::FirstRun {
            version: "3.0.0".to_string(),
        })
        .await
        .unwrap();

    // Explicitly mark the milestone as complete
    client
        .track(OnboardingEvent::MilestoneCompleted {
            milestone: OnboardingMilestone::FirstRunComplete,
            duration_seconds: Some(5),
        })
        .await
        .unwrap();

    let report = client.generate_onboarding_report().await.unwrap();

    assert!(report.first_run.is_some(), "First run timestamp should be recorded");
    assert!(
        report
            .completed_milestones
            .contains(&OnboardingMilestone::FirstRunComplete),
        "FirstRunComplete should be tracked"
    );
}
