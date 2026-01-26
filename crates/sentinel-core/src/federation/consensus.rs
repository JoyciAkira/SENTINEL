//! Consensus Engine - Layer 10: Swarm Consensus
//!
//! Gestisce il processo di votazione e risoluzione dei conflitti tra
//! molteplici agenti che propongono modifiche al Goal Manifold.

use crate::types::{Timestamp, GoalStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Una proposta di modifica allo stato del progetto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub goal_id: Uuid,
    pub suggested_status: GoalStatus,
    pub rationale: String,
    pub timestamp: Timestamp,
}

/// Un voto espresso da un agente o dall'utente umano
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub agent_id: Uuid,
    pub proposal_id: Uuid,
    pub approve: bool,
    pub authority_weight: f64, // 0.0 - 1.0
}

pub struct ConsensusEngine {
    pub active_proposals: HashMap<Uuid, Proposal>,
    pub votes: HashMap<Uuid, Vec<Vote>>,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            active_proposals: HashMap::new(),
            votes: HashMap::new(),
        }
    }

    /// Sottomette una nuova proposta per il consenso
    pub fn submit_proposal(&mut self, proposal: Proposal) {
        let proposal_id = proposal.id;
        self.active_proposals.insert(proposal_id, proposal);
        self.votes.insert(proposal_id, Vec::new());
    }

    /// Registra un voto per una proposta esistente
    pub fn cast_vote(&mut self, vote: Vote) -> bool {
        if let Some(proposal_votes) = self.votes.get_mut(&vote.proposal_id) {
            // Un agente può votare una sola volta per proposta
            if !proposal_votes.iter().any(|v| v.agent_id == vote.agent_id) {
                proposal_votes.push(vote);
                return true;
            }
        }
        false
    }

    /// Valuta se una proposta ha raggiunto il consenso
    pub fn evaluate_consensus(&self, proposal_id: Uuid, threshold: f64) -> bool {
        if let Some(proposal_votes) = self.votes.get(&proposal_id) {
            let total_authority: f64 = proposal_votes.iter().map(|v| v.authority_weight).sum();
            let approved_authority: f64 = proposal_votes.iter()
                .filter(|v| v.approve)
                .map(|v| v.authority_weight)
                .sum();

            if total_authority > 0.0 {
                return (approved_authority / total_authority) >= threshold;
            }
        }
        false
    }

    /// Risolve conflitti tra proposte multiple per lo stesso goal
    pub fn resolve_conflicts(&self, goal_id: Uuid) -> Option<Uuid> {
        let same_goal_proposals: Vec<_> = self.active_proposals.values()
            .filter(|p| p.goal_id == goal_id)
            .collect();

        if same_goal_proposals.len() < 2 {
            return same_goal_proposals.first().map(|p| p.id);
        }

        // Strategia: vince la proposta con il peso di autorità approvato più alto
        same_goal_proposals.iter()
            .max_by(|a, b| {
                let auth_a = self.get_approved_authority(a.id);
                let auth_b = self.get_approved_authority(b.id);
                auth_a.partial_cmp(&auth_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.id)
    }

    fn get_approved_authority(&self, proposal_id: Uuid) -> f64 {
        self.votes.get(&proposal_id)
            .map(|votes| votes.iter().filter(|v| v.approve).map(|v| v.authority_weight).sum())
            .unwrap_or(0.0)
    }
}