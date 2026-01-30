//! Atomic Slicer - The engine that decomposes goals into atomic tasks.
//! 
//! This module implements the logic to break down high-level objectives 
//! into a DAG of atomic tasks with formal contracts.

use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use crate::goal_manifold::atomic::{AtomicContract, InputSpec, OutputSpec};
use uuid::Uuid;

pub struct AtomicSlicer;

impl AtomicSlicer {
    /// Decomposes a high-level goal into a vector of atomic sub-goals.
    /// 
    /// Initially, this uses a set of heuristics. In Phase 4.1 final, 
    /// it will interface with LLM to generate precise contracts.
    pub fn decompose(goal: &Goal) -> Result<Vec<Goal>> {
        let mut sub_goals = Vec::new();
        
        // Example logic: if description contains "CRUD", split into Create, Read, Update, Delete
        let desc = goal.description.to_lowercase();
        
        if desc.contains("crud") {
            let operations = vec!["Create", "Read", "Update", "Delete"];
            let mut previous_id: Option<Uuid> = None;
            
            for op in operations {
                let mut builder = Goal::builder()
                    .description(format!("{}: {}", op, goal.description))
                    .parent(goal.id);
                
                if let Some(prev) = previous_id {
                    builder = builder.add_dependency(prev);
                }
                
                // Assign an atomic contract
                let contract = AtomicContract::new();
                // TODO: Fill contract details based on operation
                
                let sub_goal = builder.atomic_contract(contract).build()?;
                previous_id = Some(sub_goal.id);
                sub_goals.push(sub_goal);
            }
        } else {
            // Default decomposition: just create one atomic task if not already atomic
            if goal.atomic_contract.is_none() {
                let sub_goal = Goal::builder()
                    .description(format!("Atomic: {}", goal.description))
                    .parent(goal.id)
                    .atomic_contract(AtomicContract::new())
                    .build()?;
                sub_goals.push(sub_goal);
            }
        }
        
        Ok(sub_goals)
    }
}
