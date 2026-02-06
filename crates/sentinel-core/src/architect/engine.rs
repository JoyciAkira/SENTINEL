//! Autonomous Architect Engine
//!
//! Questo motore trasforma l'Intento (Natural Language) in una struttura
//! formale di Goal e Invarianti. È il "primo respiro" di ogni progetto Sentinel.

use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use crate::goal_manifold::Intent;
use crate::memory::embeddings::Embedder;
use crate::types::ProbabilityDistribution;
use serde::{Deserialize, Serialize};

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

impl Default for ArchitectEngine {
    fn default() -> Self {
        Self::new()
    }
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
        let _desc = intent.description.to_lowercase();

        // Obiettivi standard che ogni progetto dovrebbe avere
        proposed_goals.push(self.create_suggested_goal(
            "Kernel Foundation",
            "Inizializzazione del core allineato ai vettori d'intento rilevati.",
            0.1,
        )?);

        // Esempio di check semantico (molto più preciso delle keyword semplici)
        if self.is_semantic_match(&intent_vector, "web service, api gateway, rest server") {
            proposed_goals.push(self.create_suggested_goal(
                "Service Mesh Layer",
                "Definizione della comunicazione tra servizi e contratti API.",
                0.25,
            )?);
            proposed_invariants.push("Zero-Trust Communication tra i moduli".to_string());
        }

        if self.is_semantic_match(
            &intent_vector,
            "secure, cryptography, blockchain, protected",
        ) {
            proposed_goals.push(self.create_suggested_goal(
                "Security Manifold",
                "Implementazione dei guardrails di sicurezza crittografica.",
                0.3,
            )?);
            proposed_invariants
                .push("Tutti i dati sensibili devono essere cifrati via Blake3".to_string());
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

    fn create_suggested_goal(
        &self,
        title: &str,
        desc: &str,
        value: f64,
    ) -> crate::error::Result<Goal> {
        let mut criteria = Vec::new();
        let lower_desc = desc.to_lowercase();
        // let lower_title = title.to_lowercase(); // Unused

        // World-Class Type System: Infer Success Criteria based on Intent Semantics
        // A Goal is only valid if it is Verifiable.

        // 1. Code Generation Criteria
        if lower_desc.contains("create")
            || lower_desc.contains("implement")
            || lower_desc.contains("api")
        {
            criteria.push(crate::goal_manifold::predicate::Predicate::TestsPassing {
                suite: "unit".to_string(),
                min_coverage: 0.8,
            });
        }

        // 2. Security Criteria
        if lower_desc.contains("secure")
            || lower_desc.contains("auth")
            || lower_desc.contains("protection")
        {
            criteria.push(crate::goal_manifold::predicate::Predicate::Custom {
                code: "cargo audit".to_string(),
                language: crate::goal_manifold::predicate::PredicateLanguage::Shell,
                description: "Security audit passes".to_string(),
            });
        }

        // 3. Performance Criteria
        if lower_desc.contains("fast")
            || lower_desc.contains("performant")
            || lower_desc.contains("latency")
        {
            criteria.push(crate::goal_manifold::predicate::Predicate::Performance {
                metric: "response_time_ms".to_string(),
                threshold: 200.0,
                comparison: crate::types::Comparison::LessThan,
            });
        }

        // 4. Fallback Criterion (Strictness: We never allow an empty goal)
        if criteria.is_empty() {
            criteria.push(crate::goal_manifold::predicate::Predicate::Custom {
                code: "todo!(\"Manual verification\")".to_string(),
                language: crate::goal_manifold::predicate::PredicateLanguage::Rust,
                description: format!("Verify implementation of {}", title),
            });
        }

        Goal::builder()
            .description(format!("{}: {}", title, desc))
            .complexity(ProbabilityDistribution::normal(5.0, 2.0))
            .value_to_root(value)
            .success_criteria(criteria)
            .build()
    }
}
