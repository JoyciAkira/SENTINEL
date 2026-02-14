//! Outcome Compiler - Intent interpretation and atomic module decomposition
//!
//! This module implements the Outcome Compiler which:
//! 1. Interprets natural language intent into structured OutcomeEnvelope
//! 2. Compiles outcomes into atomic modules with guardrails
//! 3. Generates scaffolding for module implementation
//!
//! # Architecture
//!
//! - **Interpreter**: Parses natural language → OutcomeEnvelope
//! - **Compiler**: Decomposes envelope → atomic modules
//! - **Scaffold**: Generates code templates
//!
//! # Examples
//!
//! ```no_run
//! use sentinel_core::outcome_compiler::{OutcomeInterpreter, AtomicModuleCompiler, InterpretContext};
//!
//! let interpreter = OutcomeInterpreter::new();
//! let envelope = interpreter.interpret(
//!     "Build a task board web app",
//!     &InterpretContext::default()
//! )?;
//!
//! let compiler = AtomicModuleCompiler::new();
//! let result = compiler.compile(&envelope)?;
//!
//! for module in result.modules {
//!     println!("Module: {} - {}", module.module_name, module.objective);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod agent_communication;
pub mod compiler;
pub mod interpreter;
pub mod orchestrator;
pub mod scaffold;
pub mod templates;

pub use agent_communication::{
    AgentCapability, AgentCommunicationBus, AgentHandle, AgentId, AgentInfo, AgentMessage,
    AgentStatus, CollaborativeAgentOrchestrator, HandoffContext, HandoffReason, LearnedPattern,
    MessagePayload, ModuleImplementationStatus, UrgencyLevel,
};
pub use compiler::{
    AtomicModule, AtomicModuleCompiler, CompilationResult, DecompositionAuditLog, GuardrailRuleType,
    GuardrailSeverity, ModuleBoundaries, ModuleGuardrail, ModuleIO, VerificationSpec,
};
pub use interpreter::{
    ExtractedIntent, GateType, InterpretContext, IntentEnvelope, IntentValidator, OutcomeEnvelope,
    OutcomeInterpreter, QualityMetric, RiskEntry, TargetDomain,
};
pub use orchestrator::{
    AgentIdentity, AgentRole, ModuleTask, OrchestrationResult,
    SplitAgentOrchestrator, TaskStatus, WorkflowConfig,
};
pub use scaffold::{
    GuardrailEnforcement, ScaffoldFile, ScaffoldGenerator, ScaffoldGuardrail, ScaffoldMetadata,
    ScaffoldResult,
};
pub use templates::TemplateManager;

// Re-export for convenience
pub use crate::error::Result;
