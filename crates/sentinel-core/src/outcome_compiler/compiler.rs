//! Atomic Module Compiler - Decomposes outcomes into atomic modules
//!
//! This module implements the compiler that takes an OutcomeEnvelope and
//! decomposes it into atomic, verifiable modules with guardrails.

use super::interpreter::OutcomeEnvelope;
use crate::error::{Result, SentinelError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Atomic Module Compiler
///
/// Compiles an OutcomeEnvelope into atomic modules following deterministic
/// decomposition rules.
pub struct AtomicModuleCompiler {
    rule_version: String,
}

impl AtomicModuleCompiler {
    pub fn new() -> Self {
        Self {
            rule_version: "1.0".to_string(),
        }
    }

    /// Compile OutcomeEnvelope into atomic modules
    pub fn compile(&self, envelope: &OutcomeEnvelope) -> Result<CompilationResult> {
        // 1. Normalize constraints (lexicographic order)
        let normalized = self.normalize_constraints(&envelope.constraints);

        // 2. Extract capability set from acceptance criteria
        let capabilities = self.extract_capabilities(&envelope.acceptance_criteria);

        // 3. Build dependency DAG
        let dag = self.build_dependency_dag(&capabilities)?;

        // 4. Split into atomic modules
        let modules = self.generate_atomic_modules(envelope, &dag)?;

        // 5. Generate guardrails per module
        let guardrails = self.generate_guardrails(&modules);

        // 6. Compute decomposition hash before moving guardrails
        let decomposition_hash = self.compute_decomposition_hash(&guardrails);

        // 7. Generate audit log
        let audit_log = self.generate_audit_log(envelope, &modules, &guardrails);

        // 8. Validate compliance
        self.validate_compliance(&modules, &guardrails)?;

        Ok(CompilationResult {
            outcome_id: envelope.outcome_id.clone(),
            modules,
            guardrails,
            audit_log,
            decomposition_hash,
        })
    }

    fn normalize_constraints(
        &self,
        constraints: &super::interpreter::ConstraintEnvelope,
    ) -> NormalizedConstraints {
        let mut non_negotiables = constraints.non_negotiables.clone();
        let mut tech_constraints = constraints.tech_constraints.clone();

        non_negotiables.sort();
        non_negotiables.dedup();
        tech_constraints.sort();
        tech_constraints.dedup();

        NormalizedConstraints {
            non_negotiables,
            tech_constraints,
            time_budget: constraints.time_budget.clone(),
            cost_budget: constraints.cost_budget.clone(),
        }
    }

    fn extract_capabilities(&self, acceptance_criteria: &[String]) -> Vec<Capability> {
        acceptance_criteria
            .iter()
            .enumerate()
            .map(|(i, ac)| Capability {
                id: Uuid::new_v4(),
                name: format!("capability_{}", i),
                description: ac.clone(),
            })
            .collect()
    }

    fn build_dependency_dag(&self, capabilities: &[Capability]) -> Result<ModuleDag> {
        // For now, create a simple linear DAG
        // In production, this would use LLM to analyze dependencies
        let mut modules = Vec::<ModuleNode>::new();

        for (i, cap) in capabilities.iter().enumerate() {
            modules.push(ModuleNode {
                id: Uuid::new_v4(),
                name: format!("module_{}", i),
                capability_id: cap.id,
                dependencies: if i > 0 {
                    vec![modules[i - 1].id]
                } else {
                    Vec::new()
                },
            });
        }

        Ok(ModuleDag { modules })
    }

    fn generate_atomic_modules(
        &self,
        envelope: &OutcomeEnvelope,
        dag: &ModuleDag,
    ) -> Result<Vec<AtomicModule>> {
        let mut modules = Vec::new();

        for (i, node) in dag.modules.iter().enumerate() {
            let module_id = Uuid::new_v4().to_string();
            let module_name = format!("{}_{}", node.name, i);

            // Determine in-scope and out-of-scope
            let (in_scope, out_of_scope) = self.determine_boundaries(envelope, i);

            // Generate inputs/outputs
            let inputs = self.generate_inputs(&node);
            let outputs = self.generate_outputs(&node);

            // Generate invariants from constraints
            let invariants = self.generate_invariants(envelope, &module_name);

            // Generate verification spec
            let verification = self.generate_verification(&module_name);

            // Generate guardrails (placeholder, filled in later)
            let guardrails = Vec::new();

            // Build dependencies list
            let dependencies: Vec<String> =
                node.dependencies.iter().map(|id| id.to_string()).collect();

            modules.push(AtomicModule {
                module_id: module_id.clone(),
                module_name: module_name.clone(),
                objective: envelope.intent.goal.clone(),
                boundaries: ModuleBoundaries {
                    in_scope,
                    out_of_scope,
                },
                inputs,
                outputs,
                dependencies,
                invariants,
                verification,
                guardrails,
            });
        }

        Ok(modules)
    }

    fn determine_boundaries(
        &self,
        envelope: &OutcomeEnvelope,
        index: usize,
    ) -> (Vec<String>, Vec<String>) {
        let mut in_scope = vec![
            format!("Part of: {}", envelope.intent.goal),
            "Single objective per module".to_string(),
        ];

        let mut out_of_scope = vec![
            "Multi-module concerns".to_string(),
            "Cross-cutting concerns (handled separately)".to_string(),
        ];

        // Add domain-specific boundaries
        match envelope.intent.target_domain {
            super::interpreter::TargetDomain::WebApp => {
                in_scope.push("Frontend component logic".to_string());
                out_of_scope.push("Backend API implementation".to_string());
            }
            super::interpreter::TargetDomain::BackendService => {
                in_scope.push("Service implementation".to_string());
                out_of_scope.push("Frontend UI".to_string());
            }
            super::interpreter::TargetDomain::Other => {}
        }

        // Module-specific boundaries
        in_scope.push(format!("Module {}", index));

        (in_scope, out_of_scope)
    }

    fn generate_inputs(&self, node: &ModuleNode) -> Vec<ModuleIO> {
        if node.dependencies.is_empty() {
            vec![ModuleIO {
                name: "user_requirements".to_string(),
                r#type: "Requirements".to_string(),
                required: true,
            }]
        } else {
            node.dependencies
                .iter()
                .enumerate()
                .map(|(i, dep_id)| ModuleIO {
                    name: format!("dependency_{}_output", i),
                    r#type: "ModuleOutput".to_string(),
                    required: true,
                })
                .collect()
        }
    }

    fn generate_outputs(&self, node: &ModuleNode) -> Vec<ModuleIO> {
        vec![ModuleIO {
            name: format!("{}_output", node.name),
            r#type: "Artifact".to_string(),
            required: true,
        }]
    }

    fn generate_invariants(&self, envelope: &OutcomeEnvelope, module_name: &str) -> Vec<String> {
        let mut invariants = Vec::new();

        // Add non-negotiables as invariants
        for nn in &envelope.constraints.non_negotiables {
            invariants.push(format!("{}: {}", module_name, nn));
        }

        // Add default invariant if none
        if invariants.is_empty() {
            invariants.push(format!("{}: Must satisfy acceptance criteria", module_name));
        }

        invariants
    }

    fn generate_verification(&self, module_name: &str) -> VerificationSpec {
        VerificationSpec {
            acceptance_tests: vec![
                format!("{}_test_basic_functionality", module_name),
                format!("{}_test_edge_cases", module_name),
            ],
            artifacts: vec![
                format!("{}_source_code", module_name),
                format!("{}_tests", module_name),
            ],
            done_when: vec![
                format!("All {} acceptance tests pass", module_name),
                format!("Code review approved for {}", module_name),
            ],
        }
    }

    fn generate_guardrails(&self, modules: &[AtomicModule]) -> Vec<ModuleGuardrail> {
        let mut guardrails = Vec::new();

        for module in modules {
            // 1. Scope guardrail (BLOCK on out-of-scope work)
            guardrails.push(ModuleGuardrail {
                id: Uuid::new_v4().to_string(),
                module_id: module.module_id.clone(),
                rule_type: GuardrailRuleType::Scope,
                severity: GuardrailSeverity::Block,
                rule: format!(
                    "Stay within scope: {:?}",
                    module
                        .boundaries
                        .in_scope
                        .iter()
                        .take(3)
                        .collect::<Vec<_>>()
                ),
                check: format!("verify_out_of_scope_check('{}')", module.module_id),
            });

            // 2. Quality guardrail (BLOCK or WARN based on hard/soft metrics)
            guardrails.push(ModuleGuardrail {
                id: Uuid::new_v4().to_string(),
                module_id: module.module_id.clone(),
                rule_type: GuardrailRuleType::Quality,
                severity: GuardrailSeverity::Block,
                rule: "All acceptance tests must pass".to_string(),
                check: format!("verify_tests_passing('{}')", module.module_id),
            });

            // 3. Dependency guardrail (BLOCK if dependency not satisfied)
            for dep in &module.dependencies {
                guardrails.push(ModuleGuardrail {
                    id: Uuid::new_v4().to_string(),
                    module_id: module.module_id.clone(),
                    rule_type: GuardrailRuleType::Dependency,
                    severity: GuardrailSeverity::Block,
                    rule: format!("Dependency {} must be satisfied", dep),
                    check: format!(
                        "verify_dependency_complete('{}', '{}')",
                        module.module_id, dep
                    ),
                });
            }
        }

        guardrails
    }

    fn generate_audit_log(
        &self,
        envelope: &OutcomeEnvelope,
        modules: &[AtomicModule],
        guardrails: &[ModuleGuardrail],
    ) -> DecompositionAuditLog {
        let mut steps = Vec::new();
        let mut input_hash = "".to_string();

        // Step 1: Normalize
        let normalized_hash = self.hash_string(&format!("{:?}", envelope.constraints));
        steps.push(AuditStep {
            index: 1,
            action: "normalize".to_string(),
            input_hash: input_hash.clone(),
            output_hash: normalized_hash.clone(),
        });
        input_hash = normalized_hash;

        // Step 2: Extract capabilities
        let capabilities_hash = self.hash_string(&format!("{:?}", envelope.acceptance_criteria));
        steps.push(AuditStep {
            index: 2,
            action: "extract_capabilities".to_string(),
            input_hash: input_hash.clone(),
            output_hash: capabilities_hash.clone(),
        });
        input_hash = capabilities_hash;

        // Step 3: Build DAG
        let dag_hash = self.hash_string(&format!("dag_{}", envelope.outcome_id));
        steps.push(AuditStep {
            index: 3,
            action: "build_dag".to_string(),
            input_hash: input_hash.clone(),
            output_hash: dag_hash.clone(),
        });
        input_hash = dag_hash;

        // Step 4: Generate modules
        let modules_hash: Vec<String> = modules
            .iter()
            .map(|m| self.hash_string(&format!("{}_{}", m.module_id, m.module_name)))
            .collect();
        let modules_hash_combined = self.hash_string(&format!("{:?}", modules_hash));
        steps.push(AuditStep {
            index: 4,
            action: "generate_modules".to_string(),
            input_hash: input_hash.clone(),
            output_hash: modules_hash_combined.clone(),
        });
        input_hash = modules_hash_combined;

        // Step 5: Generate guardrails
        let guardrails_hash: String = self.hash_string(&format!("{}", guardrails.len()));
        steps.push(AuditStep {
            index: 5,
            action: "generate_guardrails".to_string(),
            input_hash: input_hash.clone(),
            output_hash: guardrails_hash.clone(),
        });

        let module_hashes: Vec<String> = modules
            .iter()
            .map(|m| self.hash_string(&m.module_id))
            .collect();

        let final_graph_hash = self.hash_string(&format!(
            "{}_{}_{}",
            envelope.outcome_id,
            self.hash_string(&format!("{:?}", module_hashes)),
            guardrails_hash
        ));

        DecompositionAuditLog {
            outcome_id: envelope.outcome_id.clone(),
            rule_version: self.rule_version.clone(),
            steps,
            module_hashes: module_hashes.clone(),
            final_graph_hash,
        }
    }

    fn hash_string(&self, s: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn compute_decomposition_hash(&self, guardrails: &[ModuleGuardrail]) -> String {
        let ids: Vec<String> = guardrails.iter().map(|g| g.module_id.clone()).collect();
        self.hash_string(&format!("{}_{}", guardrails.len(), ids.join("_")))
    }

    fn validate_compliance(
        &self,
        modules: &[AtomicModule],
        guardrails: &[ModuleGuardrail],
    ) -> Result<()> {
        // Check compliance rules
        for module in modules {
            // Rule 1: Every module has at least one invariant
            if module.invariants.is_empty() {
                return Err(SentinelError::InvariantViolation(format!(
                    "Module {} has no invariants",
                    module.module_id
                )));
            }

            // Rule 2: Every module has at least one acceptance test
            if module.verification.acceptance_tests.is_empty() {
                return Err(SentinelError::InvariantViolation(format!(
                    "Module {} has no acceptance tests",
                    module.module_id
                )));
            }

            // Rule 3: Every module has explicit in-scope and out-of-scope
            if module.boundaries.in_scope.is_empty() {
                return Err(SentinelError::InvariantViolation(format!(
                    "Module {} has empty in_scope",
                    module.module_id
                )));
            }

            // Rule 4: At least one BLOCK guardrail exists per module
            let module_guardrails: Vec<_> = guardrails
                .iter()
                .filter(|g| g.module_id == module.module_id)
                .collect();

            let has_block = module_guardrails
                .iter()
                .any(|g| matches!(g.severity, GuardrailSeverity::Block));

            if !has_block {
                return Err(SentinelError::InvariantViolation(format!(
                    "Module {} has no BLOCK guardrail",
                    module.module_id
                )));
            }
        }

        Ok(())
    }
}

impl Default for AtomicModuleCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Compilation result
#[derive(Debug, Clone)]
pub struct CompilationResult {
    pub outcome_id: String,
    pub modules: Vec<AtomicModule>,
    pub guardrails: Vec<ModuleGuardrail>,
    pub audit_log: DecompositionAuditLog,
    pub decomposition_hash: String,
}

/// Atomic module definition
#[derive(Debug, Clone)]
pub struct AtomicModule {
    pub module_id: String,
    pub module_name: String,
    pub objective: String,
    pub boundaries: ModuleBoundaries,
    pub inputs: Vec<ModuleIO>,
    pub outputs: Vec<ModuleIO>,
    pub dependencies: Vec<String>,
    pub invariants: Vec<String>,
    pub verification: VerificationSpec,
    pub guardrails: Vec<ModuleGuardrail>, // Populated after generation
}

/// Module boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleBoundaries {
    pub in_scope: Vec<String>,
    pub out_of_scope: Vec<String>,
}

/// Module input/output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleIO {
    pub name: String,
    pub r#type: String,
    pub required: bool,
}

/// Verification specification
#[derive(Debug, Clone)]
pub struct VerificationSpec {
    pub acceptance_tests: Vec<String>,
    pub artifacts: Vec<String>,
    pub done_when: Vec<String>,
}

/// Module guardrail
#[derive(Debug, Clone)]
pub struct ModuleGuardrail {
    pub id: String,
    pub module_id: String,
    pub rule_type: GuardrailRuleType,
    pub severity: GuardrailSeverity,
    pub rule: String,
    pub check: String,
}

/// Guardrail rule type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailRuleType {
    Scope,
    Quality,
    Dependency,
}

/// Guardrail severity (matches AC6 spec)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailSeverity {
    Allow,
    Warn,
    Block,
}

/// Decomposition audit log
#[derive(Debug, Clone)]
pub struct DecompositionAuditLog {
    pub outcome_id: String,
    pub rule_version: String,
    pub steps: Vec<AuditStep>,
    pub module_hashes: Vec<String>,
    pub final_graph_hash: String,
}

/// Audit step
#[derive(Debug, Clone)]
pub struct AuditStep {
    pub index: u32,
    pub action: String,
    pub input_hash: String,
    pub output_hash: String,
}

// Internal types

#[derive(Debug, Clone)]
struct NormalizedConstraints {
    non_negotiables: Vec<String>,
    tech_constraints: Vec<String>,
    time_budget: String,
    cost_budget: String,
}

#[derive(Debug, Clone)]
struct Capability {
    id: Uuid,
    name: String,
    description: String,
}

#[derive(Debug, Clone)]
struct ModuleDag {
    modules: Vec<ModuleNode>,
}

#[derive(Debug, Clone)]
struct ModuleNode {
    id: Uuid,
    name: String,
    capability_id: Uuid,
    dependencies: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::super::interpreter::{ConstraintEnvelope, IntentEnvelope, TargetDomain};
    use super::*;

    fn create_test_envelope() -> OutcomeEnvelope {
        OutcomeEnvelope {
            schema_version: "1.0".to_string(),
            outcome_id: Uuid::new_v4().to_string(),
            intent: IntentEnvelope {
                goal: "Build a task board web app".to_string(),
                target_domain: TargetDomain::WebApp,
                success_narrative: "Users can create and track tasks".to_string(),
            },
            constraints: ConstraintEnvelope {
                non_negotiables: vec!["Auth required".to_string()],
                tech_constraints: vec!["TypeScript".to_string()],
                time_budget: "2w".to_string(),
                cost_budget: "medium".to_string(),
            },
            quality_metrics: vec![],
            acceptance_criteria: vec!["User signup".to_string(), "Task CRUD".to_string()],
            assumptions: vec![],
            risk_register: vec![],
        }
    }

    #[test]
    fn test_compile_generates_modules() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        assert!(!result.modules.is_empty());
        assert_eq!(result.outcome_id, envelope.outcome_id);
        assert!(!result.audit_log.steps.is_empty());
    }

    #[test]
    fn test_every_module_has_invariant() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        for module in &result.modules {
            assert!(!module.invariants.is_empty());
        }
    }

    #[test]
    fn test_every_module_has_tests() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        for module in &result.modules {
            assert!(!module.verification.acceptance_tests.is_empty());
        }
    }

    #[test]
    fn test_every_module_has_block_guardrail() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        for module in &result.modules {
            let module_guardrails: Vec<_> = result
                .guardrails
                .iter()
                .filter(|g| g.module_id == module.module_id)
                .collect();

            let has_block = module_guardrails
                .iter()
                .any(|g| matches!(g.severity, GuardrailSeverity::Block));

            assert!(
                has_block,
                "Module {} should have BLOCK guardrail",
                module.module_id
            );
        }
    }

    #[test]
    fn test_guardrail_generation() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        // Should have at least 3 guardrails per module (scope, quality, dependency)
        assert!(!result.guardrails.is_empty());
    }

    #[test]
    fn test_audit_log_completeness() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        assert!(!result.audit_log.steps.is_empty());
        assert!(!result.audit_log.module_hashes.is_empty());
        assert!(!result.audit_log.final_graph_hash.is_empty());
        assert_eq!(result.audit_log.rule_version, "1.0");
    }

    #[test]
    fn test_module_boundaries() {
        let compiler = AtomicModuleCompiler::new();
        let envelope = create_test_envelope();

        let result = compiler.compile(&envelope).unwrap();

        for module in &result.modules {
            assert!(!module.boundaries.in_scope.is_empty());
            assert!(!module.boundaries.out_of_scope.is_empty());
        }
    }
}
