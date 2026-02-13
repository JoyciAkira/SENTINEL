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
//! use sentinel_core::outcome_compiler::{OutcomeInterpreter, AtomicModuleCompiler};
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

pub mod compiler;
pub mod interpreter;
pub mod scaffold;
pub mod templates;

pub use compiler::{
    AtomicModule, AtomicModuleCompiler, CompilationResult, DecompositionAuditLog, GuardrailRuleType,
    GuardrailSeverity, ModuleBoundaries, ModuleGuardrail, ModuleIO, VerificationSpec,
};
pub use interpreter::{
    ExtractedIntent, GateType, InterpretContext, IntentEnvelope, IntentValidator, OutcomeEnvelope,
    OutcomeInterpreter, QualityMetric, RiskEntry, TargetDomain,
};
pub use scaffold::ScaffoldGenerator;
pub use templates::TemplateManager;

// Re-export for convenience
pub use crate::error::Result;
