//! Swarm Load Balancer
//!
//! Auto-balancing and auto-healing for swarm agents.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

use super::{
    agent::{AgentId as AgentIdType, AgentPersonality, ConcreteAgent, SwarmAgent},
    AgentId, AgentType,
};

/// Load balancer for swarm
pub struct SwarmBalancer {
    /// Health status of each agent
    health: Arc<RwLock<HashMap<AgentId, AgentHealth>>>,

    /// Rebalance strategies
    strategies: Vec<RebalanceStrategy>,

    /// Statistics
    stats: Arc<RwLock<BalancerStats>>,
}

/// Agent health status
#[derive(Debug, Clone)]
pub struct AgentHealth {
    pub agent_id: AgentId,
    pub status: HealthStatus,
    pub last_heartbeat: Instant,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
    pub avg_response_time_ms: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// Health status variants
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Slow { tasks_per_minute: f64 },
    Stuck { timeout_secs: u64 },
    Overloaded,
    Failed { error: String },
}

/// Rebalance strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebalanceStrategy {
    SpawnHelper,
    Redistribute,
    Replace,
    Quarantine,
}

/// Balancer statistics
#[derive(Debug, Clone, Default)]
pub struct BalancerStats {
    pub health_checks: u32,
    pub rebalances: u32,
    pub agent_replacements: u32,
    pub quarantines: u32,
}

impl SwarmBalancer {
    /// Create new load balancer
    pub fn new() -> Self {
        Self {
            health: Arc::new(RwLock::new(HashMap::new())),
            strategies: vec![
                RebalanceStrategy::SpawnHelper,
                RebalanceStrategy::Redistribute,
                RebalanceStrategy::Replace,
            ],
            stats: Arc::new(RwLock::new(BalancerStats::default())),
        }
    }

    /// Register agent health check
    pub async fn register(&self, agent_id: AgentId) {
        let health = AgentHealth {
            agent_id,
            status: HealthStatus::Healthy,
            last_heartbeat: Instant::now(),
            tasks_completed: 0,
            tasks_failed: 0,
            avg_response_time_ms: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
        };

        self.health.write().await.insert(agent_id, health);
    }

    /// Update agent heartbeat
    pub async fn heartbeat(&self, agent_id: AgentId) -> Result<()> {
        let mut health_map = self.health.write().await;

        if let Some(health) = health_map.get_mut(&agent_id) {
            health.last_heartbeat = Instant::now();

            // Check if recovered from stuck/slow
            if matches!(
                health.status,
                HealthStatus::Stuck { .. } | HealthStatus::Slow { .. }
            ) {
                health.status = HealthStatus::Healthy;
            }
        }

        Ok(())
    }

    /// Report task completion
    pub async fn task_completed(&self, agent_id: AgentId, success: bool, duration_ms: u64) {
        let mut health_map = self.health.write().await;

        if let Some(health) = health_map.get_mut(&agent_id) {
            if success {
                health.tasks_completed += 1;
            } else {
                health.tasks_failed += 1;
            }

            // Update average response time
            let alpha = 0.3; // Smoothing factor
            health.avg_response_time_ms =
                alpha * duration_ms as f64 + (1.0 - alpha) * health.avg_response_time_ms;

            // Check for slow performance
            if health.avg_response_time_ms > 5000.0 {
                // 5 seconds
                health.status = HealthStatus::Slow {
                    tasks_per_minute: 60.0 / (health.avg_response_time_ms / 1000.0),
                };
            }
        }
    }

    /// Check all agents and rebalance if needed
    pub async fn check_and_rebalance(
        &self,
        agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,
    ) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.health_checks += 1;
        drop(stats);

        let health_map = self.health.read().await;
        let agent_ids: Vec<_> = health_map.keys().cloned().collect();
        drop(health_map);

        for agent_id in agent_ids {
            let health = {
                let map = self.health.read().await;
                map.get(&agent_id).cloned()
            };

            if let Some(health) = health {
                match health.status {
                    HealthStatus::Slow { .. } => {
                        self.handle_slow_agent(agent_id, agents.clone()).await?;
                    }
                    HealthStatus::Stuck { timeout_secs } if timeout_secs > 30 => {
                        self.handle_stuck_agent(agent_id, agents.clone()).await?;
                    }
                    HealthStatus::Failed { .. } => {
                        self.handle_failed_agent(agent_id, agents.clone()).await?;
                    }
                    _ => {
                        // Check heartbeat timeout
                        let elapsed = Instant::now().duration_since(health.last_heartbeat);
                        if elapsed > Duration::from_secs(60) {
                            // Mark as stuck
                            let mut map = self.health.write().await;
                            if let Some(h) = map.get_mut(&agent_id) {
                                h.status = HealthStatus::Stuck {
                                    timeout_secs: elapsed.as_secs(),
                                };
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle slow agent - spawn helper
    async fn handle_slow_agent(
        &self,
        agent_id: AgentId,
        _agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,
    ) -> Result<()> {
        tracing::info!("Agent {:?} is slow, spawning helper", agent_id);

        // In real implementation, would:
        // 1. Get agent type
        // 2. Spawn helper agent of same type
        // 3. Redistribute workload

        let mut stats = self.stats.write().await;
        stats.rebalances += 1;

        Ok(())
    }

    /// Handle stuck agent - replace
    async fn handle_stuck_agent(
        &self,
        agent_id: AgentId,
        agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,
    ) -> Result<()> {
        tracing::warn!("Agent {:?} is stuck, replacing", agent_id);

        // Remove stuck agent
        {
            let mut agents_map = agents.write().await;
            agents_map.remove(&agent_id);
        }

        // Remove from health map
        self.health.write().await.remove(&agent_id);

        // In real implementation, would respawn fresh agent

        let mut stats = self.stats.write().await;
        stats.agent_replacements += 1;

        Ok(())
    }

    /// Handle failed agent - quarantine and replace
    async fn handle_failed_agent(
        &self,
        agent_id: AgentId,
        agents: Arc<RwLock<HashMap<AgentId, Arc<Mutex<Box<dyn SwarmAgent>>>>>>,
    ) -> Result<()> {
        tracing::error!("Agent {:?} failed, quarantining and replacing", agent_id);

        // Quarantine
        {
            let mut map = self.health.write().await;
            if let Some(health) = map.get_mut(&agent_id) {
                health.status = HealthStatus::Stuck { timeout_secs: 9999 };
            }
        }

        // Remove from active agents
        {
            let mut agents_map = agents.write().await;
            agents_map.remove(&agent_id);
        }

        let mut stats = self.stats.write().await;
        stats.quarantines += 1;
        stats.agent_replacements += 1;

        Ok(())
    }

    /// Redistribute workload from one agent to others
    pub async fn redistribute_workload(&self, from: AgentId, _to: Vec<AgentId>) -> Result<()> {
        tracing::info!("Redistributing workload from agent {:?}", from);

        // In real implementation, would move pending tasks

        let mut stats = self.stats.write().await;
        stats.rebalances += 1;

        Ok(())
    }

    /// Get health status of all agents
    pub async fn get_all_health(&self) -> HashMap<AgentId, AgentHealth> {
        self.health.read().await.clone()
    }

    /// Get health of specific agent
    pub async fn get_health(&self, agent_id: AgentId) -> Option<AgentHealth> {
        self.health.read().await.get(&agent_id).cloned()
    }

    /// Get balancer statistics
    pub async fn get_stats(&self) -> BalancerStats {
        self.stats.read().await.clone()
    }

    /// Get agent count by status
    pub async fn count_by_status(&self) -> HashMap<String, usize> {
        let health_map = self.health.read().await;
        let mut counts = HashMap::new();

        for health in health_map.values() {
            let status_str = match health.status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Slow { .. } => "slow",
                HealthStatus::Stuck { .. } => "stuck",
                HealthStatus::Overloaded => "overloaded",
                HealthStatus::Failed { .. } => "failed",
            };

            *counts.entry(status_str.to_string()).or_insert(0) += 1;
        }

        counts
    }
}

impl Default for SwarmBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_heartbeat() {
        let balancer = SwarmBalancer::new();
        let agent_id = AgentId::deterministic(&blake3::hash(b"test"), &AgentType::APICoder, 0);

        balancer.register(agent_id).await;

        let health = balancer.get_health(agent_id).await;
        assert!(health.is_some());
        assert!(matches!(health.unwrap().status, HealthStatus::Healthy));

        // Update heartbeat
        balancer.heartbeat(agent_id).await.unwrap();

        let health = balancer.get_health(agent_id).await.unwrap();
        let elapsed = Instant::now().duration_since(health.last_heartbeat);
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_task_completion_tracking() {
        let balancer = SwarmBalancer::new();
        let agent_id = AgentId::deterministic(&blake3::hash(b"test"), &AgentType::APICoder, 0);

        balancer.register(agent_id).await;

        // Complete some tasks
        for _ in 0..5 {
            balancer.task_completed(agent_id, true, 1000).await;
        }

        // Fail one
        balancer.task_completed(agent_id, false, 500).await;

        let health = balancer.get_health(agent_id).await.unwrap();
        assert_eq!(health.tasks_completed, 5);
        assert_eq!(health.tasks_failed, 1);
    }

    #[tokio::test]
    async fn test_slow_detection() {
        let balancer = SwarmBalancer::new();
        let agent_id = AgentId::deterministic(&blake3::hash(b"test"), &AgentType::APICoder, 0);

        balancer.register(agent_id).await;

        // Complete tasks with high latency
        // Using exponential moving average (alpha=0.3), need ~6 tasks at 6000ms to exceed 5000ms threshold
        for _ in 0..6 {
            balancer.task_completed(agent_id, true, 6000).await; // 6 seconds
        }

        let health = balancer.get_health(agent_id).await.unwrap();
        assert!(matches!(health.status, HealthStatus::Slow { .. }));
    }
}
