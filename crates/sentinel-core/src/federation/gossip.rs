//! Gossip Protocol - Propagazione virale dell'intelligenza
//!
//! Gestisce la diffusione di messaggi firmati tra i peer della rete Sentinel.

use crate::federation::{NodeIdentity, FederatedPattern, consensus::{Proposal, Vote}};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Un messaggio che viaggia nella rete globale Sentinel
#[derive(Debug, Serialize, Deserialize)]
pub struct GossipMessage {
    pub sender_id: String,
    pub sender_public_key: String,
    pub payload: GossipPayload,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GossipPayload {
    Pattern(FederatedPattern),
    ConsensusProposal(Proposal),
    ConsensusVote(Vote),
    ThreatAlert {
        dependency_name: String,
        risk_score: f64,
        reason: String,
    },
    NodeHealth {
        uptime: u64,
        alignment_avg: f64,
    },
}

pub struct GossipService {
    pub identity: NodeIdentity,
    pub message_queue: VecDeque<GossipMessage>,
}

impl GossipService {
    pub fn new(identity: NodeIdentity) -> Self {
        Self {
            identity,
            message_queue: VecDeque::new(),
        }
    }

    /// Crea un nuovo messaggio di gossip firmato
    pub fn broadcast_payload(&mut self, payload: GossipPayload) -> Option<GossipMessage> {
        let payload_json = serde_json::to_string(&payload).ok()?;
        let signature = self.identity.sign_message(payload_json.as_bytes())?;

        Some(GossipMessage {
            sender_id: self.identity.node_id.clone(),
            sender_public_key: self.identity.public_key_hex.clone(),
            payload,
            signature,
        })
    }

    /// Riceve e valida un messaggio da un altro nodo (Zero-Trust)
    pub fn receive_message(&mut self, message: GossipMessage) -> bool {
        let Ok(payload_json) = serde_json::to_string(&message.payload) else { return false; };
        
        // Verifica crittografica: il messaggio è autentico?
        if NodeIdentity::verify_signature(
            &message.sender_public_key,
            payload_json.as_bytes(),
            &message.signature
        ) {
            self.message_queue.push_back(message);
            true
        } else {
            false // Rilevato tentativo di spoofing o corruzione
        }
    }
}
