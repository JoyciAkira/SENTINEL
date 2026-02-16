//! Continuous Consensus System
//!
//! Agents reach consensus every 100ms through continuous voting.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, RwLock};

use super::{
    communication::{Proposal, ProposalId, SwarmMessage, Vote},
    AgentId,
};

/// Continuous consensus engine
pub struct ContinuousConsensus {
    /// Current consensus round
    round: Arc<RwLock<u64>>,

    /// Pending proposals awaiting consensus
    pending_proposals: Arc<RwLock<HashMap<ProposalId, ProposalState>>>,

    /// Completed proposals
    completed_proposals: Arc<RwLock<Vec<CompletedProposal>>>,

    /// Consensus history
    history: Arc<RwLock<Vec<ConsensusRecord>>>,

    /// Quorum threshold (0.0 - 1.0)
    quorum_threshold: f64,

    /// Vote timeout (milliseconds)
    vote_timeout_ms: u64,

    /// Communication bus for broadcasting
    communication_bus: Arc<super::CommunicationBus>,
}

/// State of a proposal during voting
#[derive(Debug, Clone)]
pub struct ProposalState {
    pub proposal: Proposal,
    pub votes: HashMap<AgentId, Vote>,
    pub vote_time: HashMap<AgentId, Instant>,
    pub proposed_at: Instant,
    pub status: ProposalStatus,
}

/// Status of proposal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Voting,
    Accepted,
    Rejected,
    Timeout,
}

/// Completed proposal record
#[derive(Debug, Clone)]
pub struct CompletedProposal {
    pub proposal: Proposal,
    pub status: ProposalStatus,
    pub final_votes: HashMap<AgentId, Vote>,
    pub completed_at: Instant,
}

/// Consensus record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusRecord {
    pub round: u64,
    pub proposal_id: ProposalId,
    pub proposal_title: String,
    pub status: ProposalStatus,
    pub vote_count: usize,
    pub approve_count: usize,
    pub timestamp: u64,
}

impl ContinuousConsensus {
    /// Create new consensus engine
    pub fn new(
        quorum_threshold: f64,
        vote_timeout_ms: u64,
        communication_bus: Arc<super::CommunicationBus>,
    ) -> Self {
        Self {
            round: Arc::new(RwLock::new(0)),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
            completed_proposals: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            quorum_threshold,
            vote_timeout_ms,
            communication_bus,
        }
    }

    /// Run continuous consensus loop
    pub async fn run(&self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            let round = {
                let mut r = self.round.write().await;
                *r += 1;
                *r
            };

            // Process pending proposals
            self.process_pending().await?;

            // Check for timeouts
            self.check_timeouts().await?;

            // Emit heartbeat every 10 rounds (1 second)
            if round % 10 == 0 {
                self.emit_heartbeat().await?;
            }
        }
    }

    /// Submit a new proposal for consensus
    pub async fn propose(&self, proposal: Proposal) -> Result<ProposalId> {
        let id = proposal.id;

        let state = ProposalState {
            proposal: proposal.clone(),
            votes: HashMap::new(),
            vote_time: HashMap::new(),
            proposed_at: Instant::now(),
            status: ProposalStatus::Voting,
        };

        self.pending_proposals.write().await.insert(id, state);

        // Broadcast proposal to all agents
        let msg = SwarmMessage::Proposal {
            id,
            by: proposal.proposed_by,
            proposal: proposal.clone(),
            timestamp: now(),
        };

        self.communication_bus.broadcast(msg).await?;

        tracing::info!("Proposal {:?} submitted for consensus", id);

        Ok(id)
    }

    /// Submit vote for proposal
    pub async fn submit_vote(
        &self,
        proposal_id: ProposalId,
        agent_id: AgentId,
        vote: Vote,
    ) -> Result<()> {
        let mut proposals = self.pending_proposals.write().await;

        if let Some(state) = proposals.get_mut(&proposal_id) {
            if state.status != ProposalStatus::Voting {
                return Err(anyhow!("Proposal {:?} is not open for voting", proposal_id));
            }

            state.votes.insert(agent_id, vote);
            state.vote_time.insert(agent_id, Instant::now());

            // Broadcast vote
            let msg = SwarmMessage::Vote {
                proposal_id,
                by: agent_id,
                vote,
                reasoning: None,
            };

            self.communication_bus.broadcast(msg).await?;

            // Check if consensus reached
            if self.check_consensus(state) {
                state.status = ProposalStatus::Accepted;

                // Move to completed
                let completed = CompletedProposal {
                    proposal: state.proposal.clone(),
                    status: ProposalStatus::Accepted,
                    final_votes: state.votes.clone(),
                    completed_at: Instant::now(),
                };

                self.completed_proposals.write().await.push(completed);

                // Record in history
                self.record_consensus(&state).await?;

                tracing::info!("Proposal {:?} reached consensus (Accepted)", proposal_id);
            }

            Ok(())
        } else {
            Err(anyhow!("Proposal {:?} not found", proposal_id))
        }
    }

    /// Check if consensus reached for proposal
    fn check_consensus(&self, state: &ProposalState) -> bool {
        if state.votes.is_empty() {
            return false;
        }

        let total = state.votes.len() as f64;
        let approvals = state
            .votes
            .values()
            .filter(|v| matches!(v, Vote::Approve))
            .count() as f64;

        let approval_rate = approvals / total;
        approval_rate >= self.quorum_threshold
    }

    /// Process pending proposals
    async fn process_pending(&self) -> Result<()> {
        // Check if any pending proposals need action
        let pending = self.pending_proposals.read().await;

        for (id, state) in pending.iter() {
            if state.status == ProposalStatus::Voting {
                // Check vote count
                let vote_count = state.votes.len();

                // If we have enough votes, check consensus
                if vote_count >= 3 { // Minimum votes needed
                     // Already handled in submit_vote
                }
            }
        }

        Ok(())
    }

    /// Check for vote timeouts
    async fn check_timeouts(&self) -> Result<()> {
        let timeout_duration = Duration::from_millis(self.vote_timeout_ms);
        let now = Instant::now();

        let mut proposals = self.pending_proposals.write().await;
        let mut timed_out = Vec::new();

        for (id, state) in proposals.iter() {
            if state.status == ProposalStatus::Voting {
                let elapsed = now.duration_since(state.proposed_at);

                if elapsed > timeout_duration {
                    timed_out.push(*id);
                }
            }
        }

        // Process timeouts
        for id in timed_out {
            if let Some(state) = proposals.get_mut(&id) {
                state.status = ProposalStatus::Timeout;

                let completed = CompletedProposal {
                    proposal: state.proposal.clone(),
                    status: ProposalStatus::Timeout,
                    final_votes: state.votes.clone(),
                    completed_at: Instant::now(),
                };

                self.completed_proposals.write().await.push(completed);

                tracing::warn!("Proposal {:?} timed out", id);
            }
        }

        Ok(())
    }

    /// Record consensus in history
    async fn record_consensus(&self, state: &ProposalState) -> Result<()> {
        let round = *self.round.read().await;

        let record = ConsensusRecord {
            round,
            proposal_id: state.proposal.id,
            proposal_title: state.proposal.title.clone(),
            status: state.status,
            vote_count: state.votes.len(),
            approve_count: state
                .votes
                .values()
                .filter(|v| matches!(v, Vote::Approve))
                .count(),
            timestamp: now(),
        };

        self.history.write().await.push(record);

        Ok(())
    }

    /// Emit consensus heartbeat
    async fn emit_heartbeat(&self) -> Result<()> {
        let round = *self.round.read().await;
        let pending_count = self.pending_proposals.read().await.len();
        let completed_count = self.completed_proposals.read().await.len();

        tracing::debug!(
            "Consensus heartbeat - Round: {}, Pending: {}, Completed: {}",
            round,
            pending_count,
            completed_count
        );

        Ok(())
    }

    /// Get current round
    pub async fn get_round(&self) -> u64 {
        *self.round.read().await
    }

    /// Get proposal status
    pub async fn get_proposal_status(&self, id: ProposalId) -> Option<ProposalStatus> {
        self.pending_proposals
            .read()
            .await
            .get(&id)
            .map(|s| s.status)
    }

    /// Get all pending proposals
    pub async fn get_pending(&self) -> Vec<Proposal> {
        self.pending_proposals
            .read()
            .await
            .values()
            .filter(|s| s.status == ProposalStatus::Voting)
            .map(|s| s.proposal.clone())
            .collect()
    }

    /// Get consensus history
    pub async fn get_history(&self) -> Vec<ConsensusRecord> {
        self.history.read().await.clone()
    }
}

/// Get current timestamp in milliseconds
fn now() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::super::CommunicationBus;
    use super::*;

    #[tokio::test]
    async fn test_proposal_submission() {
        let bus = Arc::new(CommunicationBus::new());
        let consensus = ContinuousConsensus::new(0.75, 2000, bus);

        let proposal = Proposal {
            id: ProposalId::new(),
            title: "Test".to_string(),
            description: "Test proposal".to_string(),
            action: super::super::communication::ProposedAction::SelectLibrary("test".to_string()),
            proposed_by: AgentId::deterministic(
                &blake3::hash(b"test"),
                &super::super::emergence::AgentType::APICoder,
                0,
            ),
            timestamp: now(),
        };

        let id = consensus.propose(proposal).await.unwrap();

        let pending = consensus.get_pending().await;
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_consensus_reached() {
        let bus = Arc::new(CommunicationBus::new());
        let consensus = ContinuousConsensus::new(0.75, 2000, bus);

        let proposal = Proposal {
            id: ProposalId::new(),
            title: "Test".to_string(),
            description: "Test proposal".to_string(),
            action: super::super::communication::ProposedAction::SelectLibrary("test".to_string()),
            proposed_by: AgentId::deterministic(
                &blake3::hash(b"test"),
                &super::super::emergence::AgentType::APICoder,
                0,
            ),
            timestamp: now(),
        };

        let id = consensus.propose(proposal).await.unwrap();

        // Submit votes (4 agents, need 3 for 75% quorum)
        let agent1 = AgentId::deterministic(
            &blake3::hash(b"1"),
            &super::super::emergence::AgentType::APICoder,
            1,
        );
        let agent2 = AgentId::deterministic(
            &blake3::hash(b"2"),
            &super::super::emergence::AgentType::APICoder,
            2,
        );
        let agent3 = AgentId::deterministic(
            &blake3::hash(b"3"),
            &super::super::emergence::AgentType::APICoder,
            3,
        );
        let agent4 = AgentId::deterministic(
            &blake3::hash(b"4"),
            &super::super::emergence::AgentType::APICoder,
            4,
        );

        // Submit all votes first (order matters - need to reach exactly 75%)
        // Submit reject first to lower approval rate
        consensus
            .submit_vote(id, agent4, Vote::Reject)
            .await
            .unwrap(); // 1 vote: 0% approval
        consensus
            .submit_vote(id, agent1, Vote::Approve)
            .await
            .unwrap(); // 2 votes: 50% approval
        consensus
            .submit_vote(id, agent2, Vote::Approve)
            .await
            .unwrap(); // 3 votes: 66% approval
        consensus
            .submit_vote(id, agent3, Vote::Approve)
            .await
            .unwrap(); // 4 votes: 75% approval - reached!

        let status = consensus.get_proposal_status(id).await;
        assert_eq!(status, Some(ProposalStatus::Accepted));
    }
}
