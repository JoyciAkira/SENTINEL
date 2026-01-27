//! Native Sentinel Agent - Revolutionary Coding Agent with Integrated Sentinel OS

pub mod codegen;
pub mod consensus;
pub mod context;
pub mod orchestrator;
pub mod planning;
pub mod reasoning;

pub use codegen::TreeSitterGenerator;
pub use consensus::P2PConsensus;
pub use context::ContextManager;
pub use orchestrator::AgentOrchestrator;
pub use planning::HierarchicalPlanner;
pub use reasoning::StructuredReasoner;

pub use crate::SentinelAgent;
