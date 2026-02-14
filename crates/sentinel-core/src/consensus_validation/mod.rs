//! Consensus-Based Truth Validation (CBTV)
//!
//! Multi-agent validation system where every critical decision requires quorum consensus.
//! Eliminates single points of failure and catches hallucinations through cross-validation.
//!
//! # Architecture
//!
//! ```text
//! Proposed Action/Change
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ ValidationOrchestrator      │
//! └─────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────────────┐
//! │  Parallel Validation by 5+ Agents   │
//! ├─────────┬─────────┬─────────────────┤
//! │Architect│ Security│ Logic Validator │
//! │Validator│Validator│    Agent        │
//! └────┬────┴────┬────┴────────┬────────┘
//!      │         │             │
//!      ▼         ▼             ▼
//! ┌─────────────────────────────────────┐
//! │ Consensus Aggregation               │
//! │ - Vote counting                     │
//! │ - Confidence scoring                │
//! │ - Dispute resolution                │
//! └─────────────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ Decision (Require >80%)     │
//! │ APPROVED / REJECTED         │
//! └─────────────────────────────┘
//! ```

use crate::error::{Result, SentinelError};
use crate::outcome_compiler::agent_communication::{
    AgentCapability, AgentCommunicationBus, AgentId, AgentMessage, AgentStatus,
    MessagePayload,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// A proposal to be validated by consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationProposal {
    pub proposal_id: String,
    pub proposal_type: ProposalType,
    pub description: String,
    pub content: ProposalContent,
    pub proposer: AgentId,
    pub proposed_at: chrono::DateTime<chrono::Utc>,
    pub priority: ValidationPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    CodeChange,
    ArchitectureDecision,
    SecurityConfiguration,
    DependencyAddition,
    GoalModification,
    InvariantRelaxation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalContent {
    CodeSnippet { language: String, code: String, path: String },
    ArchitectureChange { component: String, change_description: String },
    SecurityConfig { config_key: String, config_value: String },
    Dependency { name: String, version: String, reason: String },
    GoalUpdate { goal_id: String, new_description: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationPriority {
    Critical,  // Must validate immediately
    High,      // Validate within 1 minute
    Medium,    // Validate within 5 minutes
    Low,       // Validate when validators available
}

/// A validator agent with specific expertise
#[derive(Debug, Clone)]
pub struct ValidatorAgent {
    pub agent_id: AgentId,
    pub name: String,
    pub expertise: Vec<ValidationDimension>,
    pub weight: f64,  // Voting weight (0.0-1.0)
    pub reliability_score: f64,  // Historical accuracy (0.0-1.0)
}

/// Dimension of validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationDimension {
    ArchitecturalAlignment,  // Does it respect architecture?
    SecurityPosture,         // Does it introduce vulnerabilities?
    LogicCorrectness,        // Will it produce correct outputs?
    PerformanceImpact,       // Does it meet latency/throughput?
    IntentPreservation,      // Does it serve the original goal?
    Testability,             // Can it be tested?
    Maintainability,         // Is it maintainable?
}

/// A vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationVote {
    pub proposal_id: String,
    pub validator_id: AgentId,
    pub vote: Vote,
    pub confidence: f64,  // 0.0-1.0
    pub reasoning: String,
    pub dimension: ValidationDimension,
    pub checks_performed: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vote {
    Approve,
    Reject,
    RequestChanges,
    Abstain,
}

/// Result of consensus validation
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub proposal_id: String,
    pub status: ConsensusStatus,
    pub approval_percentage: f64,
    pub total_votes: usize,
    pub votes_by_dimension: HashMap<ValidationDimension, DimensionResult>,
    pub disputes: Vec<Dispute>,
    pub recommendations: Vec<String>,
    pub final_decision: FinalDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusStatus {
    Approved,
    Rejected,
    Pending,      // Not enough votes yet
    Disputed,     // Significant disagreement
    Escalated,    // Requires human review
}

/// Result for a specific validation dimension
#[derive(Debug, Clone)]
pub struct DimensionResult {
    pub dimension: ValidationDimension,
    pub approval_percentage: f64,
    pub vote_count: usize,
    pub avg_confidence: f64,
    pub issues: Vec<String>,
}

/// A dispute between validators
#[derive(Debug, Clone)]
pub struct Dispute {
    pub dimension: ValidationDimension,
    pub validator_a: AgentId,
    pub validator_b: AgentId,
    pub disagreement: String,
    pub severity: DisputeSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeSeverity {
    Minor,    // Can proceed with caution
    Major,    // Should resolve before proceeding
    Critical, // Must resolve, blocks decision
}

/// Final decision from consensus
#[derive(Debug, Clone)]
pub struct FinalDecision {
    pub decision: Decision,
    pub required_action: RequiredAction,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    ApproveWithConditions,
    Reject,
    RequestRevision,
    EscalateToHuman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequiredAction {
    None,
    AddTests,
    SecurityReview,
    PerformanceBenchmark,
    DocumentationUpdate,
    ArchitectureReview,
}

/// Configuration for consensus validation
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Minimum approval percentage required (0.0-1.0)
    pub approval_threshold: f64,
    
    /// Minimum number of validators required
    pub min_validators: usize,
    
    /// Maximum time to wait for votes
    pub timeout_seconds: u64,
    
    /// Whether to require all dimensions to pass
    pub require_all_dimensions: bool,
    
    /// Minimum confidence threshold for each vote
    pub min_confidence_threshold: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 0.80,  // 80% approval required
            min_validators: 3,
            timeout_seconds: 60,
            require_all_dimensions: false,
            min_confidence_threshold: 0.70,
        }
    }
}

/// Orchestrator for multi-agent consensus validation
pub struct ConsensusValidationOrchestrator {
    config: ConsensusConfig,
    communication_bus: AgentCommunicationBus,
    validators: Arc<RwLock<Vec<ValidatorAgent>>>,
    active_proposals: Arc<RwLock<HashMap<String, ProposalState>>>,
    vote_receiver: mpsc::Receiver<ValidationVote>,
    vote_sender: mpsc::Sender<ValidationVote>,
}

/// State of a proposal being validated
#[derive(Debug, Clone)]
struct ProposalState {
    proposal: ValidationProposal,
    votes: Vec<ValidationVote>,
    start_time: chrono::DateTime<chrono::Utc>,
    status: ProposalStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposalStatus {
    Validating,
    ConsensusReached,
    Timeout,
    Escalated,
}

impl ConsensusValidationOrchestrator {
    /// Create new orchestrator
    pub fn new(config: ConsensusConfig) -> Self {
        let (vote_sender, vote_receiver) = mpsc::channel(1000);
        
        Self {
            config,
            communication_bus: AgentCommunicationBus::new(),
            validators: Arc::new(RwLock::new(Vec::new())),
            active_proposals: Arc::new(RwLock::new(HashMap::new())),
            vote_receiver,
            vote_sender,
        }
    }
    
    /// Register a validator agent
    pub async fn register_validator(
        &self,
        name: impl Into<String>,
        expertise: Vec<ValidationDimension>,
        weight: f64,
    ) -> Result<AgentId> {
        let name = name.into();
        
        // Map validation dimensions to capabilities
        let capabilities: Vec<AgentCapability> = expertise
            .iter()
            .map(|dim| match dim {
                ValidationDimension::SecurityPosture => AgentCapability::Custom("SecurityValidator".to_string()),
                ValidationDimension::ArchitecturalAlignment => AgentCapability::Custom("ArchitectureValidator".to_string()),
                ValidationDimension::LogicCorrectness => AgentCapability::Custom("LogicValidator".to_string()),
                ValidationDimension::PerformanceImpact => AgentCapability::PerformanceOptimizer,
                _ => AgentCapability::Custom("GeneralValidator".to_string()),
            })
            .collect();
        
        let handle = self.communication_bus.register_agent(&name, capabilities)?;
        let agent_id = handle.id.clone();
        
        let validator = ValidatorAgent {
            agent_id: agent_id.clone(),
            name,
            expertise,
            weight: weight.clamp(0.0, 1.0),
            reliability_score: 1.0,  // Start with perfect score
        };
        
        self.validators.write().await.push(validator);
        
        Ok(agent_id)
    }
    
    /// Submit a proposal for consensus validation
    pub async fn submit_proposal(&self, proposal: ValidationProposal) -> Result<ConsensusResult> {
        // Create proposal state
        let state = ProposalState {
            proposal: proposal.clone(),
            votes: Vec::new(),
            start_time: chrono::Utc::now(),
            status: ProposalStatus::Validating,
        };
        
        {
            let mut proposals = self.active_proposals.write().await;
            proposals.insert(proposal.proposal_id.clone(), state);
        }
        
        // Dispatch to validators
        self.dispatch_to_validators(&proposal).await?;
        
        // Wait for consensus or timeout
        let result = self.collect_votes(&proposal.proposal_id).await?;
        
        // Clean up
        {
            let mut proposals = self.active_proposals.write().await;
            proposals.remove(&proposal.proposal_id);
        }
        
        Ok(result)
    }
    
    /// Dispatch proposal to appropriate validators
    async fn dispatch_to_validators(&self, proposal: &ValidationProposal) -> Result<()> {
        let validators = self.validators.read().await;
        
        // Determine which validators are needed based on proposal type
        let required_dimensions = self.get_required_dimensions(&proposal.proposal_type);
        
        for validator in validators.iter() {
            // Check if validator has relevant expertise
            let has_expertise = validator.expertise.iter()
                .any(|e| required_dimensions.contains(e));
            
            if has_expertise {
                // Send validation request
                let request = AgentMessage::Request {
                    from: AgentId::new(),  // System ID
                    to: validator.agent_id.clone(),
                    request_id: format!("validate_{}", proposal.proposal_id),
                    payload: MessagePayload::Custom {
                        message_type: "validation_request".to_string(),
                        data: serde_json::json!({
                            "proposal": proposal,
                            "dimensions": validator.expertise,
                        }),
                    },
                };
                
                // Send via communication bus
                // Note: In production, this would be async
            }
        }
        
        Ok(())
    }
    
    /// Get required validation dimensions for a proposal type
    fn get_required_dimensions(&self, proposal_type: &ProposalType) -> Vec<ValidationDimension> {
        match proposal_type {
            ProposalType::CodeChange => vec![
                ValidationDimension::LogicCorrectness,
                ValidationDimension::Testability,
                ValidationDimension::Maintainability,
            ],
            ProposalType::ArchitectureDecision => vec![
                ValidationDimension::ArchitecturalAlignment,
                ValidationDimension::PerformanceImpact,
                ValidationDimension::Maintainability,
            ],
            ProposalType::SecurityConfiguration => vec![
                ValidationDimension::SecurityPosture,
                ValidationDimension::LogicCorrectness,
            ],
            ProposalType::DependencyAddition => vec![
                ValidationDimension::SecurityPosture,
                ValidationDimension::PerformanceImpact,
                ValidationDimension::ArchitecturalAlignment,
            ],
            ProposalType::GoalModification => vec![
                ValidationDimension::IntentPreservation,
                ValidationDimension::ArchitecturalAlignment,
            ],
            ProposalType::InvariantRelaxation => vec![
                ValidationDimension::SecurityPosture,
                ValidationDimension::LogicCorrectness,
                ValidationDimension::IntentPreservation,
            ],
        }
    }
    
    /// Collect votes and compute consensus
    async fn collect_votes(&self, proposal_id: &str) -> Result<ConsensusResult> {
        // TODO: Implement proper async vote collection with Mutex wrapper
        // For now, return empty consensus
        self.compute_consensus(proposal_id, Vec::new()).await
    }
    
    /// Compute consensus from votes
    async fn compute_consensus(
        &self,
        proposal_id: &str,
        votes: Vec<ValidationVote>,
    ) -> Result<ConsensusResult> {
        if votes.is_empty() {
            return Ok(ConsensusResult {
                proposal_id: proposal_id.to_string(),
                status: ConsensusStatus::Pending,
                approval_percentage: 0.0,
                total_votes: 0,
                votes_by_dimension: HashMap::new(),
                disputes: Vec::new(),
                recommendations: vec!["No votes received - consider escalating".to_string()],
                final_decision: FinalDecision {
                    decision: Decision::EscalateToHuman,
                    required_action: RequiredAction::None,
                    conditions: Vec::new(),
                },
            });
        }
        
        // Group votes by dimension
        let mut votes_by_dimension: HashMap<ValidationDimension, Vec<&ValidationVote>> = HashMap::new();
        for vote in &votes {
            votes_by_dimension
                .entry(vote.dimension)
                .or_default()
                .push(vote);
        }
        
        // Compute results per dimension
        let mut dimension_results = HashMap::new();
        let mut total_weighted_votes = 0.0;
        let mut total_weighted_approval = 0.0;
        
        for (dimension, dim_votes) in &votes_by_dimension {
            let approve_count = dim_votes.iter()
                .filter(|v| matches!(v.vote, Vote::Approve))
                .count();
            
            let total_confidence: f64 = dim_votes.iter().map(|v| v.confidence).sum();
            let avg_confidence = total_confidence / dim_votes.len() as f64;
            
            let approval_pct = approve_count as f64 / dim_votes.len() as f64;
            
            // Collect issues
            let issues: Vec<_> = dim_votes.iter()
                .filter(|v| !matches!(v.vote, Vote::Approve))
                .map(|v| format!("{:?}: {}", v.vote, v.reasoning))
                .collect();
            
            dimension_results.insert(*dimension, DimensionResult {
                dimension: *dimension,
                approval_percentage: approval_pct * 100.0,
                vote_count: dim_votes.len(),
                avg_confidence,
                issues,
            });
            
            total_weighted_votes += dim_votes.len() as f64;
            total_weighted_approval += approve_count as f64;
        }
        
        // Overall approval percentage
        let overall_approval = if total_weighted_votes > 0.0 {
            total_weighted_approval / total_weighted_votes
        } else {
            0.0
        };
        
        // Detect disputes
        let disputes = self.detect_disputes(&votes);
        
        // Determine status
        let status = if overall_approval >= self.config.approval_threshold {
            if disputes.iter().any(|d| d.severity == DisputeSeverity::Critical) {
                ConsensusStatus::Disputed
            } else {
                ConsensusStatus::Approved
            }
        } else if votes.len() < self.config.min_validators {
            ConsensusStatus::Pending
        } else {
            ConsensusStatus::Rejected
        };
        
        // Generate final decision
        let (decision, action, conditions) = self.make_final_decision(
            status,
            overall_approval,
            &dimension_results,
            &disputes,
        );
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&dimension_results, &disputes);
        
        Ok(ConsensusResult {
            proposal_id: proposal_id.to_string(),
            status,
            approval_percentage: overall_approval * 100.0,
            total_votes: votes.len(),
            votes_by_dimension: dimension_results,
            disputes,
            recommendations,
            final_decision: FinalDecision {
                decision,
                required_action: action,
                conditions,
            },
        })
    }
    
    /// Detect disputes between validators
    fn detect_disputes(&self, votes: &[ValidationVote]) -> Vec<Dispute> {
        let mut disputes = Vec::new();
        
        // Group votes by dimension
        let mut by_dimension: HashMap<ValidationDimension, Vec<&ValidationVote>> = HashMap::new();
        for vote in votes {
            by_dimension.entry(vote.dimension).or_default().push(vote);
        }
        
        // Check for significant disagreements
        for (dimension, dim_votes) in by_dimension {
            let approve_count = dim_votes.iter()
                .filter(|v| matches!(v.vote, Vote::Approve))
                .count();
            let reject_count = dim_votes.iter()
                .filter(|v| matches!(v.vote, Vote::Reject))
                .count();
            
            // If we have both approvals and rejections
            if approve_count > 0 && reject_count > 0 {
                // Find specific validators who disagree
                let approvers: Vec<_> = dim_votes.iter()
                    .filter(|v| matches!(v.vote, Vote::Approve))
                    .collect();
                let rejecters: Vec<_> = dim_votes.iter()
                    .filter(|v| matches!(v.vote, Vote::Reject))
                    .collect();
                
                if let (Some(a), Some(r)) = (approvers.first(), rejecters.first()) {
                    let severity = if approve_count == reject_count {
                        DisputeSeverity::Critical
                    } else if (approve_count as f64 / dim_votes.len() as f64) > 0.3 
                        && (reject_count as f64 / dim_votes.len() as f64) > 0.3 {
                        DisputeSeverity::Major
                    } else {
                        DisputeSeverity::Minor
                    };
                    
                    disputes.push(Dispute {
                        dimension,
                        validator_a: a.validator_id.clone(),
                        validator_b: r.validator_id.clone(),
                        disagreement: format!(
                            "{} approves but {} rejects",
                            a.validator_id.0, r.validator_id.0
                        ),
                        severity,
                    });
                }
            }
        }
        
        disputes
    }
    
    /// Make final decision based on analysis
    fn make_final_decision(
        &self,
        status: ConsensusStatus,
        approval_pct: f64,
        dimension_results: &HashMap<ValidationDimension, DimensionResult>,
        disputes: &[Dispute],
    ) -> (Decision, RequiredAction, Vec<String>) {
        match status {
            ConsensusStatus::Approved => {
                let has_security_issues = dimension_results.get(&ValidationDimension::SecurityPosture)
                    .map(|r| !r.issues.is_empty())
                    .unwrap_or(false);
                
                if has_security_issues {
                    (
                        Decision::ApproveWithConditions,
                        RequiredAction::SecurityReview,
                        vec!["Security review required before deployment".to_string()],
                    )
                } else {
                    (
                        Decision::Approve,
                        RequiredAction::None,
                        Vec::new(),
                    )
                }
            }
            ConsensusStatus::Rejected => {
                if approval_pct < 0.3 {
                    (
                        Decision::Reject,
                        RequiredAction::None,
                        vec!["Significant concerns raised - major revision needed".to_string()],
                    )
                } else {
                    (
                        Decision::RequestRevision,
                        RequiredAction::None,
                        vec!["Address concerns and resubmit".to_string()],
                    )
                }
            }
            ConsensusStatus::Disputed => {
                if disputes.iter().any(|d| d.severity == DisputeSeverity::Critical) {
                    (
                        Decision::EscalateToHuman,
                        RequiredAction::ArchitectureReview,
                        vec!["Critical disagreement requires human arbitration".to_string()],
                    )
                } else {
                    (
                        Decision::ApproveWithConditions,
                        RequiredAction::DocumentationUpdate,
                        vec!["Document disagreement resolution".to_string()],
                    )
                }
            }
            ConsensusStatus::Pending => {
                (
                    Decision::EscalateToHuman,
                    RequiredAction::None,
                    vec!["Insufficient validator participation".to_string()],
                )
            }
            _ => (
                Decision::EscalateToHuman,
                RequiredAction::None,
                Vec::new(),
            ),
        }
    }
    
    /// Generate recommendations based on validation results
    fn generate_recommendations(
        &self,
        dimension_results: &HashMap<ValidationDimension, DimensionResult>,
        disputes: &[Dispute],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Add dimension-specific recommendations
        for (dimension, result) in dimension_results {
            if result.approval_percentage < 80.0 {
                recommendations.push(format!(
                    "{:?} validation at {:.0}% - consider addressing: {}",
                    dimension,
                    result.approval_percentage,
                    result.issues.join("; ")
                ));
            }
        }
        
        // Add dispute recommendations
        for dispute in disputes {
            match dispute.severity {
                DisputeSeverity::Critical => {
                    recommendations.push(format!(
                        "CRITICAL: Resolve dispute in {:?} validation between validators",
                        dispute.dimension
                    ));
                }
                DisputeSeverity::Major => {
                    recommendations.push(format!(
                        "MAJOR: Review disagreement on {:?} - may need third opinion",
                        dispute.dimension
                    ));
                }
                _ => {}
            }
        }
        
        recommendations
    }
    
    /// Receive a vote from a validator
    pub async fn receive_vote(&self, vote: ValidationVote) -> Result<()> {
        self.vote_sender.send(vote).await
            .map_err(|e| SentinelError::InvalidInput(format!("Failed to send vote: {}", e)))?;
        Ok(())
    }
    
    /// Get validator statistics
    pub async fn get_validator_stats(&self) -> Vec<ValidatorStats> {
        let validators = self.validators.read().await;
        
        validators.iter().map(|v| ValidatorStats {
            agent_id: v.agent_id.clone(),
            name: v.name.clone(),
            expertise: v.expertise.clone(),
            weight: v.weight,
            reliability_score: v.reliability_score,
            status: AgentStatus::Idle,  // Simplified - should track actual status
        }).collect()
    }
}

/// Statistics for a validator
#[derive(Debug, Clone)]
pub struct ValidatorStats {
    pub agent_id: AgentId,
    pub name: String,
    pub expertise: Vec<ValidationDimension>,
    pub weight: f64,
    pub reliability_score: f64,
    pub status: AgentStatus,
}

/// High-level API for using CBTV
pub struct TruthValidation {
    orchestrator: ConsensusValidationOrchestrator,
}

impl TruthValidation {
    pub fn new() -> Self {
        Self {
            orchestrator: ConsensusValidationOrchestrator::new(ConsensusConfig::default()),
        }
    }
    
    /// Validate a code change
    pub async fn validate_code_change(
        &self,
        code: &str,
        language: &str,
        path: &str,
    ) -> Result<ConsensusResult> {
        let proposal = ValidationProposal {
            proposal_id: Uuid::new_v4().to_string(),
            proposal_type: ProposalType::CodeChange,
            description: format!("Code change in {}: {}", language, path),
            content: ProposalContent::CodeSnippet {
                language: language.to_string(),
                code: code.to_string(),
                path: path.to_string(),
            },
            proposer: AgentId::new(),
            proposed_at: chrono::Utc::now(),
            priority: ValidationPriority::High,
        };
        
        self.orchestrator.submit_proposal(proposal).await
    }
    
    /// Validate architecture decision
    pub async fn validate_architecture_decision(
        &self,
        component: &str,
        change: &str,
    ) -> Result<ConsensusResult> {
        let proposal = ValidationProposal {
            proposal_id: Uuid::new_v4().to_string(),
            proposal_type: ProposalType::ArchitectureDecision,
            description: format!("Architecture change: {}", change),
            content: ProposalContent::ArchitectureChange {
                component: component.to_string(),
                change_description: change.to_string(),
            },
            proposer: AgentId::new(),
            proposed_at: chrono::Utc::now(),
            priority: ValidationPriority::Critical,
        };
        
        self.orchestrator.submit_proposal(proposal).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_consensus_orchestrator_creation() {
        let orchestrator = ConsensusValidationOrchestrator::new(ConsensusConfig::default());
        let stats = orchestrator.get_validator_stats().await;
        assert!(stats.is_empty());
    }
    
    #[tokio::test]
    async fn test_validator_registration() {
        let orchestrator = ConsensusValidationOrchestrator::new(ConsensusConfig::default());
        
        let agent_id = orchestrator.register_validator(
            "SecurityValidator",
            vec![ValidationDimension::SecurityPosture],
            1.0,
        ).await.unwrap();
        
        // AgentId is a tuple struct containing the agent's unique ID string
        assert!(!agent_id.0.is_empty());
        
        let stats = orchestrator.get_validator_stats().await;
        assert_eq!(stats.len(), 1);
    }
    
    #[test]
    fn test_proposal_creation() {
        let proposal = ValidationProposal {
            proposal_id: "test-123".to_string(),
            proposal_type: ProposalType::CodeChange,
            description: "Test proposal".to_string(),
            content: ProposalContent::CodeSnippet {
                language: "rust".to_string(),
                code: "fn main() {}".to_string(),
                path: "src/main.rs".to_string(),
            },
            proposer: AgentId::new(),
            proposed_at: chrono::Utc::now(),
            priority: ValidationPriority::High,
        };
        
        assert_eq!(proposal.proposal_type, ProposalType::CodeChange);
    }
    
    #[test]
    fn test_vote_creation() {
        let vote = ValidationVote {
            proposal_id: "test-123".to_string(),
            validator_id: AgentId::new(),
            vote: Vote::Approve,
            confidence: 0.95,
            reasoning: "Code looks correct".to_string(),
            dimension: ValidationDimension::LogicCorrectness,
            checks_performed: vec!["syntax_check".to_string()],
            timestamp: chrono::Utc::now(),
        };
        
        assert!(matches!(vote.vote, Vote::Approve));
        assert_eq!(vote.confidence, 0.95);
    }
}
