//! Semantic Memory - Knowledge graph for conceptual relationships
//!
//! Stores concepts and their relationships for reasoning.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A concept node in the knowledge graph
#[derive(Debug, Clone)]
pub struct ConceptNode {
    /// Unique identifier
    pub id: Uuid,

    /// Concept name
    pub name: String,

    /// Concept description
    pub description: String,

    /// Associated memory IDs
    pub memory_ids: Vec<Uuid>,

    /// Concept type
    pub concept_type: ConceptType,

    /// Activation level (0.0-1.0, decays over time)
    pub activation: f64,
}

/// Types of concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptType {
    /// A code entity (function, class, module)
    CodeEntity,

    /// A design pattern
    Pattern,

    /// A problem or bug
    Problem,

    /// A solution or fix
    Solution,

    /// A goal or objective
    Goal,

    /// A constraint or requirement
    Constraint,

    /// A technology or tool
    Technology,

    /// A general concept
    General,
}

/// A relationship between concepts
#[derive(Debug, Clone)]
pub struct ConceptRelation {
    /// Source concept ID
    pub from: Uuid,

    /// Target concept ID
    pub to: Uuid,

    /// Relationship type
    pub relation_type: RelationType,

    /// Strength of relationship (0.0-1.0)
    pub strength: f64,
}

/// Types of relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// A is part of B
    PartOf,

    /// A depends on B
    DependsOn,

    /// A is similar to B
    SimilarTo,

    /// A causes B
    Causes,

    /// A solves B
    Solves,

    /// A implements B
    Implements,

    /// A uses B
    Uses,

    /// A is opposite of B
    OppositeOf,

    /// Generic association
    RelatedTo,
}

/// Semantic memory with knowledge graph
#[derive(Debug)]
pub struct SemanticMemory {
    /// Concept nodes
    concepts: HashMap<Uuid, ConceptNode>,

    /// Relationships between concepts
    relations: Vec<ConceptRelation>,

    /// Index: concept name -> concept ID
    name_index: HashMap<String, Uuid>,

    /// Activation decay rate per query
    decay_rate: f64,
}

impl SemanticMemory {
    /// Create a new semantic memory
    pub fn new() -> Self {
        Self {
            concepts: HashMap::new(),
            relations: Vec::new(),
            name_index: HashMap::new(),
            decay_rate: 0.95, // 5% decay per query
        }
    }

    /// Add a concept
    pub fn add_concept(&mut self, mut concept: ConceptNode) {
        let id = concept.id;
        let name = concept.name.clone();

        // Initialize activation
        concept.activation = 1.0;

        self.concepts.insert(id, concept);
        self.name_index.insert(name.to_lowercase(), id);
    }

    /// Add a memory ID to an existing concept
    pub fn add_memory_to_concept(&mut self, concept_id: &Uuid, memory_id: Uuid) {
        if let Some(concept) = self.concepts.get_mut(concept_id) {
            if !concept.memory_ids.contains(&memory_id) {
                concept.memory_ids.push(memory_id);
            }
        }
    }

    /// Get a concept by ID
    pub fn get_concept(&self, id: &Uuid) -> Option<&ConceptNode> {
        self.concepts.get(id)
    }

    /// Get a concept by name
    pub fn get_concept_by_name(&self, name: &str) -> Option<&ConceptNode> {
        self.name_index
            .get(&name.to_lowercase())
            .and_then(|id| self.concepts.get(id))
    }

    /// Add a relationship
    pub fn add_relation(&mut self, relation: ConceptRelation) {
        // Validate that both concepts exist
        if self.concepts.contains_key(&relation.from) && self.concepts.contains_key(&relation.to) {
            self.relations.push(relation);
        }
    }

    /// Get all relations for a concept
    pub fn get_relations(&self, concept_id: &Uuid) -> Vec<&ConceptRelation> {
        self.relations
            .iter()
            .filter(|r| r.from == *concept_id || r.to == *concept_id)
            .collect()
    }

    /// Get related concepts (direct neighbors)
    pub fn get_related_concepts(&self, concept_id: &Uuid) -> Vec<(&ConceptNode, &ConceptRelation)> {
        self.relations
            .iter()
            .filter_map(|rel| {
                if rel.from == *concept_id {
                    self.concepts.get(&rel.to).map(|c| (c, rel))
                } else if rel.to == *concept_id {
                    self.concepts.get(&rel.from).map(|c| (c, rel))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Activate a concept (spreading activation)
    pub fn activate(&mut self, concept_id: &Uuid, activation: f64) {
        // Activate the concept
        if let Some(concept) = self.concepts.get_mut(concept_id) {
            concept.activation = (concept.activation + activation).min(1.0);
        }

        // Spread activation to related concepts
        let related: Vec<_> = self
            .relations
            .iter()
            .filter(|r| r.from == *concept_id || r.to == *concept_id)
            .map(|r| {
                let target = if r.from == *concept_id { r.to } else { r.from };
                (target, activation * r.strength * 0.5) // 50% spread
            })
            .collect();

        for (target_id, spread_activation) in related {
            if let Some(concept) = self.concepts.get_mut(&target_id) {
                concept.activation = (concept.activation + spread_activation).min(1.0);
            }
        }
    }

    /// Decay all activations
    pub fn decay_activations(&mut self) {
        for concept in self.concepts.values_mut() {
            concept.activation *= self.decay_rate;
        }
    }

    /// Query by concept name with spreading activation
    pub fn query(&mut self, query: &str, limit: usize) -> Vec<Uuid> {
        // Decay previous activations
        self.decay_activations();

        // Find matching concepts and activate them
        let query_lower = query.to_lowercase();
        let matching: Vec<_> = self
            .concepts
            .iter()
            .filter(|(_, concept)| {
                concept.name.to_lowercase().contains(&query_lower)
                    || concept.description.to_lowercase().contains(&query_lower)
            })
            .map(|(id, _)| *id)
            .collect();

        // Activate matching concepts
        for id in &matching {
            self.activate(id, 1.0);
        }

        // Return concepts sorted by activation
        let mut results: Vec<_> = self
            .concepts
            .iter()
            .filter(|(_, c)| c.activation > 0.1) // Threshold
            .map(|(id, concept)| (*id, concept.activation))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(limit);

        results.into_iter().map(|(id, _)| id).collect()
    }

    /// Find path between two concepts (BFS)
    pub fn find_path(&self, from: &Uuid, to: &Uuid, max_depth: usize) -> Option<Vec<Uuid>> {
        use std::collections::VecDeque;

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<Uuid, Uuid> = HashMap::new();

        queue.push_back((*from, 0));
        visited.insert(*from);

        while let Some((current, depth)) = queue.pop_front() {
            if current == *to {
                // Reconstruct path
                let mut path = vec![current];
                let mut node = current;
                while let Some(&prev) = parent.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(path);
            }

            if depth >= max_depth {
                continue;
            }

            // Explore neighbors
            for rel in &self.relations {
                let neighbor = if rel.from == current {
                    rel.to
                } else if rel.to == current {
                    rel.from
                } else {
                    continue;
                };

                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }

        None
    }

    /// Get most activated concepts
    pub fn get_most_activated(&self, limit: usize) -> Vec<&ConceptNode> {
        let mut concepts: Vec<_> = self.concepts.values().collect();
        concepts.sort_by(|a, b| b.activation.partial_cmp(&a.activation).unwrap());
        concepts.truncate(limit);
        concepts
    }

    /// Get total number of concepts
    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    /// Clear all concepts and relations
    pub fn clear(&mut self) {
        self.concepts.clear();
        self.relations.clear();
        self.name_index.clear();
    }
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_concept(name: &str) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: format!("Description of {}", name),
            memory_ids: Vec::new(),
            concept_type: ConceptType::General,
            activation: 1.0,
        }
    }

    #[test]
    fn test_add_concept() {
        let mut sm = SemanticMemory::new();
        let concept = create_test_concept("Authentication");
        let id = concept.id;

        sm.add_concept(concept);
        assert_eq!(sm.len(), 1);
        assert!(sm.get_concept(&id).is_some());
    }

    #[test]
    fn test_get_concept_by_name() {
        let mut sm = SemanticMemory::new();
        let concept = create_test_concept("Authentication");

        sm.add_concept(concept);

        let found = sm.get_concept_by_name("authentication");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Authentication");
    }

    #[test]
    fn test_add_relation() {
        let mut sm = SemanticMemory::new();

        let concept1 = create_test_concept("Login");
        let concept2 = create_test_concept("Authentication");
        let id1 = concept1.id;
        let id2 = concept2.id;

        sm.add_concept(concept1);
        sm.add_concept(concept2);

        let relation = ConceptRelation {
            from: id1,
            to: id2,
            relation_type: RelationType::PartOf,
            strength: 0.9,
        };

        sm.add_relation(relation);

        let relations = sm.get_relations(&id1);
        assert_eq!(relations.len(), 1);
    }

    #[test]
    fn test_get_related_concepts() {
        let mut sm = SemanticMemory::new();

        let concept1 = create_test_concept("Login");
        let concept2 = create_test_concept("Authentication");
        let id1 = concept1.id;
        let id2 = concept2.id;

        sm.add_concept(concept1);
        sm.add_concept(concept2);

        sm.add_relation(ConceptRelation {
            from: id1,
            to: id2,
            relation_type: RelationType::PartOf,
            strength: 0.9,
        });

        let related = sm.get_related_concepts(&id1);
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].0.name, "Authentication");
    }

    #[test]
    fn test_spreading_activation() {
        let mut sm = SemanticMemory::new();

        let concept1 = create_test_concept("Login");
        let concept2 = create_test_concept("Authentication");
        let id1 = concept1.id;
        let id2 = concept2.id;

        sm.add_concept(concept1);
        sm.add_concept(concept2);

        sm.add_relation(ConceptRelation {
            from: id1,
            to: id2,
            relation_type: RelationType::PartOf,
            strength: 0.8,
        });

        // Reset activations
        for concept in sm.concepts.values_mut() {
            concept.activation = 0.0;
        }

        // Activate concept1
        sm.activate(&id1, 1.0);

        // concept2 should be partially activated
        let concept2 = sm.get_concept(&id2).unwrap();
        assert!(concept2.activation > 0.0);
        assert!(concept2.activation < 1.0);
    }

    #[test]
    fn test_activation_decay() {
        let mut sm = SemanticMemory::new();
        let concept = create_test_concept("Test");
        let id = concept.id;

        sm.add_concept(concept);

        let initial = sm.get_concept(&id).unwrap().activation;
        sm.decay_activations();
        let after_decay = sm.get_concept(&id).unwrap().activation;

        assert!(after_decay < initial);
    }

    #[test]
    fn test_query() {
        let mut sm = SemanticMemory::new();

        sm.add_concept(create_test_concept("Authentication"));
        sm.add_concept(create_test_concept("Authorization"));
        sm.add_concept(create_test_concept("Database"));

        let results = sm.query("auth", 10);
        // Should find at least Authentication and Authorization
        assert!(results.len() >= 2);

        // Verify the concepts are actually auth-related
        let names: Vec<_> = results
            .iter()
            .filter_map(|id| sm.get_concept(id))
            .map(|c| c.name.as_str())
            .collect();
        assert!(names.contains(&"Authentication"));
        assert!(names.contains(&"Authorization"));
    }

    #[test]
    fn test_find_path() {
        let mut sm = SemanticMemory::new();

        let c1 = create_test_concept("A");
        let c2 = create_test_concept("B");
        let c3 = create_test_concept("C");

        let id1 = c1.id;
        let id2 = c2.id;
        let id3 = c3.id;

        sm.add_concept(c1);
        sm.add_concept(c2);
        sm.add_concept(c3);

        sm.add_relation(ConceptRelation {
            from: id1,
            to: id2,
            relation_type: RelationType::RelatedTo,
            strength: 1.0,
        });

        sm.add_relation(ConceptRelation {
            from: id2,
            to: id3,
            relation_type: RelationType::RelatedTo,
            strength: 1.0,
        });

        let path = sm.find_path(&id1, &id3, 5);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3); // A -> B -> C
    }

    #[test]
    fn test_get_most_activated() {
        let mut sm = SemanticMemory::new();

        let c1 = create_test_concept("High");
        let c2 = create_test_concept("Low");

        let id1 = c1.id;
        let id2 = c2.id;

        sm.add_concept(c1);
        sm.add_concept(c2);

        // Set activations after adding (since add_concept resets to 1.0)
        if let Some(concept) = sm.concepts.get_mut(&id1) {
            concept.activation = 0.9;
        }
        if let Some(concept) = sm.concepts.get_mut(&id2) {
            concept.activation = 0.3;
        }

        let most_activated = sm.get_most_activated(1);
        assert_eq!(most_activated.len(), 1);
        assert_eq!(most_activated[0].name, "High");
    }
}
