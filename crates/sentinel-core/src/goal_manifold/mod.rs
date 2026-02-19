//! Goal Manifold - Immutable truth for goal-aligned execution
//!
//! This module implements the Goal Manifold, the foundational data structure
//! of Sentinel. It provides cryptographically verified, immutable representation
//! of project objectives with formal success criteria.

pub mod atomic;
pub mod dag;
pub mod goal;
pub mod predicate;
pub mod predicate_sandbox;
pub mod slicer;

pub use self::InvariantSeverity as GoalInvariantSeverity;
pub use dag::GoalDag;
pub use goal::Goal;

use crate::error::{Result, ResultExt};
use crate::types::{Blake3Hash, Timestamp};
use predicate::Predicate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The immutable core of Sentinel - the Goal Manifold
///
/// The Goal Manifold is the single source of truth for what the agent is trying to achieve.
/// It is:
/// - **Immutable**: Root intent never changes (append-only versioning)
/// - **Verifiable**: Cryptographically hashed for integrity
/// - **Formal**: Success defined by formal predicates
/// - **Hierarchical**: Goals organized in a DAG
///
/// # Invariants
///
/// 1. Root intent is immutable after creation
/// 2. Integrity hash matches current state
/// 3. Goal DAG is acyclic
/// 4. All goals have valid success criteria
///
/// # Examples
///
/// ```
/// use sentinel_core::goal_manifold::{GoalManifold, Intent};
/// use sentinel_core::goal_manifold::predicate::Predicate;
/// use sentinel_core::goal_manifold::goal::Goal;
///
/// let intent = Intent::new(
///     "Build a REST API with authentication",
///     vec!["Use TypeScript", "Test coverage >80%"]
/// );
///
/// let mut manifold = GoalManifold::new(intent);
///
/// // Add goals
/// let goal = Goal::builder()
///     .description("Implement JWT authentication")
///     .add_success_criterion(Predicate::TestsPassing {
///         suite: "auth".to_string(),
///         min_coverage: 0.8,
///     })
///     .value_to_root(0.4)
///     .build()
///     .unwrap();
///
/// manifold.add_goal(goal).unwrap();
/// ```
use crate::types::{HandoverNote, HumanOverride};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalManifold {
    /// The immutable original intent
    pub root_intent: Intent,

    /// Configuration for alignment sensitivity
    pub sensitivity: f64, // 0.0 (Flexible) to 1.0 (Rigid)

    /// Runtime reliability policy used for hard enforcement during execution.
    #[serde(default)]
    pub reliability: ReliabilityPolicy,

    /// Runtime governance contract for dependencies/frameworks/endpoints.
    #[serde(default)]
    pub governance: GovernancePolicy,

    /// History of human overrides (for learning)
    pub overrides: Vec<HumanOverride>,

    /// Cognitive handover notes between agents
    pub handover_log: Vec<HandoverNote>,

    /// Active locks on specific files (File -> AgentID)
    pub file_locks: std::collections::HashMap<std::path::PathBuf, uuid::Uuid>,

    /// Graph of goals and their dependencies
    pub goal_dag: GoalDag,

    /// Hard constraints that can NEVER be violated
    pub invariants: Vec<Invariant>,

    /// Creation timestamp (for audit trail)
    pub created_at: Timestamp,

    /// Last update timestamp
    pub updated_at: Timestamp,

    /// Cryptographic hash for integrity verification
    ///
    /// This hash is computed over the entire manifold state.
    /// It serves as a tamper-proof seal.
    pub integrity_hash: Blake3Hash,

    /// Version history (append-only log)
    pub version_history: Vec<ManifoldVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReliabilityPolicy {
    pub thresholds: crate::execution::ReliabilityThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GovernancePolicy {
    pub required_dependencies: Vec<String>,
    pub allowed_dependencies: Vec<String>,
    pub required_frameworks: Vec<String>,
    pub allowed_frameworks: Vec<String>,
    pub allowed_endpoints: std::collections::HashMap<String, String>,
    pub allowed_ports: Vec<u16>,
    pub pending_proposal: Option<GovernanceChangeProposal>,
    pub history: Vec<GovernanceChangeProposal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceChangeProposal {
    pub id: Uuid,
    pub created_at: Timestamp,
    pub rationale: String,
    pub proposed_dependencies: Vec<String>,
    #[serde(default)]
    pub proposed_dependency_removals: Vec<String>,
    pub proposed_frameworks: Vec<String>,
    #[serde(default)]
    pub proposed_framework_removals: Vec<String>,
    pub proposed_endpoints: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub proposed_endpoint_removals: Vec<String>,
    pub proposed_ports: Vec<u16>,
    #[serde(default)]
    pub proposed_port_removals: Vec<u16>,
    #[serde(default)]
    pub deterministic_confidence: f64,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub status: GovernanceProposalStatus,
    pub user_note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceProposalStatus {
    PendingUserApproval,
    ApprovedByUser,
    RejectedByUser,
}

/// The original user intent
///
/// This captures what the user wants to achieve in natural language,
/// along with any constraints they specified.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    /// Natural language description of the objective
    pub description: String,

    /// User-specified constraints (e.g., "Use TypeScript", "Deploy to AWS")
    pub constraints: Vec<String>,

    /// Expected outcomes (informal, user-provided)
    pub expected_outcomes: Vec<String>,

    /// Target platform (if specified)
    pub target_platform: Option<String>,

    /// Programming languages (if specified)
    pub languages: Vec<String>,

    /// Frameworks (if specified)
    pub frameworks: Vec<String>,

    /// Official infrastructure endpoints (e.g., "frontend" -> "192.168.1.50")
    pub infrastructure_map: std::collections::HashMap<String, String>,
}

impl Intent {
    /// Create a new intent
    pub fn new(description: impl Into<String>, constraints: Vec<impl Into<String>>) -> Self {
        Self {
            description: description.into(),
            constraints: constraints.into_iter().map(|c| c.into()).collect(),
            expected_outcomes: Vec::new(),
            target_platform: None,
            languages: Vec::new(),
            frameworks: Vec::new(),
            infrastructure_map: std::collections::HashMap::new(),
        }
    }

    /// Add an infrastructure endpoint
    pub fn with_endpoint(mut self, name: impl Into<String>, url: impl Into<String>) -> Self {
        self.infrastructure_map.insert(name.into(), url.into());
        self
    }

    /// Add an expected outcome
    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.expected_outcomes.push(outcome.into());
        self
    }

    /// Set target platform
    pub fn with_platform(mut self, platform: impl Into<String>) -> Self {
        self.target_platform = Some(platform.into());
        self
    }

    /// Add a language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.languages.push(language.into());
        self
    }

    /// Add a framework
    pub fn with_framework(mut self, framework: impl Into<String>) -> Self {
        self.frameworks.push(framework.into());
        self
    }
}

impl GovernancePolicy {
    pub fn from_intent(intent: &Intent) -> Self {
        let mut policy = Self::default();
        policy.allowed_frameworks = dedup_sorted(intent.frameworks.clone());
        policy.required_frameworks = policy.allowed_frameworks.clone();
        policy.allowed_endpoints = intent.infrastructure_map.clone();
        let mut ports = Vec::new();
        for value in intent.infrastructure_map.values() {
            if let Some(port) = extract_port(value) {
                ports.push(port);
            }
        }
        policy.allowed_ports = dedup_sorted(ports);
        policy
    }
}

/// A hard constraint that must never be violated
///
/// Invariants are checked before every action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    /// Unique identifier
    pub id: Uuid,

    /// Human-readable description
    pub description: String,

    /// The predicate that must always be true
    pub predicate: Predicate,

    /// Severity if violated
    pub severity: InvariantSeverity,
}

impl Invariant {
    /// Create a new invariant
    pub fn new(
        description: impl Into<String>,
        predicate: Predicate,
        severity: InvariantSeverity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            predicate,
            severity,
        }
    }

    /// Create a critical invariant (violation stops execution)
    pub fn critical(description: impl Into<String>, predicate: Predicate) -> Self {
        Self::new(description, predicate, InvariantSeverity::Critical)
    }
}

/// Severity of invariant violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvariantSeverity {
    /// Warning: Log but continue
    Warning,

    /// Error: Attempt correction
    Error,

    /// Critical: Stop execution immediately
    Critical,
}

/// A version in the manifold history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldVersion {
    /// Version number (sequential)
    pub version: u64,

    /// Timestamp of this version
    pub timestamp: Timestamp,

    /// Hash of the manifold at this version
    pub hash: Blake3Hash,

    /// Description of what changed
    pub change_description: String,
}

impl GoalManifold {
    /// Create a new Goal Manifold
    ///
    /// # Examples
    ///
    /// ```
    /// use sentinel_core::goal_manifold::{GoalManifold, Intent};
    ///
    /// let intent = Intent::new("Build a web app", vec!["TypeScript"]);
    /// let manifold = GoalManifold::new(intent);
    /// ```
    pub fn new(root_intent: Intent) -> Self {
        let now = chrono::Utc::now();
        let mut manifold = Self {
            root_intent,
            sensitivity: 0.5,
            reliability: ReliabilityPolicy::default(),
            governance: GovernancePolicy::default(),
            overrides: Vec::new(),
            handover_log: Vec::new(),
            file_locks: std::collections::HashMap::new(),
            goal_dag: GoalDag::new(),
            invariants: Vec::new(),
            created_at: now,
            updated_at: now,
            integrity_hash: Blake3Hash::empty(),
            version_history: Vec::new(),
        };
        manifold.governance = GovernancePolicy::from_intent(&manifold.root_intent);

        // Initialize integrity hash and version 1
        manifold.update_hash("Manifold initialized");

        manifold
    }

    /// Compute the cryptographic hash of the entire manifold
    ///
    /// This uses Blake3 for fast, secure hashing.
    pub fn compute_hash(&self) -> Blake3Hash {
        let mut hasher = blake3::Hasher::new();

        // Hash the root intent
        hasher.update(self.root_intent.description.as_bytes());
        for constraint in &self.root_intent.constraints {
            hasher.update(constraint.as_bytes());
        }
        hasher.update(
            &self
                .reliability
                .thresholds
                .min_task_success_rate
                .to_le_bytes(),
        );
        hasher.update(
            &self
                .reliability
                .thresholds
                .min_no_regression_rate
                .to_le_bytes(),
        );
        hasher.update(&self.reliability.thresholds.max_rollback_rate.to_le_bytes());
        hasher.update(
            &self
                .reliability
                .thresholds
                .max_invariant_violation_rate
                .to_le_bytes(),
        );
        for dep in sorted_strings(&self.governance.required_dependencies) {
            hasher.update(dep.as_bytes());
        }
        for dep in sorted_strings(&self.governance.allowed_dependencies) {
            hasher.update(dep.as_bytes());
        }
        for framework in sorted_strings(&self.governance.required_frameworks) {
            hasher.update(framework.as_bytes());
        }
        for framework in sorted_strings(&self.governance.allowed_frameworks) {
            hasher.update(framework.as_bytes());
        }
        let mut endpoints: Vec<_> = self.governance.allowed_endpoints.iter().collect();
        endpoints.sort_by(|a, b| a.0.cmp(b.0));
        for (name, value) in endpoints {
            hasher.update(name.as_bytes());
            hasher.update(value.as_bytes());
        }
        let mut ports = self.governance.allowed_ports.clone();
        ports.sort_unstable();
        for port in ports {
            hasher.update(&port.to_le_bytes());
        }

        // Hash all goals (in sorted order for determinism)
        let mut goal_ids: Vec<_> = self.goal_dag.goals().map(|g| g.id).collect();
        goal_ids.sort();

        for goal_id in goal_ids {
            if let Some(goal) = self.goal_dag.get_goal(&goal_id) {
                // Hash goal data
                hasher.update(goal.id.as_bytes());
                hasher.update(goal.description.as_bytes());
                hasher.update(&goal.value_to_root.to_le_bytes());

                // Hash success criteria
                let criteria_json =
                    serde_json::to_string(&goal.success_criteria).unwrap_or_default();
                hasher.update(criteria_json.as_bytes());
            }
        }

        // Hash invariants
        for invariant in &self.invariants {
            hasher.update(invariant.id.as_bytes());
            hasher.update(invariant.description.as_bytes());
        }

        Blake3Hash::from(hasher.finalize())
    }

    /// Verify the integrity hash matches current state
    ///
    /// Returns `true` if the hash is valid, `false` if tampered.
    pub fn verify_integrity(&self) -> bool {
        let computed = self.compute_hash();
        computed == self.integrity_hash
    }

    /// Update the integrity hash and record a new version
    fn update_hash(&mut self, change_description: impl Into<String>) {
        let hash = self.compute_hash();
        self.integrity_hash = hash;
        self.updated_at = crate::types::now();

        let version = self.version_history.len() as u64 + 1;
        self.version_history.push(ManifoldVersion {
            version,
            timestamp: self.updated_at,
            hash,
            change_description: change_description.into(),
        });
    }

    /// Add a goal to the manifold
    ///
    /// # Errors
    ///
    /// Returns `Err` if the goal is invalid or already exists.
    pub fn add_goal(&mut self, goal: Goal) -> Result<()> {
        goal.validate().context("Goal validation failed")?;

        let description = format!("Added goal: {}", goal.description);
        self.goal_dag.add_goal(goal)?;
        self.update_hash(description);

        Ok(())
    }

    /// Add a dependency between two goals
    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) -> Result<()> {
        self.goal_dag.add_dependency(from, to)?;
        self.update_hash(format!("Added dependency: {} -> {}", from, to));
        Ok(())
    }

    /// Add an invariant
    pub fn add_invariant(&mut self, invariant: Invariant) -> Result<()> {
        let description = format!("Added invariant: {}", invariant.description);
        self.invariants.push(invariant);
        self.update_hash(description);
        Ok(())
    }

    /// Record a governance proposal that requires explicit user approval.
    pub fn record_governance_proposal(&mut self, proposal: GovernanceChangeProposal) {
        self.governance.pending_proposal = Some(proposal.clone());
        self.governance.history.push(proposal);
        self.update_hash("Governance proposal recorded");
    }

    /// Approve the currently pending governance proposal and apply it to runtime policy.
    pub fn approve_pending_governance_proposal(
        &mut self,
        user_note: Option<String>,
    ) -> Result<Uuid> {
        let pending = self.governance.pending_proposal.clone().ok_or_else(|| {
            crate::error::SentinelError::InvariantViolation(
                "No pending governance proposal".to_string(),
            )
        })?;
        let proposal_id = pending.id;

        self.governance.allowed_dependencies.retain(|dep| {
            !pending
                .proposed_dependency_removals
                .iter()
                .any(|candidate| candidate == dep)
        });
        self.governance.allowed_frameworks.retain(|framework| {
            !pending
                .proposed_framework_removals
                .iter()
                .any(|candidate| candidate == framework)
        });
        self.governance.allowed_ports.retain(|port| {
            !pending
                .proposed_port_removals
                .iter()
                .any(|candidate| candidate == port)
        });
        if !pending.proposed_endpoint_removals.is_empty() {
            self.governance
                .allowed_endpoints
                .retain(|_, endpoint| !pending.proposed_endpoint_removals.contains(endpoint));
        }

        for dep in pending.proposed_dependencies {
            self.governance.allowed_dependencies.push(dep);
        }
        for framework in pending.proposed_frameworks {
            self.governance.allowed_frameworks.push(framework);
        }
        for (name, endpoint) in pending.proposed_endpoints {
            self.governance.allowed_endpoints.insert(name, endpoint);
        }
        self.governance.allowed_ports.extend(pending.proposed_ports);

        self.governance.allowed_dependencies =
            dedup_sorted(self.governance.allowed_dependencies.clone());
        self.governance.allowed_frameworks =
            dedup_sorted(self.governance.allowed_frameworks.clone());
        self.governance.allowed_ports = dedup_sorted(self.governance.allowed_ports.clone());
        self.governance.required_dependencies = self
            .governance
            .required_dependencies
            .iter()
            .filter(|dep| self.governance.allowed_dependencies.contains(dep))
            .cloned()
            .collect();
        self.governance.required_frameworks = self
            .governance
            .required_frameworks
            .iter()
            .filter(|framework| self.governance.allowed_frameworks.contains(framework))
            .cloned()
            .collect();

        if let Some(history_entry) = self
            .governance
            .history
            .iter_mut()
            .rev()
            .find(|proposal| proposal.id == proposal_id)
        {
            history_entry.status = GovernanceProposalStatus::ApprovedByUser;
            history_entry.user_note = user_note.clone();
        }
        self.governance.pending_proposal = None;
        self.update_hash("Governance proposal approved");
        Ok(proposal_id)
    }

    /// Reject the currently pending governance proposal.
    pub fn reject_pending_governance_proposal(
        &mut self,
        user_note: Option<String>,
    ) -> Result<Uuid> {
        let pending = self.governance.pending_proposal.clone().ok_or_else(|| {
            crate::error::SentinelError::InvariantViolation(
                "No pending governance proposal".to_string(),
            )
        })?;
        let proposal_id = pending.id;

        if let Some(history_entry) = self
            .governance
            .history
            .iter_mut()
            .rev()
            .find(|proposal| proposal.id == proposal_id)
        {
            history_entry.status = GovernanceProposalStatus::RejectedByUser;
            history_entry.user_note = user_note;
        }
        self.governance.pending_proposal = None;
        self.update_hash("Governance proposal rejected");
        Ok(proposal_id)
    }

    /// Seed or regenerate governance baseline from a deterministic workspace observation.
    pub fn apply_governance_seed(
        &mut self,
        dependencies: Vec<String>,
        frameworks: Vec<String>,
        endpoints: Vec<String>,
        ports: Vec<u16>,
        lock_required_to_allowed: bool,
    ) {
        self.governance.allowed_dependencies = dedup_sorted(dependencies);
        self.governance.allowed_frameworks = dedup_sorted(frameworks);
        self.governance.allowed_ports = dedup_sorted(ports);
        self.governance.allowed_endpoints.clear();
        for (index, endpoint) in endpoints.into_iter().enumerate() {
            self.governance
                .allowed_endpoints
                .insert(format!("seeded_{}", index + 1), endpoint);
        }

        if lock_required_to_allowed {
            self.governance.required_dependencies = self.governance.allowed_dependencies.clone();
            self.governance.required_frameworks = self.governance.allowed_frameworks.clone();
        } else {
            self.governance.required_dependencies = self
                .governance
                .required_dependencies
                .iter()
                .filter(|dep| self.governance.allowed_dependencies.contains(dep))
                .cloned()
                .collect();
            self.governance.required_frameworks = self
                .governance
                .required_frameworks
                .iter()
                .filter(|framework| self.governance.allowed_frameworks.contains(framework))
                .cloned()
                .collect();
        }

        self.governance.pending_proposal = None;
        self.update_hash("Governance baseline seeded");
    }

    /// Get a goal by ID
    pub fn get_goal(&self, id: &Uuid) -> Option<&Goal> {
        self.goal_dag.get_goal(id)
    }

    /// Get a mutable reference to a goal
    pub fn get_goal_mut(&mut self, id: &Uuid) -> Option<&mut Goal> {
        self.goal_dag.get_goal_mut(id)
    }

    /// Get all goals that are ready to work on
    pub fn get_ready_goals(&self) -> Vec<&Goal> {
        self.goal_dag.get_ready_goals()
    }

    /// Get all goals in the manifold
    pub fn all_goals(&self) -> Vec<&Goal> {
        self.goal_dag.goals().collect()
    }

    /// Get the total number of goals
    pub fn goal_count(&self) -> usize {
        self.goal_dag.len()
    }

    /// Get completion percentage (0.0-1.0)
    pub fn completion_percentage(&self) -> f64 {
        let total = self.goal_dag.len();
        if total == 0 {
            return 0.0;
        }

        let completed = self
            .goal_dag
            .goals()
            .filter(|g| g.status == crate::types::GoalStatus::Completed)
            .count();

        completed as f64 / total as f64
    }

    /// Calculate total estimated time to completion (hours)
    ///
    /// Uses the critical path algorithm.
    pub fn estimated_time_to_completion(&self) -> f64 {
        let critical_path = self.goal_dag.critical_path();
        critical_path
            .iter()
            .map(|g| g.complexity_estimate.mean * 2.0) // hours
            .sum()
    }

    /// Get the current version number
    pub fn current_version(&self) -> u64 {
        self.version_history.len() as u64
    }

    /// Get version history
    pub fn version_history(&self) -> &[ManifoldVersion] {
        &self.version_history
    }

    /// Validate all invariants
    ///
    /// Returns violations if any are found.
    pub async fn validate_invariants(
        &self,
        state: &predicate::PredicateState,
    ) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();

        for invariant in &self.invariants {
            match invariant.predicate.evaluate(state).await {
                Ok(true) => {
                    // Invariant satisfied
                }
                Ok(false) => {
                    violations.push(InvariantViolation {
                        invariant_id: invariant.id,
                        description: invariant.description.clone(),
                        severity: invariant.severity,
                    });
                }
                Err(e) => {
                    // Error evaluating invariant - treat as violation
                    violations.push(InvariantViolation {
                        invariant_id: invariant.id,
                        description: format!("{}: {}", invariant.description, e),
                        severity: invariant.severity,
                    });
                }
            }
        }

        violations
    }
}

/// An invariant violation
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub invariant_id: Uuid,
    pub description: String,
    pub severity: InvariantSeverity,
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut items = values.to_vec();
    items.sort();
    items.dedup();
    items
}

fn dedup_sorted<T>(mut values: Vec<T>) -> Vec<T>
where
    T: Ord,
{
    values.sort();
    values.dedup();
    values
}

fn extract_port(endpoint: &str) -> Option<u16> {
    let after_scheme = endpoint
        .split_once("://")
        .map(|(_, value)| value)
        .unwrap_or(endpoint);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    let port_candidate = host_port.rsplit_once(':').map(|(_, port)| port)?;
    port_candidate.parse::<u16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_creation() {
        let intent = Intent::new("Build REST API", vec!["TypeScript", "PostgreSQL"])
            .with_outcome("Users can authenticate")
            .with_platform("web")
            .with_language("typescript");

        assert_eq!(intent.description, "Build REST API");
        assert_eq!(intent.constraints.len(), 2);
        assert_eq!(intent.expected_outcomes.len(), 1);
        assert_eq!(intent.target_platform, Some("web".to_string()));
    }

    #[test]
    fn test_manifold_creation() {
        let intent = Intent::new("Test project", vec!["Constraint 1"]);
        let manifold = GoalManifold::new(intent);

        assert_eq!(manifold.goal_count(), 0);
        assert_eq!(manifold.completion_percentage(), 0.0);
        assert_eq!(manifold.current_version(), 1);
    }

    #[test]
    fn test_manifold_integrity() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);

        // Initial hash should be valid
        assert!(manifold.verify_integrity());

        // Recomputing should give same hash
        let hash1 = manifold.compute_hash();
        let hash2 = manifold.compute_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_add_goal_updates_hash() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        let initial_hash = manifold.integrity_hash;

        let goal = Goal::builder()
            .description("Test goal")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        manifold.add_goal(goal).unwrap();

        // Hash should have changed
        assert_ne!(manifold.integrity_hash, initial_hash);

        // New hash should be valid
        assert!(manifold.verify_integrity());

        // Version history should have 2 entries
        assert_eq!(manifold.current_version(), 2);
    }

    #[test]
    fn test_add_invariant() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        let invariant = Invariant::critical(
            "No hardcoded secrets",
            Predicate::AlwaysFalse, // Would need real check
        );

        manifold.add_invariant(invariant).unwrap();

        assert_eq!(manifold.invariants.len(), 1);
        assert!(manifold.verify_integrity());
    }

    #[test]
    fn test_completion_percentage() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        let goal1 = Goal::builder()
            .description("Goal 1")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let goal2 = Goal::builder()
            .description("Goal 2")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();

        let id1 = goal1.id;

        manifold.add_goal(goal1).unwrap();
        manifold.add_goal(goal2).unwrap();

        // 0% complete
        assert_eq!(manifold.completion_percentage(), 0.0);

        // Complete one goal with proper transitions
        let goal1 = manifold.get_goal_mut(&id1).unwrap();
        goal1.mark_ready().unwrap();
        goal1.start().unwrap();
        goal1.begin_validation().unwrap();
        goal1.complete().unwrap();

        // 50% complete
        assert!((manifold.completion_percentage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_apply_governance_seed_lock_required_to_allowed() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);

        manifold.governance.required_dependencies = vec!["cargo:legacy".to_string()];
        manifold.governance.required_frameworks = vec!["framework:legacy".to_string()];
        manifold.governance.pending_proposal = Some(GovernanceChangeProposal {
            id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            rationale: "legacy proposal".to_string(),
            proposed_dependencies: vec!["cargo:old".to_string()],
            proposed_dependency_removals: vec![],
            proposed_frameworks: vec!["framework:old".to_string()],
            proposed_framework_removals: vec![],
            proposed_endpoints: std::collections::HashMap::new(),
            proposed_endpoint_removals: vec![],
            proposed_ports: vec![1111],
            proposed_port_removals: vec![],
            deterministic_confidence: 1.0,
            evidence: vec!["seed reset".to_string()],
            status: GovernanceProposalStatus::PendingUserApproval,
            user_note: None,
        });

        manifold.apply_governance_seed(
            vec!["cargo:tokio".to_string(), "cargo:serde".to_string()],
            vec!["framework:axum".to_string()],
            vec!["/health".to_string()],
            vec![3000, 3001],
            true,
        );

        assert_eq!(
            manifold.governance.allowed_dependencies,
            vec!["cargo:serde".to_string(), "cargo:tokio".to_string()]
        );
        assert_eq!(
            manifold.governance.required_dependencies,
            manifold.governance.allowed_dependencies
        );
        assert_eq!(
            manifold.governance.required_frameworks,
            manifold.governance.allowed_frameworks
        );
        assert_eq!(manifold.governance.allowed_endpoints.len(), 1);
        assert_eq!(
            manifold.governance.allowed_endpoints.get("seeded_1"),
            Some(&"/health".to_string())
        );
        assert!(manifold.governance.pending_proposal.is_none());
    }

    #[test]
    fn test_apply_governance_seed_intersects_required_when_not_locked() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);
        manifold.governance.required_dependencies =
            vec!["cargo:serde".to_string(), "cargo:nonexistent".to_string()];
        manifold.governance.required_frameworks =
            vec!["framework:axum".to_string(), "framework:legacy".to_string()];

        manifold.apply_governance_seed(
            vec!["cargo:serde".to_string(), "cargo:tokio".to_string()],
            vec!["framework:axum".to_string()],
            vec![],
            vec![],
            false,
        );

        assert_eq!(
            manifold.governance.required_dependencies,
            vec!["cargo:serde".to_string()]
        );
        assert_eq!(
            manifold.governance.required_frameworks,
            vec!["framework:axum".to_string()]
        );
    }

    #[test]
    fn test_approve_governance_proposal_applies_additions_and_removals() {
        let intent = Intent::new("Test project", Vec::<String>::new());
        let mut manifold = GoalManifold::new(intent);
        manifold.governance.allowed_dependencies =
            vec!["cargo:tokio".to_string(), "cargo:serde".to_string()];
        manifold.governance.required_dependencies = vec!["cargo:serde".to_string()];
        manifold.governance.allowed_frameworks =
            vec!["framework:axum".to_string(), "framework:react".to_string()];
        manifold.governance.required_frameworks = vec!["framework:axum".to_string()];
        manifold
            .governance
            .allowed_endpoints
            .insert("api".to_string(), "http://localhost:8080".to_string());
        manifold.governance.allowed_ports = vec![8080, 5173];

        manifold.record_governance_proposal(GovernanceChangeProposal {
            id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            rationale: "deterministic contract update".to_string(),
            proposed_dependencies: vec!["cargo:reqwest".to_string()],
            proposed_dependency_removals: vec!["cargo:serde".to_string()],
            proposed_frameworks: vec!["framework:nextjs".to_string()],
            proposed_framework_removals: vec!["framework:axum".to_string()],
            proposed_endpoints: std::collections::HashMap::from([(
                "preview".to_string(),
                "http://localhost:4173".to_string(),
            )]),
            proposed_endpoint_removals: vec!["http://localhost:8080".to_string()],
            proposed_ports: vec![4173],
            proposed_port_removals: vec![8080],
            deterministic_confidence: 1.0,
            evidence: vec!["scan".to_string()],
            status: GovernanceProposalStatus::PendingUserApproval,
            user_note: None,
        });

        let _ = manifold
            .approve_pending_governance_proposal(Some("approved".to_string()))
            .expect("proposal should be approved");

        assert!(manifold
            .governance
            .allowed_dependencies
            .contains(&"cargo:reqwest".to_string()));
        assert!(!manifold
            .governance
            .allowed_dependencies
            .contains(&"cargo:serde".to_string()));
        assert!(manifold
            .governance
            .allowed_frameworks
            .contains(&"framework:nextjs".to_string()));
        assert!(!manifold
            .governance
            .allowed_frameworks
            .contains(&"framework:axum".to_string()));
        assert_eq!(
            manifold.governance.required_dependencies,
            Vec::<String>::new()
        );
        assert_eq!(
            manifold.governance.required_frameworks,
            Vec::<String>::new()
        );
        assert!(manifold
            .governance
            .allowed_endpoints
            .values()
            .any(|value| value == "http://localhost:4173"));
        assert!(!manifold
            .governance
            .allowed_endpoints
            .values()
            .any(|value| value == "http://localhost:8080"));
        assert!(manifold.governance.allowed_ports.contains(&4173));
        assert!(!manifold.governance.allowed_ports.contains(&8080));
        assert!(manifold.governance.pending_proposal.is_none());
    }
}
