#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_orchestrator_initialization() {
        let orchestrator = AgentOrchestrator::new();

        assert!(!orchestrator.agents.is_empty());
        assert_eq!(orchestrator.stats.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_decompose_goal_into_tasks() {
        let orchestrator = AgentOrchestrator::new();

        let goal = Goal {
            id: Uuid::new_v4(),
            description: "Implement JWT authentication system with token generation and validation".to_string(),
            success_criteria: vec![],
            dependencies: vec![],
            anti_dependencies: vec![],
            complexity_estimate: sentinel_core::tests::ProbabilityDistribution {
                mean: 75.0,
                std_dev: 5.0,
            },
            value_to_root: 1.0,
            status: sentinel_core::goal_manifold::goal::GoalStatus::Pending,
            parent_id: None,
            validation_tests: vec![],
            metadata: sentinel_core::goal_manifold::goal::GoalMetadata::default(),
        };

        let tasks = orchestrator
            .decompose_goal_into_tasks(&goal)
            .await
            .expect("Failed to decompose goal");

        assert!(!tasks.is_empty());

        // Should have at least: design, implementation, testing, documentation
        assert!(tasks.len() >= 4);

        // Check task types
        let has_design = tasks.iter().any(|t| t.required_agent == AgentType::Refactoring);
        let has_implementation = tasks.iter().any(|t| {
            matches!(t.required_agent, AgentType::CodeGeneration | AgentType::Testing | AgentType::Documentation)
        });
        let has_testing = tasks.iter().any(|t| t.required_agent == AgentType::Testing);
        let has_docs = tasks.iter().any(|t| t.required_agent == AgentType::Documentation);

        assert!(has_design);
        assert!(has_implementation);
        assert!(has_testing);
        assert!(has_docs);
    }

    #[tokio::test]
    async fn test_assign_tasks_to_agents() {
        let mut orchestrator = AgentOrchestrator::new();

        // Create tasks
        let tasks = vec![
            Task {
                id: Uuid::new_v4(),
                description: "Task 1".to_string(),
                parent_id: None,
                required_agent: AgentType::CodeGeneration,
                priority: 0.9,
                estimated_duration_ms: 10000,
                dependencies: vec![],
                anti_dependencies: vec![],
            },
            Task {
                id: Uuid::new_v4(),
                description: "Task 2".to_string(),
                parent_id: None,
                required_agent: AgentType::CodeGeneration,
                priority: 0.8,
                estimated_duration_ms: 10000,
                dependencies: vec![],
                anti_dependencies: vec![],
            },
            Task {
                id: Uuid::new_v4(),
                description: "Task 3".to_string(),
                parent_id: None,
                required_agent: AgentType::Testing,
                priority: 0.7,
                estimated_duration_ms: 5000,
                dependencies: vec![],
                anti_dependencies: vec![],
            },
        ];

        let assignments = orchestrator
            .assign_tasks_to_agents(&tasks)
            .await
            .expect("Failed to assign tasks");

        assert_eq!(assignments.len(), 3);

        // Check load balancing
        let mut load_counts: HashMap<Uuid, u64> = HashMap::new();
        for assignment in &assignments {
            *load_counts.entry(assignment.agent_id).or_insert(0) += 1;
        }

        // CodeGen agents should have 1 task each (2 agents)
        let codegen_agents = assignments
            .iter()
            .filter(|a| {
                // Find agent specialization
                for agents in orchestrator.agents.get(&AgentType::CodeGeneration).unwrap_or(&vec![]) {
                    if agents.iter().any(|agent| agent.agent_id == a.agent_id) {
                        return true;
                    }
                }
                false
            })
            .count();

        assert_eq!(codegen_agents, 2);
    }

    #[tokio::test]
    async fn test_build_dependency_graph() {
        let mut orchestrator = AgentOrchestrator::new();

        // Create tasks with dependencies
        let task1 = Task {
            id: Uuid::new_v4(),
            description: "Task 1".to_string(),
            parent_id: None,
            required_agent: AgentType::CodeGeneration,
            priority: 0.9,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![],
        };

        let task2 = Task {
            id: Uuid::new_v4(),
            description: "Task 2".to_string(),
            parent_id: Some(task1.id),
            required_agent: AgentType::Testing,
            priority: 0.8,
            estimated_duration_ms: 5000,
            dependencies: vec![task1.id],
            anti_dependencies: vec![],
        };

        let tasks = vec![task1, task2];

        let result = orchestrator
            .build_dependency_graph(&tasks);

        assert!(result.is_ok());

        let graph = &orchestrator.dependency_graph;

        assert!(graph.nodes.len() == 2);
        assert!(graph.edges.len() == 1);
        assert!(graph.edges.contains_key(&task1.id));
    }

    #[tokio::test]
    fn test_detect_dependency_cycle() {
        let mut orchestrator = AgentOrchestrator::new();

        // Create tasks that form a cycle
        let task1 = Task {
            id: Uuid::new_v4(),
            description: "Task 1".to_string(),
            parent_id: None,
            required_agent: AgentType::CodeGeneration,
            priority: 0.9,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![],
        };

        let task2 = Task {
            id: Uuid::new_v4(),
            description: "Task 2".to_string(),
            parent_id: Some(task1.id),
            required_agent: AgentType::Testing,
            priority: 0.8,
            estimated_duration_ms: 5000,
            dependencies: vec![task1.id],
            anti_dependencies: vec![],
        };

        let task3 = Task {
            id: Uuid::new_v4(),
            description: "Task 3".to_string(),
            parent_id: Some(task2.id),
            required_agent: AgentType::Refactoring,
            priority: 0.7,
            estimated_duration_ms: 5000,
            dependencies: vec![task2.id],
            anti_dependencies: vec![],
        };

        let tasks = vec![task1, task2, task3];

        // Build graph (this should detect cycle)
        let _ = orchestrator
            .build_dependency_graph(&tasks);

        // Check for cycle
        let cycle = orchestrator.detect_dependency_cycle();

        assert!(cycle.is_some());

        if let Some(cycle_tasks) = cycle {
            assert!(cycle_tasks.len() == 3); // All 3 tasks in cycle
        }
    }

    #[tokio::test]
    fn test_detect_anti_dependency_violation() {
        let mut orchestrator = AgentOrchestrator::new();

        // Create tasks with anti-dependencies
        let task1 = Task {
            id: Uuid::new_v4(),
            description: "Task 1".to_string(),
            parent_id: None,
            required_agent: AgentType::CodeGeneration,
            priority: 0.9,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![],
        };

        let task2 = Task {
            id: Uuid::new_v4(),
            description: "Task 2".to_string(),
            parent_id: None,
            required_agent: AgentType::CodeGeneration,
            priority: 0.8,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![task1.id], // Anti-dep on task1
        };

        let tasks = vec![task1, task2];

        let _ = orchestrator.build_dependency_graph(&tasks);

        // Detect anti-dependency violation
        let violation = orchestrator.detect_anti_dependency_violation();

        // task1 and task2 both require CodeGeneration (same agent)
        // task2 has anti-dependency on task1 (cannot run simultaneously)
        // This IS an anti-dependency violation
        assert!(violation.is_some());

        if let Some(violating_tasks) = violation {
            assert!(violating_tasks.len() == 2);
        }
    }

    #[tokio::test]
    fn test_separate_conflicted_and_parallel_tasks() {
        let mut orchestrator = AgentOrchestrator::new();

        let assignments = vec![
            TaskAssignment {
                task_id: Uuid::new_v4(),
                agent_id: Uuid::new_v4(),
                assigned_at: chrono::Utc::now(),
            },
        ];

        let conflicts = vec![];

        let (conflicted, parallel) = orchestrator
            .separate_conflicted_and_parallel_tasks(&assignments, &conflicts);

        // No conflicts, all parallel
        assert_eq!(conflicted.len(), 0);
        assert_eq!(parallel.len(), 1);
    }

    #[tokio::test]
    fn test_conflict_detector() {
        let detector = ConflictDetector::new();

        assert!(detector.resource_conflicts.is_empty());
        assert!(detector.goal_conflicts.is_empty());
        assert!(detector.resolved_conflicts.is_empty());
    }

    #[tokio::test]
    fn test_tasks_share_resource() {
        let detector = ConflictDetector::new();

        let task1 = Task {
            id: Uuid::new_v4(),
            description: "Create auth.rs".to_string(),
            parent_id: None,
            required_agent: AgentType::CodeGeneration,
            priority: 0.9,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![],
        };

        let task2 = Task {
            id: Uuid::new_v4(),
            description: "Edit auth.rs".to_string(),
            parent_id: None,
            required_agent: AgentType::Refactoring,
            priority: 0.8,
            estimated_duration_ms: 10000,
            dependencies: vec![],
            anti_dependencies: vec![],
        };

        let assignments = vec![
            TaskAssignment {
                task_id: task1.id,
                agent_id: Uuid::new_v4(),
                assigned_at: chrono::Utc::now(),
            },
            TaskAssignment {
                task_id: task2.id,
                agent_id: Uuid::new_v4(),
                assigned_at: chrono::Utc::now(),
            },
        ];

        // Both tasks share auth.rs file
        let shares = detector.tasks_share_resource(
            &assignments,
            &task1.id,
            &task2.id,
        );

        assert!(shares);
    }
}
