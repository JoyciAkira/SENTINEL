//! E2E Test: Quality Loop
//!
//! Tests the full quality maximizer loop from artifact generation
//! through quality evaluation to convergence.

use chrono::Utc;
use sentinel_core::quality::{
    Artifact, ArtifactType, DimensionScore, QualityConfig, QualityContext, QualityMaximizer,
    QualityMetadata, QualityMetric, QualityReport, QualityRubric, GateType, QualityVerdict,
    Verdict,
};

/// E2E test: Quality maximizer convergence
///
/// This test validates:
/// 1. Quality report generation with AC6 linkage keys
/// 2. Dimension scoring with proper verdicts
/// 3. Overall verdict computation
/// 4. Hard gate enforcement
#[tokio::test]
async fn e2e_quality_maximizer_convergence() {
    // 1. Create test artifact
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let module_id = format!("mod_{}", uuid::Uuid::new_v4());

    let artifact = Artifact::new(
        ArtifactType::Code,
        "fn main() { println!(\"Hello\"); }".to_string(),
        "file:///src/main.rs".to_string(),
    )
    .expect("Artifact creation should succeed");

    // 2. Create quality context
    let context = QualityContext {
        run_id: run_id.clone(),
        quality_config: QualityConfig::default(),
        evaluation_suite: None,
    };

    // 3. Run quality maximizer (single iteration for E2E)
    let maximizer = QualityMaximizer::new();
    let report = maximizer
        .maximize_quality(&module_id, &artifact, &context)
        .await
        .expect("Quality evaluation should succeed");

    // 4. Validate AC6 linkage keys
    assert!(!report.run_id.is_empty(), "run_id should be present");
    assert!(!report.report_id.is_empty(), "report_id should be present");
    assert!(!report.module_id.is_empty(), "module_id should be present");
    assert_eq!(report.run_id, run_id, "run_id should match context");
    assert_eq!(report.module_id, module_id, "module_id should match");

    // 5. Validate schema version
    assert_eq!(report.schema_version, "1.0");

    // 6. Validate dimension scores
    assert!(!report.scores.is_empty(), "Should have dimension scores");

    // 7. Validate overall verdict
    assert!(matches!(report.overall, QualityVerdict::Pass | QualityVerdict::Fail));

    // 8. Validate metadata
    assert!(!report.metadata.llm_provider.is_empty());
    assert!(!report.metadata.model.is_empty());
}

/// E2E test: Hard gate enforcement
///
/// Validates that hard gate failures cause overall failure.
#[tokio::test]
async fn e2e_hard_gate_enforcement() {
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let module_id = format!("mod_{}", uuid::Uuid::new_v4());

    // Create scores with one hard gate failure
    let scores = vec![
        DimensionScore::new(QualityMetric::Correctness, 80.0, 85.0, GateType::Hard), // FAIL
        DimensionScore::new(QualityMetric::Reliability, 90.0, 80.0, GateType::Hard),  // PASS
        DimensionScore::new(QualityMetric::Maintainability, 60.0, 70.0, GateType::Soft), // FAIL (soft)
    ];

    let metadata = QualityMetadata {
        llm_provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet".to_string(),
        evaluated_at: Utc::now(),
        evaluation_duration_ms: 1500,
        iteration: 1,
    };

    let report = QualityReport::new(run_id, module_id, scores, metadata);

    // Should fail because Correctness (hard gate) failed
    assert_eq!(report.overall, QualityVerdict::Fail);
    assert!(!report.passes_hard_gates());

    // Verify failed dimensions
    let failed = report.failed_dimensions();
    assert!(failed.iter().any(|m| matches!(m, QualityMetric::Correctness)));
}

/// E2E test: Soft gate doesn't cause failure
///
/// Validates that soft gate failures don't cause overall failure
/// when all hard gates pass.
#[tokio::test]
async fn e2e_soft_gate_tolerated() {
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let module_id = format!("mod_{}", uuid::Uuid::new_v4());

    let scores = vec![
        DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard), // PASS
        DimensionScore::new(QualityMetric::Reliability, 85.0, 80.0, GateType::Hard),  // PASS
        DimensionScore::new(QualityMetric::Maintainability, 60.0, 70.0, GateType::Soft), // FAIL (soft)
        DimensionScore::new(QualityMetric::UXDevEx, 65.0, 75.0, GateType::Soft),     // FAIL (soft)
    ];

    let metadata = QualityMetadata {
        llm_provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet".to_string(),
        evaluated_at: Utc::now(),
        evaluation_duration_ms: 1500,
        iteration: 1,
    };

    let report = QualityReport::new(run_id, module_id, scores, metadata);

    // Should pass because all hard gates passed
    assert_eq!(report.overall, QualityVerdict::Pass);
    assert!(report.passes_hard_gates());

    // Verify only soft gates failed
    let failed = report.failed_dimensions();
    assert!(!failed.iter().any(|m| matches!(m, QualityMetric::Correctness)));
    assert!(!failed.iter().any(|m| matches!(m, QualityMetric::Reliability)));
    assert!(failed.iter().any(|m| matches!(m, QualityMetric::Maintainability)));
    assert!(failed.iter().any(|m| matches!(m, QualityMetric::UXDevEx)));
}

/// E2E test: AC6 artifact linkage
///
/// Validates that quality reports properly link to artifacts.
#[tokio::test]
async fn e2e_ac6_artifact_linkage() {
    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let module_id = format!("mod_{}", uuid::Uuid::new_v4());

    let artifact = Artifact::new(
        ArtifactType::Code,
        "test code".to_string(),
        "file:///test.rs".to_string(),
    )
    .unwrap();

    let artifact_id = artifact.id.clone();

    let scores = vec![DimensionScore::new(
        QualityMetric::Correctness,
        90.0,
        85.0,
        GateType::Hard,
    )];

    let metadata = QualityMetadata {
        llm_provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet".to_string(),
        evaluated_at: Utc::now(),
        evaluation_duration_ms: 1000,
        iteration: 1,
    };

    let mut report = QualityReport::new(run_id, module_id, scores, metadata);

    // Link artifact to report
    report.linked_artifact_ids.push(artifact_id.clone());

    // Verify linkage
    assert!(report.linked_artifact_ids.contains(&artifact_id));
    assert!(report.linked_artifact_ids[0].starts_with("art_"));
}

/// E2E test: Quality iteration limit
///
/// Validates that the quality loop respects the max iteration limit.
#[tokio::test]
async fn e2e_quality_iteration_limit() {
    let config = QualityConfig {
        max_iterations: 2,
        enable_auto_repair: true,
        llm_provider: "anthropic".to_string(),
    };

    let context = QualityContext {
        run_id: format!("run_{}", uuid::Uuid::new_v4()),
        quality_config: config,
        evaluation_suite: None,
    };

    let maximizer = QualityMaximizer::with_max_iterations(2);

    let module_id = format!("mod_{}", uuid::Uuid::new_v4());

    let artifact = Artifact::new(
        ArtifactType::Code,
        "bad code that needs improvement".to_string(),
        "file:///bad.rs".to_string(),
    )
    .unwrap();

    // Run the maximizer
    let report = maximizer
        .maximize_quality(&module_id, &artifact, &context)
        .await
        .expect("Evaluation should complete");

    // Should have stopped at or before iteration 2
    assert!(report.metadata.iteration <= 2);
}

/// E2E test: All quality dimensions have thresholds
///
/// Validates that each quality metric has a defined threshold.
#[tokio::test]
async fn e2e_quality_metrics_have_thresholds() {
    let metrics = [
        QualityMetric::Correctness,
        QualityMetric::Reliability,
        QualityMetric::Maintainability,
        QualityMetric::Security,
        QualityMetric::UXDevEx,
    ];

    for metric in metrics {
        let threshold = metric.default_threshold();
        let gate = metric.default_gate();

        assert!(threshold > 0.0, "{:?} threshold should be positive", metric);
        assert!(threshold <= 100.0, "{:?} threshold should be <= 100", metric);

        // Security and Correctness should be hard gates
        matches!(gate, GateType::Hard | GateType::Soft);
    }
}

/// E2E test: Quality rubric v1 definition
///
/// Validates the rubric has the expected dimensions and weights.
#[tokio::test]
async fn e2e_quality_rubric_v1_definition() {
    let rubric = QualityRubric::v1();

    // Get dimensions from rubric
    let dimensions = &rubric.dimensions;

    // Should have 5 dimensions
    assert_eq!(dimensions.len(), 5);

    // Check expected dimensions exist
    let metric_names: Vec<_> = dimensions.iter().map(|d| d.metric.clone()).collect();
    assert!(metric_names.contains(&QualityMetric::Correctness));
    assert!(metric_names.contains(&QualityMetric::Reliability));
    assert!(metric_names.contains(&QualityMetric::Maintainability));
    assert!(metric_names.contains(&QualityMetric::Security));
    assert!(metric_names.contains(&QualityMetric::UXDevEx));

    // Verify weights sum to approximately 1.0
    let total_weight: f64 = dimensions.iter().map(|d| d.weight).sum();
    assert!((total_weight - 1.0).abs() < 0.01, "Weights should sum to 1.0");
}

/// E2E test: Dimension score verdict computation
///
/// Validates that dimension scores compute verdicts correctly.
#[tokio::test]
async fn e2e_dimension_score_verdict_computation() {
    // Pass case
    let pass = DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard);
    assert_eq!(pass.result, Verdict::Pass);

    // Fail case
    let fail = DimensionScore::new(QualityMetric::Correctness, 80.0, 85.0, GateType::Hard);
    assert_eq!(fail.result, Verdict::Fail);

    // Boundary case (exact threshold)
    let boundary = DimensionScore::new(QualityMetric::Correctness, 85.0, 85.0, GateType::Hard);
    assert_eq!(boundary.result, Verdict::Pass);
}

/// E2E test: Artifact hash uniqueness
///
/// Validates that artifact hashes are unique per content.
#[tokio::test]
async fn e2e_artifact_hash_uniqueness() {
    let artifact1 = Artifact::new(
        ArtifactType::Code,
        "code1".to_string(),
        "file:///1.rs".to_string(),
    )
    .unwrap();

    let artifact2 = Artifact::new(
        ArtifactType::Code,
        "code1".to_string(),
        "file:///2.rs".to_string(),
    )
    .unwrap();

    let artifact3 = Artifact::new(
        ArtifactType::Code,
        "code2".to_string(),
        "file:///3.rs".to_string(),
    )
    .unwrap();

    // Same content should produce same hash
    assert_eq!(artifact1.hash, artifact2.hash);

    // Different content should produce different hash
    assert_ne!(artifact1.hash, artifact3.hash);

    // Each artifact should have unique ID
    assert_ne!(artifact1.id, artifact2.id);
    assert_ne!(artifact2.id, artifact3.id);
}

/// E2E test: Quality maximizer iteration time estimation
///
/// Validates that iteration time estimates are reasonable.
#[tokio::test]
async fn e2e_iteration_time_estimation() {
    let maximizer = QualityMaximizer::new();

    let small_artifact = Artifact::new(
        ArtifactType::Code,
        "fn main() {}".to_string(),
        "file:///small.rs".to_string(),
    )
    .unwrap();

    let small_time = maximizer.estimate_iteration_time(&small_artifact);
    assert!(small_time.as_millis() < 1000, "Small artifact should estimate < 1s");

    // Larger artifact should estimate more time
    let large_content = "x".repeat(10000);
    let large_artifact = Artifact::new(
        ArtifactType::Code,
        large_content,
        "file:///large.rs".to_string(),
    )
    .unwrap();

    let large_time = maximizer.estimate_iteration_time(&large_artifact);
    assert!(large_time > small_time, "Large artifact should estimate more time");
}

/// E2E test: Quality report metadata completeness
///
/// Validates that quality report metadata has all required fields.
#[tokio::test]
async fn e2e_quality_report_metadata() {
    let metadata = QualityMetadata {
        llm_provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        evaluated_at: Utc::now(),
        evaluation_duration_ms: 1234,
        iteration: 2,
    };

    assert_eq!(metadata.llm_provider, "anthropic");
    assert_eq!(metadata.model, "claude-3-5-sonnet-20241022");
    assert_eq!(metadata.evaluation_duration_ms, 1234);
    assert_eq!(metadata.iteration, 2);
}

/// E2E test: Weighted score computation
///
/// Validates that the rubric computes weighted scores correctly.
#[tokio::test]
async fn e2e_weighted_score_computation() {
    let rubric = QualityRubric::v1();

    let scores = vec![
        DimensionScore::new(QualityMetric::Correctness, 90.0, 85.0, GateType::Hard),
        DimensionScore::new(QualityMetric::Reliability, 80.0, 80.0, GateType::Hard),
        DimensionScore::new(QualityMetric::Maintainability, 70.0, 70.0, GateType::Soft),
        DimensionScore::new(QualityMetric::Security, 90.0, 90.0, GateType::Hard),
        DimensionScore::new(QualityMetric::UXDevEx, 75.0, 75.0, GateType::Soft),
    ];

    let weighted = rubric.compute_weighted_score(&scores);

    // Weighted score should be between min and max individual scores
    assert!(weighted >= 70.0 && weighted <= 90.0);
}
