//! Knowledge Base - Implementazione in-memory
//!
//! Versione semplificata che usa HashMap invece di Neo4j.
//! Questo permette di procedere rapidamente con l'implementazione
//! senza blocarsi su problemi di compatibilità neo4rs.

use crate::error::Result;
use crate::learning::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Struttura per la persistenza della Knowledge Base
#[derive(Serialize, Deserialize)]
struct KnowledgeBaseData {
    patterns: HashMap<Uuid, SuccessPattern>,
    pattern_relations: Vec<((Uuid, Uuid), f64)>,
}

/// Knowledge Base - Implementazione con persistenza
///
/// Gestisce lo storage e il retrieval di pattern appresi.
/// Supporta il salvataggio automatico su disco in formato JSON.
#[derive(Debug)]
pub struct KnowledgeBase {
    patterns: Arc<RwLock<HashMap<Uuid, SuccessPattern>>>,
    pattern_by_goal_type: Arc<RwLock<HashMap<GoalType, Vec<Uuid>>>>,
    pattern_relations: Arc<RwLock<HashMap<(Uuid, Uuid), f64>>>,
    storage_path: Option<PathBuf>,
}

impl KnowledgeBase {
    /// Crea una nuova istanza di Knowledge Base (solo memoria)
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            pattern_by_goal_type: Arc::new(RwLock::new(HashMap::new())),
            pattern_relations: Arc::new(RwLock::new(HashMap::new())),
            storage_path: None,
        }
    }

    /// Crea una nuova istanza con persistenza su disco
    pub async fn with_storage(path: impl AsRef<Path>) -> Result<Self> {
        let mut kb = Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            pattern_by_goal_type: Arc::new(RwLock::new(HashMap::new())),
            pattern_relations: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(path.as_ref().to_path_buf()),
        };

        if path.as_ref().exists() {
            kb.load_from_disk().await?;
        }

        Ok(kb)
    }

    /// Carica i dati dal disco
    async fn load_from_disk(&mut self) -> Result<()> {
        let path = self.storage_path.as_ref().unwrap();
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(crate::error::SentinelError::Io)?;

        let data: KnowledgeBaseData =
            serde_json::from_str(&content).map_err(crate::error::SentinelError::Serialization)?;

        // Popola pattern e indici
        {
            let mut patterns = self.patterns.write().await;
            let mut by_type = self.pattern_by_goal_type.write().await;

            for (id, pattern) in data.patterns {
                for goal_type in &pattern.applicable_to_goal_types {
                    by_type.entry(goal_type.clone()).or_default().push(id);
                }
                patterns.insert(id, pattern);
            }
        }

        // Popola relazioni
        {
            let mut relations = self.pattern_relations.write().await;
            for (pair, strength) in data.pattern_relations {
                relations.insert(pair, strength);
            }
        }

        Ok(())
    }

    /// Salva i dati su disco
    async fn save_to_disk(&self) -> Result<()> {
        if let Some(path) = &self.storage_path {
            let data = {
                let patterns = self.patterns.read().await;
                let relations = self.pattern_relations.read().await;

                KnowledgeBaseData {
                    patterns: patterns.clone(),
                    pattern_relations: relations.iter().map(|(k, v)| (*k, *v)).collect(),
                }
            };

            let content = serde_json::to_string_pretty(&data)
                .map_err(crate::error::SentinelError::Serialization)?;

            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            tokio::fs::write(path, content)
                .await
                .map_err(crate::error::SentinelError::Io)?;
        }
        Ok(())
    }

    /// Inizializza lo schema del database (no-op per questa versione)
    pub async fn initialize_schema(&self) -> Result<()> {
        Ok(())
    }

    /// Salva un pattern di successo nel knowledge base
    pub async fn store_pattern(&self, pattern: &SuccessPattern) -> Result<()> {
        {
            let mut patterns = self.patterns.write().await;

            // Verifica se il pattern esiste già
            if let Some(existing) = patterns.get(&pattern.id) {
                let new_success_rate = existing.success_rate * 0.7 + pattern.success_rate * 0.3;
                let new_support = existing.support + 1;

                let updated = SuccessPattern {
                    success_rate: new_success_rate,
                    support: new_support,
                    ..existing.clone()
                };

                patterns.insert(pattern.id, updated);
            } else {
                patterns.insert(pattern.id, pattern.clone());

                // Aggiorna indici per goal type solo per nuovi pattern
                let mut by_type = self.pattern_by_goal_type.write().await;
                for goal_type in &pattern.applicable_to_goal_types {
                    by_type
                        .entry(goal_type.clone())
                        .or_default()
                        .push(pattern.id);
                }
            }
        }

        // Persistenza automatica
        self.save_to_disk().await?;

        Ok(())
    }

    /// Recupera pattern applicabili a un goal specifico
    ///
    /// Restituisce tutti i pattern che sono stati classificati come
    /// applicabili al tipo di goal specificato, ordinati per success rate.
    pub async fn find_applicable_patterns(
        &self,
        goal: &crate::goal_manifold::goal::Goal,
    ) -> Result<Vec<SuccessPattern>> {
        let goal_type = self.classify_goal(goal);
        let by_type = self.pattern_by_goal_type.read().await;

        if let Some(pattern_ids) = (*by_type).get(&goal_type) {
            let patterns = self.patterns.read().await;

            let mut applicable: Vec<SuccessPattern> = pattern_ids
                .iter()
                .filter_map(|id| patterns.get(id).cloned())
                .collect();

            // Ordina per success rate
            applicable.sort_by(|a, b| {
                b.success_rate
                    .partial_cmp(&a.success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            Ok(applicable)
        } else {
            Ok(Vec::new())
        }
    }

    /// Trova pattern simili a un pattern dato
    ///
    /// Usa le relazioni tra pattern per trovare quelli correlati.
    pub async fn find_similar_patterns(
        &self,
        pattern: &SuccessPattern,
        limit: usize,
    ) -> Result<Vec<(SuccessPattern, f64)>> {
        let relations = self.pattern_relations.read().await;
        let patterns = self.patterns.read().await;

        let mut similar: Vec<(SuccessPattern, f64)> = Vec::new();

        // Cerca pattern con relazioni al pattern dato
        for ((id1, id2), strength) in relations.iter() {
            if id1 == &pattern.id {
                if let Some(similar_pattern) = patterns.get(id2).cloned() {
                    similar.push((similar_pattern, *strength));
                }
            } else if id2 == &pattern.id {
                if let Some(similar_pattern) = patterns.get(id1).cloned() {
                    similar.push((similar_pattern, *strength));
                }
            }
        }

        // Ordina per strength
        similar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limita al numero richiesto
        similar.truncate(limit);

        Ok(similar)
    }

    /// Registra un progetto completato nel knowledge base
    pub async fn record_project_completion(
        &self,
        project_id: Uuid,
        root_goal: &crate::goal_manifold::goal::Goal,
        patterns_used: &[Uuid],
        _final_alignment: f64, // Prefisso con underscore per disabilitare warning
    ) -> Result<()> {
        // In-memory implementation: non salviamo i progetti per ora
        // In futuro, potremmo salvare statistiche aggregate
        Ok(())
    }

    /// Aggiunge una relazione tra due pattern
    ///
    /// Indica che i due pattern sono correlati o spesso usati insieme.
    pub async fn add_pattern_relation(
        &self,
        pattern_id_1: Uuid,
        pattern_id_2: Uuid,
        strength: f64,
    ) -> Result<()> {
        let mut relations = self.pattern_relations.write().await;

        // Aggiorna relazione (media pesata)
        let new_strength = if let Some(&existing) = relations.get(&(pattern_id_1, pattern_id_2)) {
            existing * 0.7 + strength * 0.3
        } else {
            strength
        };

        relations.insert((pattern_id_1, pattern_id_2), new_strength);

        Ok(())
    }

    /// Ottiene statistiche sul knowledge base
    pub async fn get_statistics(&self) -> Result<KnowledgeBaseStats> {
        let patterns = self.patterns.read().await;
        Ok(KnowledgeBaseStats {
            total_patterns: patterns.len(),
        })
    }

    /// Classifica un goal per tipo
    fn classify_goal(&self, goal: &crate::goal_manifold::goal::Goal) -> GoalType {
        let description = goal.description.to_lowercase();

        if description.contains("bug") || description.contains("fix") {
            GoalType::BugFix
        } else if description.contains("test") {
            GoalType::Testing
        } else if description.contains("refactor") {
            GoalType::Refactoring
        } else if description.contains("auth") {
            GoalType::Authentication
        } else if description.contains("api") {
            GoalType::Api
        } else if description.contains("database") || description.contains("db") {
            GoalType::Database
        } else if description.contains("performance") || description.contains("optimize") {
            GoalType::PerformanceOptimization
        } else if description.contains("security") {
            GoalType::Security
        } else if description.contains("payment") || description.contains("billing") {
            GoalType::Payment
        } else if description.contains("ui") || description.contains("interface") {
            GoalType::Ui
        } else if description.contains("infrastructure") || description.contains("deploy") {
            GoalType::Infrastructure
        } else if description.contains("document") {
            GoalType::Documentation
        } else {
            GoalType::FeatureImplementation
        }
    }
}

/// Statistiche del Knowledge Base
#[derive(Debug, Clone)]
pub struct KnowledgeBaseStats {
    pub total_patterns: usize,
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_knowledge_base_creation() {
        let kb = KnowledgeBase::new();
        assert_eq!(kb.get_statistics().await.unwrap().total_patterns, 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_pattern() {
        let kb = KnowledgeBase::new();
        kb.initialize_schema().await.unwrap();

        let pattern = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Test Pattern".to_string(),
            description: "Test pattern for unit tests".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::BugFix],
            success_rate: 0.9,
            support: 10,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.8,
            learned_at: crate::types::now(),
        };

        kb.store_pattern(&pattern).await.unwrap();

        let stats = kb.get_statistics().await.unwrap();
        assert_eq!(stats.total_patterns, 1);
    }

    #[tokio::test]
    async fn test_find_applicable_patterns() {
        let kb = KnowledgeBase::new();
        kb.initialize_schema().await.unwrap();

        let pattern1 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "BugFix Pattern".to_string(),
            description: "Pattern for bug fixes".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::BugFix],
            success_rate: 0.9,
            support: 10,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.8,
            learned_at: crate::types::now(),
        };

        let pattern2 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Feature Pattern".to_string(),
            description: "Pattern for features".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::FeatureImplementation],
            success_rate: 0.8,
            support: 5,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.7,
            learned_at: crate::types::now(),
        };

        kb.store_pattern(&pattern1).await.unwrap();
        kb.store_pattern(&pattern2).await.unwrap();

        let test_goal = crate::goal_manifold::goal::Goal::builder()
            .description("Fix authentication bug")
            .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
            .value_to_root(1.0)
            .build()
            .unwrap();

        let applicable = kb.find_applicable_patterns(&test_goal).await.unwrap();
        assert_eq!(applicable.len(), 1);
        assert_eq!(applicable[0].name, "BugFix Pattern");
    }

    #[tokio::test]
    async fn test_pattern_relationships() {
        let kb = KnowledgeBase::new();
        kb.initialize_schema().await.unwrap();

        let pattern1 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Pattern 1".to_string(),
            description: "First test pattern".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::FeatureImplementation],
            success_rate: 0.8,
            support: 5,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.7,
            learned_at: crate::types::now(),
        };

        let pattern2 = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Pattern 2".to_string(),
            description: "Second test pattern".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::FeatureImplementation],
            success_rate: 0.7,
            support: 3,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.6,
            learned_at: crate::types::now(),
        };

        kb.store_pattern(&pattern1).await.unwrap();
        kb.store_pattern(&pattern2).await.unwrap();

        kb.add_pattern_relation(pattern1.id, pattern2.id, 0.8)
            .await
            .unwrap();

        let similar = kb.find_similar_patterns(&pattern1, 5).await.unwrap();
        assert!(!similar.is_empty(), "Should find similar patterns");
    }

    #[tokio::test]
    async fn test_knowledge_base_persistence() {
        let temp_dir = std::env::temp_dir();
        let kb_path = temp_dir.join(format!("kb_test_{}.json", Uuid::new_v4()));

        let pattern = SuccessPattern {
            id: Uuid::new_v4(),
            name: "Persistent Pattern".to_string(),
            description: "Should be saved".to_string(),
            action_sequence: vec![],
            applicable_to_goal_types: vec![GoalType::BugFix],
            success_rate: 0.9,
            support: 1,
            preconditions: vec![],
            expected_outcomes: vec![],
            confidence: 0.8,
            learned_at: crate::types::now(),
        };

        // 1. Salva pattern in una KB con storage
        {
            let kb = KnowledgeBase::with_storage(&kb_path).await.unwrap();
            kb.store_pattern(&pattern).await.unwrap();
            // kb dropped qui, save_to_disk chiamato da store_pattern
        }

        // 2. Ricarica da disco in una nuova istanza
        {
            let kb = KnowledgeBase::with_storage(&kb_path).await.unwrap();
            let stats = kb.get_statistics().await.unwrap();
            assert_eq!(stats.total_patterns, 1);

            let test_goal = crate::goal_manifold::goal::Goal::builder()
                .description("Fix bug")
                .add_success_criterion(crate::goal_manifold::predicate::Predicate::AlwaysTrue)
                .value_to_root(1.0)
                .build()
                .unwrap();

            let applicable = kb.find_applicable_patterns(&test_goal).await.unwrap();
            assert_eq!(applicable[0].name, "Persistent Pattern");
        }

        // Pulizia
        std::fs::remove_file(kb_path).ok();
    }
}
