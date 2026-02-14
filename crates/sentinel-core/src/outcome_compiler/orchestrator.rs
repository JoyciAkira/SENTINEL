//! Split Agent Orchestrator - Coordinates Architect and Worker agents
//!
//! This module implements the split agent architecture where:
//! 1. **Architect Agent** interprets user intent and generates atomic module scaffolding
//! 2. **Worker Agents** implement each atomic module within strict guardrails

use crate::error::Result;
use crate::outcome_compiler::{
    AtomicModuleCompiler, CompilationResult, InterpretContext, OutcomeEnvelope, OutcomeInterpreter,
    ScaffoldGenerator, ScaffoldResult,
};
use std::path::PathBuf;

/// Agent role in the split architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Architect,
    Worker,
}

/// Agent identity
#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub role: AgentRole,
    pub module_id: Option<String>,
}

/// Task status for worker coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Blocked,
}

/// Module implementation task
#[derive(Debug, Clone)]
pub struct ModuleTask {
    pub task_id: String,
    pub module_id: String,
    pub module_name: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
}

/// Split Agent Orchestrator
pub struct SplitAgentOrchestrator {
    interpreter: OutcomeInterpreter,
    compiler: AtomicModuleCompiler,
    scaffold_generator: ScaffoldGenerator,
}

/// Workflow configuration
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    pub language: String,
    pub framework: String,
    pub output_directory: PathBuf,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            language: "rust".to_string(),
            framework: "axum".to_string(),
            output_directory: PathBuf::from("./generated"),
        }
    }
}

/// Orchestration result
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub project_id: String,
    pub success: bool,
    pub modules_generated: usize,
    pub modules_implemented: usize,
    pub output_directory: PathBuf,
    pub errors: Vec<String>,
}

impl SplitAgentOrchestrator {
    pub fn new(language: &str, framework: &str) -> Self {
        Self {
            interpreter: OutcomeInterpreter::new(),
            compiler: AtomicModuleCompiler::new(),
            scaffold_generator: ScaffoldGenerator::new(language, framework),
        }
    }

    pub fn execute_workflow_blocking(
        &self,
        intent: &str,
        context: &InterpretContext,
        config: &WorkflowConfig,
    ) -> Result<OrchestrationResult> {
        // Phase 1: Intent Interpretation (Architect Agent)
        let envelope = self.interpret_intent(intent, context)?;

        // Phase 2: Module Compilation (Architect Agent)
        let compilation = self.compile_modules(&envelope)?;

        // Phase 3: Scaffold Generation (Architect Agent)
        let scaffolds = self.generate_scaffolds(&compilation)?;

        // Phase 4: Emit scaffolds to filesystem
        let mut emitted_count = 0;
        for scaffold in &scaffolds {
            match self
                .scaffold_generator
                .emit_scaffold(scaffold, &config.output_directory)
            {
                Ok(files) => emitted_count += files.len(),
                Err(e) => eprintln!("Emit error: {}", e),
            }
        }

        Ok(OrchestrationResult {
            project_id: envelope.outcome_id,
            success: true,
            modules_generated: compilation.modules.len(),
            modules_implemented: emitted_count,
            output_directory: config.output_directory.clone(),
            errors: Vec::new(),
        })
    }

    fn interpret_intent(
        &self,
        intent: &str,
        context: &InterpretContext,
    ) -> Result<OutcomeEnvelope> {
        let envelope = self.interpreter.interpret(intent, context)?;
        Ok(envelope)
    }

    fn compile_modules(&self, envelope: &OutcomeEnvelope) -> Result<CompilationResult> {
        let result = self.compiler.compile(envelope)?;
        Ok(result)
    }

    fn generate_scaffolds(&self, compilation: &CompilationResult) -> Result<Vec<ScaffoldResult>> {
        let mut scaffolds = Vec::new();

        for module in &compilation.modules {
            let scaffold = self.scaffold_generator.generate_scaffold(module)?;
            scaffolds.push(scaffold);
        }

        Ok(scaffolds)
    }
}

impl Default for SplitAgentOrchestrator {
    fn default() -> Self {
        Self::new("rust", "axum")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_creation() {
        let _orchestrator = SplitAgentOrchestrator::new("rust", "axum");
    }

    #[test]
    fn test_workflow_config_default() {
        let config = WorkflowConfig::default();
        assert_eq!(config.language, "rust");
        assert_eq!(config.framework, "axum");
    }
}
