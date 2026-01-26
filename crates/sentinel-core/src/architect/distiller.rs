//! Cognitive Distiller - Il distillatore dell'onniscienza
//!
//! Trasforma il complesso Goal Manifold in una struttura gerarchica
//! ottimizzata per il consumo da parte degli LLM.

use crate::GoalManifold;

/// Risultato della distillazione con metriche di efficienza
pub struct DistillationReport {
    pub content: String,
    pub strategic_density: f64,
    pub tactical_density: f64,
    pub operational_density: f64,
    pub total_tokens_estimated: usize,
}

pub struct CognitiveDistiller;

impl CognitiveDistiller {
    pub fn distill(manifold: &GoalManifold) -> DistillationReport {
        let mut doc = String::new();
        doc.push_str("# SENTINEL COGNITIVE MAP\n\n");

        // 1. STRATEGIC TIER
        doc.push_str("## 🌟 STRATEGIC TIER\n");
        doc.push_str(&format!("**NORTH STAR:** {}\n", manifold.root_intent.description));
        for inv in &manifold.invariants {
            doc.push_str(&format!(