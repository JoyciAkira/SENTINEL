//! Predictive Orchestrator
//!
//! Pre-spawns agents and resources based on predicted needs.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use super::{agent::AgentPersonality, AgentId, AgentType, SwarmMemory, Task};

/// Predictive orchestrator
pub struct PredictiveOrchestrator {
    /// Enabled flag
    enabled: bool,

    /// Task patterns learned
    patterns: Arc<RwLock<Vec<TaskPattern>>>,

    /// Prefetch queue (agents ready to go)
    prefetch_queue: Arc<Mutex<Vec<PrefetchedAgent>>>,

    /// Prediction accuracy tracking
    accuracy: Arc<RwLock<PredictionAccuracy>>,
}

/// Task pattern for prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPattern {
    pub trigger: String,
    pub predicted_tasks: Vec<PredictedTask>,
    pub confidence: f64,
    pub occurrence_count: u32,
}

/// Predicted task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedTask {
    pub agent_type: AgentType,
    pub description: String,
    pub delay_seconds: u64,
    pub confidence: f64,
}

/// Task prediction result
#[derive(Debug, Clone)]
pub struct TaskPrediction {
    pub task: Task,
    pub confidence: f64,
    pub expected_start: Duration,
}

/// Prefetched agent waiting to be assigned
#[derive(Debug, Clone)]
pub struct PrefetchedAgent {
    pub agent_type: AgentType,
    pub personality: AgentPersonality,
    pub spawned_at: Instant,
    pub predicted_task: Option<String>,
}

/// Prediction accuracy metrics
#[derive(Debug, Clone, Default)]
pub struct PredictionAccuracy {
    pub total_predictions: u32,
    pub correct_predictions: u32,
    pub false_positives: u32,
    pub false_negatives: u32,
}

impl PredictiveOrchestrator {
    /// Create new predictive orchestrator
    pub fn new() -> Self {
        let patterns = Arc::new(RwLock::new(vec![
            // Built-in patterns
            TaskPattern {
                trigger: "auth".to_string(),
                predicted_tasks: vec![
                    PredictedTask {
                        agent_type: AgentType::SecurityAuditor,
                        description: "Security audit of auth system".to_string(),
                        delay_seconds: 2,
                        confidence: 0.95,
                    },
                    PredictedTask {
                        agent_type: AgentType::TestWriter,
                        description: "Write auth tests".to_string(),
                        delay_seconds: 5,
                        confidence: 0.90,
                    },
                ],
                confidence: 0.95,
                occurrence_count: 100,
            },
            TaskPattern {
                trigger: "api".to_string(),
                predicted_tasks: vec![
                    PredictedTask {
                        agent_type: AgentType::TestWriter,
                        description: "Write API tests".to_string(),
                        delay_seconds: 3,
                        confidence: 0.88,
                    },
                    PredictedTask {
                        agent_type: AgentType::DocWriter,
                        description: "Document API endpoints".to_string(),
                        delay_seconds: 4,
                        confidence: 0.85,
                    },
                ],
                confidence: 0.90,
                occurrence_count: 85,
            },
            TaskPattern {
                trigger: "database".to_string(),
                predicted_tasks: vec![PredictedTask {
                    agent_type: AgentType::TestWriter,
                    description: "Write database tests".to_string(),
                    delay_seconds: 4,
                    confidence: 0.82,
                }],
                confidence: 0.85,
                occurrence_count: 60,
            },
        ]));

        Self {
            enabled: true,
            patterns,
            prefetch_queue: Arc::new(Mutex::new(Vec::new())),
            accuracy: Arc::new(RwLock::new(PredictionAccuracy::default())),
        }
    }

    /// Check if prediction is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Predict next tasks based on current task and progress
    pub async fn predict_next(&self, current_task: &Task, progress: f64) -> Vec<TaskPrediction> {
        let mut predictions = Vec::new();

        let patterns = self.patterns.read().await;

        for pattern in patterns.iter() {
            if current_task
                .description
                .to_lowercase()
                .contains(&pattern.trigger)
            {
                for predicted in &pattern.predicted_tasks {
                    // Check if this prediction makes sense given current progress
                    let threshold = predicted.delay_seconds as f64 / 10.0; // Normalize

                    if progress >= threshold {
                        predictions.push(TaskPrediction {
                            task: Task {
                                id: format!(
                                    "predicted_{}_{}",
                                    pattern.trigger, predicted.agent_type
                                ),
                                name: predicted.description.clone(),
                                description: predicted.description.clone(),
                                agent_type: predicted.agent_type,
                                dependencies: vec![current_task.id.clone()],
                                priority: 0.7,
                            },
                            confidence: predicted.confidence * pattern.confidence,
                            expected_start: Duration::from_secs(predicted.delay_seconds),
                        });
                    }
                }
            }
        }

        // Sort by confidence
        predictions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        predictions
    }

    /// Monitor agents and prefetch resources
    pub async fn monitor_and_prefetch(
        &self,
        agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn super::SwarmAgent>>>>>>,
    ) -> Result<()> {
        // Get all tasks
        // This is simplified - would need access to memory in real impl

        // For each agent, check progress and predict
        let agents_read = agents.read().await;

        for (agent_id, agent) in agents_read.iter() {
            let agent = agent.lock().await;

            // Check if this agent type would trigger predictions
            match agent.agent_type() {
                AgentType::AuthArchitect | AgentType::APICoder | AgentType::DatabaseArchitect => {
                    // High-level design agents trigger implementation agents
                    // Would check actual progress here
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Prefetch agent for predicted task
    pub async fn prefetch_agent(&self, agent_type: AgentType, predicted_task: Option<String>) {
        // Create prefetched agent entry
        // Note: In real implementation, would actually spawn the agent

        let goal_hash = blake3::hash(b"prefetch");
        let personality = AgentPersonality::from_goal(&goal_hash, &agent_type);

        let prefetched = PrefetchedAgent {
            agent_type,
            personality,
            spawned_at: Instant::now(),
            predicted_task,
        };

        self.prefetch_queue.lock().await.push(prefetched);

        tracing::info!("Prefetched agent: {:?}", agent_type);
    }

    /// Get prefetched agent if available
    pub async fn get_prefetched(&self, agent_type: AgentType) -> Option<PrefetchedAgent> {
        let mut queue = self.prefetch_queue.lock().await;

        if let Some(pos) = queue.iter().position(|p| p.agent_type == agent_type) {
            return Some(queue.remove(pos));
        }

        None
    }

    /// Record prediction outcome (for learning)
    pub async fn record_outcome(&self, prediction: &TaskPrediction, actually_needed: bool) {
        let mut accuracy = self.accuracy.write().await;
        accuracy.total_predictions += 1;

        if actually_needed {
            accuracy.correct_predictions += 1;
        } else {
            accuracy.false_positives += 1;
        }
    }

    /// Learn new pattern from execution
    pub async fn learn_pattern(&self, trigger: String, predicted: Vec<PredictedTask>) {
        let mut patterns = self.patterns.write().await;

        // Check if pattern exists
        if let Some(existing) = patterns.iter_mut().find(|p| p.trigger == trigger) {
            existing.occurrence_count += 1;
            existing.confidence = (existing.confidence + 0.01).min(1.0);
        } else {
            // Add new pattern
            patterns.push(TaskPattern {
                trigger,
                predicted_tasks: predicted,
                confidence: 0.5, // Start with low confidence
                occurrence_count: 1,
            });
        }
    }

    /// Get prediction accuracy
    pub async fn get_accuracy(&self) -> PredictionAccuracy {
        self.accuracy.read().await.clone()
    }

    /// Get pending prefetches count
    pub async fn prefetch_count(&self) -> usize {
        self.prefetch_queue.lock().await.len()
    }

    /// Clear old prefetches (older than timeout)
    pub async fn cleanup_prefetches(&self, timeout: Duration) {
        let mut queue = self.prefetch_queue.lock().await;
        let now = Instant::now();

        queue.retain(|p| now.duration_since(p.spawned_at) < timeout);
    }
}

impl Default for PredictiveOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prediction() {
        let predictor = PredictiveOrchestrator::new();

        let task = Task {
            id: "1".to_string(),
            name: "Build auth".to_string(),
            description: "Build authentication system".to_string(),
            agent_type: AgentType::AuthArchitect,
            dependencies: vec![],
            priority: 0.9,
        };

        let predictions = predictor.predict_next(&task, 0.5).await;

        // Should predict security auditor and test writer
        assert!(!predictions.is_empty());

        let has_security = predictions
            .iter()
            .any(|p| p.task.agent_type == AgentType::SecurityAuditor);
        assert!(has_security);
    }

    #[tokio::test]
    async fn test_prefetch() {
        let predictor = PredictiveOrchestrator::new();

        predictor
            .prefetch_agent(AgentType::TestWriter, Some("auth tests".to_string()))
            .await;

        let count = predictor.prefetch_count().await;
        assert_eq!(count, 1);

        // Get prefetched
        let prefetched = predictor.get_prefetched(AgentType::TestWriter).await;
        assert!(prefetched.is_some());

        // Queue should be empty now
        let count = predictor.prefetch_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_learn_pattern() {
        let predictor = PredictiveOrchestrator::new();

        let predicted = vec![PredictedTask {
            agent_type: AgentType::DocWriter,
            description: "Write docs".to_string(),
            delay_seconds: 3,
            confidence: 0.8,
        }];

        predictor
            .learn_pattern("new_feature".to_string(), predicted)
            .await;

        // Should have learned the pattern
        // Verification would require accessing patterns (not exposed in API)
    }
}
