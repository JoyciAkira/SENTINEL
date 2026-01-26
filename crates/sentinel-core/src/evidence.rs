//! Evidence System - Certificazione dell'Allineamento
//!
//! Genera prove crittografiche (Certificati) che un'azione è stata
//! validata e approvata dall'OS Sentinel.

use crate::types::{Blake3Hash, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Certificato di Allineamento emesso da Sentinel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentCertificate {
    pub id: Uuid,
    pub action_hash: Blake3Hash,
    pub agent_id: Uuid,
    pub goal_id: Uuid,
    pub timestamp: Timestamp,
    pub signature: String, // Placeholder per firma crittografica reale
}

impl AlignmentCertificate {
    pub fn issue(agent_id: Uuid, goal_id: Uuid, action_content: &str) -> Self {
        let action_hash = blake3::hash(action_content.as_bytes()).into();
        
        Self {
            id: Uuid::new_v4(),
            action_hash,
            agent_id,
            goal_id,
            timestamp: chrono::Utc::now(),
            signature: format!("SENTINEL-SIG-{}", Uuid::new_v4()),
        }
    }

    pub fn verify(&self, action_content: &str) -> bool {
        let current_hash: Blake3Hash = blake3::hash(action_content.as_bytes()).into();
        current_hash == self.action_hash
    }
}
