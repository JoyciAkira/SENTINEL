//! Intent-Preserving Guardrails (IPG)
//!
//! This module implements the Intent-Preserving Guardrails system that prevents
//! goal drift through cryptographic intent anchoring and continuous alignment monitoring.
//!
//! # Architecture
//!
//! ```text
//! Initial Intent (Immutable Root)
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ IntentAnchor (Blake3 Hash)  │
//! └─────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ Goal Manifold (DAG)         │
//! │  ├── Root Goal              │
//! │  ├── Sub-goal 1             │
//! │  └── Sub-goal 2             │
//! └─────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ DriftDetector               │
//! │  - Monitors every action    │
//! │  - Computes alignment score │
//! │  - Triggers on drift        │
//! └─────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────┐
//! │ Guardrail Action            │
//! │  BLOCK (< 50%)              │
//! │  WARN (50-80%)              │
//! │  SUGGEST (> 80%)            │
//! └─────────────────────────────┘
//! ```

use crate::error::Result;
use crate::goal_manifold::{Goal, GoalManifold};
use crate::GoalStatus;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Immutable anchor of the original intent
/// 
/// This is the cryptographic root of truth that never changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnchor {
    /// Unique identifier for this intent
    pub anchor_id: String,
    
    /// The original natural language intent
    pub original_intent: String,
    
    /// Blake3 hash of the original intent + constraints
    pub integrity_hash: String,
    
    /// Timestamp when intent was anchored
    pub anchored_at: chrono::DateTime<chrono::Utc>,
    
    /// Non-negotiable constraints extracted from intent
    pub constraints: Vec<Constraint>,
    
    /// Success criteria that must be met
    pub success_criteria: Vec<SuccessCriterion>,
}

/// A constraint extracted from the intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub description: String,
    pub constraint_type: ConstraintType,
    pub severity: ConstraintSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    Technical,    // Language, framework, etc.
    Security,     // Security requirements
    Performance,  // Latency, throughput
    Compliance,   // Legal, regulatory
    Domain,       // Business domain rules
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    Critical,  // Cannot be violated under any circumstances
    High,      // Should not be violated
    Medium,    // Prefer not to violate
    Low,       // Nice to have
}

/// Success criterion for the intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriterion {
    pub id: String,
    pub description: String,
    pub validation_method: ValidationMethod,
    pub is_mandatory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationMethod {
    TestPass { test_name: String },
    MetricThreshold { metric: String, threshold: f64 },
    HumanReview { reviewer_role: String },
    AutomatedCheck { check_id: String },
}

impl IntentAnchor {
    /// Create a new intent anchor from natural language
    pub fn from_intent(intent: impl Into<String>) -> Self {
        let intent_str = intent.into();
        let anchor_id = Uuid::new_v4().to_string();
        
        // Compute cryptographic hash
        let mut hasher = Hasher::new();
        hasher.update(intent_str.as_bytes());
        hasher.update(anchor_id.as_bytes());
        let integrity_hash = hasher.finalize().to_hex().to_string();
        
        // Extract constraints using NLP patterns
        let constraints = Self::extract_constraints(&intent_str);
        
        // Extract success criteria
        let success_criteria = Self::extract_success_criteria(&intent_str);
        
        Self {
            anchor_id,
            original_intent: intent_str,
            integrity_hash,
            anchored_at: chrono::Utc::now(),
            constraints,
            success_criteria,
        }
    }
    
    /// Extract constraints from intent text using pattern matching
    fn extract_constraints(intent: &str) -> Vec<Constraint> {
        let mut constraints = Vec::new();
        let intent_lower = intent.to_lowercase();
        
        // Technical constraints
        if intent_lower.contains("rust") || intent_lower.contains("typescript") {
            constraints.push(Constraint {
                id: Uuid::new_v4().to_string(),
                description: "Use specified programming language".to_string(),
                constraint_type: ConstraintType::Technical,
                severity: ConstraintSeverity::Critical,
            });
        }
        
        if intent_lower.contains("secure") || intent_lower.contains("authentication") {
            constraints.push(Constraint {
                id: Uuid::new_v4().to_string(),
                description: "Implement security best practices".to_string(),
                constraint_type: ConstraintType::Security,
                severity: ConstraintSeverity::Critical,
            });
        }
        
        if intent_lower.contains("fast") || intent_lower.contains("performance") {
            constraints.push(Constraint {
                id: Uuid::new_v4().to_string(),
                description: "Meet performance requirements".to_string(),
                constraint_type: ConstraintType::Performance,
                severity: ConstraintSeverity::High,
            });
        }
        
        constraints
    }
    
    /// Extract success criteria from intent
    fn extract_success_criteria(intent: &str) -> Vec<SuccessCriterion> {
        let mut criteria = Vec::new();
        
        // Default: at least one test must pass
        criteria.push(SuccessCriterion {
            id: Uuid::new_v4().to_string(),
            description: "Core functionality tests pass".to_string(),
            validation_method: ValidationMethod::TestPass {
                test_name: "core_functionality".to_string(),
            },
            is_mandatory: true,
        });
        
        criteria
    }
    
    /// Verify integrity of the anchor
    pub fn verify_integrity(&self) -> bool {
        let mut hasher = Hasher::new();
        hasher.update(self.original_intent.as_bytes());
        hasher.update(self.anchor_id.as_bytes());
        let computed_hash = hasher.finalize().to_hex().to_string();
        computed_hash == self.integrity_hash
    }
}

/// Drift detection and alignment monitoring
pub struct DriftDetector {
    /// The immutable intent anchor
    anchor: IntentAnchor,
    
    /// Current goal manifold
    manifold: Arc<RwLock<GoalManifold>>,
    
    /// History of alignment scores
    alignment_history: Arc<RwLock<Vec<AlignmentSnapshot>>>,
    
    /// Drift threshold configuration
    config: DriftConfig,
}

/// Configuration for drift detection
#[derive(Debug, Clone, Copy)]
pub struct DriftConfig {
    /// Score below which actions are blocked (0-100)
    pub block_threshold: f64,
    
    /// Score below which warnings are issued
    pub warn_threshold: f64,
    
    /// Number of consecutive low scores before escalating
    pub consecutive_violations_before_escalation: usize,
    
    /// Maximum allowed deviation from optimal path
    pub max_path_deviation: f64,
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            block_threshold: 50.0,
            warn_threshold: 80.0,
            consecutive_violations_before_escalation: 3,
            max_path_deviation: 0.3,
        }
    }
}

/// Snapshot of alignment at a point in time
#[derive(Debug, Clone)]
pub struct AlignmentSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub score: f64,
    pub action_description: String,
    pub goal_contributions: HashMap<String, f64>, // goal_id -> contribution
}

/// Result of drift analysis
#[derive(Debug, Clone)]
pub struct DriftAnalysis {
    /// Current alignment score
    pub current_score: f64,
    
    /// Trend (improving, stable, degrading)
    pub trend: AlignmentTrend,
    
    /// Distance from optimal path (0.0 = on path, 1.0 = completely off)
    pub path_deviation: f64,
    
    /// Specific violations detected
    pub violations: Vec<ConstraintViolation>,
    
    /// Recommended action
    pub recommendation: GuardrailAction,
    
    /// Suggestions to improve alignment
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentTrend {
    Improving,
    Stable,
    Degrading,
    Critical,
}

/// A detected constraint violation
#[derive(Debug, Clone)]
pub struct ConstraintViolation {
    pub constraint_id: String,
    pub constraint_description: String,
    pub severity: ConstraintSeverity,
    pub violation_details: String,
}

/// Action to take based on drift analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailAction {
    /// Allow the action to proceed
    Allow,
    
    /// Warn but allow with acknowledgment
    Warn,
    
    /// Suggest alternative actions
    Suggest,
    
    /// Block the action completely
    Block,
    
    /// Escalate to human review
    Escalate,
}

impl DriftDetector {
    /// Create new drift detector
    pub fn new(anchor: IntentAnchor, manifold: GoalManifold) -> Self {
        Self {
            anchor,
            manifold: Arc::new(RwLock::new(manifold)),
            alignment_history: Arc::new(RwLock::new(Vec::new())),
            config: DriftConfig::default(),
        }
    }
    
    /// Analyze an action before execution
    pub async fn analyze_action(&self, action_description: &str) -> Result<DriftAnalysis> {
        // Compute alignment score
        let score = self.compute_alignment_score(action_description).await?;
        
        // Record snapshot
        let snapshot = AlignmentSnapshot {
            timestamp: chrono::Utc::now(),
            score,
            action_description: action_description.to_string(),
            goal_contributions: self.compute_goal_contributions(action_description).await?,
        };
        
        {
            let mut history = self.alignment_history.write().await;
            history.push(snapshot);
            
            // Keep only last 100 snapshots
            if history.len() > 100 {
                history.remove(0);
            }
        }
        
        // Detect violations
        let violations = self.detect_violations(action_description).await?;
        
        // Determine trend
        let trend = self.compute_trend().await;
        
        // Calculate path deviation
        let path_deviation = self.compute_path_deviation().await?;
        
        // Determine action
        let recommendation = self.determine_action(score, &violations, trend, path_deviation);
        
        // Generate suggestions
        let suggestions = self.generate_suggestions(score, &violations).await?;
        
        Ok(DriftAnalysis {
            current_score: score,
            trend,
            path_deviation,
            violations,
            recommendation,
            suggestions,
        })
    }
    
    /// Compute alignment score for an action (0-100)
    async fn compute_alignment_score(&self, action_description: &str) -> Result<f64> {
        let manifold = self.manifold.read().await;
        
        // Check contribution to active goals
        let mut goal_contributions = Vec::new();
        for goal in manifold.all_goals() {
            if goal.status == GoalStatus::InProgress || goal.status == GoalStatus::Ready {
                let contribution = self.assess_goal_contribution(action_description, goal).await?;
                goal_contributions.push(contribution * goal.value_to_root);
            }
        }
        
        // Weighted average
        let goal_score = if goal_contributions.is_empty() {
            0.0
        } else {
            goal_contributions.iter().sum::<f64>() / goal_contributions.len() as f64
        };
        
        // Check constraint violations
        let constraint_penalty = self.assess_constraint_violations(action_description).await?;
        
        // Final score
        let score = goal_score * (1.0 - constraint_penalty) * 100.0;
        
        Ok(score.clamp(0.0, 100.0))
    }
    
    /// Assess how much an action contributes to a specific goal
    async fn assess_goal_contribution(&self, action_description: &str, goal: &Goal) -> Result<f64> {
        let action_lower = action_description.to_lowercase();
        let goal_lower = goal.description.to_lowercase();
        
        // Simple keyword matching (in production, use embeddings + LLM)
        let action_words: Vec<_> = action_lower.split_whitespace().collect();
        let goal_words: Vec<_> = goal_lower.split_whitespace().collect();
        
        let matching_words: Vec<_> = action_words
            .iter()
            .filter(|word| goal_words.contains(word) && word.len() > 3)
            .collect();
        
        let base_score = matching_words.len() as f64 / goal_words.len().max(1) as f64;
        
        // Boost score if action directly addresses goal
        let boost = if self.is_directly_related(action_description, goal) {
            0.3
        } else {
            0.0
        };
        
        Ok((base_score + boost).clamp(0.0, 1.0))
    }
    
    /// Check if action directly relates to goal
    fn is_directly_related(&self, action: &str, goal: &Goal) -> bool {
        // Check for direct implementation terms
        let implementation_terms = ["implement", "create", "build", "develop", "add"];
        let action_lower = action.to_lowercase();
        
        implementation_terms.iter().any(|term| {
            action_lower.contains(term) && goal.description.to_lowercase().contains(term)
        })
    }
    
    /// Assess constraint violations
    async fn assess_constraint_violations(&self, action_description: &str) -> Result<f64> {
        let mut total_penalty = 0.0;
        
        for constraint in &self.anchor.constraints {
            let penalty = match constraint.constraint_type {
                ConstraintType::Technical => {
                    if self.violates_technical_constraint(action_description, constraint) {
                        match constraint.severity {
                            ConstraintSeverity::Critical => 0.5,
                            ConstraintSeverity::High => 0.3,
                            ConstraintSeverity::Medium => 0.1,
                            ConstraintSeverity::Low => 0.05,
                        }
                    } else {
                        0.0
                    }
                }
                ConstraintType::Security => {
                    if action_description.to_lowercase().contains("skip") 
                        && action_description.to_lowercase().contains("auth") {
                        0.8 // Huge penalty for skipping auth
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            };
            
            total_penalty += penalty;
        }
        
        Ok((total_penalty as f64).clamp(0.0, 1.0))
    }
    
    /// Check if action violates technical constraint
    fn violates_technical_constraint(&self, action: &str, constraint: &Constraint) -> bool {
        let action_lower = action.to_lowercase();
        
        // Example: If constraint says "use Rust", penalize adding Python files
        if constraint.description.contains("Rust") && action_lower.contains("python") {
            return true;
        }
        
        false
    }
    
    /// Detect specific violations
    async fn detect_violations(&self, action_description: &str) -> Result<Vec<ConstraintViolation>> {
        let mut violations = Vec::new();
        
        // Check each constraint
        for constraint in &self.anchor.constraints {
            if let Some(details) = self.check_constraint_violation(action_description, constraint) {
                violations.push(ConstraintViolation {
                    constraint_id: constraint.id.clone(),
                    constraint_description: constraint.description.clone(),
                    severity: constraint.severity,
                    violation_details: details,
                });
            }
        }
        
        // Check for scope creep
        if self.is_scope_creep(action_description).await? {
            violations.push(ConstraintViolation {
                constraint_id: "scope".to_string(),
                constraint_description: "Stay within defined scope".to_string(),
                severity: ConstraintSeverity::High,
                violation_details: "Action appears to be outside defined scope".to_string(),
            });
        }
        
        Ok(violations)
    }
    
    /// Check if specific constraint is violated
    fn check_constraint_violation(&self, action: &str, constraint: &Constraint) -> Option<String> {
        let action_lower = action.to_lowercase();
        
        // Security constraint checks
        if constraint.constraint_type == ConstraintType::Security {
            if action_lower.contains("hardcoded") && action_lower.contains("password") {
                return Some("Hardcoded passwords violate security requirements".to_string());
            }
            if action_lower.contains("disable") && action_lower.contains("verification") {
                return Some("Disabling verification compromises security".to_string());
            }
        }
        
        None
    }
    
    /// Check for scope creep
    async fn is_scope_creep(&self, action_description: &str) -> Result<bool> {
        let manifold = self.manifold.read().await;
        
        // If action doesn't contribute to any active goal, it might be scope creep
        let mut contributes = false;
        for goal in manifold.all_goals() {
            if goal.status == GoalStatus::InProgress || goal.status == GoalStatus::Ready {
                let contribution = self.assess_goal_contribution(action_description, goal).await?;
                if contribution > 0.2 {
                    contributes = true;
                    break;
                }
            }
        }
        
        // Also check against original intent
        let intent_lower = self.anchor.original_intent.to_lowercase();
        let intent_words: Vec<_> = intent_lower
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .collect();
        
        let action_lower = action_description.to_lowercase();
        let action_words: Vec<_> = action_lower
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .collect();
        
        let matching = action_words.iter()
            .filter(|w| intent_words.contains(w))
            .count();
        
        let relevance = matching as f64 / action_words.len().max(1) as f64;
        
        // If not contributing to goals AND low relevance to intent = scope creep
        Ok(!contributes && relevance < 0.3)
    }
    
    /// Compute trend from alignment history
    async fn compute_trend(&self) -> AlignmentTrend {
        let history = self.alignment_history.read().await;
        
        if history.len() < 3 {
            return AlignmentTrend::Stable;
        }
        
        let recent: Vec<_> = history.iter().rev().take(5).collect();
        let first = recent.last().unwrap().score;
        let last = recent.first().unwrap().score;
        
        let diff = last - first;
        
        if diff > 10.0 {
            AlignmentTrend::Improving
        } else if diff < -20.0 {
            AlignmentTrend::Critical
        } else if diff < -10.0 {
            AlignmentTrend::Degrading
        } else {
            AlignmentTrend::Stable
        }
    }
    
    /// Compute deviation from optimal path
    async fn compute_path_deviation(&self) -> Result<f64> {
        let history = self.alignment_history.read().await;
        
        if history.len() < 2 {
            return Ok(0.0);
        }
        
        // Calculate variance in scores
        let scores: Vec<_> = history.iter().map(|s| s.score).collect();
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        
        let variance = scores.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f64>() / scores.len() as f64;
        
        // Higher variance = more deviation
        let deviation = (variance / 10000.0).sqrt().clamp(0.0, 1.0);
        
        Ok(deviation)
    }
    
    /// Determine guardrail action based on analysis
    fn determine_action(
        &self,
        score: f64,
        violations: &[ConstraintViolation],
        trend: AlignmentTrend,
        path_deviation: f64,
    ) -> GuardrailAction {
        // Critical violations always block
        let has_critical = violations.iter()
            .any(|v| v.severity == ConstraintSeverity::Critical);
        
        if has_critical || score < self.config.block_threshold {
            return GuardrailAction::Block;
        }
        
        // Degrading trend with low score = escalate
        if trend == AlignmentTrend::Critical && score < 60.0 {
            return GuardrailAction::Escalate;
        }
        
        // Significant deviation = warn
        if score < self.config.warn_threshold || path_deviation > self.config.max_path_deviation {
            return GuardrailAction::Warn;
        }
        
        // Slight deviation = suggest
        if score < 90.0 || !violations.is_empty() {
            return GuardrailAction::Suggest;
        }
        
        GuardrailAction::Allow
    }
    
    /// Generate suggestions to improve alignment
    async fn generate_suggestions(&self, score: f64, violations: &[ConstraintViolation]) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();
        
        if score < 80.0 {
            suggestions.push(format!(
                "Current alignment is {:.0}%. Consider focusing on core goals first.",
                score
            ));
        }
        
        for violation in violations {
            match violation.severity {
                ConstraintSeverity::Critical => {
                    suggestions.push(format!(
                        "CRITICAL: {} - This must be resolved before proceeding.",
                        violation.violation_details
                    ));
                }
                ConstraintSeverity::High => {
                    suggestions.push(format!(
                        "WARNING: {} - Consider addressing this.",
                        violation.constraint_description
                    ));
                }
                _ => {}
            }
        }
        
        // Get manifold for context
        let manifold = self.manifold.read().await;
        let ready_goals: Vec<_> = manifold.all_goals()
            .into_iter()
            .filter(|g| g.status == GoalStatus::Ready)
            .collect();
        
        if !ready_goals.is_empty() {
            suggestions.push(format!(
                "Ready to work on: {}",
                ready_goals.first().unwrap().description
            ));
        }
        
        Ok(suggestions)
    }
    
    /// Compute contributions to individual goals
    async fn compute_goal_contributions(&self, action_description: &str) -> Result<HashMap<String, f64>> {
        let manifold = self.manifold.read().await;
        let mut contributions = HashMap::new();
        
        for goal in manifold.all_goals() {
            let contribution = self.assess_goal_contribution(action_description, goal).await?;
            contributions.insert(goal.id.to_string(), contribution);
        }
        
        Ok(contributions)
    }
    
    /// Get alignment history
    pub async fn get_history(&self) -> Vec<AlignmentSnapshot> {
        self.alignment_history.read().await.clone()
    }
    
    /// Get current drift report
    pub async fn get_drift_report(&self) -> DriftReport {
        let history = self.alignment_history.read().await;
        
        DriftReport {
            total_actions: history.len(),
            average_alignment: if history.is_empty() {
                100.0
            } else {
                history.iter().map(|s| s.score).sum::<f64>() / history.len() as f64
            },
            lowest_score: history.iter().map(|s| s.score).fold(100.0, f64::min),
            highest_score: history.iter().map(|s| s.score).fold(0.0, f64::max),
            trend: self.compute_trend().await,
            anchor_integrity: self.anchor.verify_integrity(),
        }
    }
}

/// Report on overall drift status
#[derive(Debug, Clone)]
pub struct DriftReport {
    pub total_actions: usize,
    pub average_alignment: f64,
    pub lowest_score: f64,
    pub highest_score: f64,
    pub trend: AlignmentTrend,
    pub anchor_integrity: bool,
}

/// IPG Integration layer
pub struct IntentPreservationGuardrails {
    detector: DriftDetector,
}

impl IntentPreservationGuardrails {
    pub fn new(anchor: IntentAnchor, manifold: GoalManifold) -> Self {
        Self {
            detector: DriftDetector::new(anchor, manifold),
        }
    }
    
    /// Validate action before execution
    pub async fn validate_action(&self, action: &str) -> Result<ValidationResult> {
        let analysis = self.detector.analyze_action(action).await?;
        
        Ok(ValidationResult {
            allowed: matches!(analysis.recommendation, GuardrailAction::Allow | GuardrailAction::Suggest),
            action: analysis.recommendation,
            score: analysis.current_score,
            violations: analysis.violations,
            suggestions: analysis.suggestions,
        })
    }
    
    /// Get full drift analysis
    pub async fn analyze(&self, action: &str) -> Result<DriftAnalysis> {
        self.detector.analyze_action(action).await
    }
    
    /// Get drift report
    pub async fn report(&self) -> DriftReport {
        self.detector.get_drift_report().await
    }
}

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub allowed: bool,
    pub action: GuardrailAction,
    pub score: f64,
    pub violations: Vec<ConstraintViolation>,
    pub suggestions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_intent_anchor_creation() {
        let anchor = IntentAnchor::from_intent("Build a secure task management API in Rust");
        
        assert!(!anchor.anchor_id.is_empty());
        assert!(!anchor.integrity_hash.is_empty());
        assert!(anchor.verify_integrity());
        
        // Should extract security constraint
        let has_security = anchor.constraints.iter()
            .any(|c| c.constraint_type == ConstraintType::Security);
        assert!(has_security);
    }
    
    #[test]
    fn test_intent_anchor_integrity() {
        let anchor = IntentAnchor::from_intent("Test intent");
        assert!(anchor.verify_integrity());
        
        // Tamper with intent
        let mut tampered = anchor.clone();
        tampered.original_intent = "Tampered".to_string();
        assert!(!tampered.verify_integrity());
    }
    
    #[tokio::test]
    async fn test_drift_detector_basic() {
        let anchor = IntentAnchor::from_intent("Build auth system");
        let intent = crate::goal_manifold::Intent::new("Build auth system", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);
        
        // Add a goal for authentication
        let mut goal = crate::goal_manifold::Goal::builder()
            .description("Implement authentication system")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::FileExists("auth.rs".into()))
            .value_to_root(0.5)
            .build()
            .unwrap();
        goal.mark_ready().unwrap();
        manifold.add_goal(goal).unwrap();
        
        let detector = DriftDetector::new(anchor, manifold);
        
        // Valid action should score well
        let analysis = detector.analyze_action("Implement authentication handler").await.unwrap();
        assert!(analysis.current_score > 40.0, "Expected score > 40, got {}", analysis.current_score);
    }
    
    #[tokio::test]
    async fn test_scope_creep_detection() {
        let anchor = IntentAnchor::from_intent("Build auth system");
        let intent = crate::goal_manifold::Intent::new("Build auth system", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);
        
        // Add a goal for authentication
        let mut goal = crate::goal_manifold::Goal::builder()
            .description("Implement authentication system")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::FileExists("auth.rs".into()))
            .value_to_root(0.5)
            .build()
            .unwrap();
        goal.mark_ready().unwrap();
        manifold.add_goal(goal).unwrap();
        
        let detector = DriftDetector::new(anchor, manifold);
        
        // First, do an on-topic action to establish baseline
        let _ = detector.analyze_action("Implement authentication handler").await.unwrap();
        
        // Off-topic action should be detected as scope creep
        let analysis = detector.analyze_action("Add marketing analytics dashboard").await.unwrap();
        assert!(analysis.path_deviation > 0.1, "Expected path deviation > 0.1, got {}", analysis.path_deviation);
    }
    
    #[tokio::test]
    async fn test_guardrail_action_determination() {
        let anchor = IntentAnchor::from_intent("Build secure API");
        let intent = crate::goal_manifold::Intent::new("Build secure API", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);
        
        // Add a goal for API security
        let mut goal = crate::goal_manifold::Goal::builder()
            .description("Implement secure API endpoints")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::FileExists("api.rs".into()))
            .value_to_root(0.5)
            .build()
            .unwrap();
        goal.mark_ready().unwrap();
        manifold.add_goal(goal).unwrap();
        
        let detector = DriftDetector::new(anchor, manifold);
        
        // Security violation should block
        let analysis = detector.analyze_action("Add hardcoded password for testing").await.unwrap();
        assert!(matches!(analysis.recommendation, GuardrailAction::Block));
    }
}
