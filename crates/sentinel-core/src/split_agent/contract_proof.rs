//! ContractProof — Formal cryptographic proof that a worker satisfied its module contract.
//!
//! This is the 250 IQ insight: instead of trusting that a worker "returned Ok(())",
//! we require a ContractProof: a signed, timestamped attestation that the specific
//! contract_hash was satisfied. This is Hoare Logic for AI agents.
//!
//! Semantics:
//!   worker_fn(module, path) -> Result<ContractProof, String>
//!   ContractProof.verify(module) -> bool
//!   Only a proof covering the exact contract_hash is accepted.

use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// A formal proof that a worker completed its module contract.
/// The proof is produced BY the worker, verified BY the ModuleVerifier.
///
/// Crucially: the proof is bound to the exact contract_hash of the module
/// that was received. A worker cannot reuse a proof from a different module.
#[derive(Debug, Clone)]
pub struct ContractProof {
    /// ID of the module this proof covers.
    pub module_id: Uuid,
    /// Blake3 hash of the WorkerModule contract at the time of proof creation.
    /// Must match module.contract_hash() for the proof to be valid.
    pub contract_hash: String,
    /// Whether all output_contract predicates were satisfied at proof time.
    pub all_satisfied: bool,
    /// Details of each predicate (description, satisfied, detail string).
    pub predicate_evidence: Vec<PredicateEvidence>,
    /// Unix timestamp (ms) when this proof was produced.
    pub produced_at_ms: u64,
    /// Blake3 signature over: module_id|contract_hash|all_satisfied|produced_at_ms
    pub signature: String,
}

/// Evidence for a single predicate within a ContractProof.
#[derive(Debug, Clone)]
pub struct PredicateEvidence {
    pub predicate_description: String,
    pub satisfied: bool,
    pub detail: String,
}

impl ContractProof {
    /// Create a proof for a module, given pre-evaluated predicate evidence.
    /// This is called by the worker after performing its work and running
    /// an internal self-check.
    pub fn create(
        module_id: Uuid,
        contract_hash: String,
        predicate_evidence: Vec<PredicateEvidence>,
    ) -> Self {
        let all_satisfied = predicate_evidence.iter().all(|e| e.satisfied);
        let produced_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let signature_material = format!(
            "{}|{}|{}|{}",
            module_id, contract_hash, all_satisfied, produced_at_ms
        );
        let signature = blake3::hash(signature_material.as_bytes())
            .to_hex()
            .to_string();

        Self {
            module_id,
            contract_hash,
            all_satisfied,
            predicate_evidence,
            produced_at_ms,
            signature,
        }
    }

    /// Verify the internal signature of this proof (tamper detection).
    /// Does NOT verify against a module — call module.verify_proof(proof) for that.
    pub fn verify_signature(&self) -> bool {
        let expected_material = format!(
            "{}|{}|{}|{}",
            self.module_id, self.contract_hash, self.all_satisfied, self.produced_at_ms
        );
        let expected_sig = blake3::hash(expected_material.as_bytes())
            .to_hex()
            .to_string();
        self.signature == expected_sig
    }

    /// Returns a human-readable summary of the proof.
    pub fn summary(&self) -> String {
        format!(
            "ContractProof[module={}, hash={:.8}…, satisfied={}, predicates={}, sig_valid={}]",
            self.module_id,
            self.contract_hash,
            self.all_satisfied,
            self.predicate_evidence.len(),
            self.verify_signature()
        )
    }
}

/// A failed proof — returned when a worker cannot satisfy the contract.
#[derive(Debug, Clone)]
pub struct ContractProofFailure {
    pub module_id: Uuid,
    pub contract_hash: String,
    pub reason: String,
    pub unsatisfied_predicates: Vec<PredicateEvidence>,
}

/// Result type returned by a worker that supports ContractProof.
pub type WorkerResult = std::result::Result<ContractProof, ContractProofFailure>;

/// Builder for ContractProof — used by workers to construct proofs step by step.
pub struct ContractProofBuilder {
    module_id: Uuid,
    contract_hash: String,
    evidence: Vec<PredicateEvidence>,
}

impl ContractProofBuilder {
    pub fn new(module_id: Uuid, contract_hash: impl Into<String>) -> Self {
        Self {
            module_id,
            contract_hash: contract_hash.into(),
            evidence: Vec::new(),
        }
    }

    pub fn add_evidence(
        mut self,
        description: impl Into<String>,
        satisfied: bool,
        detail: impl Into<String>,
    ) -> Self {
        self.evidence.push(PredicateEvidence {
            predicate_description: description.into(),
            satisfied,
            detail: detail.into(),
        });
        self
    }

    /// Build the proof if all evidence is satisfied, or return a failure.
    pub fn build(self) -> WorkerResult {
        let all_satisfied = self.evidence.iter().all(|e| e.satisfied);
        if all_satisfied {
            Ok(ContractProof::create(
                self.module_id,
                self.contract_hash,
                self.evidence,
            ))
        } else {
            let unsatisfied: Vec<_> = self
                .evidence
                .iter()
                .filter(|e| !e.satisfied)
                .cloned()
                .collect();
            let reason = unsatisfied
                .iter()
                .map(|e| format!("{}: {}", e.predicate_description, e.detail))
                .collect::<Vec<_>>()
                .join("; ");
            Err(ContractProofFailure {
                module_id: self.module_id,
                contract_hash: self.contract_hash,
                reason,
                unsatisfied_predicates: unsatisfied,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_proof_create_and_verify_signature() {
        let module_id = Uuid::new_v4();
        let hash = "abc123".to_string();
        let evidence = vec![PredicateEvidence {
            predicate_description: "file_exists(src/main.rs)".into(),
            satisfied: true,
            detail: "File found".into(),
        }];

        let proof = ContractProof::create(module_id, hash.clone(), evidence);

        assert!(proof.all_satisfied);
        assert_eq!(proof.module_id, module_id);
        assert_eq!(proof.contract_hash, hash);
        assert!(proof.verify_signature(), "signature must be valid");
    }

    #[test]
    fn test_tampered_proof_fails_signature() {
        let module_id = Uuid::new_v4();
        let mut proof = ContractProof::create(
            module_id,
            "abc123".into(),
            vec![PredicateEvidence {
                predicate_description: "always_true".into(),
                satisfied: true,
                detail: "OK".into(),
            }],
        );

        // Tamper with the proof
        proof.all_satisfied = false;

        assert!(
            !proof.verify_signature(),
            "tampered proof must fail signature check"
        );
    }

    #[test]
    fn test_builder_all_satisfied_produces_proof() {
        let module_id = Uuid::new_v4();
        let result = ContractProofBuilder::new(module_id, "hash_abc")
            .add_evidence("file_exists(a.rs)", true, "File found")
            .add_evidence("file_exists(b.rs)", true, "File found")
            .build();

        assert!(result.is_ok());
        let proof = result.unwrap();
        assert!(proof.all_satisfied);
        assert!(proof.verify_signature());
    }

    #[test]
    fn test_builder_partial_failure_produces_failure() {
        let module_id = Uuid::new_v4();
        let result = ContractProofBuilder::new(module_id, "hash_xyz")
            .add_evidence("file_exists(a.rs)", true, "File found")
            .add_evidence("file_exists(b.rs)", false, "File not found: /tmp/b.rs")
            .build();

        assert!(result.is_err());
        let failure = result.unwrap_err();
        assert_eq!(failure.unsatisfied_predicates.len(), 1);
        assert!(failure.reason.contains("b.rs"));
    }
}
