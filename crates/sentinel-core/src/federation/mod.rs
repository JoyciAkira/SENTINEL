//! Sentinel Federation - Layer 9: Distributed Intelligence
//!
//! Gestisce l'identità crittografica del nodo e il protocollo di
//! comunicazione peer-to-peer tra istanze di Sentinel.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identità crittografica di un nodo Sentinel
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub node_id: String,
    #[serde(skip)]
    signing_key: Option<SigningKey>,
    pub public_key_hex: String,
}

impl NodeIdentity {
    /// Genera una nuova identità per il nodo attuale
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);

        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key: VerifyingKey = (&signing_key).into();

        let public_key_hex = hex::encode(verifying_key.to_bytes());
        let node_id = format!("sentinel-node-{}", &public_key_hex[..12]);

        Self {
            node_id,
            signing_key: Some(signing_key),
            public_key_hex,
        }
    }

    /// Firma un messaggio per dimostrare l'autenticità del nodo
    pub fn sign_message(&self, message: &[u8]) -> Option<String> {
        self.signing_key.as_ref().map(|key| {
            let signature = key.sign(message);
            hex::encode(signature.to_bytes())
        })
    }

    /// Verifica la firma di un altro nodo
    pub fn verify_signature(public_key_hex: &str, message: &[u8], signature_hex: &str) -> bool {
        let Ok(public_key_bytes) = hex::decode(public_key_hex) else {
            return false;
        };
        let Ok(signature_bytes) = hex::decode(signature_hex) else {
            return false;
        };

        let Ok(public_key_arr): Result<[u8; 32], _> = public_key_bytes.try_into() else {
            return false;
        };
        let Ok(verifying_key) = VerifyingKey::from_bytes(&public_key_arr) else {
            return false;
        };
        let Ok(signature) = Signature::from_slice(&signature_bytes) else {
            return false;
        };

        verifying_key.verify(message, &signature).is_ok()
    }
}

/// Un pattern di successo o una minaccia anonimizzata per la federazione
#[derive(Debug, Serialize, Deserialize)]
pub struct FederatedPattern {
    pub source_node: String,
    pub goal_type_abstract: String,
    pub action_sequence_hashes: Vec<String>,
    pub efficiency_score: f64,
    pub timestamp: i64,
}

impl FederatedPattern {
    /// Trasforma un pattern locale in uno federato (Anonimizzazione)
    pub fn anonymize(node_id: &str, pattern: &crate::learning::types::SuccessPattern) -> Self {
        Self {
            source_node: node_id.to_string(),
            goal_type_abstract: format!("{:?}", pattern.applicable_to_goal_types[0]),
            action_sequence_hashes: pattern
                .action_sequence
                .iter()
                .map(|a| hex::encode(blake3::hash(format!("{:?}", a).as_bytes()).as_bytes()))
                .collect(),
            efficiency_score: pattern.success_rate,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Types of threats detected by Sentinel nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatType {
    /// AI is deviating from its assigned goals
    AlignmentDeviation,
    /// Vulnerable dependency detected
    SecurityVulnerability,
    /// Unauthorized action attempt
    UnauthorizedAction,
    /// Network-level attack detected
    NetworkAttack,
}

/// Severity level of a threat or violation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// A threat detected by a Sentinel node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAlert {
    pub threat_id: Uuid,
    pub threat_type: ThreatType,
    pub severity: Severity,
    pub description: String,
    pub source_agent_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub mod consensus;
pub mod gossip;
pub mod network;

pub use consensus::{Proposal, Vote};
pub use gossip::{GossipMessage, GossipPayload};
