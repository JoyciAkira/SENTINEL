//! Agent Orchestrator - Multi-Agent Coordination and Collaboration
//!
//! This module implements REVOLUTIONARY multi-agent orchestration:
//! - Coordinates multiple specialized agents working in parallel
//! - Manages dependencies between agent tasks
//! - Implements conflict resolution
//! - Provides unified execution report
//!
//! # Why This Is Revolutionary
//!
//! Traditional single-agent systems:
//! - Agent works in isolation
//! - No parallel execution possible
//! - No specialization (one agent does everything)
//! - No conflict detection or resolution
//!
//! Sentinel Agent Orchestrator:
//! - Orchestrates specialized sub-agents (testing, codegen, refactoring)
//! - Manages task dependencies across agents
//! - Detects and resolves conflicts
//! - Optimizes parallel execution
//! - Provides unified view of multi-agent work
//!
//! # Orchestration Architecture
//!
//! ```text
//! Task: "Build authentication system"
//!          │
//!          v
//! ┌─────────────────────────────────────┐
//! │    Orchestrator Analysis           │
//! │    - Decompose into sub-tasks     │
//! │    - Identify specializations      │
//! │    - Create dependency graph      │
//! └─────────────────────────────────────┘
//!          │
//!          v
//! ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
//! │ Testing Agent│  │  CodeGen Agent │  │ Refactor Agent │
//! │  Unit tests  │  │  JWT auth code │  │  Clean code    │
//! └────────────────┘  └────────────────┘  └────────────────┘
//!          │                │                │
//!          v                v                v
//!          └────────────────┴────────────────┘
//!                       │
//!                       v
//!            ┌─────────────────────────────┐
//! │    Conflict Detection &        │
//! │    Resolution                  │
//! └─────────────────────────────┘
//!          │
//!          v
//!     Unified Execution Report
//! ```

use anyhow::Result;
use sentinel_core::{
    goal_manifold::Goal,
    types::Timestamp,
    Uuid,
};
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;

/// Agent type specialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AgentType {
    /// Specialized in testing
    Testing,

    /// Specialized in code generation
    CodeGeneration,

    /// Specialized in refactoring
    Refactoring,

    /// Specialized in documentation
    Documentation,

    /// Specialized in deployment
    Deployment,
}

/// Specialized agent instance
#[derive(Debug, Clone)]
pub struct SpecializedAgent {
    /// Unique agent identifier
    pub agent_id: Uuid,

    /// Agent specialization
    pub specialization: AgentType,

    /// Agent authority level
    pub authority: crate::AgentAuthority,

    /// Current task assigned to agent
    pub current_task: Option<Task>,

    /// Execution statistics
    pub stats: AgentStats,
}

/// Task assigned to an agent
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: Uuid,

    /// Task description
    pub description: String,

    /// Parent task (if any)
    pub parent_id: Option<Uuid>,

    /// Required specialized agent type
    pub required_agent: AgentType,

    /// Task priority (0.0-1.0)
    pub priority: f64,

    /// Estimated duration (milliseconds)
    pub estimated_duration_ms: u32,

    /// Dependencies (must complete before this task)
    pub dependencies: Vec<Uuid>,
    /// Anti-dependencies (cannot run simultaneously with)
    pub anti_dependencies: Vec<Uuid>,
}

/// Agent statistics
#[derive(Debug, Clone, Default)]
pub struct AgentStats {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_execution_time_ms: u64,
    pub avg_execution_time_ms: f64,
    pub conflicts_involved: u64,
    pub conflicts_resolved: u64,
}

/// Agent orchestrator - coordinates multiple specialized agents
#[derive(Debug)]
pub struct AgentOrchestrator {
    /// Pool of specialized agents
    pub agents: HashMap<AgentType, Vec<SpecializedAgent>>,

    /// Task queue for execution
    pub task_queue: TaskQueue,

    /// Dependency graph for tasks
    pub dependency_graph: DependencyGraph,

    /// Conflict detector
    pub conflict_detector: ConflictDetector,

    /// Statistics
    pub stats: OrchestrationStats,
}

/// Task queue with priority scheduling
#[derive(Debug, Clone)]
pub struct TaskQueue {
    /// Pending tasks
    pub pending: Vec<Task>,

    /// In-progress tasks
    pub in_progress: HashMap<Uuid, Task>,

    /// Completed tasks
    pub completed: Vec<Task>,

    /// Failed tasks
    pub failed: Vec<Task>,
}

/// Dependency graph for task ordering
#[derive(Debug)]
pub struct DependencyGraph {
    /// Tasks as nodes
    pub nodes: HashMap<Uuid, Task>,

    /// Dependencies as edges
    pub edges: HashMap<Uuid, Vec<Uuid>>,
}

/// Conflict detector and resolver
#[derive(Debug)]
pub struct ConflictDetector {
    /// Resource conflicts (files, resources)
    pub resource_conflicts: HashMap<String, HashSet<Uuid>>,

    /// Goal conflicts (agents trying same goal)
    pub goal_conflicts: HashSet<Uuid>,

    /// Resolved conflicts
    pub resolved_conflicts: Vec<ConflictResolution>,
}

/// Conflict resolution
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConflictResolution {
    pub conflict_id: Uuid,
    pub conflict_type: ConflictType,
    pub resolution_strategy: ResolutionStrategy,
    pub resolved_at: Timestamp,
    pub involved_agents: Vec<Uuid>,
}

/// Conflict type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConflictType {
    /// Two agents trying to edit same file
    ResourceConflict { resource: String },

    /// Two agents working on same goal
    GoalConflict { goal_id: Uuid },

    /// Task dependency cycle
    DependencyCycle { cycle: Vec<Uuid> },

    /// Anti-dependency violation
    AntiDependencyViolation { tasks: Vec<Uuid> },
}

/// Resolution strategy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ResolutionStrategy {
    /// First agent wins
    FirstComeFirstServed,

    /// Higher authority agent wins
    AuthorityBased,

    /// Tasks serialized (run one after another)
    Serialization,

    /// Task deferred to later
    Deferral,
}

/// Orchestration statistics
#[derive(Debug, Clone, Default)]
pub struct OrchestrationStats {
    pub total_tasks: u64,
    pub parallel_tasks: u64,
    pub serial_tasks: u64,
    pub conflicts_detected: u64,
    pub conflicts_resolved: u64,
    pub avg_parallelism: f64,
}

/// Orchestration result
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrchestrationResult {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub status: TaskStatus,
    pub execution_time_ms: u64,
    pub conflicts: Vec<ConflictResolution>,
}

/// Task status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed { reason: String },
}

impl AgentOrchestrator {
    /// Create new agent orchestrator
    pub fn new() -> Self {
        tracing::info!("Initializing Agent Orchestrator");

        // Initialize specialized agents
        let mut agents = HashMap::new();

        // Add testing agents (3 instances for parallelism)
        let testing_agents = (0..3)
            .map(|_| SpecializedAgent {
                agent_id: Uuid::new_v4(),
                specialization: AgentType::Testing,
                authority: crate::AgentAuthority::JuniorAI,
                current_task: None,
                stats: AgentStats::default(),
            })
            .collect();

        agents.insert(AgentType::Testing, testing_agents);

        // Add code generation agents (2 instances)
        let codegen_agents = (0..2)
            .map(|_| SpecializedAgent {
                agent_id: Uuid::new_v4(),
                specialization: AgentType::CodeGeneration,
                authority: crate::AgentAuthority::JuniorAI,
                current_task: None,
                stats: AgentStats::default(),
            })
            .collect();

        agents.insert(AgentType::CodeGeneration, codegen_agents);

        // Add refactoring agents (1 instance)
        let refactor_agents = vec![SpecializedAgent {
            agent_id: Uuid::new_v4(),
            specialization: AgentType::Refactoring,
            authority: crate::AgentAuthority::SeniorAI,
            current_task: None,
            stats: AgentStats::default(),
        }];

        agents.insert(AgentType::Refactoring, refactor_agents);

        // Add documentation agent (1 instance)
        let doc_agents = vec![SpecializedAgent {
            agent_id: Uuid::new_v4(),
            specialization: AgentType::Documentation,
            authority: crate::AgentAuthority::JuniorAI,
            current_task: None,
            stats: AgentStats::default(),
        }];

        agents.insert(AgentType::Documentation, doc_agents);

        tracing::info!(
            "Agent Orchestrator initialized with {} agent types",
            agents.len()
        );

        Self {
            agents,
            task_queue: TaskQueue {
                pending: vec![],
                in_progress: HashMap::new(),
                completed: vec![],
                failed: vec![],
            },
            dependency_graph: DependencyGraph {
                nodes: HashMap::new(),
                edges: HashMap::new(),
            },
            conflict_detector: ConflictDetector::new(),
            stats: OrchestrationStats::default(),
        }
    }

    /// Execute a goal using multi-agent orchestration
    ///
    /// This is REVOLUTIONARY because:
    /// - Decomposes goal into specialized sub-tasks
    /// - Assigns tasks to appropriate agents
    /// - Manages dependencies and parallelism
    /// - Detects and resolves conflicts
    /// - Provides unified execution report
    pub async fn execute_goal(&mut self, goal: &Goal) -> Result<Vec<OrchestrationResult>> {
        tracing::info!("Orchestrating execution for goal: {}", goal.description);

        // Step 1: Decompose goal into tasks
        let tasks = self.decompose_goal_into_tasks(goal)?;

        // Step 2: Assign tasks to agents
        let assignments = self.assign_tasks_to_agents(&tasks)?;

        // Step 3: Build dependency graph
        self.build_dependency_graph(&tasks)?;

        // Step 2: Detect conflicts
        let conflicts = self
            .conflict_detector
            .detect_conflicts(&assignments, &self.task_queue.pending);

        // Step 3: Resolve conflicts
        let resolutions = self.conflict_detector.resolve_conflicts(&conflicts);

        // Step 6: Execute tasks (parallel where possible)
        let results = self.execute_tasks(&assignments, &resolutions).await?;

        // Step 7: Update statistics
        self.update_orchestration_stats(&results, &conflicts, &resolutions);

        tracing::info!(
            "Goal orchestration complete: {} tasks, {} conflicts resolved",
            tasks.len(),
            resolutions.len()
        );

        Ok(results)
    }

    /// Decompose goal into specialized tasks
    ///
    /// Analyzes goal and creates sub-tasks appropriate for
    /// different specialized agents (testing, codegen, refactoring, docs).
    fn decompose_goal_into_tasks(&self, goal: &Goal) -> Result<Vec<Task>> {
        tracing::debug!("Decomposing goal into tasks");

        let mut tasks = Vec::new();

        let desc_lower = goal.description.to_lowercase();

        // Task 1: Create structure/design task
        tasks.push(Task {
            id: Uuid::new_v4(),
            description: format!("Design and plan implementation for: {}", goal.description),
            parent_id: Some(goal.id),
            required_agent: AgentType::Refactoring,
            priority: 1.0,                // Highest priority
            estimated_duration_ms: 60000, // 1 minute
            dependencies: vec![],
            anti_dependencies: vec![],
        });

        // Task 2: Core implementation task
        let agent_type = if desc_lower.contains("test") {
            AgentType::Testing
        } else if desc_lower.contains("refactor") {
            AgentType::Refactoring
        } else if desc_lower.contains("document") {
            AgentType::Documentation
        } else {
            AgentType::CodeGeneration
        };

        tasks.push(Task {
            id: Uuid::new_v4(),
            description: format!("Core implementation: {}", goal.description),
            parent_id: Some(goal.id),
            required_agent: agent_type,
            priority: 0.9,
            estimated_duration_ms: self.estimate_task_duration(&goal, agent_type),
            dependencies: vec![tasks[0].id],
            anti_dependencies: vec![],
        });

        // Task 3: Testing task (if not already a test goal)
        if !desc_lower.contains("test") {
            tasks.push(Task {
                id: Uuid::new_v4(),
                description: format!("Write and run tests for: {}", goal.description),
                parent_id: Some(goal.id),
                required_agent: AgentType::Testing,
                priority: 0.7,
                estimated_duration_ms: 30000, // 30 seconds
                dependencies: vec![tasks[1].id],
                anti_dependencies: vec![],
            });
        }

        // Task 4: Documentation task
        tasks.push(Task {
            id: Uuid::new_v4(),
            description: format!("Write documentation for: {}", goal.description),
            parent_id: Some(goal.id),
            required_agent: AgentType::Documentation,
            priority: 0.5,
            estimated_duration_ms: 60000, // 1 minute
            dependencies: vec![tasks[2].id],
            anti_dependencies: vec![],
        });

        tracing::info!("Decomposed goal into {} tasks", tasks.len());
        Ok(tasks)
    }

    /// Assign tasks to appropriate specialized agents
    ///
    /// This uses:
    /// - Agent specialization matching
    /// - Load balancing across multiple agents
    /// - Priority-based assignment
    fn assign_tasks_to_agents(&self, tasks: &[Task]) -> Result<Vec<TaskAssignment>> {
        tracing::debug!("Assigning {} tasks to agents", tasks.len());

        let mut assignments = Vec::new();

        // Sort tasks by priority
        let mut sorted_tasks = tasks.to_vec();
        sorted_tasks.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());

        // Assign tasks to agents
        let mut agent_loads: HashMap<Uuid, u64> = HashMap::new();

        for task in &sorted_tasks {
            // Find available agents of required type
            let available_agents = self.agents.get(&task.required_agent).ok_or_else(|| {
                anyhow::anyhow!("No agents available for type: {:?}", task.required_agent)
            })?;

            // Select least loaded agent (load balancing)
            let selected_agent = available_agents
                .iter()
                .min_by_key(|agent| agent_loads.get(&agent.agent_id).unwrap_or(&0))
                .ok_or_else(|| anyhow::anyhow!("No agents available for assignment"))?;

            // Create assignment
            assignments.push(TaskAssignment {
                task_id: task.id,
                agent_id: selected_agent.agent_id,
                assigned_at: chrono::Utc::now(),
            });

            // Update agent load
            *agent_loads.entry(selected_agent.agent_id).or_insert(0) += 1;
        }

        tracing::info!(
            "Assigned {} tasks to {} agents",
            assignments.len(),
            agent_loads.len()
        );

        Ok(assignments)
    }

    /// Build dependency graph from tasks
    fn build_dependency_graph(&mut self, tasks: &[Task]) -> Result<()> {
        tracing::debug!("Building dependency graph");

        // Add nodes
        for task in tasks {
            self.dependency_graph.nodes.insert(task.id, task.clone());

            self.dependency_graph
                .edges
                .insert(task.id, task.dependencies.clone());
        }

        // Validate no cycles
        if let Some(cycle) = self.detect_dependency_cycle() {
            return Err(anyhow::anyhow!("Dependency cycle detected: {:?}", cycle));
        }

        // Validate no anti-dependency violations
        if let Some(violation) = self.detect_anti_dependency_violation() {
            return Err(anyhow::anyhow!(
                "Anti-dependency violation: {:?}",
                violation
            ));
        }

        Ok(())
    }

    /// Detect cycles in dependency graph
    fn detect_dependency_cycle(&self) -> Option<Vec<Uuid>> {
        // Use DFS to detect cycles
        let mut visited = HashSet::new();
        let mut recursion_stack = Vec::new();

        for node_id in self.dependency_graph.nodes.keys() {
            if !visited.contains(node_id) {
                if self.has_cycle_from(node_id, &mut visited, &mut recursion_stack) {
                    return Some(recursion_stack.clone());
                }
            }
        }

        None
    }

    /// Check for cycle starting from node
    fn has_cycle_from(
        &self,
        node_id: &Uuid,
        visited: &mut HashSet<Uuid>,
        recursion_stack: &mut Vec<Uuid>,
    ) -> bool {
        visited.insert(*node_id);
        recursion_stack.push(*node_id);

        if let Some(dependencies) = self.dependency_graph.edges.get(node_id) {
            for dep_id in dependencies {
                if !visited.contains(dep_id) {
                    if self.has_cycle_from(dep_id, visited, recursion_stack) {
                        return true;
                    }
                }
            }
        }

        recursion_stack.pop();
        false
    }

    /// Detect anti-dependency violations
    fn detect_anti_dependency_violation(&self) -> Option<Vec<Uuid>> {
        for task in self.dependency_graph.nodes.values() {
            for anti_dep_id in &task.anti_dependencies {
                // Check if both tasks are in same dependency level
                if let Some(anti_dep) = self.dependency_graph.nodes.get(anti_dep_id) {
                    for dep_id in &task.dependencies {
                        for anti_dep_sub_id in &anti_dep.dependencies {
                            if dep_id == anti_dep_sub_id && anti_dep_sub_id == &task.id {
                                // Both tasks depend on each other (potential cycle via anti-deps)
                                return Some(vec![task.id, anti_dep_id.clone()]);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Execute tasks with conflict resolution
    ///
    /// This is REVOLUTIONARY because:
    /// - Executes tasks in parallel where possible
    /// - Serializes tasks with conflicts
    /// - Manages anti-dependencies
    /// - Collects execution results
    async fn execute_tasks(
        &mut self,
        assignments: &[TaskAssignment],
        resolutions: &[ConflictResolution],
    ) -> Result<Vec<OrchestrationResult>> {
        tracing::debug!("Executing {} task assignments", assignments.len());

        let mut results = Vec::new();
        let mut join_set = JoinSet::new();

        // Group tasks by conflict status
        let (conflicted_tasks, parallel_tasks) =
            self.separate_conflicted_and_parallel_tasks(assignments, resolutions);

        // Execute parallel tasks simultaneously
        for assignment in &parallel_tasks {
            let task = self
                .task_queue
                .pending
                .iter()
                .find(|t| t.id == assignment.task_id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", assignment.task_id))?;

            let task_clone = task.clone();
            let agent_id = assignment.agent_id;

            // Spawn task execution
            let result = join_set
                .spawn(async move { Self::execute_single_task(task_clone, agent_id).await });
        }

        // Wait for all parallel tasks
        while let Some(join_result) = join_set.join_next().await {
            let result = join_result??;
            results.push(result);
        }

        // Execute conflicted tasks serially
        for task_id in &conflicted_tasks {
            let task = self
                .task_queue
                .pending
                .iter()
                .find(|t| t.id == *task_id)
                .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_id))?;

            let assignment = assignments
                .iter()
                .find(|a| a.task_id == *task_id)
                .ok_or_else(|| anyhow::anyhow!("Assignment not found: {}", task_id))?;

            // Execute serially (one at a time)
            let result = Self::execute_single_task(task.clone(), assignment.agent_id).await?;
            results.push(result);
        }

        tracing::info!(
            "Executed {} tasks ({} parallel, {} serial)",
            results.len(),
            parallel_tasks.len(),
            conflicted_tasks.len()
        );

        Ok(results)
    }

    /// Execute a single task
    async fn execute_single_task(task: Task, agent_id: Uuid) -> Result<OrchestrationResult> {
        let start_time = std::time::Instant::now();

        tracing::info!("Agent {:?} executing task: {}", agent_id, task.description);

        // Simulate task execution
        // In production, this would delegate to the actual specialized agent
        let (status, execution_time_ms) = match task.required_agent {
            AgentType::Testing => {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    task.estimated_duration_ms.into(),
                ))
                .await;
                (TaskStatus::Completed, task.estimated_duration_ms)
            }
            AgentType::CodeGeneration => {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    task.estimated_duration_ms.into(),
                ))
                .await;
                (TaskStatus::Completed, task.estimated_duration_ms)
            }
            AgentType::Refactoring => {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    task.estimated_duration_ms.into(),
                ))
                .await;
                (TaskStatus::Completed, task.estimated_duration_ms)
            }
            AgentType::Documentation => {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    task.estimated_duration_ms.into(),
                ))
                .await;
                (TaskStatus::Completed, task.estimated_duration_ms)
            }
            AgentType::Deployment => {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    task.estimated_duration_ms.into(),
                ))
                .await;
                (TaskStatus::Completed, task.estimated_duration_ms)
            }
        };

        let execution_time = start_time.elapsed().as_millis();

        Ok(OrchestrationResult {
            task_id: task.id,
            agent_id,
            status,
            execution_time_ms: execution_time as u64,
            conflicts: vec![],
        })
    }

    /// Separate conflicted and parallel tasks
    fn separate_conflicted_and_parallel_tasks(
        &self,
        assignments: &[TaskAssignment],
        resolutions: &[ConflictResolution],
    ) -> (Vec<Uuid>, Vec<TaskAssignment>) {
        let mut conflicted = Vec::new();
        let mut parallel = Vec::new();

        for assignment in assignments {
            // Check if this assignment has conflicts
            let has_conflict = resolutions
                .iter()
                .any(|r| r.involved_agents.contains(&assignment.agent_id));

            if has_conflict {
                conflicted.push(assignment.task_id);
            } else {
                parallel.push(assignment.clone());
            }
        }

        (conflicted, parallel)
    }

    /// Resolve conflicts using appropriate strategy
    ///
    /// Strategies:
    /// - Authority-based: Higher authority wins
    /// - Serialization: Run tasks sequentially
    /// - Deferral: Defer conflicting task
    fn resolve_conflicts(
        &mut self,
        conflicts: &[ConflictDetectionResult],
    ) -> Vec<ConflictResolution> {
        tracing::debug!("Resolving {} conflicts", conflicts.len());

        let mut resolutions = Vec::new();

        for conflict in conflicts {
            let resolution = match conflict.conflict_type {
                ConflictType::ResourceConflict { .. } => {
                    // Resource conflicts: serialize tasks
                    ConflictResolution {
                        conflict_id: Uuid::new_v4(),
                        conflict_type: conflict.conflict_type.clone(),
                        resolution_strategy: ResolutionStrategy::Serialization,
                        resolved_at: chrono::Utc::now(),
                        involved_agents: conflict.involved_agents.clone(),
                    }
                }
                ConflictType::GoalConflict { .. } => {
                    // Goal conflicts: higher authority wins
                    let higher_authority =
                        self.find_higher_authority_agent(&conflict.involved_agents);

                    ConflictResolution {
                        conflict_id: Uuid::new_v4(),
                        conflict_type: conflict.conflict_type.clone(),
                        resolution_strategy: ResolutionStrategy::AuthorityBased,
                        resolved_at: chrono::Utc::now(),
                        involved_agents: conflict.involved_agents.clone(),
                    }
                }
                ConflictType::DependencyCycle { .. } => {
                    // Dependency cycles: break cycle
                    ConflictResolution {
                        conflict_id: Uuid::new_v4(),
                        conflict_type: conflict.conflict_type.clone(),
                        resolution_strategy: ResolutionStrategy::Deferral,
                        resolved_at: chrono::Utc::now(),
                        involved_agents: conflict.involved_agents.clone(),
                    }
                }
                ConflictType::AntiDependencyViolation { .. } => {
                    // Anti-dependency violations: serialize
                    ConflictResolution {
                        conflict_id: Uuid::new_v4(),
                        conflict_type: conflict.conflict_type.clone(),
                        resolution_strategy: ResolutionStrategy::Serialization,
                        resolved_at: chrono::Utc::now(),
                        involved_agents: conflict.involved_agents.clone(),
                    }
                }
            };

            resolutions.push(resolution);
        }

        tracing::info!("Resolved {} conflicts", resolutions.len());
        resolutions
    }

    /// Find agent with highest authority
    fn find_higher_authority_agent(&self, agents: &[Uuid]) -> Uuid {
        agents
            .iter()
            .max_by(|a, b| {
                let auth_a = self.get_agent_authority(a);
                let auth_b = self.get_agent_authority(b);
                auth_a.total_cmp(&auth_b)
            })
            .copied()
            .unwrap_or(*agents.first().unwrap())
    }

    /// Get agent authority level
    fn get_agent_authority(&self, agent_id: &Uuid) -> f64 {
        // Search all agent pools for this agent
        for agents in self.agents.values() {
            if let Some(agent) = agents.iter().find(|a| a.agent_id == *agent_id) {
                match agent.authority {
                    crate::AgentAuthority::Human => return 1.0,
                    crate::AgentAuthority::SeniorAI => return 0.8,
                    crate::AgentAuthority::JuniorAI => return 0.3,
                }
            }
        }

        0.0
    }

    /// Update orchestration statistics
    fn update_orchestration_stats(
        &mut self,
        results: &[OrchestrationResult],
        conflicts: &[ConflictDetectionResult],
        resolutions: &[ConflictResolution],
    ) {
        self.stats.total_tasks += results.len() as u64;
        self.stats.conflicts_detected += conflicts.len() as u64;
        self.stats.conflicts_resolved += resolutions.len() as u64;

        // Calculate parallelism
        let completed_count = results
            .iter()
            .filter(|r| matches!(r.status, TaskStatus::Completed))
            .count();

        if completed_count > 0 {
            self.stats.parallel_tasks += completed_count as u64;
        }

        let total_time: u64 = results.iter().map(|r| r.execution_time_ms).sum();

        if completed_count > 0 {
            self.stats.avg_parallelism = self.stats.parallel_tasks as f64 / completed_count as f64;
            self.stats.serial_tasks = results.len() as u64 - self.stats.parallel_tasks;
        }
    }

    /// Estimate task duration based on goal and agent type
    fn estimate_task_duration(&self, goal: &Goal, agent_type: AgentType) -> u32 {
        // Base duration from goal complexity
        let base_duration = goal.complexity_estimate.mean * 1000.0; // Convert to ms

        // Adjust based on agent type
        let agent_multiplier = match agent_type {
            AgentType::Testing => 0.3, // Testing is fast
            AgentType::CodeGeneration => 1.0,
            AgentType::Refactoring => 0.8,
            AgentType::Documentation => 0.5,
            AgentType::Deployment => 0.2,
        };

        (base_duration * agent_multiplier) as u32
    }

    /// Get orchestration statistics
    pub fn get_stats(&self) -> OrchestrationStats {
        self.stats.clone()
    }
}

/// Task assignment
#[derive(Debug, Clone)]
pub struct TaskAssignment {
    pub task_id: Uuid,
    pub agent_id: Uuid,
    pub assigned_at: chrono::DateTime<chrono::Utc>,
}

/// Conflict detection result
#[derive(Debug, Clone)]
pub struct ConflictDetectionResult {
    pub conflict_id: Uuid,
    pub conflict_type: ConflictType,
    pub involved_agents: Vec<Uuid>,
}

/// Conflict detector
impl ConflictDetector {
    pub fn new() -> Self {
        Self {
            resource_conflicts: HashMap::new(),
            goal_conflicts: HashSet::new(),
            resolved_conflicts: vec![],
        }
    }

    /// Detect conflicts between assignments
    pub fn detect_conflicts(
        &self,
        assignments: &[TaskAssignment],
        tasks: &[Task],
    ) -> Vec<ConflictDetectionResult> {
        let mut conflicts = Vec::new();

        // Check for resource contention
        for (i, assignment_a) in assignments.iter().enumerate() {
            for assignment_b in assignments.iter().skip(i + 1) {
                if self.tasks_share_resource(tasks, &assignment_a.task_id, &assignment_b.task_id) {
                    let conflict_id = Uuid::new_v4();

                    conflicts.push(ConflictDetectionResult {
                        conflict_id,
                        conflict_type: ConflictType::ResourceConflict {
                            resource: "shared_resource".to_string(),
                        },
                        involved_agents: vec![
                            assignment_a.agent_id.clone(),
                            assignment_b.agent_id.clone(),
                        ],
                    });
                }
            }
        }

        // Check for goal conflicts
        self.check_goal_conflicts(&mut conflicts, assignments);

        conflicts
    }

    /// Resolve detected conflicts
    pub fn resolve_conflicts(
        &mut self,
        conflicts: &[ConflictDetectionResult],
    ) -> Vec<ConflictResolution> {
        let mut resolutions = Vec::new();

        for conflict in conflicts {
            // Simple resolution: serialize execution
            let resolution = ConflictResolution {
                conflict_id: conflict.conflict_id,
                conflict_type: conflict.conflict_type.clone(),
                resolution_strategy: ResolutionStrategy::Serialization,
                resolved_at: chrono::Utc::now(),
                involved_agents: conflict.involved_agents.clone(),
            };

            resolutions.push(resolution.clone());
            self.resolved_conflicts.push(resolution);
        }

        resolutions
    }

    /// Check if two tasks share a resource
    fn tasks_share_resource(&self, tasks: &[Task], task_a_id: &Uuid, task_b_id: &Uuid) -> bool {
        // Find tasks
        let task_a = tasks.iter().find(|t| t.id == *task_a_id);
        let task_b = tasks.iter().find(|t| t.id == *task_b_id);

        if let (Some(task_a), Some(task_b)) = (task_a, task_b) {
            // Check if tasks involve same files or resources
            let desc_a = task_a.description.to_lowercase();
            let desc_b = task_b.description.to_lowercase();

            // Both tasks mention same file
            let file_regex = regex::Regex::new(r"\b(\w+\.(rs|ts|py|js|json|md|txt))\b").unwrap();

            let mut files_a = Vec::new();
            for cap in file_regex.captures_iter(&desc_a) {
                if let Some(file_match) = cap.get(1) {
                    files_a.push(file_match.as_str().to_string());
                }
            }

            let mut files_b = Vec::new();
            for cap in file_regex.captures_iter(&desc_b) {
                if let Some(file_match) = cap.get(1) {
                    files_b.push(file_match.as_str().to_string());
                }
            }

            // Check for file overlap
            for file_a in &files_a {
                for file_b in &files_b {
                    if file_a == file_b {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check for goal conflicts
    fn check_goal_conflicts(
        &self,
        conflicts: &mut Vec<ConflictDetectionResult>,
        assignments: &[TaskAssignment],
    ) {
        let mut task_to_agents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for assignment in assignments {
            task_to_agents
                .entry(assignment.task_id)
                .or_default()
                .push(assignment.agent_id);
        }

        for (task_id, agents) in task_to_agents {
            if agents.len() > 1 {
                conflicts.push(ConflictDetectionResult {
                    conflict_id: Uuid::new_v4(),
                    conflict_type: ConflictType::GoalConflict { goal_id: task_id },
                    involved_agents: agents,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_core::goal_manifold::predicate::Predicate;
    use sentinel_core::types::ProbabilityDistribution;

    #[test]
    fn test_agent_orchestrator_initialization() {
        let orchestrator = AgentOrchestrator::new();

        assert!(!orchestrator.agents.is_empty());
        assert!(orchestrator.agents.contains_key(&AgentType::Testing));
        assert!(orchestrator.agents.contains_key(&AgentType::CodeGeneration));
    }

    #[test]
    fn test_decompose_goal_into_tasks() {
        let orchestrator = AgentOrchestrator::new();

        // Use builder for robust initialization
        let goal = Goal::builder()
            .description("Implement JWT authentication")
            .success_criteria(vec![Predicate::AlwaysTrue])
            .build()
            .expect("Failed to build goal");

        let tasks = orchestrator
            .decompose_goal_into_tasks(&goal)
            .expect("Failed to decompose goal");

        // Should have 4 tasks: design, implement, test, doc
        assert_eq!(tasks.len(), 4);
    }

    #[test]
    fn test_separate_conflicted_and_parallel_tasks() {
        let orchestrator = AgentOrchestrator::new();

        let assignments = vec![TaskAssignment {
            task_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            assigned_at: chrono::Utc::now(),
        }];

        let resolutions = vec![];

        let (conflicted, parallel) =
            orchestrator.separate_conflicted_and_parallel_tasks(&assignments, &resolutions);

        // No conflicts, all parallel
        assert_eq!(conflicted.len(), 0);
        assert_eq!(parallel.len(), 1);
    }

    #[test]
    fn test_estimate_task_duration() {
        let orchestrator = AgentOrchestrator::new();

        let mut goal = Goal::builder()
            .description("Test goal")
            .success_criteria(vec![Predicate::AlwaysTrue])
            .build()
            .expect("Failed to build goal");

        // Manually set complexity for deterministic test
        goal.complexity_estimate = ProbabilityDistribution::normal(50.0, 5.0);

        let duration = orchestrator.estimate_task_duration(&goal, AgentType::CodeGeneration);

        // Base: 50.0 * 1000 = 50000ms = 50s
        // CodeGen multiplier: 1.0
        // Result: 50s
        assert_eq!(duration, 50000);
    }
}
