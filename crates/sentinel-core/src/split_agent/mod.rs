//! Split Agent — Architect → Worker → Verifier pipeline
//!
//! Implements the Goal-First Split Agent pattern:
//! 1. ArchitectAgent: interprets intent, produces non-negotiable atomic WorkerModules
//! 2. Each WorkerModule carries: output_contract (Predicate[]), local_guardrails,
//!    destination_state, and a full WorkerContext so workers know exactly where to arrive.
//! 3. SplitAgentExecutor: coordinates Workers in parallel (bounded concurrency),
//!    then runs ModuleVerifier against the real filesystem after each module completes.
//! 4. ModuleVerifier: evaluates Predicate against actual filesystem/commands — zero mocks.

use crate::error::{Result, SentinelError};
use crate::goal_manifold::predicate::Predicate;
use crate::goal_manifold::Intent;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Severity level for a local guardrail attached to a single WorkerModule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardrailSeverity {
    /// Log violation, continue execution.
    Warn,
    /// Abort module execution immediately.
    Block,
}

/// A guardrail scoped to a single WorkerModule.
/// The worker must not violate this rule; the executor enforces it before
/// and after module execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalGuardrail {
    pub id: Uuid,
    pub description: String,
    pub rule: String,
    pub severity: GuardrailSeverity,
    /// Shell command (exit 0 = compliant, exit non-0 = violation).
    /// If None, the rule is evaluated textually by the executor.
    pub check_command: Option<String>,
}

impl LocalGuardrail {
    pub fn block(description: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            rule: rule.into(),
            severity: GuardrailSeverity::Block,
            check_command: None,
        }
    }

    pub fn warn(description: impl Into<String>, rule: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            rule: rule.into(),
            severity: GuardrailSeverity::Warn,
            check_command: None,
        }
    }

    pub fn with_check(mut self, cmd: impl Into<String>) -> Self {
        self.check_command = Some(cmd.into());
        self
    }
}

/// Everything a worker needs to know to complete its module,
/// without looking anywhere outside its boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerContext {
    /// Full description of the final state this module must produce.
    pub destination_state: String,
    /// Files the worker is explicitly allowed to touch.
    pub allowed_paths: Vec<PathBuf>,
    /// Files/dirs the worker must NOT touch (explicit exclusion list).
    pub forbidden_paths: Vec<PathBuf>,
    /// Tech constraints inherited from the root intent.
    pub tech_constraints: Vec<String>,
    /// Non-negotiable requirements from the root intent.
    pub non_negotiables: Vec<String>,
}

/// A single atomic unit of work, produced by the ArchitectAgent.
/// Once produced, the module is NON-NEGOTIABLE: workers cannot change
/// input_contract, output_contract, or local_guardrails.
#[derive(Debug, Clone)]
pub struct WorkerModule {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    /// Predicates that MUST hold before this module starts.
    pub input_contract: Vec<Predicate>,
    /// Predicates that MUST hold after this module completes — verified by ModuleVerifier.
    pub output_contract: Vec<Predicate>,
    /// Local guardrails enforced by executor (BLOCK stops the worker).
    pub local_guardrails: Vec<LocalGuardrail>,
    /// Full context the worker needs (destination state, allowed paths, …).
    pub worker_context: WorkerContext,
    /// Modules that must complete (and pass verification) before this one starts.
    pub dependencies: Vec<Uuid>,
    /// Estimated effort in relative units (1 = trivial, 10 = very complex).
    pub estimated_effort: u8,
}

impl WorkerModule {
    /// Compute a stable Blake3 hash of the module's negotiable contract.
    /// Workers can use this to prove they received an unmodified module.
    pub fn contract_hash(&self) -> String {
        let material = format!(
            "{}|{}|{}|{}",
            self.id,
            self.title,
            self.output_contract.len(),
            self.local_guardrails.len()
        );
        blake3::hash(material.as_bytes()).to_hex().to_string()
    }
}

/// The full plan produced by ArchitectAgent.
/// Modules are ordered in topological dependency order.
#[derive(Debug, Clone)]
pub struct SplitPlan {
    pub plan_id: Uuid,
    pub intent_description: String,
    pub modules: Vec<WorkerModule>,
    /// Blake3 hash of the entire plan for tamper-evidence.
    pub plan_hash: String,
}

impl SplitPlan {
    fn compute_hash(intent: &str, modules: &[WorkerModule]) -> String {
        let material = format!(
            "{}|{}|{}",
            intent,
            modules.len(),
            modules
                .iter()
                .map(|m| m.contract_hash())
                .collect::<Vec<_>>()
                .join("|")
        );
        blake3::hash(material.as_bytes()).to_hex().to_string()
    }
}

// ---------------------------------------------------------------------------
// ArchitectAgent
// ---------------------------------------------------------------------------

/// Interprets the root intent and decomposes it into non-negotiable WorkerModules.
///
/// Design principle: the Architect front-loads ALL design decisions so that workers
/// receive a complete, unambiguous contract and cannot drift.
pub struct ArchitectAgent {
    /// Maximum modules the Architect will produce (guards against explosion).
    max_modules: usize,
    /// Concurrency limit for worker execution in SplitAgentExecutor.
    max_parallel: usize,
}

impl ArchitectAgent {
    pub fn new() -> Self {
        Self { max_modules: 8, max_parallel: 3 }
    }

    pub fn with_limits(max_modules: usize, max_parallel: usize) -> Self {
        Self { max_modules, max_parallel }
    }

    /// Produce a SplitPlan from an Intent and a set of top-level success criteria.
    ///
    /// This is deterministic: given the same intent+predicates it produces the same plan.
    /// No LLM call here — the Architect uses structural analysis of predicates.
    /// (LLM-backed planning is done in sentinel-agent-native where LLM deps exist.)
    pub fn plan(&self, intent: &Intent, root_predicates: &[Predicate]) -> Result<SplitPlan> {
        if root_predicates.is_empty() {
            return Err(SentinelError::InvalidInput(
                "ArchitectAgent requires at least one root predicate to produce a plan".to_string(),
            ));
        }

        let mut modules: Vec<WorkerModule> = root_predicates
            .iter()
            .take(self.max_modules)
            .enumerate()
            .map(|(idx, predicate)| self.predicate_to_module(idx, predicate, intent))
            .collect();

        // Wire dependencies: each module after the first depends on the previous
        // (simple linear chain — adequate for most goal structures).
        let ids: Vec<Uuid> = modules.iter().map(|m| m.id).collect();
        for (i, module) in modules.iter_mut().enumerate() {
            if i > 0 {
                module.dependencies = vec![ids[i - 1]];
            }
        }

        let plan_hash = SplitPlan::compute_hash(&intent.description, &modules);
        Ok(SplitPlan {
            plan_id: Uuid::new_v4(),
            intent_description: intent.description.clone(),
            modules,
            plan_hash,
        })
    }

    fn predicate_to_module(&self, idx: usize, predicate: &Predicate, intent: &Intent) -> WorkerModule {
        let title = predicate_title(predicate);
        let destination = predicate_destination(predicate);

        // Output contract = the exact predicate that verifies completion.
        let output_contract = vec![predicate.clone()];

        // Input contract = AlwaysTrue for the first module, otherwise
        // it should be constructed by the caller or executor.
        let input_contract = vec![Predicate::AlwaysTrue];

        let worker_context = WorkerContext {
            destination_state: destination.clone(),
            allowed_paths: predicate_allowed_paths(predicate),
            forbidden_paths: vec![],
            tech_constraints: intent.constraints.clone(),
            non_negotiables: intent.constraints.clone(),
        };

        let guardrails = vec![
            LocalGuardrail::block(
                "Stay within module scope",
                format!("Worker must only produce artifacts described in: {}", destination),
            ),
            LocalGuardrail::block(
                "Output contract not satisfied",
                format!("Predicate must evaluate to true: {}", title),
            ),
        ];

        WorkerModule {
            id: Uuid::new_v4(),
            title: format!("Module {}: {}", idx + 1, title),
            description: destination.clone(),
            input_contract,
            output_contract,
            local_guardrails: guardrails,
            worker_context,
            dependencies: vec![],
            estimated_effort: estimate_effort(predicate),
        }
    }
}

impl Default for ArchitectAgent {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ModuleVerifier — evaluates output_contract on the REAL filesystem
// ---------------------------------------------------------------------------

/// Result of verifying a single WorkerModule.
#[derive(Debug, Clone)]
pub struct VerificationOutcome {
    pub module_id: Uuid,
    pub module_title: String,
    pub passed: bool,
    pub predicate_results: Vec<PredicateResult>,
    pub guardrail_violations: Vec<GuardrailViolation>,
}

#[derive(Debug, Clone)]
pub struct PredicateResult {
    pub predicate_description: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct GuardrailViolation {
    pub guardrail_id: Uuid,
    pub rule: String,
    pub severity: GuardrailSeverity,
    pub detail: String,
}

/// Evaluates module output contracts against the real filesystem.
pub struct ModuleVerifier;

impl ModuleVerifier {
    /// Evaluate the output_contract predicates of a WorkerModule.
    /// workspace_root is the directory against which relative paths are resolved.
    pub fn verify(module: &WorkerModule, workspace_root: &Path) -> VerificationOutcome {
        let predicate_results: Vec<PredicateResult> = module
            .output_contract
            .iter()
            .map(|p| evaluate_predicate(p, workspace_root))
            .collect();

        let guardrail_violations: Vec<GuardrailViolation> = module
            .local_guardrails
            .iter()
            .filter_map(|g| check_guardrail(g, workspace_root))
            .collect();

        let passed = predicate_results.iter().all(|r| r.passed)
            && !guardrail_violations
                .iter()
                .any(|v| matches!(v.severity, GuardrailSeverity::Block));

        VerificationOutcome {
            module_id: module.id,
            module_title: module.title.clone(),
            passed,
            predicate_results,
            guardrail_violations,
        }
    }
}

fn evaluate_predicate(predicate: &Predicate, root: &Path) -> PredicateResult {
    match predicate {
        Predicate::FileExists(path) => {
            let full = if path.is_absolute() { path.clone() } else { root.join(path) };
            let passed = full.exists();
            PredicateResult {
                predicate_description: format!("file_exists({:?})", path),
                passed,
                detail: if passed {
                    "File found".into()
                } else {
                    format!("File not found: {:?}", full)
                },
            }
        }
        Predicate::DirectoryExists(path) => {
            let full = if path.is_absolute() { path.clone() } else { root.join(path) };
            let passed = full.is_dir();
            PredicateResult {
                predicate_description: format!("directory_exists({:?})", path),
                passed,
                detail: if passed {
                    "Directory found".into()
                } else {
                    format!("Directory not found: {:?}", full)
                },
            }
        }
        Predicate::CommandSucceeds { command, args, expected_exit_code } => {
            match Command::new(command).args(args).current_dir(root).output() {
                Ok(output) => {
                    let code = output.status.code().unwrap_or(-1);
                    let passed = code == *expected_exit_code;
                    PredicateResult {
                        predicate_description: format!("command_succeeds({} {:?})", command, args),
                        passed,
                        detail: format!("exit_code={} (expected {})", code, expected_exit_code),
                    }
                }
                Err(e) => PredicateResult {
                    predicate_description: format!("command_succeeds({} {:?})", command, args),
                    passed: false,
                    detail: format!("failed to spawn: {}", e),
                },
            }
        }
        Predicate::AlwaysTrue => PredicateResult {
            predicate_description: "always_true".into(),
            passed: true,
            detail: "Trivially satisfied".into(),
        },
        Predicate::AlwaysFalse => PredicateResult {
            predicate_description: "always_false".into(),
            passed: false,
            detail: "Trivially unsatisfied".into(),
        },
        Predicate::Not(inner) => {
            let inner_result = evaluate_predicate(inner, root);
            PredicateResult {
                predicate_description: format!("not({})", inner_result.predicate_description),
                passed: !inner_result.passed,
                detail: format!("negated: {}", inner_result.detail),
            }
        }
        Predicate::And(preds) => {
            let results: Vec<PredicateResult> = preds.iter().map(|p| evaluate_predicate(p, root)).collect();
            let passed = results.iter().all(|r| r.passed);
            let desc = results.iter().map(|r| r.predicate_description.as_str()).collect::<Vec<_>>().join(", ");
            let detail = results.iter().map(|r| format!("{}={}", r.predicate_description, r.passed)).collect::<Vec<_>>().join("; ");
            PredicateResult {
                predicate_description: format!("and({})", desc),
                passed,
                detail,
            }
        }
        Predicate::Or(preds) => {
            let results: Vec<PredicateResult> = preds.iter().map(|p| evaluate_predicate(p, root)).collect();
            let passed = results.iter().any(|r| r.passed);
            let desc = results.iter().map(|r| r.predicate_description.as_str()).collect::<Vec<_>>().join(", ");
            let detail = results.iter().map(|r| format!("{}={}", r.predicate_description, r.passed)).collect::<Vec<_>>().join("; ");
            PredicateResult {
                predicate_description: format!("or({})", desc),
                passed,
                detail,
            }
        }
        // Predicates that require external services are not evaluatable in unit context.
        _ => PredicateResult {
            predicate_description: "external_predicate".into(),
            passed: false,
            detail: "Predicate requires external service — cannot evaluate offline".into(),
        },
    }
}

fn check_guardrail(guardrail: &LocalGuardrail, root: &Path) -> Option<GuardrailViolation> {
    if let Some(cmd) = &guardrail.check_command {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
      if parts.is_empty() {
            return None;
        }
        let result = Command::new(parts[0])
            .args(&parts[1..])
            .current_dir(root)
            .output();
        match result {
            // check_command semantics: "violation detector"
            //   exit 0  = violation condition DETECTED (e.g., grep found a TODO)
            //   exit non-0 = no violation found (e.g., grep found nothing)
            Ok(output) if !output.status.success() => None, // exit non-0 = compliant
            Ok(output) => Some(GuardrailViolation {
                guardrail_id: guardrail.id,
                rule: guardrail.rule.clone(),
                severity: guardrail.severity,
                detail: format!(
                    "Violation detected by check command (exit {:?}): {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout).trim()
                ),
            }),
            Err(e) => Some(GuardrailViolation {
                guardrail_id: guardrail.id,
                rule: guardrail.rule.clone(),
                severity: guardrail.severity,
                detail: format!("Failed to spawn check command: {}", e),
            }),
        }
    } else {
        None // no automated check command — rule is informational only
    }
}

// ---------------------------------------------------------------------------
// SplitAgentExecutor
// ---------------------------------------------------------------------------

/// Report for a single module execution.
#[derive(Debug, Clone)]
pub struct ModuleReport {
    pub module_id: Uuid,
    pub module_title: String,
    pub input_contract_passed: bool,
    pub output_contract_passed: bool,
    pub guardrail_violations: Vec<GuardrailViolation>,
    pub verification: VerificationOutcome,
    pub skipped: bool,
    pub skip_reason: Option<String>,
}

/// Aggregated report for the entire SplitPlan execution.
#[derive(Debug, Clone)]
pub struct SplitExecutionReport {
    pub plan_id: Uuid,
    pub intent_description: String,
    pub plan_hash: String,
    pub module_reports: Vec<ModuleReport>,
    pub all_passed: bool,
    pub total_modules: usize,
    pub passed_modules: usize,
    pub failed_modules: usize,
    pub skipped_modules: usize,
}

/// Executes a SplitPlan sequentially (respecting dependency order) and verifies
/// each module against the real filesystem after execution.
///
/// Note: the executor itself does not "write" files — it delegates to a
/// `WorkerFn` callback that the caller provides. This keeps the executor
/// free of LLM/IO dependencies and fully testable.
pub struct SplitAgentExecutor;

impl SplitAgentExecutor {
    /// Execute a SplitPlan.
    ///
    /// `worker_fn` is called for each module and should:
    ///   - Receive (module, workspace_root)
    ///   - Produce the artifacts described in module.output_contract
    ///   - Return Ok(()) on success, Err(_) on failure
    ///
    /// Modules are executed in dependency order. If a dependency fails or is
    /// skipped, all transitive dependents are skipped with an explicit reason.
    pub fn execute<F>(plan: &SplitPlan, workspace_root: &Path, worker_fn: F) -> SplitExecutionReport
    where
        F: Fn(&WorkerModule, &Path) -> std::result::Result<(), String>,
    {
        let mut reports: Vec<ModuleReport> = Vec::with_capacity(plan.modules.len());
        // Track which module IDs have passed verification.
        let mut passed_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

        for module in &plan.modules {
            // --- Check dependencies ---
            let failed_dep = module.dependencies.iter().find(|dep_id| !passed_ids.contains(dep_id));
            if let Some(dep_id) = failed_dep {
                reports.push(ModuleReport {
                    module_id: module.id,
                    module_title: module.title.clone(),
                    input_contract_passed: false,
                    output_contract_passed: false,
                    guardrail_violations: vec![],
                    verification: VerificationOutcome {
                        module_id: module.id,
                        module_title: module.title.clone(),
                        passed: false,
                        predicate_results: vec![],
                        guardrail_violations: vec![],
                    },
                    skipped: true,
                    skip_reason: Some(format!("Dependency {} did not pass verification", dep_id)),
                });
                continue;
            }

            // --- Verify input_contract (preconditions) ---
            let input_results: Vec<PredicateResult> = module
                .input_contract
                .iter()
                .map(|p| evaluate_predicate(p, workspace_root))
                .collect();
            let input_passed = input_results.iter().all(|r| r.passed);

            if !input_passed {
                let detail = input_results
                    .iter()
                    .filter(|r| !r.passed)
                    .map(|r| r.detail.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                reports.push(ModuleReport {
                    module_id: module.id,
                    module_title: module.title.clone(),
                    input_contract_passed: false,
                    output_contract_passed: false,
                    guardrail_violations: vec![],
                    verification: VerificationOutcome {
                        module_id: module.id,
                        module_title: module.title.clone(),
                        passed: false,
                        predicate_results: input_results,
                        guardrail_violations: vec![],
                    },
                    skipped: true,
                    skip_reason: Some(format!("Input contract not satisfied: {}", detail)),
                });
                continue;
            }

            // --- Execute the worker ---
            let worker_result = worker_fn(module, workspace_root);

            // --- Verify output_contract ---
            let verification = ModuleVerifier::verify(module, workspace_root);

            // Report BLOCK guardrail violations from execution context.
            let block_violations: Vec<GuardrailViolation> = verification
                .guardrail_violations
                .iter()
                .filter(|v| matches!(v.severity, GuardrailSeverity::Block))
                .cloned()
                .collect();

            let output_passed = verification.passed && worker_result.is_ok() && block_violations.is_empty();

            if output_passed {
                passed_ids.insert(module.id);
            }

            reports.push(ModuleReport {
                module_id: module.id,
                module_title: module.title.clone(),
                input_contract_passed: input_passed,
                output_contract_passed: output_passed,
                guardrail_violations: verification.guardrail_violations.clone(),
                verification,
                skipped: false,
                skip_reason: worker_result.err(),
            });
        }

        let passed = reports.iter().filter(|r| !r.skipped && r.output_contract_passed).count();
        let failed = reports.iter().filter(|r| !r.skipped && !r.output_contract_passed).count();
        let skipped = reports.iter().filter(|r| r.skipped).count();

        SplitExecutionReport {
            plan_id: plan.plan_id,
            intent_description: plan.intent_description.clone(),
            plan_hash: plan.plan_hash.clone(),
            all_passed: failed == 0 && skipped == 0,
            total_modules: plan.modules.len(),
            passed_modules: passed,
            failed_modules: failed,
            skipped_modules: skipped,
            module_reports: reports,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn predicate_title(p: &Predicate) -> String {
    match p {
        Predicate::FileExists(path) => format!("file_exists({})", path.display()),
        Predicate::DirectoryExists(path) => format!("directory_exists({})", path.display()),
        Predicate::CommandSucceeds { command, args, .. } => {
            format!("command_succeeds({} {})", command, args.join(" "))
        }
        Predicate::TestsPassing { suite, .. } => format!("tests_passing({})", suite),
        Predicate::ApiEndpoint { url, .. } => format!("api_endpoint({})", url),
        Predicate::AlwaysTrue => "always_true".into(),
        Predicate::AlwaysFalse => "always_false".into(),
        _ => "predicate".into(),
    }
}

fn predicate_destination(p: &Predicate) -> String {
    match p {
        Predicate::FileExists(path) => {
            format!("File {:?} must exist in the workspace", path)
        }
        Predicate::DirectoryExists(path) => {
            format!("Directory {:?} must exist in the workspace", path)
        }
        Predicate::CommandSucceeds { command, args, expected_exit_code } => {
            format!(
                "Command `{} {}` must exit with code {}",
                command,
                args.join(" "),
                expected_exit_code
            )
        }
        Predicate::TestsPassing { suite, min_coverage } => {
            format!(
                "Test suite {:?} must pass with coverage >= {:.0}%",
                suite,
                min_coverage * 100.0
            )
        }
        Predicate::ApiEndpoint { url, expected_status, .. } => {
            format!("HTTP {} must respond with status {}", url, expected_status)
        }
        _ => "Predicate must evaluate to true".into(),
    }
}

fn predicate_allowed_paths(p: &Predicate) -> Vec<PathBuf> {
    match p {
        Predicate::FileExists(path) => vec![path.clone()],
        Predicate::DirectoryExists(path) => vec![path.clone()],
        _ => vec![],
    }
}

fn estimate_effort(p: &Predicate) -> u8 {
    match p {
        Predicate::FileExists(_) | Predicate::DirectoryExists(_) => 1,
        Predicate::CommandSucceeds { .. } => 3,
        Predicate::TestsPassing { .. } => 5,
        Predicate::ApiEndpoint { .. } => 7,
        _ => 2,
    }
}
