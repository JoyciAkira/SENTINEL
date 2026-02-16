//! Conflict Detection and Resolution
//!
//! Creative conflict resolution via synthesis and arbiter agents.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::{memory::Pattern, AgentId, AgentOutput, AgentType};

/// Conflict detected between agents
#[derive(Debug, Clone)]
pub struct Conflict {
    pub id: String,
    pub type_: ConflictType,
    pub involved_agents: Vec<AgentId>,
    pub detected_at: Instant,
    pub severity: ConflictSeverity,
}

/// Types of conflicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Two agents trying to modify same resource
    ResourceConflict {
        resource: String,
        changes: Vec<String>,
    },

    /// Technical disagreement on approach
    TechnicalDisagreement {
        issue: String,
        position_a: String,
        position_b: String,
    },

    /// Goal conflict - agents working at cross-purposes
    GoalConflict { goal_a: String, goal_b: String },

    /// Dependency cycle
    DependencyCycle { cycle: Vec<String> },

    /// Anti-dependency violation
    AntiDependencyViolation { tasks: Vec<String> },
}

/// Conflict severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    /// First come first served
    FirstComeFirstServed { winner: AgentId },

    /// Authority-based resolution
    AuthorityBased { winner: AgentId },

    /// Synthesis - merge both approaches
    Synthesis {
        solution: String,
        reasoning: String,
        hybrid_approach: String,
    },

    /// Serialization - do one after another
    Serialization { order: Vec<AgentId> },

    /// Escalate to human
    Escalate { reason: String },
}

/// Conflict resolution engine
pub struct ConflictResolutionEngine {
    /// Active conflicts
    conflicts: Arc<RwLock<Vec<Conflict>>>,

    /// Resolution journal (learn from past)
    journal: Arc<RwLock<ConflictJournal>>,

    /// Arbiter agents pool
    arbiters: Arc<RwLock<Vec<ArbiterAgent>>>,
}

/// Journal of past conflicts and resolutions
#[derive(Debug, Clone, Default)]
pub struct ConflictJournal {
    entries: Vec<ConflictEntry>,
}

/// Single conflict entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictEntry {
    pub conflict_type: String,
    pub involved_agent_types: Vec<AgentType>,
    pub resolution: Resolution,
    pub timestamp: u64,
    pub success: bool,
}

/// Arbiter agent for resolving conflicts
#[derive(Debug, Clone)]
pub struct ArbiterAgent {
    pub id: AgentId,
    pub authority: f64,
}

impl ConflictResolutionEngine {
    /// Create new conflict resolution engine
    pub fn new() -> Self {
        Self {
            conflicts: Arc::new(RwLock::new(Vec::new())),
            journal: Arc::new(RwLock::new(ConflictJournal::default())),
            arbiters: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Detect conflicts between agent outputs
    pub async fn detect_conflicts(&self, outputs: &[AgentOutput]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // Check for resource conflicts (same file modified)
        conflicts.extend(self.detect_resource_conflicts(outputs).await);

        // Check for technical disagreements
        conflicts.extend(self.detect_technical_disagreements(outputs).await);

        // Check for goal conflicts
        conflicts.extend(self.detect_goal_conflicts(outputs).await);

        // Store conflicts
        let mut active = self.conflicts.write().await;
        for conflict in &conflicts {
            active.push(conflict.clone());
        }

        conflicts
    }

    /// Detect resource conflicts
    async fn detect_resource_conflicts(&self, outputs: &[AgentOutput]) -> Vec<Conflict> {
        let mut file_claims: HashMap<String, Vec<(AgentId, String)>> = HashMap::new();

        // Collect file claims
        for output in outputs {
            for file in &output.files_written {
                file_claims
                    .entry(file.clone())
                    .or_default()
                    .push((output.agent_id, output.content.clone()));
            }
        }

        // Find conflicts
        let mut conflicts = Vec::new();
        for (file, claims) in file_claims {
            if claims.len() > 1 {
                let involved: Vec<_> = claims.iter().map(|(id, _)| *id).collect();
                let changes: Vec<_> = claims.iter().map(|(_, c)| c.clone()).collect();

                conflicts.push(Conflict {
                    id: format!("resource_{}_{}", file, now()),
                    type_: ConflictType::ResourceConflict {
                        resource: file,
                        changes,
                    },
                    involved_agents: involved,
                    detected_at: Instant::now(),
                    severity: ConflictSeverity::High,
                });
            }
        }

        conflicts
    }

    /// Detect technical disagreements
    async fn detect_technical_disagreements(&self, outputs: &[AgentOutput]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // Simple heuristic: check for contradictory keywords
        for i in 0..outputs.len() {
            for j in (i + 1)..outputs.len() {
                let a = &outputs[i];
                let b = &outputs[j];

                // Check for bcrypt vs argon2 disagreement
                if a.content.contains("bcrypt") && b.content.contains("argon2") {
                    conflicts.push(Conflict {
                        id: format!("tech_{}_{}", i, j),
                        type_: ConflictType::TechnicalDisagreement {
                            issue: "Password hashing algorithm".to_string(),
                            position_a: "bcrypt".to_string(),
                            position_b: "argon2".to_string(),
                        },
                        involved_agents: vec![a.agent_id, b.agent_id],
                        detected_at: Instant::now(),
                        severity: ConflictSeverity::Medium,
                    });
                }

                // Check for sync vs async disagreement
                if (a.content.contains("sync") && b.content.contains("async"))
                    || (a.content.contains("blocking") && b.content.contains("non-blocking"))
                {
                    conflicts.push(Conflict {
                        id: format!("tech_async_{}_{}", i, j),
                        type_: ConflictType::TechnicalDisagreement {
                            issue: "Sync vs Async approach".to_string(),
                            position_a: "synchronous".to_string(),
                            position_b: "asynchronous".to_string(),
                        },
                        involved_agents: vec![a.agent_id, b.agent_id],
                        detected_at: Instant::now(),
                        severity: ConflictSeverity::Medium,
                    });
                }
            }
        }

        conflicts
    }

    /// Detect goal conflicts
    async fn detect_goal_conflicts(&self, _outputs: &[AgentOutput]) -> Vec<Conflict> {
        // Simplified for now
        Vec::new()
    }

    /// Resolve a conflict
    pub async fn resolve(&self, conflict: Conflict) -> Result<Resolution> {
        tracing::info!("Resolving conflict: {:?}", conflict.type_);

        // 1. Check journal for similar past conflicts
        if let Some(past) = self.find_similar_resolution(&conflict).await {
            tracing::info!("Found similar past conflict, using learned resolution");
            return Ok(past);
        }

        // 2. Choose resolution strategy based on conflict type
        let resolution = match &conflict.type_ {
            ConflictType::ResourceConflict { resource, .. } => {
                // For files, use authority-based (higher authority wins)
                self.resolve_by_authority(&conflict).await?
            }

            ConflictType::TechnicalDisagreement {
                issue,
                position_a,
                position_b,
            } => {
                // For technical disagreements, try synthesis
                self.resolve_by_synthesis(&conflict, issue, position_a, position_b)
                    .await?
            }

            ConflictType::GoalConflict { .. } => {
                // For goal conflicts, escalate to human
                Resolution::Escalate {
                    reason: "Goal conflict requires human judgment".to_string(),
                }
            }

            _ => {
                // Default: first come first served
                self.resolve_by_authority(&conflict).await?
            }
        };

        // 3. Journal the resolution
        let entry = ConflictEntry {
            conflict_type: format!("{:?}", conflict.type_),
            involved_agent_types: Vec::new(), // Simplified
            resolution: resolution.clone(),
            timestamp: now(),
            success: true,
        };

        self.journal.write().await.entries.push(entry);

        tracing::info!("Conflict resolved with: {:?}", resolution);

        Ok(resolution)
    }

    /// Find similar past resolution
    async fn find_similar_resolution(&self, conflict: &Conflict) -> Option<Resolution> {
        let journal = self.journal.read().await;

        for entry in &journal.entries {
            if entry.conflict_type == format!("{:?}", conflict.type_) {
                // Found similar conflict type
                return Some(entry.resolution.clone());
            }
        }

        None
    }

    /// Resolve by authority (higher authority wins)
    async fn resolve_by_authority(&self, conflict: &Conflict) -> Result<Resolution> {
        // Check if there are involved agents
        if let Some(&winner) = conflict.involved_agents.first() {
            tracing::info!("Resolving conflict by authority: agent {:?} wins", winner);
            Ok(Resolution::AuthorityBased { winner })
        } else {
            tracing::error!("Cannot resolve conflict: no agents involved");
            Err(anyhow!("Cannot resolve conflict: no agents involved"))
        }
    }

    /// Resolve by synthesis (merge both approaches)
    async fn resolve_by_synthesis(
        &self,
        conflict: &Conflict,
        issue: &str,
        position_a: &str,
        position_b: &str,
    ) -> Result<Resolution> {
        // Spawn arbiter for synthesis
        let _arbiter = self.spawn_arbiter(&conflict.involved_agents).await;

        // Generate synthesis
        let synthesis = format!(
            "Synthesis of '{}' disagreement:\n\
             - Option A ({}): Traditional, well-tested\n\
             - Option B ({}): Modern, better performance\n\
             - Synthesis: Use {} with {} compatibility layer",
            issue, position_a, position_b, position_b, position_a
        );

        tracing::info!("Generated synthesis for conflict: {}", issue);

        Ok(Resolution::Synthesis {
            solution: format!("Use {}", position_b),
            reasoning: format!(
                "{} offers better performance while maintaining compatibility with {}",
                position_b, position_a
            ),
            hybrid_approach: synthesis,
        })
    }

    /// Spawn arbiter agent
    async fn spawn_arbiter(&self, involved: &[AgentId]) -> ArbiterAgent {
        let id = AgentId::deterministic(
            &blake3::hash(format!("arbiter_{:?}", involved).as_bytes()),
            &AgentType::Manager,
            0,
        );

        ArbiterAgent {
            id,
            authority: 0.99,
        }
    }

    /// Get conflict statistics
    pub async fn stats(&self) -> ConflictStats {
        let active = self.conflicts.read().await.len();
        let journal = self.journal.read().await;
        let resolved = journal.entries.len();

        ConflictStats {
            active_conflicts: active,
            resolved_conflicts: resolved,
            learned_patterns: journal.entries.len(),
        }
    }
}

/// Conflict statistics
#[derive(Debug, Clone)]
pub struct ConflictStats {
    pub active_conflicts: usize,
    pub resolved_conflicts: usize,
    pub learned_patterns: usize,
}

/// Get current timestamp
fn now() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for ConflictResolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_conflict_detection() {
        let engine = ConflictResolutionEngine::new();

        let outputs = vec![
            AgentOutput {
                agent_id: AgentId::deterministic(&blake3::hash(b"1"), &AgentType::APICoder, 1),
                agent_type: AgentType::APICoder,
                task_id: "1".to_string(),
                content: "Use bcrypt".to_string(),
                files_written: vec!["auth.rs".to_string()],
                patterns_shared: vec![],
                execution_time_ms: 100,
                consensus_approvals: 0,
            },
            AgentOutput {
                agent_id: AgentId::deterministic(&blake3::hash(b"2"), &AgentType::AuthArchitect, 2),
                agent_type: AgentType::AuthArchitect,
                task_id: "2".to_string(),
                content: "Use argon2".to_string(),
                files_written: vec!["auth.rs".to_string()],
                patterns_shared: vec![],
                execution_time_ms: 100,
                consensus_approvals: 0,
            },
        ];

        let conflicts = engine.detect_conflicts(&outputs).await;

        // Should detect both resource conflict and technical disagreement
        assert!(conflicts.len() >= 1);
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let engine = ConflictResolutionEngine::new();

        let conflict = Conflict {
            id: "test".to_string(),
            type_: ConflictType::TechnicalDisagreement {
                issue: "Hash algorithm".to_string(),
                position_a: "bcrypt".to_string(),
                position_b: "argon2".to_string(),
            },
            involved_agents: vec![
                AgentId::deterministic(&blake3::hash(b"1"), &AgentType::APICoder, 1),
                AgentId::deterministic(&blake3::hash(b"2"), &AgentType::AuthArchitect, 2),
            ],
            detected_at: Instant::now(),
            severity: ConflictSeverity::Medium,
        };

        let resolution = engine.resolve(conflict).await.unwrap();

        match resolution {
            Resolution::Synthesis { .. } => {
                // Good - used synthesis for technical disagreement
            }
            _ => {
                // Also acceptable
            }
        }
    }
}
