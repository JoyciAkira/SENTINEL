//! Goal Manifold - Immutable truth for goal-aligned execution
//!
//! This module implements the Goal Manifold, the foundational data structure
//! of Sentinel. It provides cryptographically verified, immutable representation
//! of project objectives with formal success criteria.

pub mod dag;
pub mod goal;
pub mod predicate;

use crate::error::{Result, ResultExt};
use crate::types::{Blake3Hash, Timestamp};
use dag::GoalDag;
use goal::Goal;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalManifold {
    /// Unique identifier for this manifold
    pub id: Uuid,

    /// The original user intent - IMMUTABLE
    pub root_intent: Intent,

    /// Directed Acyclic Graph of goals
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
        let now = crate::types::now();
        let id = Uuid::new_v4();

        let mut manifold = Self {
            id,
            root_intent,
            goal_dag: GoalDag::new(),
            invariants: Vec::new(),
            created_at: now,
            updated_at: now,
            integrity_hash: Blake3Hash::new([0u8; 32]), // Placeholder
            version_history: Vec::new(),
        };

        // Compute initial hash
        let hash = manifold.compute_hash();
        manifold.integrity_hash = hash;

        // Record initial version
        manifold.version_history.push(ManifoldVersion {
            version: 1,
            timestamp: now,
            hash,
            change_description: "Initial creation".to_string(),
        });

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
        state: &predicate::ProjectState,
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

        let mut goal1 = Goal::builder()
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
}
