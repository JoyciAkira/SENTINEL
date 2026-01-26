//! Meta-Learning Engine
//!
//! This module implements Sentinel's self-improving capabilities.
//! It learns from completed projects and uses that knowledge to improve
//! future performance.

pub mod classifier;
pub mod knowledge_base;
pub mod pattern_mining;
pub mod strategy;
pub mod types;

use crate::error::Result;
use crate::goal_manifold::goal::Goal;
use std::sync::Arc;
use uuid::Uuid;
use crate::learning::types::GoalType;

// Re-exports
pub use classifier::DeviationClassifier;
pub use knowledge_base::KnowledgeBase;
pub use pattern_mining::PatternMiningEngine;
pub use strategy::StrategySynthesizer;
pub use types::*;

/// Orchestratore centrale per il Meta-Learning
pub struct LearningEngine {
    miner: PatternMiningEngine,
    knowledge_base: Arc<KnowledgeBase>,
    synthesizer: StrategySynthesizer,
}

impl LearningEngine {
    /// Crea un nuovo LearningEngine
    pub fn new(knowledge_base: Arc<KnowledgeBase>) -> Self {
        let miner = PatternMiningEngine::new();
        let synthesizer = StrategySynthesizer::new(knowledge_base.clone());
        
        Self {
            miner,
            knowledge_base,
            synthesizer,
        }
    }

    /// Apprende da un progetto appena completato
    pub async fn learn_from_completion(&mut self, project: &CompletedProject) -> Result<LearningReport> {
        // 1. Estrazione pattern di successo
        let success_patterns = self.miner.extract_success_patterns(project);
        
        // 2. Estrazione pattern di deviazione
        let deviation_patterns = self.miner.extract_deviation_patterns(project);

        // 3. Memorizzazione nella Knowledge Base
        for pattern in &success_patterns {
            self.knowledge_base.store_pattern(pattern).await?;
        }

        // 4. Generazione report
        let kb_stats = self.knowledge_base.get_statistics().await?;

        Ok(LearningReport {
            project_id: project.id,
            timestamp: crate::types::now(),
            success_patterns_extracted: success_patterns.len(),
            deviation_patterns_extracted: deviation_patterns.len(),
            knowledge_base_size: kb_stats.total_patterns,
            training_examples_added: project.actions.len(),
            cross_patterns_discovered: 0, // Placeholder
            confidence_improvement: 0.05,  // Heuristic
        })
    }

    /// Suggerisce una strategia per un nuovo obiettivo
    pub async fn suggest_strategy(&self, goal: &Goal) -> Result<Strategy> {
        self.synthesizer.suggest_strategy(goal).await
    }

    /// Accesso alla Knowledge Base
    pub fn knowledge_base(&self) -> Arc<KnowledgeBase> {
        self.knowledge_base.clone()
    }
}
