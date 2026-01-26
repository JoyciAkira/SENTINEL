//! Autonomous Architect Engine
//!
//! Questo motore trasforma l'Intento (Natural Language) in una struttura
//! formale di Goal e Invarianti. È il "primo respiro" di ogni progetto Sentinel.

use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use crate::goal_manifold::Intent;
use crate::types::ProbabilityDistribution;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Una proposta architettonica generata da Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalProposal {
    pub root_intent: Intent,
    pub proposed_goals: Vec<Goal>,
    pub proposed_invariants: Vec<String>,
    pub confidence_score: f64,
}

pub struct ArchitectEngine {
    // In futuro ospiterà il riferimento al modello SLM (Candle)
}

impl ArchitectEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Analizza un intento e propone una struttura di progetto
    pub fn propose_architecture(&self, intent: Intent) -> Result<ArchitecturalProposal> {
        let mut proposed_goals = Vec::new();
        let mut proposed_invariants = Vec::new();

        // Logica di decomposizione deterministica (Iterazione 2.1)
        // Analizziamo le parole chiave nell'intento per suggerire i primi goal tecnici
        let desc = intent.description.to_lowercase();

        // Goal standard per ogni progetto software di qualità
        proposed_goals.push(self.create_suggested_goal(
            "Project Scaffolding",
            "Inizializzare la struttura del repository e le dipendenze base.",
            0.1
        ));

        if desc.contains("api") || desc.contains("rest") || desc.contains("backend") {
            proposed_goals.push(self.create_suggested_goal(
                "API Schema Design",
                "Definire i contratti OpenAPI/TypeSafe per gli endpoint.",
                0.2
            ));
            proposed_invariants.push("Tutti gli endpoint devono rispondere in < 200ms".to_string());
        }

        if desc.contains("test") || desc.contains("sicuro") || desc.contains("qualità") {
            proposed_goals.push(self.create_suggested_goal(
                "Test Suite Foundation",
                "Configurare il framework di test e gli obiettivi di copertura.",
                0.15
            ));
            proposed_invariants.push("Copertura del codice mai inferiore all'80%".to_string());
        }

        Ok(ArchitecturalProposal {
            root_intent: intent,
            proposed_goals,
            proposed_invariants,
            confidence_score: 0.85, // Basato sulla chiarezza dell'intento
        })
    }

    fn create_suggested_goal(&self, title: &str, desc: &str, value: f64) -> Goal {
        Goal::builder()
            .description(format!("{}: {}", title, desc))
            .complexity(ProbabilityDistribution::normal(5.0, 2.0))
            .value_to_root(value)
            .build()
            .unwrap()
    }
}
