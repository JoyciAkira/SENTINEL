//! Swarm Memory System
//!
//! Multi-layered shared memory: Working, Episodic, Semantic, Procedural

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::{AgentId, AgentOutput, Task};

/// Multi-layer swarm memory
pub struct SwarmMemory {
    /// Working memory (short-term, TTL 1-5 minutes)
    working: Arc<DashMap<String, MemoryEntry>>,

    /// Episodic memory (events and experiences)
    episodic: Arc<DashMap<String, Vec<Episode>>>,

    /// Semantic memory (concepts and knowledge)
    semantic: Arc<DashMap<String, Concept>>,

    /// Procedural memory (patterns and procedures)
    procedural: Arc<DashMap<String, Pattern>>,

    /// Agent outputs
    outputs: Arc<RwLock<HashMap<AgentId, Vec<AgentOutput>>>>,

    /// Task assignments
    tasks: Arc<RwLock<HashMap<AgentId, Task>>>,

    /// Shared context
    context: Arc<RwLock<HashMap<String, String>>>,
}

/// Memory entry with TTL
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub value: Vec<u8>,
    pub written_by: AgentId,
    pub written_at: Instant,
    pub expires_at: Instant,
}

impl MemoryEntry {
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Episode (event in episodic memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub timestamp: u64,
    pub category: String,
    pub description: String,
    pub agents_involved: Vec<AgentId>,
    pub outcome: String,
}

/// Concept (semantic memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub definition: String,
    pub related: Vec<String>,
    pub confidence: f64,
}

/// Pattern (procedural memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub title: String,
    pub description: String,
    pub code_template: Option<String>,
    pub applicable_to: Vec<String>,
    pub success_rate: f64,
    pub usage_count: u32,
}

impl SwarmMemory {
    /// Create new swarm memory with TTL cleanup
    pub fn new() -> Self {
        let working: Arc<DashMap<String, MemoryEntry>> = Arc::new(DashMap::new());
        let working_clone = working.clone();

        // Start TTL cleanup task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let now = Instant::now();
                let expired: Vec<String> = working_clone
                    .iter()
                    .filter(|entry| entry.value().expires_at < now)
                    .map(|entry| entry.key().clone())
                    .collect();

                if !expired.is_empty() {
                    for key in &expired {
                        working_clone.remove(key);
                    }
                    tracing::debug!("Cleaned up {} expired memory entries", expired.len());
                }
            }
        });

        Self {
            working,
            episodic: Arc::new(DashMap::new()),
            semantic: Arc::new(DashMap::new()),
            procedural: Arc::new(DashMap::new()),
            outputs: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Write to working memory with TTL
    pub fn write<T: Serialize>(
        &self,
        key: impl Into<String>,
        value: T,
        ttl: Duration,
        written_by: AgentId,
    ) -> Result<()> {
        let entry = MemoryEntry {
            value: serde_json::to_vec(&value)?,
            written_by,
            written_at: Instant::now(),
            expires_at: Instant::now() + ttl,
        };

        self.working.insert(key.into(), entry);

        Ok(())
    }

    /// Read from memory (checks all layers)
    pub fn read<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        // 1. Check working memory
        if let Some(entry) = self.working.get(key) {
            if !entry.is_expired() {
                return serde_json::from_slice(&entry.value).ok();
            }
        }

        // 2. Check semantic memory
        if let Some(concept) = self.semantic.get(key) {
            return serde_json::from_str(&concept.definition).ok();
        }

        None
    }

    /// Store concept in semantic memory
    pub fn store_concept(&self, concept: Concept) {
        self.semantic.insert(concept.id.clone(), concept);
    }

    /// Get concept from semantic memory
    pub fn get_concept(&self, id: &str) -> Option<Concept> {
        self.semantic.get(id).map(|c| c.clone())
    }

    /// Record episode in episodic memory
    pub fn record_episode(&self, episode: Episode) {
        let key = episode.category.clone();

        self.episodic
            .entry(key)
            .or_default()
            .push(episode);
    }

    /// Get episodes by category
    pub fn get_episodes(&self, category: &str) -> Vec<Episode> {
        self.episodic
            .get(category)
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Store pattern in procedural memory
    pub fn store_pattern(&self, pattern: Pattern) {
        self.procedural.insert(pattern.id.clone(), pattern);
    }

    /// Get pattern from procedural memory
    pub fn get_pattern(&self, id: &str) -> Option<Pattern> {
        self.procedural.get(id).map(|p| p.clone())
    }

    /// Find applicable patterns
    pub fn find_patterns(&self, context: &str) -> Vec<Pattern> {
        self.procedural
            .iter()
            .filter(|entry| {
                let pattern = entry.value();
                pattern
                    .applicable_to
                    .iter()
                    .any(|applicable| context.to_lowercase().contains(&applicable.to_lowercase()))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Store agent output
    pub async fn store_output(&self, agent_id: AgentId, output: AgentOutput) {
        let mut outputs = self.outputs.write().await;
        outputs
            .entry(agent_id)
            .or_default()
            .push(output);
    }

    /// Get outputs from agent
    pub async fn get_outputs(&self, agent_id: AgentId) -> Vec<AgentOutput> {
        self.outputs
            .read()
            .await
            .get(&agent_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Assign task to agent
    pub async fn assign_task(&self, agent_id: AgentId, task: Task) {
        self.tasks.write().await.insert(agent_id, task);
    }

    /// Get task for agent
    pub async fn get_task(&self, agent_id: AgentId) -> Option<Task> {
        self.tasks.read().await.get(&agent_id).cloned()
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> HashMap<AgentId, Task> {
        self.tasks.read().await.clone()
    }

    /// Store context
    pub async fn set_context(&self, key: impl Into<String>, value: impl Into<String>) {
        self.context.write().await.insert(key.into(), value.into());
    }

    /// Get context
    pub async fn get_context(&self, key: &str) -> Option<String> {
        self.context.read().await.get(key).cloned()
    }

    /// Get full context as string (for LLM prompts)
    pub async fn get_context_string(&self, agent_id: AgentId) -> String {
        let mut context_parts = Vec::new();

        // Add task context
        if let Some(task) = self.get_task(agent_id).await {
            context_parts.push(format!("Current task: {}", task.description));
        }

        // Add shared context
        let shared = self.context.read().await;
        for (key, value) in shared.iter() {
            context_parts.push(format!("{}: {}", key, value));
        }

        // Add relevant patterns
        let patterns = self.find_patterns("general");
        if !patterns.is_empty() {
            context_parts.push("Relevant patterns:".to_string());
            for pattern in patterns.iter().take(3) {
                context_parts.push(format!("  - {}: {}", pattern.title, pattern.description));
            }
        }

        context_parts.join("\n")
    }

    /// Adopt pattern (used by agents)
    pub async fn adopt_pattern(&self, agent_id: AgentId, pattern: Pattern) -> Result<()> {
        // Store in working memory for this agent
        let key = format!("agent_{:?}_pattern_{}", agent_id, pattern.id);

        self.write(
            key,
            pattern,
            Duration::from_secs(300), // 5 minute TTL
            agent_id,
        )?;

        Ok(())
    }

    /// Share insight between agents (cross-pollination)
    pub async fn share_insight(
        &self,
        from: AgentId,
        to: Vec<AgentId>,
        insight: impl Into<String>,
    ) -> Result<()> {
        let insight_str = insight.into();

        for agent_id in to {
            let key = format!("insight_from_{:?}_to_{:?}", from, agent_id);

            self.write(key, insight_str.clone(), Duration::from_secs(60), from)?;
        }

        Ok(())
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            working_entries: self.working.len(),
            episodic_categories: self.episodic.len(),
            semantic_concepts: self.semantic.len(),
            procedural_patterns: self.procedural.len(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub working_entries: usize,
    pub episodic_categories: usize,
    pub semantic_concepts: usize,
    pub procedural_patterns: usize,
}

impl Default for SwarmMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_working_memory() {
        let memory = SwarmMemory::new();
        let agent_id = AgentId::deterministic(
            &blake3::hash(b"test"),
            &super::super::emergence::AgentType::APICoder,
            0,
        );

        memory
            .write("test_key", "test_value", Duration::from_secs(60), agent_id)
            .unwrap();

        let value: Option<String> = memory.read("test_key");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_pattern_storage() {
        let memory = SwarmMemory::new();

        let pattern = Pattern {
            id: "jwt_pattern".to_string(),
            title: "JWT Authentication".to_string(),
            description: "Use JWT for stateless auth".to_string(),
            code_template: Some("use jsonwebtoken...".to_string()),
            applicable_to: vec!["auth".to_string(), "security".to_string()],
            success_rate: 0.95,
            usage_count: 42,
        };

        memory.store_pattern(pattern.clone());

        let found = memory.find_patterns("auth");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "jwt_pattern");
    }

    #[tokio::test]
    async fn test_task_assignment() {
        let memory = SwarmMemory::new();
        let agent_id = AgentId::deterministic(
            &blake3::hash(b"test"),
            &super::super::emergence::AgentType::APICoder,
            0,
        );

        let task = Task {
            id: "task_1".to_string(),
            name: "Test task".to_string(),
            description: "Do something".to_string(),
            agent_type: super::super::emergence::AgentType::APICoder,
            dependencies: vec![],
            priority: 0.8,
        };

        memory.assign_task(agent_id, task.clone()).await;

        let retrieved = memory.get_task(agent_id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "task_1");
    }
}
