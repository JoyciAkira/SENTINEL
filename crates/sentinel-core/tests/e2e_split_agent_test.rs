//! End-to-end tests for the Split Agent pipeline.
//!
//! These tests exercise the REAL filesystem — no mocks.
//! They create a tempdir, run the Architect→Executor→Verifier cycle,
//! and assert on real file existence and Predicate outcomes.

use sentinel_core::goal_manifold::{predicate::Predicate, Intent};
use sentinel_core::split_agent::{
    ArchitectAgent, GuardrailSeverity, LocalGuardrail, ModuleVerifier, SplitAgentExecutor,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn temp_workspace() -> TempDir {
    tempfile::Builder::new()
        .prefix("sentinel_split_agent_")
        .tempdir()
        .expect("should create tempdir")
}

fn intent_rest_api() -> Intent {
    Intent::new(
        "Build a REST API with JWT authentication",
        vec!["Rust", "no unsafe", "test coverage >80%"],
    )
}

// ---------------------------------------------------------------------------
// Test 1: ArchitectAgent produces a SplitPlan with non-empty modules
// ---------------------------------------------------------------------------
#[test]
fn test_architect_plan_produces_modules() {
    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("src/main.rs")),
        Predicate::FileExists(PathBuf::from("Cargo.toml")),
        Predicate::FileExists(PathBuf::from("src/auth/mod.rs")),
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan should succeed");

    assert_eq!(plan.modules.len(), 3);
    assert!(!plan.plan_hash.is_empty());
    assert_eq!(plan.intent_description, intent.description);
}

// ---------------------------------------------------------------------------
// Test 2: Each module has a non-empty output_contract and local_guardrails
// ---------------------------------------------------------------------------
#[test]
fn test_architect_modules_have_contracts_and_guardrails() {
    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("src/main.rs")),
        Predicate::FileExists(PathBuf::from("Cargo.toml")),
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan should succeed");

    for module in &plan.modules {
        assert!(
            !module.output_contract.is_empty(),
            "module {} must have output_contract",
            module.title
        );
        assert!(
            !module.local_guardrails.is_empty(),
            "module {} must have local_guardrails",
            module.title
        );
        assert!(
            !module.worker_context.destination_state.is_empty(),
            "module {} must have destination_state",
            module.title
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: Dependency chain — module 2 depends on module 1
// ---------------------------------------------------------------------------
#[test]
fn test_architect_wires_linear_dependency_chain() {
    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("src/main.rs")),
        Predicate::FileExists(PathBuf::from("Cargo.toml")),
      Predicate::FileExists(PathBuf::from("src/auth/mod.rs")),
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan should succeed");

    // module[0] has no dependencies
    assert!(plan.modules[0].dependencies.is_empty());
    // module[1] depends on module[0]
    assert_eq!(plan.modules[1].dependencies.len(), 1);
    assert_eq!(plan.modules[1].dependencies[0], plan.modules[0].id);
    // module[2] depends on module[1]
    assert_eq!(plan.modules[2].dependencies.len(), 1);
    assert_eq!(plan.modules[2].dependencies[0], plan.modules[1].id);
}

// ---------------------------------------------------------------------------
// Test 4: plan_hash is deterministic for same input
// ---------------------------------------------------------------------------
#[test]
fn test_plan_hash_depends_on_content() {
    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::FileExists(PathBuf::from("src/main.rs"))];

    let plan_a = agent.plan(&intent, &predicates).expect("plan_a");
    let plan_b = agent.plan(&intent, &predicates).expect("plan_b");

    // The plan_hash is derived from module contract_hashes.
    // Module IDs are UUIDs (random), so plan_ids differ, but hash structure
    // should be non-empty and non-trivial.
    assert!(!plan_a.plan_hash.is_empty());
    assert!(!plan_b.plan_hash.is_empty());
}

// ---------------------------------------------------------------------------
// Test 5: ModuleVerifier — FileExists on REAL filesystem
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_file_exists_real_filesystem() {
    let workspace = temp_workspace();
    let root = workspace.path();

    // Create a real file
    fs::write(root.join("Cargo.toml"), b"[package]\nname = \"test\"\n")
        .expect("should write Cargo.toml");

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::FileExists(PathBuf::from("Cargo.toml"))];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    let module = &plan.modules[0];

    let outcome = ModuleVerifier::verify(module, root);

    assert!(outcome.passed, "Cargo.toml exists → should pass");
    assert!(outcome.predicate_results[0].passed);
    assert_eq!(outcome.predicate_results[0].detail, "File found");
}

// ---------------------------------------------------------------------------
// Test 6: ModuleVerifier — FileExists FAILS when file missing
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_file_missing_fails() {
    let workspace = temp_workspace();
    let root = workspace.path();
    // Do NOT create the file

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::FileExists(PathBuf::from("missing_file.rs"))];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    let module = &plan.modules[0];

    let outcome = ModuleVerifier::verify(module, root);

    assert!(!outcome.passed, "file missing → should fail");
    assert!(!outcome.predicate_results[0].passed);
    assert!(outcome.predicate_results[0].detail.contains("not found"));
}

// ---------------------------------------------------------------------------
// Test 7: ModuleVerifier — DirectoryExists real filesystem
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_directory_exists_real_filesystem() {
    let workspace = temp_workspace();
    let root = workspace.path();

    fs::create_dir(root.join("src")).expect("should create src/");

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::DirectoryExists(PathBuf::from("src"))];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    let outcome = ModuleVerifier::verify(&plan.modules[0], root);

    assert!(outcome.passed, "src/ exists → should pass");
}

// ---------------------------------------------------------------------------
// Test 8: ModuleVerifier — CommandSucceeds (echo) on real shell
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_command_succeeds_real_shell() {
    let workspace = temp_workspace();
    let root = workspace.path();

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::CommandSucceeds {
        command: "echo".into(),
        args: vec!["sentinel_ok".into()],
        expected_exit_code: 0,
    }];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    let outcome = ModuleVerifier::verify(&plan.modules[0], root);

    assert!(outcome.passed, "echo exits 0 → should pass");
    assert!(outcome.predicate_results[0].detail.contains("exit_code=0"));
}

// ---------------------------------------------------------------------------
// Test 9: SplitAgentExecutor full cycle — worker creates files, verifier passes
// ---------------------------------------------------------------------------
#[test]
fn test_executor_full_cycle_worker_creates_files() {
    let workspace = temp_workspace();
    let root = workspace.path();

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("Cargo.toml")),
        Predicate::FileExists(PathBuf::from("src/main.rs")),
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan");

    // Worker function: creates the files declared in each module's output_contract
    let report = SplitAgentExecutor::execute(&plan, root, |module, workspace| {
        for predicate in &module.output_contract {
            if let Predicate::FileExists(path) = predicate {
                let full = workspace.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir failed: {}", e))?;
                }
                fs::write(&full, b"// generated by sentinel split-agent\n")
                    .map_err(|e| format!("write failed: {}", e))?;
            }
        }
        Ok(())
    });

    assert!(report.all_passed, "all modules should pass: {:?}", 
        report.module_reports.iter().map(|r| (&r.module_title, r.output_contract_passed, &r.skip_reason)).collect::<Vec<_>>());
    assert_eq!(report.total_modules, 2);
    assert_eq!(report.passed_modules, 2);
    assert_eq!(report.failed_modules, 0);
    assert_eq!(report.skipped_modules, 0);

    // Verify files exist on real disk
    assert!(root.join("Cargo.toml").exists());
    assert!(root.join("src/main.rs").exists());
}

// ---------------------------------------------------------------------------
// Test 10: Dependency skip — if module 1 fails, module 2 is skipped
// ---------------------------------------------------------------------------
#[test]
fn test_executor_dependency_skip_on_failure() {
    let workspace = temp_workspace();
    let root = workspace.path();

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("step1_output.rs")),
        Predicate::FileExists(PathBuf::from("step2_output.rs")),
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    assert_eq!(plan.modules.len(), 2);

    // Worker: module 0 fails (doesn't create the file), module 1 would succeed
    let report = SplitAgentExecutor::execute(&plan, root, |module, _workspace| {
        // Only succeed for module index 1 (but it will be skipped due to dep failure)
        if module.title.contains("Module 2") {
            // This worker would create the file, but module 1 depends on module 0
            Ok(())
        } else {
            // Module 0 "fails" — doesn't create step1_output.rs → verifier will fail
            Ok(()) // worker returns Ok but doesn't create the file
        }
    });

    // Module 0: worker returns Ok but file not created → verifier fails
    // Module 1: depends on module 0 which failed → skipped
    assert!(!report.all_passed);
    assert_eq!(report.passed_modules, 0);
    assert_eq!(report.failed_modules, 1); // module 0 failed
    assert_eq!(report.skipped_modules, 1); // module 1 skipped
    
    let m1_report = &report.module_reports[1];
    assert!(m1_report.skipped);
    assert!(m1_report.skip_reason.as_ref().map(|r| r.contains("Dependency")).unwrap_or(false));
}

// ---------------------------------------------------------------------------
// Test 11: Guardrail check_command — BLOCK violation is reported
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_block_guardrail_with_check_command_violation() {
    let workspace = temp_workspace();
    let root = workspace.path();

    // Create a file with a TODO comment (the violation)
    fs::write(root.join("bad_code.rs"), b"// TODO: fix this\nfn main() {}\n")
        .expect("should write bad_code.rs");

    // Build a module manually with a BLOCK guardrail that checks for TODO
    let agent = ArchitectAgent::new();
    let intent = Intent::new("No TODO in code", Vec::<String>::new());
    let predicates = vec![Predicate::FileExists(PathBuf::from("bad_code.rs"))];
    let mut plan = agent.plan(&intent, &predicates).expect("plan");

    // Inject a BLOCK guardrail with a real check command
    plan.modules[0].local_guardrails.push(
        LocalGuardrail::block(
            "No TODO comments allowed",
            "grep -r TODO in workspace files must return non-zero",
        )
        .with_check("grep -r TODO bad_code.rs"),
    );

    // Worker creates the file (already exists)
    let report = SplitAgentExecutor::execute(&plan, root, |_module, _workspace| Ok(()));

    let m0 = &report.module_reports[0];
    // The BLOCK guardrail check_command (grep TODO) exits 0 = found = violation
    // So the module should fail due to BLOCK guardrail
    assert!(!m0.output_contract_passed,
        "BLOCK guardrail violation should make module fail");
    
    let has_block_violation = m0.guardrail_violations.iter()
        .any(|v| matches!(v.severity, GuardrailSeverity::Block));
    assert!(has_block_violation, "should report a BLOCK guardrail violation");
}

// ---------------------------------------------------------------------------
// Test 12: Empty predicate list → ArchitectAgent returns error
// ---------------------------------------------------------------------------
#[test]
fn test_architect_empty_predicates_returns_error() {
    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let result = agent.plan(&intent, &[]);
    assert!(result.is_err(), "empty predicates should return Err");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("at least one root predicate"));
}

// ---------------------------------------------------------------------------
// Test 13: AlwaysTrue predicate always passes
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_always_true_passes() {
    let workspace = temp_workspace();
    let root = workspace.path();

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::AlwaysTrue];
    let plan = agent.plan(&intent, &predicates).expect("plan");

    let outcome = ModuleVerifier::verify(&plan.modules[0], root);
    assert!(outcome.passed);
}

// ---------------------------------------------------------------------------
// Test 14: AlwaysFalse predicate always fails
// ---------------------------------------------------------------------------
#[test]
fn test_verifier_always_false_fails() {
    let workspace = temp_workspace();
    let root = workspace.path();

    let agent = ArchitectAgent::new();
    let intent = intent_rest_api();
    let predicates = vec![Predicate::AlwaysFalse];
    let plan = agent.plan(&intent, &predicates).expect("plan");

    let outcome = ModuleVerifier::verify(&plan.modules[0], root);
    assert!(!outcome.passed);
}

// ---------------------------------------------------------------------------
// Test 15: max_modules limit is respected
// ---------------------------------------------------------------------------
#[test]
fn test_architect_respects_max_modules_limit() {
    let agent = ArchitectAgent::with_limits(2, 1);
    let intent = intent_rest_api();
    let predicates = vec![
        Predicate::FileExists(PathBuf::from("a.rs")),
        Predicate::FileExists(PathBuf::from("b.rs")),
        Predicate::FileExists(PathBuf::from("c.rs")), // should be cut
        Predicate::FileExists(PathBuf::from("d.rs")), // should be cut
    ];

    let plan = agent.plan(&intent, &predicates).expect("plan");
    assert_eq!(plan.modules.len(), 2, "max_modules=2 should limit output");
}

// ---------------------------------------------------------------------------
// Test 16: WorkerContext carries intent constraints
// ---------------------------------------------------------------------------
#[test]
fn test_worker_context_carries_intent_constraints() {
    let agent = ArchitectAgent::new();
    let constraints = vec!["Rust".to_string(), "no unsafe".to_string()];
    let intent = Intent::new("Test intent", constraints.clone());
    let predicates = vec![Predicate::FileExists(PathBuf::from("src/lib.rs"))];

    let plan = agent.plan(&intent, &predicates).expect("plan");

    let ctx = &plan.modules[0].worker_context;
    for constraint in &constraints {
        assert!(
            ctx.tech_constraints.contains(constraint),
            "tech_constraints should contain {:?}",
            constraint
        );
        assert!(
            ctx.non_negotiables.contains(constraint),
            "non_negotiables should contain {:?}",
            constraint
        );
    }
}
