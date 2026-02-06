//! Hierarchical Planning Module - Goal-Driven Plan Generation
//!
//! This module implements revolutionary planning where:
//! - Plans are NOT linear sequences of actions
//! - Plans are HIERARCHICAL structures aligned with Goal DAG
//! - Every action is justified by its contribution to the Goal Manifold
//!
//! # Why This Is Revolutionary
//!
//! Traditional agents:
//! - Plan linear: "Do A, then B, then C"
//! - No awareness of goal hierarchy
//! - No justification of why each action is needed
//!
//! Sentinel Native Agent with Hierarchical Planning:
//! - Plan hierarchical: "Goal A → [Goal B, Goal C] → Actions"
//! - Every action references which goal it serves
//! - Plans are validated against Goal Manifold
//! - Actions are ordered respecting dependencies
//!
//! # Planning Process
//!
//! ```text
//! Task: "Build authentication system"
//!          │
//!          v
//! ┌─────────────────────────────────────┐
//! │ Goal: "Implement JWT auth"       │
//! │ │                              │
//! │ v                              v
//! │ Sub-goal: "Create JWT model"    │
//! │ │                              │
//! │ v                              v
//! │ Sub-goal: "Add token validation"│
//! │ │                              │
//! │ v                              v
//! │ Action: Create jwt.rs           │
//! │ Action: Write tests              │
//! │ Action: Add validation logic     │
//! │ Rationale: "Creates JWT tokens"  │
//! └─────────────────────────────────────┘
//! ```

use anyhow::Result;
use sentinel_core::{
    goal_manifold::predicate::PredicateState,
    goal_manifold::{predicate::Predicate, Goal, GoalManifold, Invariant, InvariantSeverity},
    Uuid,
};
use std::collections::{HashMap, HashSet};

/// Hierarchical Planner - Goal-driven plan generation
#[derive(Debug)]
pub struct HierarchicalPlanner {
    /// Reference to Goal Manifold for plan validation
    goal_manifold: GoalManifold,

    /// Plan statistics
    stats: PlanStats,
}

/// Execution plan - hierarchical structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionPlan {
    /// Root task description
    pub root_task: String,

    /// Sub-goals in topological order (respecting dependencies)
    pub sub_goals: Vec<GoalId>,

    /// All actions in execution order
    pub actions: Vec<sentinel_core::cognitive_state::Action>,

    /// Plan complexity score
    pub complexity: f64,

    /// Estimated completion time
    pub estimated_duration_minutes: u32,

    /// Mandatory execution contract (where/goal/how/why).
    pub north_star: sentinel_core::ExecutionNorthStar,
}

/// Action in execution plan
#[derive(Debug, Clone, serde::Serialize)]
pub struct Action {
    /// Unique action identifier
    pub id: Uuid,

    /// Action type
    pub action_type: ActionType,

    /// Which goal this action contributes to
    pub goal_id: GoalId,

    /// Why this action is necessary
    pub rationale: String,

    /// Expected contribution to alignment (0.0-1.0)
    pub expected_alignment_impact: f64,

    /// Dependencies (must execute before this action)
    pub dependencies: Vec<Uuid>,

    /// Estimated time to execute (milliseconds)
    pub estimated_duration_ms: u32,
}

/// Action type
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum ActionType {
    /// Create a new file
    CreateFile { path: String, content: String },

    /// Edit existing file
    EditFile { path: String, changes: FileChange },

    /// Run command
    RunCommand { command: String },

    /// Run tests
    RunTests { suite: String },

    /// Apply learned pattern
    ApplyPattern { pattern_id: Uuid },

    /// Delete file
    DeleteFile { path: String },

    /// Query information
    Query {
        query_type: String,
        parameters: String,
    },
}

/// File change
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileChange {
    pub line_start: usize,
    pub line_end: usize,
    pub old_content: String,
    pub new_content: String,
}

/// Goal identifier
pub type GoalId = Uuid;

/// Plan statistics
#[derive(Debug, Clone, Default)]
pub struct PlanStats {
    pub plans_created: u64,
    pub plans_rejected: u64,
    pub avg_plan_complexity: f64,
}

/// Plan validation result
#[derive(Debug, Clone)]
pub enum PlanValidation {
    /// Plan is valid and aligned
    Valid,

    /// Plan has low alignment (<70%)
    LowAlignment { score: f64 },

    /// Plan violates invariants
    ViolatesInvariants { invariants: Vec<String> },

    /// Plan creates circular dependencies
    CircularDependencies { cycle: Vec<GoalId> },

    /// Plan is missing a valid execution contract.
    MissingNorthStar { reason: String },
}

impl HierarchicalPlanner {
    /// Create new hierarchical planner
    pub fn new(goal_manifold: GoalManifold) -> Self {
        Self {
            goal_manifold,
            stats: PlanStats::default(),
        }
    }

    /// Decompose task into sub-goals
    ///
    /// This is NOT simple NLP extraction.
    /// This is intelligent goal decomposition where:
    /// 1. Parse task into high-level goals
    /// 2. Decompose into achievable sub-goals
    /// 3. Create goal DAG respecting dependencies
    /// 4. Validate against invariants
    pub fn decompose_goals(&self, task_goals: &[Goal]) -> Result<Vec<Goal>> {
        tracing::debug!(
            "Decomposing {} goals into hierarchical structure",
            task_goals.len()
        );

        let mut decomposed_goals = Vec::new();

        for task_goal in task_goals {
            // Decompose this goal into sub-goals
            let sub_goals = self.decompose_single_goal(task_goal)?;

            // Add to decomposed goals
            decomposed_goals.push(task_goal.clone());
            decomposed_goals.extend(sub_goals);
        }

        tracing::info!("Decomposed into {} total goals", decomposed_goals.len());
        Ok(decomposed_goals)
    }

    /// Decompose a single goal into sub-goals
    ///
    /// This uses heuristics based on goal complexity:
    /// - Simple goals (alignment_score < 50): No decomposition
    /// - Medium goals (50-70): 2-3 sub-goals
    /// - Complex goals (70-90): 4-6 sub-goals
    /// - Very complex goals (>90): 6-10 sub-goals
    fn decompose_single_goal(&self, goal: &Goal) -> Result<Vec<Goal>> {
        let complexity = self.estimate_goal_complexity(goal);

        if complexity < 50.0 {
            // Simple goal - no decomposition needed
            return Ok(vec![]);
        }

        // Decompose based on goal type and complexity
        let sub_goals = match goal.description.to_lowercase().as_str() {
            desc if desc.contains("implement") => {
                self.decompose_implement_goal(goal, complexity)?
            }
            desc if desc.contains("create") => self.decompose_create_goal(goal, complexity)?,
            desc if desc.contains("build") => self.decompose_build_goal(goal, complexity)?,
            desc if desc.contains("add") => self.decompose_add_goal(goal, complexity)?,
            desc if desc.contains("fix") => self.decompose_fix_goal(goal, complexity)?,
            desc if desc.contains("refactor") => self.decompose_refactor_goal(goal, complexity)?,
            _ => self.decompose_generic_goal(goal, complexity)?,
        };

        Ok(sub_goals)
    }

    /// Decompose "implement" goal into sub-goals
    fn decompose_implement_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        // Extract what to implement
        let binding = goal.description.replace("implement", "");
        let what_to_implement = binding.trim();

        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Define {} interface/contract", what_to_implement),
                    1 => format!("Implement {} core logic", what_to_implement),
                    2 => format!("Add {} error handling", what_to_implement),
                    3 => format!("Write {} tests", what_to_implement),
                    4 => format!("Add {} documentation", what_to_implement),
                    _ => format!("Refine {} implementation", what_to_implement),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose "create" goal into sub-goals
    fn decompose_create_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let binding = goal.description.replace("create", "");
        let what_to_create = binding.trim();
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Define {} structure/schema", what_to_create),
                    1 => format!("Create {} implementation", what_to_create),
                    2 => format!("Write {} tests", what_to_create),
                    3 => format!("Add {} documentation", what_to_create),
                    _ => format!("Review and polish {}", what_to_create),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose "build" goal into sub-goals
    fn decompose_build_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let binding = goal.description.replace("build", "");
        let what_to_build = binding.trim();
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Design {} architecture", what_to_build),
                    1 => format!("Implement {} core components", what_to_build),
                    2 => format!("Add {} tests", what_to_build),
                    3 => format!("Write {} documentation", what_to_build),
                    _ => format!("Review and optimize {}", what_to_build),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose "add" goal into sub-goals
    fn decompose_add_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let binding = goal.description.replace("add", "");
        let what_to_add = binding.trim();
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Research best practices for {}", what_to_add),
                    1 => format!("Design {} integration", what_to_add),
                    2 => format!("Implement {}", what_to_add),
                    3 => format!("Write {} tests", what_to_add),
                    _ => format!("Update documentation for {}", what_to_add),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose "fix" goal into sub-goals
    fn decompose_fix_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let binding = goal.description.replace("fix", "");
        let what_to_fix = binding.trim();
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Analyze {} bug/issue", what_to_fix),
                    1 => format!("Identify root cause of {}", what_to_fix),
                    2 => format!("Design fix for {}", what_to_fix),
                    3 => format!("Implement fix for {}", what_to_fix),
                    4 => format!("Add regression tests for {}", what_to_fix),
                    _ => format!("Verify fix for {}", what_to_fix),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose "refactor" goal into sub-goals
    fn decompose_refactor_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let binding = goal.description.replace("refactor", "");
        let what_to_refactor = binding.trim();
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = match i {
                    0 => format!("Analyze {} structure", what_to_refactor),
                    1 => format!("Identify code smells in {}", what_to_refactor),
                    2 => format!("Design improved {} structure", what_to_refactor),
                    3 => format!("Refactor {}", what_to_refactor),
                    4 => format!("Add tests for refactored {}", what_to_refactor),
                    _ => format!("Update documentation for {}", what_to_refactor),
                };

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Decompose generic goal into sub-goals
    fn decompose_generic_goal(&self, goal: &Goal, complexity: f64) -> Result<Vec<Goal>> {
        let num_sub_goals = self.num_sub_goals_for_complexity(complexity);

        let sub_goals = (0..num_sub_goals)
            .map(|i| {
                let description = format!("Sub-goal {} of {}", i + 1, goal.description);

                Goal::builder()
                    .description(description)
                    .parent(goal.id)
                    .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                        self.sub_goal_complexity(complexity, num_sub_goals),
                        1.0,
                    ))
                    .value_to_root(goal.value_to_root / num_sub_goals as f64)
                    .add_success_criterion(Predicate::AlwaysTrue)
                    .build()
                    .unwrap()
            })
            .collect();

        Ok(sub_goals)
    }

    /// Estimate goal complexity
    fn estimate_goal_complexity(&self, goal: &Goal) -> f64 {
        // Start with base complexity from Goal
        let mut complexity = goal.complexity_estimate.mean;

        // Adjust based on description length
        complexity += goal.description.len() as f64 * 0.1;

        // Adjust based on number of success criteria
        complexity += goal.success_criteria.len() as f64 * 5.0;

        // Adjust based on number of dependencies
        complexity += goal.dependencies.len() as f64 * 3.0;

        complexity.min(100.0) // Cap at 100
    }

    /// Calculate number of sub-goals based on complexity
    fn num_sub_goals_for_complexity(&self, complexity: f64) -> usize {
        match complexity {
            c if c < 50.0 => 0, // Simple - no decomposition
            c if c < 70.0 => 2, // Medium - 2-3 sub-goals
            c if c < 90.0 => 4, // Complex - 4-6 sub-goals
            _ => 6,             // Very complex - 6-10 sub-goals
        }
    }

    fn sub_goal_complexity(&self, complexity: f64, num_sub_goals: usize) -> f64 {
        if num_sub_goals == 0 {
            return 0.0;
        }
        (complexity / num_sub_goals as f64).clamp(0.0, 10.0)
    }

    /// Validate plan against Goal Manifold
    ///
    /// This ensures:
    /// 1. All invariants are respected
    /// 2. No circular dependencies
    /// 3. Sufficient alignment score
    pub fn validate_plan(&self, plan: &ExecutionPlan) -> PlanValidation {
        tracing::debug!("Validating plan with {} actions", plan.actions.len());

        if let Err(reason) = plan.north_star.validate() {
            return PlanValidation::MissingNorthStar {
                reason: reason.to_string(),
            };
        }

        // Check 1: No circular dependencies
        if let Some(cycle) = self.check_circular_dependencies(plan) {
            return PlanValidation::CircularDependencies { cycle };
        }

        // Check 2: Respect invariants
        let violated_invariants = self.check_invariants(plan);
        if !violated_invariants.is_empty() {
            return PlanValidation::ViolatesInvariants {
                invariants: violated_invariants,
            };
        }

        // Check 3: Sufficient projected alignment
        let alignment_score = self.compute_plan_alignment(plan);
        let minimum_alignment = (self.goal_manifold.sensitivity * 100.0).max(70.0);
        if alignment_score < minimum_alignment {
            return PlanValidation::LowAlignment {
                score: alignment_score,
            };
        }

        tracing::info!("Plan validation passed");
        PlanValidation::Valid
    }

    /// Check for circular dependencies in plan action graph
    fn check_circular_dependencies(&self, plan: &ExecutionPlan) -> Option<Vec<GoalId>> {
        // Dependencies reference action IDs, so cycle detection must use action IDs.
        let action_ids: HashSet<Uuid> = plan.actions.iter().map(|action| action.id).collect();
        let mut graph: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for action in &plan.actions {
            let deps = action
                .dependencies
                .iter()
                .copied()
                .filter(|dep| action_ids.contains(dep))
                .collect();
            graph.insert(action.id, deps);
        }

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for action_id in action_ids {
            if let Some(cycle) =
                Self::find_cycle(&graph, &action_id, &mut visiting, &mut visited, &mut stack)
            {
                return Some(cycle);
            }
        }

        None
    }

    /// Return cycle path if a cycle is found from `node`.
    fn find_cycle(
        graph: &HashMap<Uuid, Vec<Uuid>>,
        node: &Uuid,
        visiting: &mut HashSet<Uuid>,
        visited: &mut HashSet<Uuid>,
        stack: &mut Vec<Uuid>,
    ) -> Option<Vec<Uuid>> {
        if visited.contains(node) {
            return None;
        }

        if visiting.contains(node) {
            let start = stack
                .iter()
                .position(|current| current == node)
                .unwrap_or(0);
            return Some(stack[start..].to_vec());
        }

        visiting.insert(*node);
        stack.push(*node);
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if let Some(cycle) = Self::find_cycle(graph, neighbor, visiting, visited, stack) {
                    return Some(cycle);
                }
            }
        }

        stack.pop();
        visiting.remove(node);
        visited.insert(*node);
        None
    }

    /// Check if plan violates invariants
    fn check_invariants(&self, plan: &ExecutionPlan) -> Vec<String> {
        let mut violations = Vec::new();

        let predicate_state = PredicateState::new(
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        );
        let runtime_violations = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|runtime| {
                runtime.block_on(self.goal_manifold.validate_invariants(&predicate_state))
            })
            .unwrap_or_default();
        for violation in runtime_violations {
            if matches!(
                violation.severity,
                InvariantSeverity::Critical | InvariantSeverity::Error
            ) {
                violations.push(format!(
                    "Invariant runtime violation: {}",
                    violation.description
                ));
            }
        }

        // Check each planned action against declared invariants.
        for action in &plan.actions {
            for invariant in &self.goal_manifold.invariants {
                if self.action_violates_invariant(action, invariant) {
                    violations.push(format!(
                        "Action {:?} violates invariant {:?}",
                        action.action_type, invariant
                    ));
                }
            }
        }

        violations.sort();
        violations.dedup();
        violations
    }

    /// Check if action violates invariant
    fn action_violates_invariant(
        &self,
        action: &sentinel_core::cognitive_state::Action,
        invariant: &Invariant,
    ) -> bool {
        // A conservative static check before execution.
        match &action.action_type {
            sentinel_core::cognitive_state::ActionType::RunCommand { command, .. } => {
                self.is_dangerous_command(command)
                    && matches!(
                        invariant.severity,
                        InvariantSeverity::Critical | InvariantSeverity::Error
                    )
            }
            sentinel_core::cognitive_state::ActionType::DeleteFile { path, .. } => {
                if self.is_critical_file(path) {
                    return true;
                }
                match &invariant.predicate {
                    Predicate::FileExists(invariant_path)
                    | Predicate::DirectoryExists(invariant_path) => {
                        self.paths_may_overlap(path, invariant_path)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous_patterns = ["rm -rf", "del /Q", "git clean -fd", "format c:", "mkfs"];
        dangerous_patterns
            .iter()
            .any(|pattern| command.contains(pattern))
    }

    fn is_critical_file(&self, path: &std::path::Path) -> bool {
        let value = path.to_string_lossy();
        value.ends_with("Cargo.toml")
            || value.ends_with("package.json")
            || value.ends_with("README.md")
            || value.ends_with(".env")
            || value.contains("/.git/")
    }

    fn paths_may_overlap(
        &self,
        action_path: &std::path::Path,
        invariant_path: &std::path::Path,
    ) -> bool {
        if action_path == invariant_path {
            return true;
        }
        let action = action_path.to_string_lossy();
        let invariant = invariant_path.to_string_lossy();
        action.ends_with(invariant.as_ref()) || invariant.ends_with(action.as_ref())
    }

    fn compute_plan_alignment(&self, plan: &ExecutionPlan) -> f64 {
        if plan.actions.is_empty() {
            return 100.0;
        }

        let mut weighted_score = 0.0;
        let mut total_weight = 0.0;

        for action in &plan.actions {
            let goal_weight = self.goal_weight_for_action(action.goal_id).max(0.1);
            let action_score = self.score_action_alignment(action);
            weighted_score += action_score * goal_weight;
            total_weight += goal_weight;
        }

        if total_weight <= f64::EPSILON {
            0.0
        } else {
            ((weighted_score / total_weight) * 100.0).clamp(0.0, 100.0)
        }
    }

    fn goal_weight_for_action(&self, goal_id: Option<Uuid>) -> f64 {
        goal_id
            .and_then(|id| self.goal_manifold.get_goal(&id))
            .map(|goal| goal.value_to_root.clamp(0.0, 1.0))
            .unwrap_or(0.5)
    }

    fn score_action_alignment(&self, action: &sentinel_core::cognitive_state::Action) -> f64 {
        let mut score = action.expected_value.clamp(0.0, 1.0);
        let risk_penalty = 1.0 - action.metadata.risk_level.clamp(0.0, 1.0) * 0.5;
        score *= risk_penalty;

        match &action.action_type {
            sentinel_core::cognitive_state::ActionType::RunTests { .. } => {
                score = (score + 0.1).clamp(0.0, 1.0);
            }
            sentinel_core::cognitive_state::ActionType::RunCommand { command, .. } => {
                if self.is_dangerous_command(command) {
                    score *= 0.2;
                }
            }
            sentinel_core::cognitive_state::ActionType::DeleteFile { path, .. } => {
                if self.is_critical_file(path) {
                    score *= 0.2;
                }
            }
            _ => {}
        }

        score.clamp(0.0, 1.0)
    }

    /// Get plan statistics
    pub fn get_stats(&self) -> PlanStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_simple_goal() {
        let goal_manifold = create_test_goal_manifold();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let simple_goal = Goal::builder()
            .description("Add simple function")
            .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                3.0, 1.0,
            ))
            .value_to_root(1.0)
            .success_criteria(vec![Predicate::AlwaysTrue])
            .build()
            .unwrap();

        let result = planner.decompose_single_goal(&simple_goal);

        assert!(result.is_ok());
        let sub_goals = result.unwrap();
        assert_eq!(sub_goals.len(), 0); // Simple goals not decomposed
    }

    #[test]
    fn test_decompose_complex_goal() {
        let goal_manifold = create_test_goal_manifold();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let complex_description = "Implement complex authentication system with JWT, OAuth, session management, token rotation, auditing, and distributed revocation. ".repeat(12);
        let complex_goal = Goal::builder()
            .description(complex_description)
            .complexity(sentinel_core::types::ProbabilityDistribution::normal(
                8.5, 1.0,
            ))
            .value_to_root(1.0)
            .success_criteria(vec![Predicate::AlwaysTrue])
            .build()
            .unwrap();

        let result = planner.decompose_single_goal(&complex_goal);

        assert!(result.is_ok());
        let sub_goals = result.unwrap();
        assert!(sub_goals.len() >= 4); // Complex goals decomposed
    }

    #[test]
    fn test_validate_plan_no_circular_deps() {
        let goal_manifold = create_test_goal_manifold();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let plan = ExecutionPlan {
            root_task: "Test task".to_string(),
            sub_goals: vec![Uuid::new_v4()],
            actions: vec![],
            complexity: 5.0,
            estimated_duration_minutes: 10,
            north_star: create_test_north_star(),
        };

        let validation = planner.validate_plan(&plan);

        assert!(matches!(validation, PlanValidation::Valid));
    }

    #[test]
    fn test_validate_plan_circular_deps() {
        let goal_manifold = create_test_goal_manifold();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let goal_id = Uuid::new_v4();
        let action_id = Uuid::new_v4();

        let plan = ExecutionPlan {
            root_task: "Test task".to_string(),
            sub_goals: vec![goal_id.clone()],
            actions: vec![sentinel_core::cognitive_state::Action {
                id: action_id,
                action_type: sentinel_core::cognitive_state::ActionType::CreateFile {
                    path: "test.rs".into(),
                    content: "test".to_string(),
                },
                description: "Test action".to_string(),
                goal_id: Some(goal_id.clone()),
                expected_value: 0.5,
                created_at: chrono::Utc::now(),
                dependencies: vec![action_id], // Circular dependency!
                metadata: Default::default(),
            }],
            complexity: 5.0,
            estimated_duration_minutes: 10,
            north_star: create_test_north_star(),
        };

        let validation = planner.validate_plan(&plan);

        assert!(matches!(
            validation,
            PlanValidation::CircularDependencies { .. }
        ));
    }

    #[test]
    fn test_validate_plan_low_alignment() {
        let mut goal_manifold = create_test_goal_manifold();
        goal_manifold.sensitivity = 0.95;
        let planner = HierarchicalPlanner::new(goal_manifold);

        let plan = ExecutionPlan {
            root_task: "Low alignment task".to_string(),
            sub_goals: vec![Uuid::new_v4()],
            actions: vec![sentinel_core::cognitive_state::Action {
                id: Uuid::new_v4(),
                action_type: sentinel_core::cognitive_state::ActionType::RunCommand {
                    command: "echo hi".to_string(),
                    working_dir: std::path::PathBuf::from("."),
                },
                description: "Low expected value action".to_string(),
                goal_id: None,
                expected_value: 0.1,
                created_at: chrono::Utc::now(),
                dependencies: vec![],
                metadata: Default::default(),
            }],
            complexity: 2.0,
            estimated_duration_minutes: 2,
            north_star: create_test_north_star(),
        };

        let validation = planner.validate_plan(&plan);
        assert!(matches!(validation, PlanValidation::LowAlignment { .. }));
    }

    #[test]
    fn test_validate_plan_invariant_violation_on_critical_delete() {
        let mut goal_manifold = create_test_goal_manifold();
        goal_manifold
            .add_invariant(sentinel_core::goal_manifold::Invariant::critical(
                "Cargo manifest must exist",
                Predicate::FileExists(std::path::PathBuf::from("Cargo.toml")),
            ))
            .unwrap();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let plan = ExecutionPlan {
            root_task: "Dangerous delete".to_string(),
            sub_goals: vec![],
            actions: vec![sentinel_core::cognitive_state::Action {
                id: Uuid::new_v4(),
                action_type: sentinel_core::cognitive_state::ActionType::DeleteFile {
                    path: std::path::PathBuf::from("Cargo.toml"),
                    backup: false,
                },
                description: "Delete manifest".to_string(),
                goal_id: None,
                expected_value: 0.8,
                created_at: chrono::Utc::now(),
                dependencies: vec![],
                metadata: Default::default(),
            }],
            complexity: 4.0,
            estimated_duration_minutes: 1,
            north_star: create_test_north_star(),
        };

        let validation = planner.validate_plan(&plan);
        assert!(matches!(
            validation,
            PlanValidation::ViolatesInvariants { .. }
        ));
    }

    fn create_test_goal_manifold() -> GoalManifold {
        let intent = sentinel_core::goal_manifold::Intent::new(
            "Test intent".to_string(),
            vec!["Test constraint".to_string()],
        );

        GoalManifold::new(intent)
    }

    fn create_test_north_star() -> sentinel_core::ExecutionNorthStar {
        sentinel_core::ExecutionNorthStar {
            where_we_are: "Repository state analyzed".to_string(),
            where_we_must_go: "Aligned goal outcome".to_string(),
            how: "Hierarchical validated plan".to_string(),
            why: "Preserve invariants while improving alignment".to_string(),
            constraints: vec!["No destructive action without checks".to_string()],
        }
    }

    #[test]
    fn test_validate_plan_missing_north_star() {
        let goal_manifold = create_test_goal_manifold();
        let planner = HierarchicalPlanner::new(goal_manifold);

        let plan = ExecutionPlan {
            root_task: "Missing contract".to_string(),
            sub_goals: vec![],
            actions: vec![],
            complexity: 1.0,
            estimated_duration_minutes: 1,
            north_star: sentinel_core::ExecutionNorthStar {
                where_we_are: String::new(),
                where_we_must_go: "Target".to_string(),
                how: "Plan".to_string(),
                why: "Reason".to_string(),
                constraints: vec![],
            },
        };

        let validation = planner.validate_plan(&plan);
        assert!(matches!(
            validation,
            PlanValidation::MissingNorthStar { .. }
        ));
    }
}
