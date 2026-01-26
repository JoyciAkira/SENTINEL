//! Directed Acyclic Graph (DAG) for goal dependencies
//!
//! This module implements a type-safe DAG for managing goal dependencies.
//! It prevents cycles and provides efficient traversal and topological sorting.

use crate::error::{DagError, Result};
use crate::goal_manifold::goal::Goal;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Directed Acyclic Graph of goals
///
/// The DAG enforces dependencies between goals and prevents cycles.
/// It uses `petgraph` for efficient graph operations and provides
/// high-level operations for goal management.
///
/// # Invariants
///
/// - No cycles: Adding an edge that would create a cycle is rejected
/// - Anti-dependencies are respected: Goals with anti-dependencies cannot have edges
/// - All node IDs map to valid goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDag {
    /// The underlying petgraph DiGraph
    #[serde(skip)]
    graph: DiGraph<Uuid, ()>,

    /// Mapping from UUID to NodeIndex
    #[serde(skip)]
    node_map: HashMap<Uuid, NodeIndex>,

    /// Goals indexed by UUID (source of truth)
    goals: HashMap<Uuid, Goal>,
}

impl GoalDag {
    /// Create a new empty DAG
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            goals: HashMap::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a goal to the DAG
    ///
    /// # Errors
    ///
    /// Returns `Err` if a goal with this ID already exists.
    pub fn add_goal(&mut self, goal: Goal) -> Result<()> {
        if self.goals.contains_key(&goal.id) {
            return Err(DagError::NodeNotFound(goal.id).into());
        }

        let id = goal.id;
        let node_idx = self.graph.add_node(id);
        self.node_map.insert(id, node_idx);
        self.goals.insert(id, goal);

        Ok(())
    }

    /// Get a goal by ID
    pub fn get_goal(&self, id: &Uuid) -> Option<&Goal> {
        self.goals.get(id)
    }

    /// Get a mutable reference to a goal by ID
    pub fn get_goal_mut(&mut self, id: &Uuid) -> Option<&mut Goal> {
        self.goals.get_mut(id)
    }

    /// Remove a goal from the DAG
    ///
    /// This also removes all edges connected to this goal.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the goal doesn't exist.
    pub fn remove_goal(&mut self, id: &Uuid) -> Result<Goal> {
        let node_idx = self
            .node_map
            .remove(id)
            .ok_or(DagError::NodeNotFound(*id))?;

        self.graph.remove_node(node_idx);

        let goal = self.goals.remove(id).ok_or(DagError::NodeNotFound(*id))?;

        Ok(goal)
    }

    /// Add a dependency edge: `from` depends on `to`
    ///
    /// This means `to` must complete before `from` can start.
    /// In the graph, we store this as `to -> from` (standard DAG semantics).
    ///
    /// # Errors
    ///
    /// - Returns `Err` if either node doesn't exist
    /// - Returns `Err` if adding this edge would create a cycle
    /// - Returns `Err` if there's an anti-dependency conflict
    pub fn add_dependency(&mut self, from: Uuid, to: Uuid) -> Result<()> {
        // Check nodes exist
        let from_idx = self
            .node_map
            .get(&from)
            .ok_or(DagError::NodeNotFound(from))?;
        let to_idx = self.node_map.get(&to).ok_or(DagError::NodeNotFound(to))?;

        // Check anti-dependency conflicts
        let from_goal = self.goals.get(&from).unwrap();
        if from_goal.anti_dependencies.contains(&to) {
            return Err(DagError::AntiDependencyConflict {
                goal: from,
                blocked: to,
            }
            .into());
        }

        // Check if edge would create cycle (note: reversed direction)
        if self.would_create_cycle(*to_idx, *from_idx) {
            return Err(DagError::WouldCreateCycle { from, to }.into());
        }

        // Add edge: to -> from (standard DAG semantics: to must complete before from)
        self.graph.add_edge(*to_idx, *from_idx, ());

        // Update goal's dependencies list
        if let Some(goal) = self.goals.get_mut(&from) {
            if !goal.dependencies.contains(&to) {
                goal.dependencies.push(to);
            }
        }

        Ok(())
    }

    /// Remove a dependency edge
    ///
    /// # Errors
    ///
    /// Returns `Err` if the edge doesn't exist.
    pub fn remove_dependency(&mut self, from: Uuid, to: Uuid) -> Result<()> {
        let from_idx = self
            .node_map
            .get(&from)
            .ok_or(DagError::NodeNotFound(from))?;
        let to_idx = self.node_map.get(&to).ok_or(DagError::NodeNotFound(to))?;

        // Find and remove the edge (note: reversed direction)
        if let Some(edge) = self.graph.find_edge(*to_idx, *from_idx) {
            self.graph.remove_edge(edge);
        } else {
            return Err(DagError::EdgeNotFound { from, to }.into());
        }

        // Update goal's dependencies list
        if let Some(goal) = self.goals.get_mut(&from) {
            goal.dependencies.retain(|&dep| dep != to);
        }

        Ok(())
    }

    /// Check if adding an edge would create a cycle
    ///
    /// This performs a DFS from `to` to see if we can reach `from`.
    /// If we can, then adding `from -> to` would create a cycle.
    fn would_create_cycle(&self, from: NodeIndex, to: NodeIndex) -> bool {
        // Can we reach `from` starting from `to`?
        let mut dfs = Dfs::new(&self.graph, to);
        while let Some(node) = dfs.next(&self.graph) {
            if node == from {
                return true;
            }
        }
        false
    }

    /// Get all goals that have no dependencies (can start immediately)
    pub fn get_ready_goals(&self) -> Vec<&Goal> {
        self.goals
            .values()
            .filter(|goal| {
                // Ready if:
                // 1. Status is Ready
                // 2. OR status is Pending and all dependencies are completed
                matches!(goal.status, crate::types::GoalStatus::Ready)
                    || (matches!(goal.status, crate::types::GoalStatus::Pending)
                        && self.dependencies_satisfied(goal.id))
            })
            .collect()
    }

    /// Check if all dependencies for a goal are satisfied
    pub fn dependencies_satisfied(&self, goal_id: Uuid) -> bool {
        let goal = match self.goals.get(&goal_id) {
            Some(g) => g,
            None => return false,
        };

        goal.dependencies.iter().all(|dep_id| {
            self.goals
                .get(dep_id)
                .map(|dep| dep.status == crate::types::GoalStatus::Completed)
                .unwrap_or(false)
        })
    }

    /// Get all direct dependencies of a goal
    pub fn get_dependencies(&self, goal_id: Uuid) -> Result<Vec<&Goal>> {
        let node_idx = self
            .node_map
            .get(&goal_id)
            .ok_or(DagError::NodeNotFound(goal_id))?;

        // Since edges go from dependency to dependent, we look at incoming edges
        let dep_goals: Vec<&Goal> = self
            .graph
            .neighbors_directed(*node_idx, Direction::Incoming)
            .filter_map(|neighbor_idx| {
                let dep_id = self.graph[neighbor_idx];
                self.goals.get(&dep_id)
            })
            .collect();

        Ok(dep_goals)
    }

    /// Get all goals that depend on this goal
    pub fn get_dependents(&self, goal_id: Uuid) -> Result<Vec<&Goal>> {
        let node_idx = self
            .node_map
            .get(&goal_id)
            .ok_or(DagError::NodeNotFound(goal_id))?;

        // Since edges go from dependency to dependent, we look at outgoing edges
        let dependent_goals: Vec<&Goal> = self
            .graph
            .neighbors_directed(*node_idx, Direction::Outgoing)
            .filter_map(|neighbor_idx| {
                let dependent_id = self.graph[neighbor_idx];
                self.goals.get(&dependent_id)
            })
            .collect();

        Ok(dependent_goals)
    }

    /// Get topological sort of goals
    ///
    /// Returns goals in an order such that dependencies come before dependents.
    /// This is useful for execution planning.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the graph contains a cycle (should never happen if invariants maintained).
    pub fn topological_sort(&self) -> Result<Vec<&Goal>> {
        use petgraph::algo::toposort;

        let sorted_indices = toposort(&self.graph, None)
            .map_err(|cycle| DagError::CycleDetected(vec![self.graph[cycle.node_id()]]))?;

        let sorted_goals: Vec<&Goal> = sorted_indices
            .iter()
            .filter_map(|&idx| {
                let id = self.graph[idx];
                self.goals.get(&id)
            })
            .collect();

        Ok(sorted_goals)
    }

    /// Detect cycles in the graph (defensive check)
    ///
    /// Returns the cycle if found.
    pub fn detect_cycle(&self) -> Option<Vec<Uuid>> {
        use petgraph::algo::tarjan_scc;

        // Strongly connected components with size > 1 indicate cycles
        let sccs = tarjan_scc(&self.graph);

        for scc in sccs {
            if scc.len() > 1 {
                let cycle: Vec<Uuid> = scc.iter().map(|&idx| self.graph[idx]).collect();
                return Some(cycle);
            }
        }

        None
    }

    /// Get the number of goals in the DAG
    pub fn len(&self) -> usize {
        self.goals.len()
    }

    /// Check if the DAG is empty
    pub fn is_empty(&self) -> bool {
        self.goals.is_empty()
    }

    /// Get all goals
    pub fn goals(&self) -> impl Iterator<Item = &Goal> {
        self.goals.values()
    }

    /// Get all goals (mutable)
    pub fn goals_mut(&mut self) -> impl Iterator<Item = &mut Goal> {
        self.goals.values_mut()
    }

    /// Calculate critical path (longest path from root to leaf)
    ///
    /// This is useful for estimating project completion time.
    pub fn critical_path(&self) -> Vec<&Goal> {
        // Find root nodes (no dependencies)
        let roots: Vec<_> = self
            .goals
            .values()
            .filter(|g| g.dependencies.is_empty())
            .collect();

        if roots.is_empty() {
            return vec![];
        }

        // BFS to find longest path
        let mut longest_path = vec![];
        let mut max_length = 0.0;

        for root in roots {
            let path = self.find_longest_path_from(root.id);
            let path_length: f64 = path.iter().map(|g| g.complexity_estimate.mean).sum();

            if path_length > max_length {
                max_length = path_length;
                longest_path = path;
            }
        }

        longest_path
    }

    /// Find longest path from a given goal (using DFS)
    fn find_longest_path_from(&self, goal_id: Uuid) -> Vec<&Goal> {
        let goal = match self.goals.get(&goal_id) {
            Some(g) => g,
            None => return vec![],
        };

        if goal.dependencies.is_empty() {
            return vec![goal];
        }

        // Recursively find longest path through dependencies
        let mut longest = vec![];
        let mut max_length = 0.0;

        for dep_id in &goal.dependencies {
            let dep_path = self.find_longest_path_from(*dep_id);
            let path_length: f64 = dep_path.iter().map(|g| g.complexity_estimate.mean).sum();

            if path_length > max_length {
                max_length = path_length;
                longest = dep_path;
            }
        }

        longest.push(goal);
        longest
    }
}

impl Default for GoalDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_manifold::predicate::Predicate;
    use std::collections::HashSet;

    fn create_test_goal(desc: &str) -> Goal {
        Goal::builder()
            .description(desc)
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap()
    }

    #[test]
    fn test_dag_creation() {
        let dag = GoalDag::new();
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
    }

    #[test]
    fn test_add_goal() {
        let mut dag = GoalDag::new();
        let goal = create_test_goal("Test goal");
        let id = goal.id;

        dag.add_goal(goal).unwrap();

        assert_eq!(dag.len(), 1);
        assert!(dag.get_goal(&id).is_some());
    }

    #[test]
    fn test_add_dependency() {
        let mut dag = GoalDag::new();

        let goal1 = create_test_goal("Goal 1");
        let goal2 = create_test_goal("Goal 2");
        let id1 = goal1.id;
        let id2 = goal2.id;

        dag.add_goal(goal1).unwrap();
        dag.add_goal(goal2).unwrap();

        // goal1 depends on goal2
        dag.add_dependency(id1, id2).unwrap();

        let deps = dag.get_dependencies(id1).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, id2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut dag = GoalDag::new();

        let goal1 = create_test_goal("Goal 1");
        let goal2 = create_test_goal("Goal 2");
        let id1 = goal1.id;
        let id2 = goal2.id;

        dag.add_goal(goal1).unwrap();
        dag.add_goal(goal2).unwrap();

        // Add edge 1 -> 2
        dag.add_dependency(id1, id2).unwrap();

        // Try to add edge 2 -> 1 (would create cycle)
        let result = dag.add_dependency(id2, id1);
        assert!(result.is_err());
    }

    #[test]
    fn test_topological_sort() {
        let mut dag = GoalDag::new();

        let goal1 = create_test_goal("Goal 1");
        let goal2 = create_test_goal("Goal 2");
        let goal3 = create_test_goal("Goal 3");
        let id1 = goal1.id;
        let id2 = goal2.id;
        let id3 = goal3.id;

        dag.add_goal(goal1).unwrap();
        dag.add_goal(goal2).unwrap();
        dag.add_goal(goal3).unwrap();

        // Dependencies: 3 -> 2 -> 1
        dag.add_dependency(id1, id2).unwrap();
        dag.add_dependency(id2, id3).unwrap();

        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);

        // goal3 should come before goal2, goal2 before goal1
        let pos1 = sorted.iter().position(|g| g.id == id1).unwrap();
        let pos2 = sorted.iter().position(|g| g.id == id2).unwrap();
        let pos3 = sorted.iter().position(|g| g.id == id3).unwrap();

        assert!(pos3 < pos2);
        assert!(pos2 < pos1);
    }

    #[test]
    fn test_dependencies_satisfied() {
        let mut dag = GoalDag::new();

        let mut goal1 = create_test_goal("Goal 1");
        let goal2 = create_test_goal("Goal 2");
        let id1 = goal1.id;
        let id2 = goal2.id;

        dag.add_goal(goal1).unwrap();
        dag.add_goal(goal2).unwrap();

        dag.add_dependency(id1, id2).unwrap();

        // goal2 not completed yet
        assert!(!dag.dependencies_satisfied(id1));

        // Complete goal2 with proper transitions
        let goal2_mut = dag.get_goal_mut(&id2).unwrap();
        goal2_mut.mark_ready().unwrap();
        goal2_mut.start().unwrap();
        goal2_mut.begin_validation().unwrap();
        goal2_mut.complete().unwrap();

        // Now goal1's dependencies are satisfied
        assert!(dag.dependencies_satisfied(id1));
    }

    #[test]
    fn test_remove_goal() {
        let mut dag = GoalDag::new();

        let goal = create_test_goal("Test goal");
        let id = goal.id;

        dag.add_goal(goal).unwrap();
        assert_eq!(dag.len(), 1);

        let removed = dag.remove_goal(&id).unwrap();
        assert_eq!(removed.id, id);
        assert_eq!(dag.len(), 0);
    }

    #[test]
    fn test_get_dependents() {
        let mut dag = GoalDag::new();

        let goal1 = create_test_goal("Goal 1");
        let goal2 = create_test_goal("Goal 2");
        let goal3 = create_test_goal("Goal 3");
        let id1 = goal1.id;
        let id2 = goal2.id;
        let id3 = goal3.id;

        dag.add_goal(goal1).unwrap();
        dag.add_goal(goal2).unwrap();
        dag.add_goal(goal3).unwrap();

        // Both goal1 and goal2 depend on goal3
        dag.add_dependency(id1, id3).unwrap();
        dag.add_dependency(id2, id3).unwrap();

        let dependents = dag.get_dependents(id3).unwrap();
        assert_eq!(dependents.len(), 2);

        let dependent_ids: HashSet<Uuid> = dependents.iter().map(|g| g.id).collect();
        assert!(dependent_ids.contains(&id1));
        assert!(dependent_ids.contains(&id2));
    }
}
