//! Guardrail Engine - La barriera fisica all'esecuzione
//!
//! Impedisce l'esecuzione di comandi o la compilazione se lo stato
//! di allineamento è sotto la soglia critica o se ci sono violazioni invarianti.

use crate::goal_manifold::predicate::PredicateState;
use crate::GoalManifold;
use futures::executor::block_on;

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
        let state = PredicateState::new(
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        );
        Self::evaluate_with_state(manifold, &state)
    }

    /// Valuta se permettere l'esecuzione con uno stato predicativo esplicito.
    pub fn evaluate_with_state(
        manifold: &GoalManifold,
        state: &PredicateState,
    ) -> GuardrailDecision {
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

        let critical_violations = block_on(async {
            manifold
                .validate_invariants(state)
                .await
                .into_iter()
                .filter(|violation| {
                    matches!(
                        violation.severity,
                        crate::goal_manifold::InvariantSeverity::Critical
                    )
                })
                .count()
        });
        if critical_violations > 0 {
            return GuardrailDecision {
                allowed: false,
                reason: Some(format!(
                    "CRITICAL INVARIANTS VIOLATED: {} invarianti critiche non soddisfatte.",
                    critical_violations
                )),
                score_at_check: current_score,
            };
        }

        // Check violazioni invarianti critiche (se presenti)
        if manifold.overrides.len() > 10 {
            // Esempio: troppi override indicano instabilità
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_manifold::{goal::Goal, predicate::Predicate, Intent, Invariant};

    #[test]
    fn guardrail_blocks_when_below_sensitivity() {
        let mut manifold = GoalManifold::new(Intent::new("Test", Vec::<String>::new()));
        manifold.sensitivity = 0.9;

        let decision = GuardrailEngine::evaluate_with_state(
            &manifold,
            &PredicateState::new(std::path::PathBuf::from(".")),
        );
        assert!(!decision.allowed);
    }

    #[test]
    fn guardrail_blocks_on_critical_invariant_violation() {
        let mut manifold = GoalManifold::new(Intent::new("Test", Vec::<String>::new()));
        manifold.sensitivity = 0.0;

        let goal = Goal::builder()
            .description("Always complete")
            .add_success_criterion(Predicate::AlwaysTrue)
            .build()
            .unwrap();
        manifold.add_goal(goal).unwrap();
        manifold
            .add_invariant(Invariant::critical("must fail", Predicate::AlwaysFalse))
            .unwrap();

        let decision = GuardrailEngine::evaluate_with_state(
            &manifold,
            &PredicateState::new(std::path::PathBuf::from(".")),
        );
        assert!(!decision.allowed);
        assert!(decision
            .reason
            .unwrap_or_default()
            .contains("CRITICAL INVARIANTS"));
    }
}
