//! Guardrail Engine - La barriera fisica all'esecuzione
//!
//! Impedisce l'esecuzione di comandi o la compilazione se lo stato
//! di allineamento è sotto la soglia critica o se ci sono violazioni invarianti.

use crate::GoalManifold;
use crate::error::{Result, SentinelError};

pub struct GuardrailEngine;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuardrailDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub score_at_check: f64,
}

impl GuardrailEngine {
    /// Valuta se permettere l'esecuzione di un'operazione
    pub fn evaluate(manifold: &GoalManifold) -> GuardrailDecision {
        let current_score = manifold.completion_percentage();
        
        // Se sensitivity è 1.0, vogliamo allineamento perfetto (1.0)
        // Se sensitivity è 0.5, vogliamo almeno 0.5, e così via.
        let threshold = manifold.sensitivity; 
        
        if current_score < threshold {
            return GuardrailDecision {
                allowed: false,
                reason: Some(format!(
                    "ALIGNMENT CRITICAL: Il progetto ha un allineamento del {:.1}%, inferiore alla soglia di sicurezza ({:.1}%). Esecuzione bloccata.",
                    current_score * 100.0,
                    threshold * 100.0
                )),
                score_at_check: current_score,
            };
        }

        // Check violazioni invarianti critiche (se presenti)
        if manifold.overrides.len() > 10 { // Esempio: troppi override indicano instabilità
             return GuardrailDecision {
                allowed: false,
                reason: Some("SYSTEM INSTABILITY: Rilevati troppi Human Overrides. Calibrare il sistema prima di procedere.".to_string()),
                score_at_check: current_score,
            };
        }

        GuardrailDecision {
            allowed: true,
            reason: None,
            score_at_check: current_score,
        }
    }
}
