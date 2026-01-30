//! Strategy Synthesizer - Genera strategie basate sull'esperienza passata
//!
//! Questo modulo analizza i pattern appresi e suggerisce la migliore
//! linea d'azione per raggiungere un nuovo obiettivo.

use crate::error::Result;
use crate::learning::knowledge_base::KnowledgeBase;
use crate::learning::types::*;
use crate::goal_manifold::goal::Goal;
use std::sync::Arc;
use std::time::Duration;

/// Sintetizzatore di strategie basato su meta-learning
#[derive(Debug, Clone)]
pub struct StrategySynthesizer {
    knowledge_base: Arc<KnowledgeBase>,
}

impl StrategySynthesizer {
    /// Crea un nuovo sintetizzatore
    pub fn new(knowledge_base: Arc<KnowledgeBase>) -> Self {
        Self { knowledge_base }
    }

    /// Suggerisce una strategia per un dato obiettivo
    ///
    /// Analizza la Knowledge Base per trovare pattern applicabili,
    /// li classifica e sintetizza una raccomandazione strutturata.
    pub async fn suggest_strategy(&self, goal: &Goal) -> Result<Strategy> {
        // 1. Recupero pattern applicabili
        let applicable_patterns = self.knowledge_base.find_applicable_patterns(goal).await?;

        if applicable_patterns.is_empty() {
            return Ok(self.default_strategy());
        }

        // 2. Selezione dei migliori approcci (Ranking)
        let recommended = self.rank_patterns(applicable_patterns);

        // 3. Identificazione potenziali rischi (basato su similarità)
        // Nota: Qui potremmo integrare DeviationClassifier in futuro
        let pitfalls = vec![]; 

        // 4. Calcolo metriche aggregate
        let confidence = self.calculate_aggregate_confidence(&recommended);
        let estimated_time = self.estimate_completion_time(&recommended);

        // 5. Generazione rationale (Stub per LLM integration)
        let rationale = self.generate_rationale(&recommended, goal);

        Ok(Strategy {
            recommended_approaches: recommended,
            pitfalls_to_avoid: pitfalls,
            estimated_completion_time: estimated_time,
            confidence,
            rationale,
            generated_at: crate::types::now(),
        })
    }

    /// Ordina e filtra i pattern per rilevanza
    fn rank_patterns(&self, mut patterns: Vec<SuccessPattern>) -> Vec<SuccessPattern> {
        // Ordina per (success_rate * log(support)) per bilanciare qualità e frequenza
        patterns.sort_by(|a, b| {
            let score_a = a.success_rate * (a.support as f64).ln().max(1.0);
            let score_b = b.success_rate * (b.support as f64).ln().max(1.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Prendiamo i top 3 pattern per non sovraccaricare la strategia
        patterns.into_iter().take(3).collect()
    }

    /// Calcola la confidenza aggregata della strategia
    fn calculate_aggregate_confidence(&self, patterns: &[SuccessPattern]) -> f64 {
        if patterns.is_empty() {
            return 0.2; // Baseline confidence per task ignoti
        }

        // Media pesata della confidenza dei pattern individuali
        let sum: f64 = patterns.iter().map(|p| p.confidence).sum();
        (sum / patterns.len() as f64).clamp(0.0, 1.0)
    }

    /// Stima il tempo di completamento basandosi sulla complessità dei pattern
    fn estimate_completion_time(&self, patterns: &[SuccessPattern]) -> Duration {
        let base_minutes = 30;
        let pattern_minutes: u64 = patterns.iter()
            .map(|p| p.action_sequence.len() as u64 * 10) // 10 min per azione media
            .sum();
        
        Duration::from_secs((base_minutes + pattern_minutes) * 60)
    }

    /// Genera una spiegazione testuale per la strategia suggerita
    fn generate_rationale(&self, patterns: &[SuccessPattern], goal: &Goal) -> String {
        if patterns.is_empty() {
            return "Nessuna esperienza passata trovata per questo tipo di obiettivo. Si raccomanda un approccio esplorativo.".to_string();
        }

        let mut rationale = format!(
            "Basato sull'analisi di {} pattern di successo simili a '{}':\n",
            patterns.len(),
            goal.description
        );

        for (i, p) in patterns.iter().enumerate() {
            rationale.push_str(&format!(
                "{}. Approccio '{}': consigliato per l'alto tasso di successo ({:.0}%).\n",
                i + 1,
                p.name,
                p.success_rate * 100.0
            ));
        }

        rationale.push_str("\nStrategia consigliata: Seguire la sequenza di azioni collaudata per minimizzare i rischi di deviazione.");
        rationale
    }

    /// Strategia di fallback quando non ci sono dati
    fn default_strategy(&self) -> Strategy {
        Strategy {
            recommended_approaches: vec![],
            pitfalls_to_avoid: vec![],
            estimated_completion_time: Duration::from_secs(3600),
            confidence: 0.2,
            rationale: "Approccio standard: Analisi iniziale seguita da implementazione incrementale.".to_string(),
            generated_at: crate::types::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::types::GoalType;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_suggest_strategy_empty_kb() {
        let kb = Arc::new(KnowledgeBase::new());
        let synthesizer = StrategySynthesizer::new(kb);
        
        let goal = Goal::builder()
            .description("Task ignoto")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .value_to_root(1.0)
            .build()
            .unwrap();

        let strategy = synthesizer.suggest_strategy(&goal).await.unwrap();
        assert_eq!(strategy.confidence, 0.2);
        assert!(strategy.recommended_approaches.is_empty());
    }

    #[tokio::test]
    async fn test_strategy_ranking() {
        let kb = Arc::new(KnowledgeBase::new());
        
        // Mock pattern 1: Alto successo, basso supporto
        let p1 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Pattern A".to_string(),
            description: "A".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::BugFix],
            success_rate: 0.95,
            support: 2,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.9,
            learned_at: crate::types::now(),
        };

        // Mock pattern 2: Medio successo, alto supporto
        let p2 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Pattern B".to_string(),
            description: "B".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::BugFix],
            success_rate: 0.85,
            support: 50,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.8,
            learned_at: crate::types::now(),
        };

        kb.store_pattern(&p1).await.unwrap();
        kb.store_pattern(&p2).await.unwrap();

        let synthesizer = StrategySynthesizer::new(kb);
        let goal = Goal::builder()
            .description("Fix bug critico")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .value_to_root(1.0)
            .build()
            .unwrap();

        let strategy = synthesizer.suggest_strategy(&goal).await.unwrap();
        
        assert!(!strategy.recommended_approaches.is_empty());
        // Il pattern B dovrebbe vincere grazie al supporto molto più alto nonostante il success rate leggermente inferiore
        assert_eq!(strategy.recommended_approaches[0].name, "Pattern B");
    }
}
