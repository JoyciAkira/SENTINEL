//! Cognitive Distiller - Il distillatore dell'onniscienza
//!
//! Trasforma il complesso Goal Manifold in una struttura gerarchica
//! ottimizzata per il consumo da parte degli LLM.

use crate::GoalManifold;
use crate::goal_manifold::goal::Goal;

pub struct CognitiveDistiller;

impl CognitiveDistiller {
    /// Distilla il manifold in una stringa Markdown "World Class" per il Context Injection
    pub fn distill(manifold: &GoalManifold) -> String {
        let mut doc = String::new();

        doc.push_str("# SENTINEL COGNITIVE MAP\n\n");

        // 1. STRATEGIC TIER: NORTH STARS
        doc.push_str("## 🌟 STRATEGIC TIER (North Stars)\n");
        doc.push_str(&format!("**ULTIMATE GOAL:** {}\n", manifold.root_intent.description));
        doc.push_str("**CORE INVARIANTS:**\n");
        for inv in &manifold.invariants {
            doc.push_str(&format!("- [CRITICAL] {}\n", inv.description));
        }
        doc.push_str("\n");

        // 2. TACTICAL TIER: MID-TERM MILESTONES
        doc.push_str("## 🎯 TACTICAL TIER (Mid-term)\n");
        // Prendiamo i goal con valore più alto o più vicini alla root
        let tactical_goals: Vec<_> = manifold.goal_dag.goals()
            .filter(|g| g.value_to_root > 0.2)
            .take(3)
            .collect();
        
        for goal in tactical_goals {
            doc.push_str(&format!("- **{}**: Status: {}\n", goal.description, goal.status));
        }
        doc.push_str("\n");

        // 3. OPERATIONAL TIER: SHORT-TERM WORK
        doc.push_str("## 🛠️ OPERATIONAL TIER (Short-term)\n");
        let active_goals: Vec<_> = manifold.goal_dag.goals()
            .filter(|g| g.status == crate::types::GoalStatus::InProgress || g.status == crate::types::GoalStatus::Ready)
            .collect();

        for goal in active_goals {
            doc.push_str(&format!("### Goal ID: {}\n", goal.id));
            doc.push_str(&format!("- **Description:** {}\n", goal.description));
            doc.push_str(&format!("- **Value to Root:** {:.2}\n", goal.value_to_root));
            
            if let Some(lock) = &goal.current_lock {
                doc.push_str(&format!("- [LOCKED] Held by Agent: {}\n", lock.agent_id));
            }
            
            // Inseriamo le note di handover relative a questo goal
            let related_notes: Vec<_> = manifold.handover_log.iter()
                .filter(|n| n.goal_id == goal.id)
                .collect();
            
            if !related_notes.is_empty() {
                doc.push_str("  **COGNITIVE TRACES:**\n");
                for note in related_notes {
                    doc.push_str(&format!("  > \"{}\"\n", note.content));
                }
            }
        }

        doc.push_str("\n---\n*Sentinel Intelligence Signature: Verified Alignment Field*");
        doc
    }
}
