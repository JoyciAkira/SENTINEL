//! E2E test for Quality Loop Integration
//!
//! Tests the complete generate -> critique -> repair -> verify loop

use sentinel_core::quality::{
    Artifact, ArtifactType, QualityConfig, QualityContext, QualityMaximizer,
};

#[tokio::test]
async fn e2e_quality_loop_full_cycle() {
    // Setup
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let context = QualityContext {
        run_id: run_id.clone(),
        quality_config: QualityConfig {
            max_iterations: 3,
            enable_auto_repair: true,
            llm_provider: "anthropic".to_string(),
        },
        evaluation_suite: None,
    };

    // Create initial artifact with some issues
    let artifact = Artifact::new(
        ArtifactType::Code,
        "fn main() { println!(\"Hello\"); } // TODO: add error handling".to_string(),
        "file:///src/main.rs".to_string(),
    )
    .unwrap();

    // Run quality maximizer
    let maximizer = QualityMaximizer::new();
    let report = maximizer
        .maximize_quality("test_module", &artifact, &context)
        .await
        .unwrap();

    // Verify report structure (AC6 compliance)
    assert_eq!(report.run_id, run_id);
    assert_eq!(report.module_id, "test_module");
    assert!(report.report_id.starts_with("qr_"));
    assert_eq!(report.schema_version, "1.0");

    // Verify all 5 dimensions were scored
    assert_eq!(report.scores.len(), 5);

    // Verify metadata
    assert_eq!(report.metadata.llm_provider, "anthropic");
    assert!(report.metadata.iteration <= 3);
}

#[tokio::test]
async fn e2e_quality_loop_convergence_on_passing_artifact() {
    // An artifact that passes quality gates should converge immediately
    let context = QualityContext {
        run_id: format!("run_{}", uuid::Uuid::new_v4()),
        quality_config: QualityConfig::default(),
        evaluation_suite: None,
    };

    // High quality artifact
    let artifact = Artifact::new(
        ArtifactType::Code,
        r#"
fn main() {
    println!("Hello, World!");
}
"#.to_string(),
        "file:///src/main.rs".to_string(),
    )
    .unwrap();

    let maximizer = QualityMaximizer::new();
    let report = maximizer
        .maximize_quality("passing_module", &artifact, &context)
        .await
        .unwrap();

    // Should converge in iteration 1
    assert!(report.metadata.iteration <= 2);
}

#[tokio::test]
async fn e2e_execution_with_quality_gate_integration() {
    // Integration test with ExecutionEngine (AC6 state transitions)
    use sentinel_core::execution::ExecutionEngine;
    use sentinel_core::quality::{QualityMetric, QualityReport};

    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let engine = ExecutionEngine::new(run_id.clone());

    let artifact = Artifact::new(
        ArtifactType::Code,
        "fn main() {}".to_string(),
        "file:///test.rs".to_string(),
    )
    .unwrap();

    // Execute with quality gate
    let result = engine
        .execute_with_quality_gate("test_module", &artifact)
        .await
        .unwrap();

    // Verify AC6 state machine transitions
    // Should be either Validated (if passed) or RevisionRequired (if failed)
    assert!(
        matches!(result.status, sentinel_core::execution::ExecutionStatus::Validated)
            || matches!(
                result.status,
                sentinel_core::execution::ExecutionStatus::RevisionRequired
            )
    );

    // Quality report should always be produced
    assert!(result.quality_report.is_some());

    let report = result.quality_report.as_ref().unwrap();
    assert_eq!(report.run_id, run_id);
}
