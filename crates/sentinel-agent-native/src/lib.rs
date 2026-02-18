//! Sentinel Native Agent - Revolutionary Coding Agent with Integrated Sentinel OS
//!
//! This is NOT a bridge or wrapper around existing agents (Cline/Kilocode).
//! This is a ground-up, native implementation of a coding agent with
//! Sentinel OS integrated at every level of the reasoning process.
//!
//! # Revolutionary Architecture
//!
//! Unlike traditional agents that:
//! - Use LLMs as black boxes
//! - Apply alignment checks AFTER generation
//! - Operate in isolation
//!
//! Sentinel Native Agent:
//! - Reasoning is SENTINEL-AWARE from the start
//! - Alignment guides EVERY decision, not just validates
//! - Consults P2P network for collective intelligence
//! - Uses hierarchical goal manifolds for planning
//! - Operates with deterministic, structured reasoning
//!
//! # Key Innovations
//!
//! 1. **Native Goal Alignment**: Every thought is aligned with Goal Manifold
//! 2. **P2P Consensus Reasoning**: Global intelligence guides local decisions
//! 3. **Deterministic Code Generation**: Tree-sitter based, not LLM hallucinations
//! 4. **Hierarchical Planning**: Goal DAG drives every action
//! 5. **Zero-Latency Validation**: Alignment is built-in, not an after-thought
//! 6. **Multi-Agent Orchestration**: Orchestrates sub-agents for parallel work

#![allow(
    dead_code,
    unused_imports,
    unused_mut,
    unused_variables,
    private_interfaces
)]

pub mod agent_communication_llm;
pub mod codegen;
pub mod consensus;
pub mod context;
pub mod context_provider;
pub mod end_to_end_agent;
pub mod gateway;
pub mod llm_integration;
pub mod openrouter;
pub mod orchestrator;
pub mod planning;
pub mod providers;
pub mod reasoning;
pub mod swarm;

pub use end_to_end_agent::{E2EConfig, E2EReport, EndToEndAgent, ModuleDetail};

use anyhow::Result;
use sentinel_core::{
    alignment::{AlignmentField, ProjectState},
    cognitive_state::{Action, ActionType, CognitiveState},
    execution::{ExecutionNorthStar, ReliabilitySnapshot},
    goal_manifold::{predicate::Predicate, Goal, GoalManifold, Intent},
    learning::{DeviationPattern, KnowledgeBase, LearningEngine},
    memory::MemoryManifold,
    Uuid,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Sentinel Native Agent - The Revolutionary Coding Agent
///
/// This agent represents a fundamental shift in how AI coding agents work.
/// Instead of being a tool that happens to have alignment checks,
/// alignment is built into the very fabric of its reasoning.
#[derive(Debug, Clone)]
pub struct SentinelAgent {
    /// Unique agent identity (Ed25519)
    pub agent_id: Uuid,

    /// Agent authority level (determines voting weight in swarm)
    pub authority: AgentAuthority,

    /// The Goal Manifold - source of all truth
    pub goal_manifold: GoalManifold,

    /// Cognitive state - working memory and beliefs
    pub cognitive_state: CognitiveState,

    /// Alignment field - continuous validation engine
    pub alignment_field: AlignmentField,

    /// P2P Consensus - Distributed truth
    pub consensus: std::sync::Arc<tokio::sync::Mutex<consensus::P2PConsensus>>,

    /// Hierarchical Planner - Goal-driven planning
    pub planner: std::sync::Arc<tokio::sync::Mutex<planning::HierarchicalPlanner>>,

    /// Structured Reasoner - Deterministic reasoning
    pub reasoner: std::sync::Arc<tokio::sync::Mutex<reasoning::StructuredReasoner>>,

    /// Code Generator - Tree-sitter based
    pub codegen: std::sync::Arc<tokio::sync::Mutex<codegen::TreeSitterGenerator>>,

    /// Context manager - hierarchical memory access
    pub context_manager: std::sync::Arc<tokio::sync::Mutex<context::ContextManager>>,

    /// Orchestrator - multi-agent coordination
    pub orchestrator: std::sync::Arc<tokio::sync::Mutex<orchestrator::AgentOrchestrator>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GovernanceObservation {
    pub dependencies: Vec<String>,
    pub frameworks: Vec<String>,
    pub endpoints: Vec<String>,
    pub ports: Vec<u16>,
}

/// Agent authority level determines voting weight in swarm consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AgentAuthority {
    /// Human user - ultimate authority (1.0 weight)
    Human,

    /// Senior AI node with proven alignment history (0.8 weight)
    SeniorAI,

    /// Junior AI node still learning (0.3 weight)
    JuniorAI,
}

impl AgentAuthority {
    pub fn weight(&self) -> f64 {
        match self {
            AgentAuthority::Human => 1.0,
            AgentAuthority::SeniorAI => 0.8,
            AgentAuthority::JuniorAI => 0.3,
        }
    }
}

impl SentinelAgent {
    /// Create a new Sentinel Native Agent
    ///
    /// This initializes the complete agent with all components
    /// integrated and ready for operation.
    pub async fn new(intent: Intent, authority: AgentAuthority) -> Result<Self> {
        tracing::info!("Initializing Sentinel Native Agent");

        // Create Goal Manifold (Layer 1) - The immutable truth
        let mut goal_manifold = GoalManifold::new(intent.clone());
        bootstrap_governance_contract_from_workspace(&mut goal_manifold)?;

        // Create Knowledge Base (Layer 5)
        let knowledge_base = std::sync::Arc::new(KnowledgeBase::new());

        // Create Learning Engine (Layer 5)
        let learning_engine = LearningEngine::new(knowledge_base.clone());

        // Create Cognitive State (Layer 3) - Working memory and meta-cognition
        let cognitive_state = CognitiveState::new(goal_manifold.clone(), learning_engine);

        // Create Alignment Field (Layer 2) - Continuous validation
        let alignment_field = AlignmentField::new(goal_manifold.clone());

        // Create P2P Consensus (Layer 10) - Distributed truth
        let consensus = consensus::P2PConsensus::new(authority).await?;

        // Create Hierarchical Planner (Layer 3) - Goal-driven planning
        let planner = planning::HierarchicalPlanner::new(goal_manifold.clone());

        // Create Structured Reasoner - Deterministic reasoning
        let reasoner = reasoning::StructuredReasoner::new();

        // Create Tree-Sitter Code Generator - Non-LLM code generation
        let codegen = codegen::TreeSitterGenerator::new()?;

        // Create Memory Manifold (Layer 4)
        let memory_manifold = std::sync::Arc::new(tokio::sync::Mutex::new(MemoryManifold::new()));

        // Create Context Manager (Layer 4) - Hierarchical memory access
        let context_manager = context::ContextManager::new(memory_manifold.clone());

        // Create Agent Orchestrator - Multi-agent coordination
        let mut orchestrator = orchestrator::AgentOrchestrator::new();
        orchestrator.set_reliability_thresholds(goal_manifold.reliability.thresholds.clone());
        orchestrator.set_governance_policy(goal_manifold.governance.clone());

        let agent_id = Uuid::new_v4();

        tracing::info!(
            "Sentinel Native Agent initialized: {} with authority {:?}",
            agent_id,
            authority
        );

        Ok(Self {
            agent_id,
            authority,
            goal_manifold,
            cognitive_state,
            alignment_field,
            consensus: std::sync::Arc::new(tokio::sync::Mutex::new(consensus)),
            planner: std::sync::Arc::new(tokio::sync::Mutex::new(planner)),
            reasoner: std::sync::Arc::new(tokio::sync::Mutex::new(reasoner)),
            codegen: std::sync::Arc::new(tokio::sync::Mutex::new(codegen)),
            context_manager: std::sync::Arc::new(tokio::sync::Mutex::new(context_manager)),
            orchestrator: std::sync::Arc::new(tokio::sync::Mutex::new(orchestrator)),
        })
    }

    /// Execute a task end-to-end with full Sentinel OS integration
    ///
    /// This is the main entry point. Unlike traditional agents that:
    /// 1. Generate code
    /// 2. Check alignment
    /// 3. Apply if aligned
    ///
    /// Sentinel Native Agent:
    /// 1. Parse task and extract goals
    /// 2. Query P2P network for collective intelligence
    /// 3. Create hierarchical plan aligned with Goal Manifold
    /// 4. Execute with continuous alignment validation
    /// 5. Learn from outcomes
    pub async fn execute_task(&mut self, task: &str) -> Result<ExecutionReport> {
        tracing::info!("Executing task: {}", task);
        self.enforce_dependency_governance()?;

        let start_time = chrono::Utc::now();

        // Phase 1: Task Analysis and Goal Extraction
        let task_analysis = self.analyze_task(task).await?;

        // Phase 2: P2P Consensus Query - Get global intelligence
        let consensus_result = self.query_consensus(&task_analysis).await?;

        // Phase 3: Hierarchical Planning - Goal-driven
        let plan = self
            .create_hierarchical_plan(&task_analysis, &consensus_result)
            .await?;
        let north_star = plan.north_star.clone();

        // Phase 4: Execution with Continuous Alignment
        let execution_result = self.execute_plan(plan).await?;
        self.enforce_dependency_governance()?;

        let reliability = self
            .build_reliability_snapshot(&execution_result, &consensus_result)
            .await;
        self.enforce_runtime_reliability(&reliability)?;

        // Phase 5: Learning and Pattern Extraction
        self.learn_from_execution(&task_analysis, &execution_result)
            .await?;

        // Phase 6: Share Learnings with P2P Network
        self.share_learnings(&task_analysis, &execution_result)
            .await?;

        let end_time = chrono::Utc::now();
        let duration = end_time
            .signed_duration_since(start_time)
            .num_milliseconds();

        Ok(ExecutionReport {
            task: task.to_string(),
            agent_id: self.agent_id,
            duration_ms: duration,
            alignment_score: execution_result.alignment_score,
            actions_taken: execution_result.actions.len(),
            deviations_detected: execution_result.deviations.len(),
            success: execution_result.success,
            north_star,
            reliability,
        })
    }

    /// Analyze task and extract structured goals
    async fn analyze_task(&self, task: &str) -> Result<TaskAnalysis> {
        tracing::debug!("Analyzing task: {}", task);

        // Use NLP to extract goals from natural language
        let goals = self.extract_goals_from_task(task)?;

        // Extract constraints and invariants
        let constraints = self.extract_constraints(task)?;

        Ok(TaskAnalysis {
            task: task.to_string(),
            goals,
            constraints,
            complexity: self.estimate_complexity(task),
        })
    }

    /// Query P2P consensus network for global intelligence
    ///
    /// This is REVOLUTIONARY - instead of reasoning in isolation,
    /// the agent queries the global network of Sentinel nodes
    /// for similar tasks, successful patterns, and collective wisdom.
    async fn query_consensus(&self, task_analysis: &TaskAnalysis) -> Result<ConsensusQueryResult> {
        tracing::debug!("Querying P2P consensus for task analysis");

        let mut consensus = self.consensus.lock().await;

        let similar_tasks = consensus.query_similar_tasks(&task_analysis.goals).await?;

        let patterns = consensus
            .query_successful_patterns(&task_analysis.goals)
            .await?;

        let threats = consensus.query_threats(&task_analysis.task).await?;

        Ok(ConsensusQueryResult {
            similar_tasks,
            patterns,
            threats,
            network_participants: consensus.active_peers_count(),
        })
    }

    /// Create hierarchical plan aligned with Goal Manifold
    ///
    /// The plan is NOT just a sequence of actions.
    /// It is a GOAL-DRIVEN hierarchy where each action
    /// is justified by its contribution to the Goal Manifold.
    async fn create_hierarchical_plan(
        &mut self,
        task_analysis: &TaskAnalysis,
        consensus: &ConsensusQueryResult,
    ) -> Result<planning::ExecutionPlan> {
        tracing::debug!("Creating hierarchical plan");

        // Ensure analyzed goals are registered in the manifold, so planning is grounded
        // in the same source of truth used by alignment and execution.
        for goal in &task_analysis.goals {
            if self.goal_manifold.get_goal(&goal.id).is_none() {
                let _ = self.goal_manifold.add_goal(goal.clone());
            }
        }

        // Decompose task into sub-goals using Goal DAG
        let planner = self.planner.lock().await;
        let sub_goals = planner.decompose_goals(&task_analysis.goals)?;
        drop(planner); // Release lock

        // Order goals topologically (respect dependencies)
        let ordered_goal_ids: Vec<Uuid> = self
            .goal_manifold
            .goal_dag
            .topological_sort()?
            .iter()
            .map(|g| g.id)
            .collect();

        // For each goal, create aligned actions
        let mut actions = Vec::new();
        for goal_id in ordered_goal_ids {
            let goal = self
                .goal_manifold
                .get_goal(&goal_id)
                .ok_or_else(|| anyhow::anyhow!("Goal not found"))?
                .clone();
            let goal_actions = self.plan_goal_actions(&goal, consensus).await?;
            actions.extend(goal_actions);
        }

        // Validate entire plan with Alignment Field
        let plan_sub_goals: Vec<Uuid> = sub_goals.iter().map(|g| g.id).collect();
        let estimated_duration_minutes = (task_analysis.complexity * 2.0) as u32;
        let action_count = actions.len();
        let plan = planning::ExecutionPlan {
            root_task: task_analysis.task.clone(),
            sub_goals: plan_sub_goals,
            actions,
            complexity: task_analysis.complexity,
            estimated_duration_minutes,
            north_star: self.build_execution_north_star(
                task_analysis,
                task_analysis.task.clone(),
                action_count,
                estimated_duration_minutes,
            ),
        };

        let planner = self.planner.lock().await;
        match planner.validate_plan(&plan) {
            planning::PlanValidation::Valid => {}
            planning::PlanValidation::LowAlignment { score } => {
                return Err(anyhow::anyhow!(
                    "Hierarchical plan rejected: low alignment {:.2}",
                    score
                ));
            }
            planning::PlanValidation::ViolatesInvariants { invariants } => {
                return Err(anyhow::anyhow!(
                    "Hierarchical plan rejected: invariant violations {:?}",
                    invariants
                ));
            }
            planning::PlanValidation::CircularDependencies { cycle } => {
                return Err(anyhow::anyhow!(
                    "Hierarchical plan rejected: circular dependencies {:?}",
                    cycle
                ));
            }
            planning::PlanValidation::MissingNorthStar { reason } => {
                return Err(anyhow::anyhow!(
                    "Hierarchical plan rejected: invalid north star ({})",
                    reason
                ));
            }
        }
        drop(planner);

        // Predictive Alignment for the plan
        let state = ProjectState::new(std::path::PathBuf::from("."));
        let prediction = self.alignment_field.predict_alignment(&state).await?;

        if prediction.expected_alignment < 80.0 {
            tracing::warn!(
                "Plan has low expected alignment score: {}",
                prediction.expected_alignment
            );
            // Try alternative approaches
            let alignment_vector = sentinel_core::alignment::AlignmentVector {
                score: prediction.expected_alignment,
                confidence: prediction.confidence,
                goal_contribution: sentinel_core::alignment::Vector::new(vec![]),
                deviation_magnitude: 100.0 - prediction.expected_alignment,
                entropy_gradient: 0.0,
            };
            return self
                .create_alternative_plan(task_analysis, consensus, &alignment_vector)
                .await;
        }

        Ok(plan)
    }

    /// Plan specific actions for a goal
    ///
    /// Each action is:
    /// 1. Justified by its contribution to the goal
    /// 2. Validated against invariants
    /// 3. Checked for deviation probability
    async fn plan_goal_actions(
        &mut self,
        goal: &Goal,
        consensus: &ConsensusQueryResult,
    ) -> Result<Vec<Action>> {
        let mut actions = Vec::new();

        // Apply successful patterns from P2P network
        for pattern in &consensus.patterns {
            if pattern.applicable_to_goal(goal) {
                let action = Action {
                    id: Uuid::new_v4(),
                    action_type: ActionType::ApplyPattern {
                        pattern_id: pattern.id,
                    },
                    description: format!("Applying proven pattern from network: {}", pattern.name),
                    goal_id: Some(goal.id),
                    expected_value: pattern.alignment_impact,
                    created_at: chrono::Utc::now(),
                    dependencies: Vec::new(),
                    metadata: sentinel_core::cognitive_state::action::ActionMetadata::default(),
                };
                actions.push(action);
            }
        }

        // Generate new actions using structured reasoning
        let mut reasoner = self.reasoner.lock().await;
        let generated_actions = reasoner.plan_actions_for_goal(goal, consensus)?;
        drop(reasoner); // Release lock

        // Validate each action with Cognitive State
        for action in generated_actions {
            let decision = self.cognitive_state.before_action(action.clone()).await?;

            match decision.decision_type {
                sentinel_core::cognitive_state::action::DecisionType::Approve => {
                    actions.push(action);
                }
                sentinel_core::cognitive_state::action::DecisionType::Reject => {
                    tracing::warn!("Action rejected: {}", decision.reason);
                }
                sentinel_core::cognitive_state::action::DecisionType::ProposeAlternative => {
                    actions.extend(decision.alternatives);
                }
                _ => {}
            }
        }

        Ok(actions)
    }

    /// Execute plan with continuous alignment validation
    async fn execute_plan(&mut self, plan: planning::ExecutionPlan) -> Result<ExecutionResult> {
        tracing::info!("Executing plan with {} actions", plan.actions.len());

        let mut executed_actions = Vec::new();
        let mut deviations = Vec::new();
        let mut total_alignment = 0.0;
        let mut success = true;

        for action in plan.actions {
            let state = ProjectState::new(std::path::PathBuf::from("."));
            let prediction = self.alignment_field.predict_alignment(&state).await?;

            if prediction.deviation_probability > 0.3 {
                tracing::warn!(
                    "Predicted high deviation probability ({}%), skipping action",
                    prediction.deviation_probability * 100.0
                );
                deviations.push(DeviationDetails {
                    action_id: action.id,
                    reason: format!(
                        "Predicted high deviation: {:.1}%",
                        prediction.deviation_probability * 100.0
                    ),
                    probability: prediction.deviation_probability,
                });
                continue;
            }

            // Execute action
            let result = self.execute_action(action.clone()).await?;

            // Check alignment AFTER execution (reactive)
            let alignment = self
                .alignment_field
                .compute_alignment(&result.state)
                .await?;

            total_alignment += alignment.score;

            if alignment.score < 70.0 {
                tracing::warn!("Low alignment detected: {}", alignment.score);
                deviations.push(DeviationDetails {
                    action_id: action.id,
                    reason: format!("Low alignment: {}", alignment.score),
                    probability: 1.0 - (alignment.score / 100.0),
                });
                success = false;
            }

            executed_actions.push(result);
        }

        let avg_alignment = if !executed_actions.is_empty() {
            total_alignment / executed_actions.len() as f64
        } else {
            0.0
        };

        Ok(ExecutionResult {
            actions: executed_actions,
            alignment_score: avg_alignment,
            deviations,
            success,
        })
    }

    /// Execute a single action
    async fn execute_action(&mut self, action: Action) -> Result<NativeActionResult> {
        let action_type = action.action_type.clone();
        tracing::debug!("Executing action: {:?}", action_type);

        match action_type {
            ActionType::CreateFile { path, content } => {
                let mut codegen = self.codegen.lock().await;
                codegen
                    .create_file(&path.to_string_lossy(), &content)
                    .await?;
            }
            ActionType::EditFile { path, .. } => {
                // Simplified for now
                tracing::info!("Editing file: {:?}", path);
            }
            ActionType::RunCommand { command, .. } => {
                let output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&command)
                    .output()
                    .await?;

                tracing::debug!("Command output: {:?}", output);
            }
            ActionType::ApplyPattern { pattern_id } => {
                let mut consensus = self.consensus.lock().await;
                let pattern = consensus.get_pattern(&pattern_id).await?;
                pattern.apply().await?;
            }
            _ => {}
        }

        // Get current project state
        let state = self.get_project_state().await?;

        Ok(NativeActionResult {
            action_id: action.id,
            action,
            state,
            duration_ms: 1000, // Placeholder
        })
    }

    /// Learn from execution and update internal models
    async fn learn_from_execution(
        &mut self,
        task_analysis: &TaskAnalysis,
        execution: &ExecutionResult,
    ) -> Result<()> {
        tracing::debug!("Learning from execution");

        // Update Cognitive State with outcomes
        for action_result in &execution.actions {
            // 6. Record Outcome (Layer 3)
            let core_result = sentinel_core::cognitive_state::ActionResult::success(
                action_result.action_id,
                "Action executed successfully".to_string(),
                action_result.duration_ms as f64 / 1000.0,
            );

            self.cognitive_state
                .after_action(action_result.action.clone(), core_result)
                .await?;
        }

        // Extract patterns if execution was successful
        if execution.success && execution.alignment_score > 80.0 {
            let patterns = self.extract_successful_patterns(task_analysis, execution)?;
            let mut consensus = self.consensus.lock().await;
            for pattern in patterns {
                consensus.learn_pattern(pattern).await?;
            }
        }

        // Identify deviation patterns if execution had low alignment
        if !execution.deviations.is_empty() {
            let deviation_patterns = self.identify_deviation_patterns(&execution.deviations)?;
            let mut consensus = self.consensus.lock().await;
            for pattern in deviation_patterns {
                consensus.learn_deviation_pattern(pattern).await?;
            }
        }

        Ok(())
    }

    /// Share learnings with P2P network
    async fn share_learnings(
        &mut self,
        task_analysis: &TaskAnalysis,
        execution: &ExecutionResult,
    ) -> Result<()> {
        tracing::debug!("Sharing learnings with P2P network");

        if execution.success && execution.alignment_score > 85.0 {
            // Share successful patterns
            let patterns = self.extract_successful_patterns(task_analysis, execution)?;
            let mut consensus = self.consensus.lock().await;
            for pattern in &patterns {
                consensus.share_pattern(pattern.clone()).await?;
            }

            tracing::info!("Shared {} successful patterns with network", patterns.len());
        }

        if !execution.deviations.is_empty() {
            // Share threat alerts about deviations
            for deviation in &execution.deviations {
                if deviation.probability > 0.8 {
                    let threat = self.create_threat_alert(deviation)?;
                    let mut consensus = self.consensus.lock().await;
                    consensus.broadcast_threat(threat).await?;
                }
            }
        }

        Ok(())
    }

    // Helper methods

    fn extract_goals_from_task(&self, task: &str) -> Result<Vec<Goal>> {
        let sanitized = task.trim();
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }

        let clauses: Vec<String> = sanitized
            .split(&['.', ';'][..])
            .flat_map(|segment| segment.split(" and "))
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToString::to_string)
            .collect();

        let mut goals = Vec::new();
        for clause in clauses.iter().take(6) {
            if let Ok(goal) = Goal::builder()
                .description(clause.clone())
                .add_success_criterion(Predicate::DirectoryExists(std::path::PathBuf::from(".")))
                .value_to_root((1.0 / clauses.len().max(1) as f64).clamp(0.05, 0.5))
                .build()
            {
                goals.push(goal);
            }
        }

        if goals.is_empty() {
            if let Ok(goal) = Goal::builder()
                .description(sanitized.to_string())
                .add_success_criterion(Predicate::DirectoryExists(std::path::PathBuf::from(".")))
                .value_to_root(1.0)
                .build()
            {
                goals.push(goal);
            }
        }

        Ok(goals)
    }

    fn extract_constraints(&self, task: &str) -> Result<Vec<String>> {
        let lower = task.to_lowercase();
        let mut constraints = Vec::new();

        for token in [
            "must", "without", "do not", "don't", "never", "only", "strictly",
        ] {
            if lower.contains(token) {
                constraints.push(format!("Constraint signaled by '{}'", token));
            }
        }

        Ok(constraints)
    }

    fn estimate_complexity(&self, task: &str) -> f64 {
        let words = task.split_whitespace().count() as f64;
        let mut score = 2.0 + (words / 8.0);
        let lower = task.to_lowercase();

        for hard_signal in [
            "refactor",
            "migrate",
            "distributed",
            "security",
            "consensus",
            "orchestrator",
            "production",
        ] {
            if lower.contains(hard_signal) {
                score += 0.8;
            }
        }

        score.clamp(1.0, 10.0)
    }

    async fn get_project_state(&self) -> Result<ProjectState> {
        let current_dir = std::env::current_dir()?;
        Ok(ProjectState::new(current_dir))
    }

    fn extract_successful_patterns(
        &self,
        task_analysis: &TaskAnalysis,
        execution: &ExecutionResult,
    ) -> Result<Vec<consensus::Pattern>> {
        // Extract patterns from successful execution
        Ok(vec![])
    }

    fn identify_deviation_patterns(
        &self,
        deviations: &[DeviationDetails],
    ) -> Result<Vec<DeviationPattern>> {
        // Identify patterns in deviations
        Ok(vec![])
    }

    fn create_threat_alert(
        &self,
        deviation: &DeviationDetails,
    ) -> Result<sentinel_core::federation::ThreatAlert> {
        Ok(sentinel_core::federation::ThreatAlert {
            threat_id: Uuid::new_v4(),
            threat_type: sentinel_core::federation::ThreatType::AlignmentDeviation,
            severity: sentinel_core::federation::Severity::High,
            description: deviation.reason.clone(),
            source_agent_id: self.agent_id,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn create_alternative_plan(
        &mut self,
        task_analysis: &TaskAnalysis,
        consensus: &ConsensusQueryResult,
        current_alignment: &sentinel_core::alignment::AlignmentVector,
    ) -> Result<planning::ExecutionPlan> {
        // Conservative fallback: prefer proven network patterns and lightweight
        // validation actions to recover alignment before high-risk operations.
        let mut actions = Vec::new();

        for goal in &task_analysis.goals {
            if let Some(pattern) = consensus
                .patterns
                .iter()
                .filter(|pattern| pattern.applicable_to_goal(goal))
                .max_by(|a, b| a.success_rate.total_cmp(&b.success_rate))
            {
                actions.push(Action {
                    id: Uuid::new_v4(),
                    action_type: ActionType::ApplyPattern {
                        pattern_id: pattern.id,
                    },
                    description: format!(
                        "Fallback alignment recovery using proven pattern '{}'",
                        pattern.name
                    ),
                    goal_id: Some(goal.id),
                    expected_value: (pattern.alignment_impact + pattern.success_rate)
                        .clamp(0.0, 1.0)
                        / 2.0,
                    created_at: chrono::Utc::now(),
                    dependencies: Vec::new(),
                    metadata: sentinel_core::cognitive_state::action::ActionMetadata::default(),
                });
            } else {
                actions.push(Action::new(
                    ActionType::RunTests {
                        suite: "smoke".to_string(),
                    },
                    format!("Fallback verification for goal '{}'", goal.description),
                ));
            }
        }

        if actions.is_empty() {
            actions.push(Action::new(
                ActionType::RunTests {
                    suite: "smoke".to_string(),
                },
                "Fallback verification without explicit goals".to_string(),
            ));
        }

        let estimated_duration_minutes =
            ((task_analysis.complexity * 1.5) + (100.0 - current_alignment.score) / 10.0) as u32;
        let action_count = actions.len();
        Ok(planning::ExecutionPlan {
            root_task: format!("{} [alternative-plan]", task_analysis.task),
            sub_goals: task_analysis.goals.iter().map(|goal| goal.id).collect(),
            actions,
            complexity: (task_analysis.complexity * 0.8).max(1.0),
            estimated_duration_minutes,
            north_star: self.build_execution_north_star(
                task_analysis,
                format!("{} [alternative-plan]", task_analysis.task),
                action_count,
                estimated_duration_minutes,
            ),
        })
    }

    fn build_execution_north_star(
        &self,
        task_analysis: &TaskAnalysis,
        target_task: String,
        action_count: usize,
        estimated_duration_minutes: u32,
    ) -> ExecutionNorthStar {
        ExecutionNorthStar {
            where_we_are: format!(
                "Task analyzed with {} goals and {} constraints",
                task_analysis.goals.len(),
                task_analysis.constraints.len()
            ),
            where_we_must_go: target_task,
            how: format!(
                "Plan with {} actions and estimated {} minutes",
                action_count, estimated_duration_minutes
            ),
            why: "Every action must increase goal alignment while respecting invariants"
                .to_string(),
            constraints: task_analysis.constraints.clone(),
        }
    }

    async fn build_reliability_snapshot(
        &self,
        execution: &ExecutionResult,
        consensus: &ConsensusQueryResult,
    ) -> ReliabilitySnapshot {
        let total_tasks = execution.actions.len() as u64 + execution.deviations.len() as u64;
        let successful_tasks = execution.actions.len() as u64;
        let invariant_violations = execution
            .deviations
            .iter()
            .filter(|deviation| deviation.reason.to_lowercase().contains("invariant"))
            .count() as u64;
        let rollbacks = execution.deviations.len() as u64;
        let recovery_events = execution.deviations.len() as u64;
        let total_recovery_ms = execution
            .deviations
            .iter()
            .map(|deviation| ((deviation.probability.clamp(0.0, 1.0) * 1000.0) as u64).max(1))
            .sum::<u64>()
            + (consensus.threats.len() as u64 * 5);

        ReliabilitySnapshot::from_counts(
            total_tasks,
            successful_tasks,
            execution.deviations.len() as u64,
            rollbacks,
            recovery_events,
            total_recovery_ms,
            invariant_violations,
        )
    }

    fn enforce_runtime_reliability(&self, reliability: &ReliabilitySnapshot) -> Result<()> {
        let thresholds = &self.goal_manifold.reliability.thresholds;
        let evaluation = reliability.evaluate(thresholds);
        if evaluation.healthy {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Runtime reliability SLO violated: {}",
            evaluation.violations.join(" | ")
        ))
    }

    fn enforce_dependency_governance(&mut self) -> Result<()> {
        if let Some(pending) = &self.goal_manifold.governance.pending_proposal {
            return Err(anyhow::anyhow!(
                "Governance change pending user approval (proposal {}): {}",
                pending.id,
                pending.rationale
            ));
        }

        let observed = observe_workspace_contract(
            &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        )?;
        let drift = compute_contract_drift(&self.goal_manifold.governance, &observed);
        if !drift.has_blocking_violation() {
            return Ok(());
        }

        let proposal = build_governance_proposal(&drift);
        let proposal_id = proposal.id;
        self.goal_manifold.record_governance_proposal(proposal);
        Err(anyhow::anyhow!(
            "Dependency/framework/endpoint governance violation detected. Proposal {} created and waiting for user approval.",
            proposal_id
        ))
    }
}

#[derive(Debug, Default)]
struct ObservedWorkspaceContract {
    dependencies: BTreeSet<String>,
    frameworks: BTreeSet<String>,
    endpoints: BTreeSet<String>,
    ports: BTreeSet<u16>,
}

#[derive(Debug, Default)]
struct GovernanceDrift {
    unexpected_dependencies: Vec<String>,
    unexpected_frameworks: Vec<String>,
    unexpected_endpoints: Vec<String>,
    unexpected_ports: Vec<u16>,
    missing_required_dependencies: Vec<String>,
    missing_required_frameworks: Vec<String>,
}

impl GovernanceDrift {
    fn has_blocking_violation(&self) -> bool {
        !self.unexpected_dependencies.is_empty()
            || !self.unexpected_frameworks.is_empty()
            || !self.unexpected_endpoints.is_empty()
            || !self.unexpected_ports.is_empty()
            || !self.missing_required_dependencies.is_empty()
            || !self.missing_required_frameworks.is_empty()
    }
}

fn bootstrap_governance_contract_from_workspace(manifold: &mut GoalManifold) -> Result<()> {
    let observed = observe_workspace_contract(
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    )?;

    if manifold.governance.allowed_dependencies.is_empty() {
        manifold.governance.allowed_dependencies = observed.dependencies.iter().cloned().collect();
    }
    if manifold.governance.required_dependencies.is_empty() {
        manifold.governance.required_dependencies =
            manifold.governance.allowed_dependencies.clone();
    }
    if manifold.governance.allowed_frameworks.is_empty() {
        manifold.governance.allowed_frameworks = observed.frameworks.iter().cloned().collect();
    }
    if manifold.governance.required_frameworks.is_empty() {
        manifold.governance.required_frameworks = manifold.governance.allowed_frameworks.clone();
    }
    if manifold.governance.allowed_endpoints.is_empty() {
        let mut endpoints = BTreeMap::new();
        for (idx, endpoint) in observed.endpoints.iter().enumerate() {
            endpoints.insert(format!("observed_{}", idx + 1), endpoint.clone());
        }
        manifold.governance.allowed_endpoints = endpoints.into_iter().collect();
    }
    if manifold.governance.allowed_ports.is_empty() {
        manifold.governance.allowed_ports = observed.ports.iter().copied().collect();
    }
    Ok(())
}

fn compute_contract_drift(
    policy: &sentinel_core::goal_manifold::GovernancePolicy,
    observed: &ObservedWorkspaceContract,
) -> GovernanceDrift {
    let allowed_dependencies: BTreeSet<_> = policy.allowed_dependencies.iter().cloned().collect();
    let required_dependencies: BTreeSet<_> = policy.required_dependencies.iter().cloned().collect();
    let allowed_frameworks: BTreeSet<_> = policy.allowed_frameworks.iter().cloned().collect();
    let required_frameworks: BTreeSet<_> = policy.required_frameworks.iter().cloned().collect();
    let allowed_endpoints: BTreeSet<_> = policy.allowed_endpoints.values().cloned().collect();
    let allowed_ports: BTreeSet<_> = policy.allowed_ports.iter().copied().collect();

    GovernanceDrift {
        unexpected_dependencies: observed
            .dependencies
            .difference(&allowed_dependencies)
            .cloned()
            .collect(),
        unexpected_frameworks: observed
            .frameworks
            .difference(&allowed_frameworks)
            .cloned()
            .collect(),
        unexpected_endpoints: observed
            .endpoints
            .difference(&allowed_endpoints)
            .cloned()
            .collect(),
        unexpected_ports: observed.ports.difference(&allowed_ports).copied().collect(),
        missing_required_dependencies: required_dependencies
            .difference(&observed.dependencies)
            .cloned()
            .collect(),
        missing_required_frameworks: required_frameworks
            .difference(&observed.frameworks)
            .cloned()
            .collect(),
    }
}

fn build_governance_proposal(
    drift: &GovernanceDrift,
) -> sentinel_core::goal_manifold::GovernanceChangeProposal {
    let mut proposed_endpoints = std::collections::HashMap::new();
    for (index, endpoint) in drift.unexpected_endpoints.iter().enumerate() {
        proposed_endpoints.insert(format!("proposed_{}", index + 1), endpoint.clone());
    }

    let rationale = format!(
        "Observed deterministic drift: deps={} frameworks={} endpoints={} ports={}. Missing required deps={} frameworks={}.",
        drift.unexpected_dependencies.len(),
        drift.unexpected_frameworks.len(),
        drift.unexpected_endpoints.len(),
        drift.unexpected_ports.len(),
        drift.missing_required_dependencies.len(),
        drift.missing_required_frameworks.len()
    );
    let mut evidence = Vec::new();
    if !drift.unexpected_dependencies.is_empty() {
        evidence.push(format!(
            "Observed new dependencies not present in contract: {}",
            drift.unexpected_dependencies.join(", ")
        ));
    }
    if !drift.unexpected_frameworks.is_empty() {
        evidence.push(format!(
            "Observed new frameworks not present in contract: {}",
            drift.unexpected_frameworks.join(", ")
        ));
    }
    if !drift.unexpected_endpoints.is_empty() {
        evidence.push(format!(
            "Observed new endpoints not present in contract: {}",
            drift.unexpected_endpoints.join(", ")
        ));
    }
    if !drift.unexpected_ports.is_empty() {
        evidence.push(format!(
            "Observed new ports not present in contract: {}",
            drift
                .unexpected_ports
                .iter()
                .map(|port| port.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !drift.missing_required_dependencies.is_empty() {
        evidence.push(format!(
            "Required dependencies missing in workspace scan: {}",
            drift.missing_required_dependencies.join(", ")
        ));
    }
    if !drift.missing_required_frameworks.is_empty() {
        evidence.push(format!(
            "Required frameworks missing in workspace scan: {}",
            drift.missing_required_frameworks.join(", ")
        ));
    }

    sentinel_core::goal_manifold::GovernanceChangeProposal {
        id: Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        rationale,
        proposed_dependencies: drift.unexpected_dependencies.clone(),
        proposed_dependency_removals: drift.missing_required_dependencies.clone(),
        proposed_frameworks: drift.unexpected_frameworks.clone(),
        proposed_framework_removals: drift.missing_required_frameworks.clone(),
        proposed_endpoints,
        proposed_endpoint_removals: Vec::new(),
        proposed_ports: drift.unexpected_ports.clone(),
        proposed_port_removals: Vec::new(),
        deterministic_confidence: 1.0,
        evidence,
        status: sentinel_core::goal_manifold::GovernanceProposalStatus::PendingUserApproval,
        user_note: None,
    }
}

pub fn observe_workspace_governance(root: &Path) -> Result<GovernanceObservation> {
    let observed = observe_workspace_contract(root)?;
    Ok(GovernanceObservation {
        dependencies: observed.dependencies.into_iter().collect(),
        frameworks: observed.frameworks.into_iter().collect(),
        endpoints: observed.endpoints.into_iter().collect(),
        ports: observed.ports.into_iter().collect(),
    })
}

fn observe_workspace_contract(root: &Path) -> Result<ObservedWorkspaceContract> {
    let mut observed = ObservedWorkspaceContract::default();
    collect_from_cargo_toml(root.join("Cargo.toml"), &mut observed)?;
    collect_from_package_json(root.join("package.json"), &mut observed)?;
    collect_from_pyproject(root.join("pyproject.toml"), &mut observed)?;
    collect_from_requirements(root.join("requirements.txt"), &mut observed)?;
    collect_from_composer(root.join("composer.json"), &mut observed)?;
    collect_endpoints_and_ports(root, &mut observed)?;
    Ok(observed)
}

fn collect_from_cargo_toml(path: PathBuf, observed: &mut ObservedWorkspaceContract) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    for section in ["dependencies", "dev-dependencies", "workspace.dependencies"] {
        if let Some(table) = get_toml_table(&value, section) {
            for key in table.keys() {
                register_dependency(observed, &format!("cargo:{}", key.to_lowercase()));
            }
        }
    }
    Ok(())
}

fn get_toml_table<'a>(
    value: &'a toml::Value,
    dotted_path: &str,
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    let mut cursor = value;
    for part in dotted_path.split('.') {
        cursor = cursor.get(part)?;
    }
    cursor.as_table()
}

fn collect_from_package_json(
    path: PathBuf,
    observed: &mut ObservedWorkspaceContract,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(map) = value.get(section).and_then(|entry| entry.as_object()) {
            for key in map.keys() {
                register_dependency(observed, &format!("npm:{}", key.to_lowercase()));
            }
        }
    }
    Ok(())
}

fn collect_from_pyproject(path: PathBuf, observed: &mut ObservedWorkspaceContract) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&content)?;
    if let Some(project_deps) = value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|deps| deps.as_array())
    {
        for dep in project_deps {
            if let Some(raw) = dep.as_str() {
                let name = extract_python_dep_name(raw);
                register_dependency(observed, &format!("pip:{}", name));
            }
        }
    }
    if let Some(poetry_deps) = value
        .get("tool")
        .and_then(|v| v.get("poetry"))
        .and_then(|v| v.get("dependencies"))
        .and_then(|deps| deps.as_table())
    {
        for key in poetry_deps.keys() {
            if key != "python" {
                register_dependency(observed, &format!("pip:{}", key.to_lowercase()));
            }
        }
    }
    Ok(())
}

fn collect_from_requirements(
    path: PathBuf,
    observed: &mut ObservedWorkspaceContract,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let name = extract_python_dep_name(trimmed);
        register_dependency(observed, &format!("pip:{}", name));
    }
    Ok(())
}

fn collect_from_composer(path: PathBuf, observed: &mut ObservedWorkspaceContract) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    for section in ["require", "require-dev"] {
        if let Some(map) = value.get(section).and_then(|entry| entry.as_object()) {
            for key in map.keys() {
                if key != "php" {
                    register_dependency(observed, &format!("composer:{}", key.to_lowercase()));
                }
            }
        }
    }
    Ok(())
}

fn collect_endpoints_and_ports(
    root: &Path,
    observed: &mut ObservedWorkspaceContract,
) -> Result<()> {
    use regex::Regex;
    let url_re = Regex::new(r"https?://[A-Za-z0-9\.\-_:]+(?:/[A-Za-z0-9\.\-_/]*)?")?;
    let host_port_re = Regex::new(r"(?:localhost|127\.0\.0\.1|0\.0\.0\.0):([0-9]{2,5})")?;
    let env_port_re = Regex::new(r"\bPORT\s*=?\s*([0-9]{2,5})\b")?;
    let arg_port_re = Regex::new(r"--port\s+([0-9]{2,5})")?;

    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | "node_modules" | "target" | "dist" | "build" | ".next"
            )
        });

    for entry in walker.filter_map(std::result::Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .metadata()
            .map(|m| m.len() > 1_000_000)
            .unwrap_or(true)
        {
            continue;
        }
        let path = entry.path();
        if let Ok(content) = std::fs::read_to_string(path) {
            for m in url_re.find_iter(&content) {
                observed.endpoints.insert(m.as_str().to_string());
            }
            for caps in host_port_re.captures_iter(&content) {
                if let Some(port) = caps.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) {
                    observed.ports.insert(port);
                }
            }
            for caps in env_port_re.captures_iter(&content) {
                if let Some(port) = caps.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) {
                    observed.ports.insert(port);
                }
            }
            for caps in arg_port_re.captures_iter(&content) {
                if let Some(port) = caps.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) {
                    observed.ports.insert(port);
                }
            }
        }
    }
    Ok(())
}

fn extract_python_dep_name(raw: &str) -> String {
    raw.chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
        .to_lowercase()
}

fn register_dependency(observed: &mut ObservedWorkspaceContract, dependency: &str) {
    let normalized = dependency.trim().to_lowercase();
    if normalized.is_empty() {
        return;
    }
    observed.dependencies.insert(normalized.clone());
    if let Some(framework) = infer_framework(&normalized) {
        observed.frameworks.insert(framework);
    }
}

fn infer_framework(dependency: &str) -> Option<String> {
    const FRAMEWORK_MARKERS: &[(&str, &str)] = &[
        ("react", "react"),
        ("next", "nextjs"),
        ("vue", "vue"),
        ("nuxt", "nuxt"),
        ("svelte", "svelte"),
        ("angular", "angular"),
        ("express", "express"),
        ("nestjs", "nestjs"),
        ("fastapi", "fastapi"),
        ("django", "django"),
        ("flask", "flask"),
        ("laravel", "laravel"),
        ("symfony", "symfony"),
        ("rails", "rails"),
        ("spring", "spring"),
        ("actix-web", "actix"),
        ("axum", "axum"),
    ];
    FRAMEWORK_MARKERS
        .iter()
        .find(|(needle, _)| dependency.contains(needle))
        .map(|(_, framework)| framework.to_string())
}

/// Task analysis result
#[derive(Debug, Clone)]
struct TaskAnalysis {
    task: String,
    goals: Vec<Goal>,
    constraints: Vec<String>,
    complexity: f64,
}

/// P2P consensus query result
type ConsensusQueryResult = consensus::ConsensusQueryResult;

/// Execution report
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionReport {
    pub task: String,
    pub agent_id: Uuid,
    pub duration_ms: i64,
    pub alignment_score: f64,
    pub actions_taken: usize,
    pub deviations_detected: usize,
    pub success: bool,
    pub north_star: ExecutionNorthStar,
    pub reliability: ReliabilitySnapshot,
}

/// Deviation details
#[derive(Debug, Clone)]
struct DeviationDetails {
    action_id: Uuid,
    reason: String,
    probability: f64,
}

/// Action execution result
#[derive(Debug, Clone)]
struct NativeActionResult {
    action_id: Uuid,
    action: Action,
    state: ProjectState,
    duration_ms: i64,
}

/// Execution result
#[derive(Debug, Clone)]
struct ExecutionResult {
    actions: Vec<NativeActionResult>,
    alignment_score: f64,
    deviations: Vec<DeviationDetails>,
    success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_core::goal_manifold::{GoalManifold, Intent};

    #[test]
    fn governance_drift_blocks_when_required_dependencies_missing() {
        let mut manifold = GoalManifold::new(Intent::new("test", Vec::<String>::new()));
        manifold.governance.allowed_dependencies = vec!["cargo:tokio".to_string()];
        manifold.governance.required_dependencies = vec!["cargo:tokio".to_string()];
        manifold.governance.allowed_frameworks = vec!["framework:axum".to_string()];
        manifold.governance.required_frameworks = vec!["framework:axum".to_string()];

        let observed = ObservedWorkspaceContract {
            dependencies: BTreeSet::new(),
            frameworks: BTreeSet::new(),
            endpoints: BTreeSet::new(),
            ports: BTreeSet::new(),
        };

        let drift = compute_contract_drift(&manifold.governance, &observed);
        assert!(drift.has_blocking_violation());
        assert_eq!(
            drift.missing_required_dependencies,
            vec!["cargo:tokio".to_string()]
        );
        assert_eq!(
            drift.missing_required_frameworks,
            vec!["framework:axum".to_string()]
        );
    }

    #[test]
    fn governance_proposal_contains_additions_and_removals() {
        let drift = GovernanceDrift {
            unexpected_dependencies: vec!["cargo:reqwest".to_string()],
            unexpected_frameworks: vec!["framework:nextjs".to_string()],
            unexpected_endpoints: vec!["http://localhost:4173".to_string()],
            unexpected_ports: vec![4173],
            missing_required_dependencies: vec!["cargo:tokio".to_string()],
            missing_required_frameworks: vec!["framework:axum".to_string()],
        };

        let proposal = build_governance_proposal(&drift);
        assert_eq!(
            proposal.proposed_dependencies,
            vec!["cargo:reqwest".to_string()]
        );
        assert_eq!(
            proposal.proposed_dependency_removals,
            vec!["cargo:tokio".to_string()]
        );
        assert_eq!(
            proposal.proposed_framework_removals,
            vec!["framework:axum".to_string()]
        );
        assert_eq!(proposal.deterministic_confidence, 1.0);
        assert!(!proposal.evidence.is_empty());
    }
}
