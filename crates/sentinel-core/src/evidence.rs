//! Evidence System - Certificazione dell'Allineamento
//!
//! Genera prove crittografiche (Certificati) che un'azione è stata
//! validata e approvata dall'OS Sentinel.

use crate::types::{Blake3Hash, Timestamp};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{OsRng, RngCore};
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
    pub signer_public_key: String,
    pub signature: String,
    pub signature_scheme: String,
}

impl AlignmentCertificate {
    pub fn issue(agent_id: Uuid, goal_id: Uuid, action_content: &str) -> Self {
        let mut secret_key = [0u8; 32];
        OsRng.fill_bytes(&mut secret_key);
        let signing_key = SigningKey::from_bytes(&secret_key);
        Self::issue_with_signing_key(agent_id, goal_id, action_content, &signing_key)
    }

    pub fn issue_with_signing_key(
        agent_id: Uuid,
        goal_id: Uuid,
        action_content: &str,
        signing_key: &SigningKey,
    ) -> Self {
        let action_hash = blake3::hash(action_content.as_bytes()).into();
        let timestamp = chrono::Utc::now();
        let mut certificate = Self {
            id: Uuid::new_v4(),
            action_hash,
            agent_id,
            goal_id,
            timestamp,
            signer_public_key: hex::encode(signing_key.verifying_key().to_bytes()),
            signature: String::new(),
            signature_scheme: "ed25519-blake3-v1".to_string(),
        };

        let payload = certificate.signing_payload();
        let signature = signing_key.sign(&payload);
        certificate.signature = hex::encode(signature.to_bytes());
        certificate
    }

    pub fn verify(&self, action_content: &str) -> bool {
        let current_hash: Blake3Hash = blake3::hash(action_content.as_bytes()).into();
        if current_hash != self.action_hash {
            return false;
        }

        let public_key_bytes = match hex::decode(&self.signer_public_key) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        if public_key_bytes.len() != 32 {
            return false;
        }
        let public_key_array: [u8; 32] = match public_key_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => return false,
        };
        let verifying_key = match VerifyingKey::from_bytes(&public_key_array) {
            Ok(key) => key,
            Err(_) => return false,
        };

        let signature_bytes = match hex::decode(&self.signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        if signature_bytes.len() != 64 {
            return false;
        }
        let signature = match Signature::from_slice(&signature_bytes) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        verifying_key
            .verify(&self.signing_payload(), &signature)
            .is_ok()
    }

    fn signing_payload(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(256);
        payload.extend_from_slice(self.id.as_bytes());
        payload.extend_from_slice(self.agent_id.as_bytes());
        payload.extend_from_slice(self.goal_id.as_bytes());
        payload.extend_from_slice(self.timestamp.timestamp_millis().to_le_bytes().as_slice());
        payload.extend_from_slice(self.action_hash.to_hex().as_bytes());
        payload.extend_from_slice(self.signature_scheme.as_bytes());
        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_verifies_with_original_content() {
        let cert =
            AlignmentCertificate::issue(Uuid::new_v4(), Uuid::new_v4(), "let x = secure_action();");
        assert!(cert.verify("let x = secure_action();"));
    }

    #[test]
    fn certificate_fails_on_tampered_content() {
        let cert =
            AlignmentCertificate::issue(Uuid::new_v4(), Uuid::new_v4(), "let x = secure_action();");
        assert!(!cert.verify("let x = insecure_action();"));
    }

    #[test]
    fn certificate_fails_on_tampered_signature() {
        let mut cert =
            AlignmentCertificate::issue(Uuid::new_v4(), Uuid::new_v4(), "let x = secure_action();");
        cert.signature = "deadbeef".to_string();
        assert!(!cert.verify("let x = secure_action();"));
    }
}
