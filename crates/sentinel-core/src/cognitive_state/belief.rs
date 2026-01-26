//! Belief System - Agent's epistemic state
//!
//! The belief system tracks what the agent believes to be true about
//! the project, along with confidence levels and supporting evidence.

use crate::types::Timestamp;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A belief held by the agent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Belief {
    /// Unique identifier
    pub id: Uuid,

    /// The proposition believed to be true
    pub proposition: String,

    /// Confidence in this belief (0.0-1.0)
    pub confidence: f64,

    /// Evidence supporting this belief
    pub evidence: Vec<String>,

    /// When this belief was formed
    pub formed_at: Timestamp,

    /// When this belief was last updated
    pub updated_at: Timestamp,

    /// Whether this belief has been validated
    pub validated: bool,
}

impl Belief {
    /// Create a new belief
    pub fn new(proposition: String, confidence: f64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            proposition,
            confidence: confidence.clamp(0.0, 1.0),
            evidence: Vec::new(),
            formed_at: now,
            updated_at: now,
            validated: false,
        }
    }

    /// Add evidence supporting this belief
    pub fn add_evidence(&mut self, evidence: String) {
        self.evidence.push(evidence);
        self.updated_at = Utc::now();
    }

    /// Update confidence based on new information
    pub fn update_confidence(&mut self, new_confidence: f64) {
        self.confidence = new_confidence.clamp(0.0, 1.0);
        self.updated_at = Utc::now();
    }

    /// Mark as validated
    pub fn validate(&mut self) {
        self.validated = true;
        self.updated_at = Utc::now();
    }

    /// Check if this is a strong belief (high confidence)
    pub fn is_strong(&self) -> bool {
        self.confidence > 0.8
    }

    /// Check if this is a weak belief (low confidence)
    pub fn is_weak(&self) -> bool {
        self.confidence < 0.3
    }
}

/// Network of beliefs with dependencies
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BeliefNetwork {
    /// All beliefs
    beliefs: HashMap<Uuid, Belief>,

    /// Dependencies between beliefs
    /// Key: belief ID, Value: IDs of beliefs it depends on
    dependencies: HashMap<Uuid, Vec<Uuid>>,
}

impl BeliefNetwork {
    /// Create a new belief network
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a belief to the network
    pub fn add_belief(&mut self, belief: Belief) -> Uuid {
        let id = belief.id;
        self.beliefs.insert(id, belief);
        id
    }

    /// Get a belief by ID
    pub fn get_belief(&self, id: &Uuid) -> Option<&Belief> {
        self.beliefs.get(id)
    }

    /// Get a mutable reference to a belief
    pub fn get_belief_mut(&mut self, id: &Uuid) -> Option<&mut Belief> {
        self.beliefs.get_mut(id)
    }

    /// Add dependency between beliefs
    pub fn add_dependency(&mut self, belief_id: Uuid, depends_on: Uuid) {
        self.dependencies
            .entry(belief_id)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }

    /// Get all beliefs
    pub fn all_beliefs(&self) -> Vec<&Belief> {
        self.beliefs.values().collect()
    }

    /// Get strong beliefs (confidence > 0.8)
    pub fn strong_beliefs(&self) -> Vec<&Belief> {
        self.beliefs.values().filter(|b| b.is_strong()).collect()
    }

    /// Get weak beliefs (confidence < 0.3)
    pub fn weak_beliefs(&self) -> Vec<&Belief> {
        self.beliefs.values().filter(|b| b.is_weak()).collect()
    }

    /// Propagate confidence changes through the network
    pub fn propagate_confidence(&mut self, belief_id: Uuid) {
        // Simple propagation: if a belief's confidence drops,
        // reduce confidence in dependent beliefs
        if let Some(belief) = self.beliefs.get(&belief_id) {
            let confidence = belief.confidence;

            // Find beliefs that depend on this one
            for (dependent_id, dependencies) in &self.dependencies {
                if dependencies.contains(&belief_id) {
                    if let Some(dependent) = self.beliefs.get_mut(dependent_id) {
                        // Reduce confidence proportionally
                        let reduction = (1.0 - confidence) * 0.5;
                        dependent.confidence = (dependent.confidence - reduction).max(0.0);
                        dependent.updated_at = Utc::now();
                    }
                }
            }
        }
    }

    /// Count total beliefs
    pub fn count(&self) -> usize {
        self.beliefs.len()
    }
}

/// Uncertainty about something
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Uncertainty {
    /// What we're uncertain about
    pub about: String,

    /// Type of uncertainty
    pub uncertainty_type: UncertaintyType,

    /// Magnitude of uncertainty (0.0-1.0)
    pub magnitude: f64,

    /// What action could resolve this uncertainty
    pub resolvable_by: Option<String>,

    /// When this uncertainty was identified
    pub identified_at: Timestamp,
}

impl Uncertainty {
    /// Create a new uncertainty
    pub fn new(about: String, uncertainty_type: UncertaintyType, magnitude: f64) -> Self {
        Self {
            about,
            uncertainty_type,
            magnitude: magnitude.clamp(0.0, 1.0),
            resolvable_by: None,
            identified_at: Utc::now(),
        }
    }

    /// Set how this uncertainty could be resolved
    pub fn resolvable_by(mut self, action: String) -> Self {
        self.resolvable_by = Some(action);
        self
    }

    /// Check if this is a high-magnitude uncertainty
    pub fn is_critical(&self) -> bool {
        self.magnitude > 0.7
    }
}

/// Type of uncertainty
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UncertaintyType {
    /// Aleatory - inherent randomness in the system
    Aleatory,

    /// Epistemic - lack of knowledge (can be reduced)
    Epistemic,

    /// Ontological - uncertainty about what exists
    Ontological,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_creation() {
        let belief = Belief::new("API endpoint works".to_string(), 0.9);
        assert_eq!(belief.proposition, "API endpoint works");
        assert_eq!(belief.confidence, 0.9);
        assert!(belief.is_strong());
    }

    #[test]
    fn test_belief_evidence() {
        let mut belief = Belief::new("Tests pass".to_string(), 0.7);
        belief.add_evidence("All 42 tests passed".to_string());
        assert_eq!(belief.evidence.len(), 1);
    }

    #[test]
    fn test_belief_confidence_update() {
        let mut belief = Belief::new("Feature complete".to_string(), 0.5);
        belief.update_confidence(0.9);
        assert_eq!(belief.confidence, 0.9);
        assert!(belief.is_strong());
    }

    #[test]
    fn test_belief_network_add() {
        let mut network = BeliefNetwork::new();
        let belief = Belief::new("System is secure".to_string(), 0.8);
        let id = network.add_belief(belief);

        assert_eq!(network.count(), 1);
        assert!(network.get_belief(&id).is_some());
    }

    #[test]
    fn test_belief_network_dependencies() {
        let mut network = BeliefNetwork::new();

        let belief1 = Belief::new("Auth works".to_string(), 0.9);
        let id1 = network.add_belief(belief1);

        let belief2 = Belief::new("System secure".to_string(), 0.7);
        let id2 = network.add_belief(belief2);

        network.add_dependency(id2, id1);

        assert_eq!(network.count(), 2);
    }

    #[test]
    fn test_belief_network_strong_beliefs() {
        let mut network = BeliefNetwork::new();
        network.add_belief(Belief::new("Strong belief".to_string(), 0.9));
        network.add_belief(Belief::new("Weak belief".to_string(), 0.2));

        let strong = network.strong_beliefs();
        assert_eq!(strong.len(), 1);
    }

    #[test]
    fn test_uncertainty_creation() {
        let uncertainty =
            Uncertainty::new("Test coverage".to_string(), UncertaintyType::Epistemic, 0.5);

        assert_eq!(uncertainty.magnitude, 0.5);
        assert!(!uncertainty.is_critical());
    }

    #[test]
    fn test_uncertainty_critical() {
        let uncertainty = Uncertainty::new("Security".to_string(), UncertaintyType::Epistemic, 0.8);

        assert!(uncertainty.is_critical());
    }

    #[test]
    fn test_uncertainty_resolvable() {
        let uncertainty =
            Uncertainty::new("Code quality".to_string(), UncertaintyType::Epistemic, 0.6)
                .resolvable_by("Run linter".to_string());

        assert_eq!(uncertainty.resolvable_by.unwrap(), "Run linter");
    }
}
