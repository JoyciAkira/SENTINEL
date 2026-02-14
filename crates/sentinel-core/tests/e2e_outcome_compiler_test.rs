//! E2E Test: Outcome Compiler
//!
//! Tests the full outcome compilation pipeline from natural language intent
//! to atomic modules with guardrails.

use sentinel_core::outcome_compiler::{
    AtomicModuleCompiler, CompilationResult, InterpretContext, OutcomeInterpreter, OutcomeEnvelope,
};
use sentinel_core::outcome_compiler::compiler::GuardrailSeverity;

/// E2E test: Web app outcome compilation
///
/// This test validates:
/// 1. Intent parsing into OutcomeEnvelope
/// 2. Compilation into atomic modules
/// 3. Compliance with non-negotiable requirements
#[tokio::test]
async fn e2e_web_app_outcome_compilation() {
    // 1. Parse natural language intent
    let intent = "Build a team task board web app with authentication, user can create tasks assign to team members and drag tasks across a kanban board";
    let context = InterpretContext {
        assumptions: vec![
            "TypeScript tech stack".to_string(),
            "PostgreSQL database".to_string(),
        ],
        workspace_type: Some("web_app".to_string()),
        existing_tech_stack: vec![],
    };

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter
        .interpret(intent, &context)
        .expect("Intent interpretation should succeed");

    // 2. Validate OutcomeEnvelope structure
    assert_eq!(envelope.schema_version, "1.0");
    assert!(!envelope.outcome_id.is_empty());
    assert!(!envelope.intent.goal.is_empty());
    assert!(!envelope.acceptance_criteria.is_empty());

    // 3. Compile to atomic modules
    let compiler = AtomicModuleCompiler::new();
    let result = compiler
        .compile(&envelope)
        .expect("Compilation should succeed");

    // 4. Validate compliance with non-negotiable requirements
    assert!(!result.modules.is_empty(), "Should generate at least 1 module");

    for module in &result.modules {
        // Non-negotiable 1: Every module must have at least one invariant
        assert!(
            !module.invariants.is_empty(),
            "Module {} should have invariants",
            module.module_id
        );

        // Non-negotiable 2: Every module must have verifiable acceptance tests
        assert!(
            !module.verification.acceptance_tests.is_empty(),
            "Module {} should have tests",
            module.module_id
        );

        // Non-negotiable 3: Every module must have explicit boundaries
        assert!(
            !module.boundaries.in_scope.is_empty() || !module.boundaries.out_of_scope.is_empty(),
            "Module {} should have explicit boundaries",
            module.module_id
        );
    }

    // Non-negotiable 4: At least one BLOCK guardrail exists per module (check at result level)
    for module in &result.modules {
        let module_guardrails: Vec<_> = result
            .guardrails
            .iter()
            .filter(|g| g.module_id == module.module_id)
            .collect();

        assert!(
            module_guardrails
                .iter()
                .any(|g| matches!(g.severity, GuardrailSeverity::Block)),
            "Module {} should have BLOCK guardrail",
            module.module_id
        );
    }

    // 5. Verify audit log is hash-complete (non-negotiable 5)
    assert!(
        !result.audit_log.steps.is_empty(),
        "Audit log should have steps"
    );
    assert!(
        !result.decomposition_hash.is_empty(),
        "Decomposition hash should exist"
    );
    assert_eq!(result.audit_log.outcome_id, envelope.outcome_id);
}

/// E2E test: Backend service outcome compilation
#[tokio::test]
async fn e2e_backend_api_outcome_compilation() {
    let intent = "Build a billing reconciliation service that ingests events, reconciles invoices daily, and exposes discrepancy reports";
    let context = InterpretContext {
        assumptions: vec!["Idempotent processing required".to_string()],
        workspace_type: Some("backend_service".to_string()),
        existing_tech_stack: vec!["Rust".to_string()],
    };

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter
        .interpret(intent, &context)
        .expect("Intent interpretation should succeed");

    let compiler = AtomicModuleCompiler::new();
    let result = compiler
        .compile(&envelope)
        .expect("Compilation should succeed");

    // Backend services should generate modules
    // Note: The simple compiler generates generic module names (module_0_0, etc.)
    // In production, NLP would extract domain-specific names
    assert!(!result.modules.is_empty(), "Should generate at least 1 module");

    // All compliance checks apply
    for module in &result.modules {
        assert!(!module.invariants.is_empty());
        assert!(!module.verification.acceptance_tests.is_empty());

        // Check guardrails at result level, not module level
        let module_guardrails: Vec<_> = result
            .guardrails
            .iter()
            .filter(|g| g.module_id == module.module_id)
            .collect();

        assert!(
            module_guardrails
                .iter()
                .any(|g| matches!(g.severity, GuardrailSeverity::Block))
        );
    }
}

/// E2E test: Verify dependency DAG is acyclic
#[tokio::test]
async fn e2e_dependency_dag_is_acyclic() {
    let intent = "Build a web app with auth, CRUD operations, and admin panel";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    let compiler = AtomicModuleCompiler::new();
    let result = compiler.compile(&envelope).unwrap();

    // Check for cycles using a simple DFS
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();

    for module in &result.modules {
        if !visited.contains(&module.module_id) {
            assert!(
                !has_cycle(&result.modules, &module.module_id, &mut visited, &mut rec_stack),
                "Dependency graph should be acyclic"
            );
        }
    }
}

fn has_cycle(
    modules: &[sentinel_core::outcome_compiler::AtomicModule],
    module_id: &str,
    visited: &mut std::collections::HashSet<String>,
    rec_stack: &mut std::collections::HashSet<String>,
) -> bool {
    visited.insert(module_id.to_string());
    rec_stack.insert(module_id.to_string());

    if let Some(module) = modules.iter().find(|m| m.module_id == module_id) {
        for dep_id in &module.dependencies {
            if !visited.contains(dep_id) {
                if has_cycle(modules, dep_id, visited, rec_stack) {
                    return true;
                }
            } else if rec_stack.contains(dep_id) {
                return true;
            }
        }
    }

    rec_stack.remove(module_id);
    false
}

/// E2E test: Quality metrics are properly extracted
#[tokio::test]
async fn e2e_quality_metrics_extraction() {
    let intent = "Build a task app with 95% test coverage and e2e tests passing";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    // Should extract quality metrics from intent
    assert!(
        envelope.quality_metrics.iter().any(|m| m.name == "e2e_pass_rate"
            || m.name == "test_coverage"),
        "Should extract test-related quality metrics"
    );
}

/// E2E test: Risk register is generated
#[tokio::test]
async fn e2e_risk_register_generation() {
    let intent = "Build a payment processing system with PCI compliance";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    // Payment/security domains should generate risk entries
    assert!(
        envelope.risk_register.len() > 0,
        "High-risk domains should generate risk register entries"
    );

    // Each risk should have mitigation
    for risk in &envelope.risk_register {
        assert!(
            !risk.mitigation.is_empty(),
            "Risk '{}' should have mitigation strategy",
            risk.risk
        );
    }
}

/// E2E test: Module boundaries are properly set
#[tokio::test]
async fn e2e_module_boundaries_are_explicit() {
    let intent = "Build a task management app with focus on core CRUD only";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    let compiler = AtomicModuleCompiler::new();
    let result = compiler.compile(&envelope).unwrap();

    for module in &result.modules {
        // Each module must define what's in scope
        let has_in_scope = !module.boundaries.in_scope.is_empty();
        // Or explicitly define what's out of scope
        let has_out_of_scope = !module.boundaries.out_of_scope.is_empty();

        assert!(
            has_in_scope || has_out_of_scope,
            "Module {} must define boundaries (in_scope or out_of_scope)",
            module.module_name
        );
    }
}

/// E2E test: Guardrail emission follows contract
#[tokio::test]
async fn e2e_guardrail_emission_contract() {
    let intent = "Build a web application";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    let compiler = AtomicModuleCompiler::new();
    let result = compiler.compile(&envelope).unwrap();

    // Verify guardrails were generated for all modules
    assert!(!result.guardrails.is_empty(), "Should have guardrails");

    for module in &result.modules {
        // Get guardrails for this module
        let module_guardrails: Vec<_> = result
            .guardrails
            .iter()
            .filter(|g| g.module_id == module.module_id)
            .collect();

        // Count guardrail types
        let mut scope_count = 0;
        let mut quality_count = 0;
        let mut dependency_count = 0;

        for guardrail in &module_guardrails {
            match guardrail.rule_type {
                sentinel_core::outcome_compiler::compiler::GuardrailRuleType::Scope => {
                    scope_count += 1
                }
                sentinel_core::outcome_compiler::compiler::GuardrailRuleType::Quality => {
                    quality_count += 1
                }
                sentinel_core::outcome_compiler::compiler::GuardrailRuleType::Dependency => {
                    dependency_count += 1
                }
            }
        }

        // Each module MUST emit:
        // - 1 scope guardrail
        assert!(scope_count >= 1, "Module {} needs scope guardrail", module.module_id);
        // - 1 quality guardrail
        assert!(
            quality_count >= 1,
            "Module {} needs quality guardrail",
            module.module_id
        );
        // - 1 dependency guardrail (unless no dependencies)
        if !module.dependencies.is_empty() {
            assert!(
                dependency_count >= 1,
                "Module {} with deps needs dependency guardrail",
                module.module_id
            );
        }
    }
}

/// E2E test: AC6 artifact linkage keys are present
#[tokio::test]
async fn e2e_ac6_artifact_linkage_keys() {
    let intent = "Build a simple app";
    let context = InterpretContext::default();

    let interpreter = OutcomeInterpreter::new();
    let envelope = interpreter.interpret(intent, &context).unwrap();

    let compiler = AtomicModuleCompiler::new();
    let result = compiler.compile(&envelope).unwrap();

    // Verify that modules have structure supporting AC6 linkage
    for module in &result.modules {
        // module_id is the AC6 linkage key - should be a valid UUID
        assert!(!module.module_id.is_empty());
        // Should be parseable as UUID
        uuid::Uuid::parse_str(&module.module_id).expect("module_id should be valid UUID");
    }

    // Compilation result should have artifacts linked to outcome
    assert!(!result.audit_log.outcome_id.is_empty());
    assert!(!result.decomposition_hash.is_empty());
}
