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

use crate::memory::embeddings::Embedder;

/// Una proposta architettonica generata da Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalProposal {
    pub root_intent: Intent,
    pub proposed_goals: Vec<Goal>,
    pub proposed_invariants: Vec<String>,
    pub confidence_score: f64,
}

pub struct ArchitectEngine {
    embedder: Embedder,
}

impl ArchitectEngine {
    pub fn new() -> Self {
        Self {
            embedder: Embedder::new(),
        }
    }

    /// Analizza un intento e propone una struttura di progetto usando SLM (Semantic Clustering)
    pub fn propose_architecture(&self, intent: Intent) -> Result<ArchitecturalProposal> {
        let mut proposed_goals = Vec::new();
        let mut proposed_invariants = Vec::new();

        // 1. Generazione Embedding dell'intento
        let intent_vector = self.embedder.embed(&intent.description);
        
        // 2. Analisi semantica (Iterazione 2.2)
        // In futuro: confronto con KB di pattern globali. Per ora: euristica potenziata semantizzata.
        let desc = intent.description.to_lowercase();

        proposed_goals.push(self.create_suggested_goal(
            "Kernel Foundation",
            "Inizializzazione del core allineato ai vettori d'intento rilevati.",
            0.1
        ));

        // Esempio di check semantico (molto più preciso delle keyword semplici)
        if self.is_semantic_match(&intent_vector, "web service, api gateway, rest server") {
            proposed_goals.push(self.create_suggested_goal(
                "Service Mesh Layer",
                "Definizione della comunicazione tra servizi e contratti API.",
                0.25
            ));
            proposed_invariants.push("Zero-Trust Communication tra i moduli".to_string());
        }

        if self.is_semantic_match(&intent_vector, "secure, cryptography, blockchain, protected") {
            proposed_goals.push(self.create_suggested_goal(
                "Security Manifold",
                "Implementazione dei guardrails di sicurezza crittografica.",
                0.3
            ));
            proposed_invariants.push("Tutti i dati sensibili devono essere cifrati via Blake3".to_string());
        }

        Ok(ArchitecturalProposal {
            root_intent: intent,
            proposed_goals,
            proposed_invariants,
            confidence_score: if self.embedder.is_sota() { 0.92 } else { 0.75 },
        })
    }

    fn is_semantic_match(&self, intent_vec: &[f32], target: &str) -> bool {
        let target_vec = self.embedder.embed(target);
        crate::memory::embeddings::cosine_similarity(intent_vec, &target_vec) > 0.4
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
