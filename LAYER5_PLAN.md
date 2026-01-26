# Layer 5: Meta-Learning Engine

## Overview

Il Meta-Learning Engine è il cervello che apprende da ogni progetto completato e usa queste conoscenze per migliorare le prestazioni future. È ciò che rende Sentinel un sistema **auto-migliorante**.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                  META-LEARNING ENGINE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │ Pattern Mining   │  │ Neural Networks  │  │ Knowledge Base │ │
│  │  Engine          │  │  (PyTorch)       │  │  (Neo4j)       │ │
│  └──────────────────┘  └──────────────────┘  └────────────────┘ │
│           │                     │                     │         │
│           └─────────────────────┼─────────────────────┘         │
│                                 ▼                               │
│                   ┌──────────────────────────┐                  │
│                   │   Strategy Synthesizer    │                  │
│                   │   (Cross-project learning) │                  │
│                   └──────────────────────────┘                  │
│                                  │                              │
│                                  ▼                              │
│                    ┌──────────────────────────┐                 │
│                    │   Prediction API         │                 │
│                    │  - Deviation risk        │                 │
│                    │  - Optimal strategies    │                 │
│                    │  - Time estimates         │                 │
│                    └──────────────────────────┘                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. Pattern Mining Engine
**File**: `crates/sentinel-core/src/learning/pattern_mining.rs`

Funzionalità:
- Estrazione di pattern sequenziali da azioni di successo
- Frequent Pattern Mining (Algoritmo FP-Growth)
- Classificazione dei pattern per tipo di goal
- Rilevamento di pattern anti-pattern (deviazioni comuni)

Strutture dati:
```rust
pub struct PatternMiningEngine {
    min_support: f64,      // Soglia supporto (default: 0.3)
    min_confidence: f64,  // Soglia confidenza (default: 0.7)
    pattern_cache: LruCache<PatternKey, Pattern>,
}

pub struct SuccessPattern {
    pub id: Uuid,
    pub name: String,
    pub action_sequence: Vec<ActionType>,
    pub applicable_to_goal_types: Vec<GoalType>,
    pub success_rate: f64,           // 0.0-1.0
    pub support: usize,              // Numero progetti
    pub preconditions: Vec<String>,
    pub expected_outcomes: Vec<String>,
}
```

### 2. Neural Networks (Python Integration)
**File**: `crates/sentinel-core/src/learning/classifier.rs`

Funzionalità:
- Deviation Classifier: Prevede probabilità di deviazione
- Policy Network: Suggerisce correzioni ottimali
- Value Estimator: Stima valore-to-root di azioni
- Feature Extraction: Da project state e action context

Integrazione Python:
```rust
pub struct DeviationClassifier {
    model_path: PathBuf,
    python_runtime: PythonRuntime,
}

pub struct DeviationRisk {
    pub probability: f64,              // 0.0-1.0
    pub similar_past_cases: Vec<DeviationCase>,
    pub risk_factors: Vec<RiskFactor>,
    pub recommended_precautions: Vec<Action>,
}
```

### 3. Knowledge Base (Neo4j)
**File**: `crates/sentinel-core/src/learning/knowledge_base.rs`

Funzionalità:
- Storage di pattern appresi
- Relazioni tra pattern
- Meta-dati cross-project
- Retrieval per goal similarity

Schema Neo4j:
```
(Pattern:Pattern)
  -[:APPLICABLE_TO]->(GoalType:GoalType)
  -[:RELATED_TO {strength: float}]->(Pattern:Pattern)
  -[:LEARNED_FROM]->(Project:Project)

(Goal:Goal)
  -[:HAS_PATTERN]->(Pattern:Pattern)
  -[:SIMILAR_TO {score: float}]->(Goal:Goal)

(Project:Project)
  -[:ACHIEVED]->(Goal:Goal)
  -[:USED_PATTERN]->(Pattern:Pattern)
```

### 4. Strategy Synthesizer
**File**: `crates/sentinel-core/src/learning/strategy.rs`

Funzionalità:
- Sintesi di strategie da pattern multipli
- Fusione di approcci di successo
- Calcolo della confidence
- Stima del tempo di completamento

Struttura:
```rust
pub struct Strategy {
    pub recommended_approaches: Vec<SuccessPattern>,
    pub pitfalls_to_avoid: Vec<DeviationPattern>,
    pub estimated_completion_time: Duration,
    pub confidence: f64,               // 0.0-1.0
    pub rationale: String,              // Spiegazione generata da LLM
}

pub struct StrategySynthesizer {
    knowledge_base: KnowledgeBase,
    llm_client: LlmClient,              // Claude/GPT per rationale
}
```

## Implementation Plan

### Week 1: Pattern Mining Engine

**Tasks**:
- [ ] Implementare struttura dati `PatternMiningEngine`
- [ ] Algoritmo FP-Growth per frequent pattern mining
- [ ] Feature extraction da azioni (action type, context, alignment score)
- [ ] Pattern classification per goal type
- [ ] Unit tests per pattern mining

**Deliverables**:
```
crates/sentinel-core/src/learning/
├── mod.rs
├── pattern_mining.rs       ← NEW
└── types.rs                ← NEW
```

### Week 2: Knowledge Base Integration

**Tasks**:
- [ ] Setup Neo4j connection
- [ ] Schema definition (migrations)
- [ ] CRUD operations per Pattern
- [ ] Retrieval per goal similarity
- [ ] Relationship tracking
- [ ] Unit tests

**Deliverables**:
```
crates/sentinel-core/src/learning/
├── knowledge_base.rs       ← NEW
└── schema.cypher           ← NEW
```

### Week 3: Deviation Classifier (Python Integration)

**Tasks**:
- [ ] Setup Python environment (PyTorch)
- [ ] Feature extraction pipeline
- [ ] Model architecture (Transformer-based)
- [ ] Training pipeline
- [ ] Prediction API
- [ ] Rust bindings

**Deliverables**:
```
crates/sentinel-core/src/learning/
└── classifier.rs           ← NEW

learning/
├── train_classifier.py     ← NEW
├── model.py                ← NEW
└── requirements.txt        ← NEW
```

### Week 4: Strategy Synthesizer

**Tasks**:
- [ ] Implementare `StrategySynthesizer`
- [ ] Pattern retrieval e ranking
- [ ] LLM integration per rationale generation
- [ ] Confidence calculation
- [ ] Time estimation
- [ ] Integration tests

**Deliverables**:
```
crates/sentinel-core/src/learning/
└── strategy.rs             ← NEW
```

### Week 5: Integration & Testing

**Tasks**:
- [ ] End-to-end integration test
- [ ] Performance benchmarking
- [ ] Synthetic project generation for testing
- [ ] A/B testing (with/without meta-learning)
- [ ] Documentation

**Deliverables**:
- Test suite completo
- Performance report
- API documentation

## Key Technical Decisions

### 1. Feature Extraction

**Decisione**: Usare feature ibride (strutturali + semantiche)

**Rationale**:
- Strutturali: Action type, duration, file types
- Semantiche: Embeddings delle azioni (usando Candle)
- Contextuali: Goal type, dependency graph state

### 2. Pattern Mining Algorithm

**Decisione**: FP-Growth invece di Apriori

**Rationale**:
- FP-Growth è O(n) vs Apriori O(2^n)
- Migliore per dataset grandi
- Cache-friendly

### 3. Neural Architecture

**Decisione**: Transformer encoder + classification head

**Rationale**:
- Capace di catturare dipendenze temporali
- State-of-the-art per sequence classification
- Transfer learning possible

### 4. Knowledge Base

**Decisione**: Neo4j (graph DB) invece di SQL/vector DB

**Rationale**:
- Pattern hanno relazioni complesse
- Neo4j ottimizzato per graph traversal
- Built-in graph algorithms

## Success Criteria

### Technical Metrics
- **Pattern Mining**: >90% precision on known patterns
- **Deviation Prediction**: >80% accuracy
- **Strategy Recommendation**: >75% user satisfaction
- **Performance**: <500ms per prediction

### Learning Metrics
- **Knowledge Base**: 100+ patterns after 50 projects
- **Confidence**: Confidence increases with more projects
- **Transfer**: Patterns learned in one project apply to others

### Integration Metrics
- **Zero-Degradation**: System doesn't get worse
- **Positive Feedback Loop**: Each project improves future performance
- **Cold Start**: Works reasonably well from day 1

## Dependencies

### New Dependencies
```toml
# Cargo.toml (Rust)
neo4rs = "0.7"           # Neo4j client
pyo3 = "0.20"            # Python bindings
numpy = "0.20"           # NumPy arrays

# requirements.txt (Python)
torch = "2.1.0"
transformers = "4.35.0"
scikit-learn = "1.3.0"
```

### Infrastructure
- Neo4j instance (Docker or cloud)
- Python 3.11+ environment
- PyTorch installation

## API Design

```rust
// Pattern Mining
impl PatternMiningEngine {
    pub async fn extract_patterns(
        &self,
        project: &CompletedProject
    ) -> Result<Vec<SuccessPattern>>;

    pub async fn classify_pattern(
        &self,
        pattern: &Pattern
    ) -> Result<GoalType>;
}

// Deviation Prediction
impl DeviationClassifier {
    pub async fn predict_deviation_risk(
        &self,
        action: &Action,
        context: &Context
    ) -> Result<DeviationRisk>;
}

// Knowledge Base
impl KnowledgeBase {
    pub async fn store_pattern(
        &self,
        pattern: &SuccessPattern
    ) -> Result<()>;

    pub async fn find_applicable_patterns(
        &self,
        goal: &Goal
    ) -> Result<Vec<SuccessPattern>>;
}

// Strategy Synthesis
impl StrategySynthesizer {
    pub async fn suggest_strategy(
        &self,
        goal: &Goal
    ) -> Result<Strategy>;
}
```

## Testing Strategy

### Unit Tests
- Pattern mining con dataset sintetico
- Feature extraction correctness
- Knowledge base CRUD
- Neural model inference

### Integration Tests
- End-to-end: project completion → pattern extraction → retrieval
- Multi-project learning: learn from 10 projects → predict for 11th
- Cold start: works with zero knowledge

### Performance Tests
- Pattern mining: <10s for 1000-action project
- Deviation prediction: <100ms
- Strategy synthesis: <1s

## Risks & Mitigations

### Risk 1: Cold Start Problem
**Mitigazione**: Pre-trained modello + few-shot learning + baseline rules

### Risk 2: Overfitting to Specific Patterns
**Mitigazione**: Regularization, diversity sampling, adversarial testing

### Risk 3: Slow Learning Rate
**Mitigazione**: Incremental updates, transfer learning from similar goals

### Risk 4: Memory Explosion
**Mitigazione**: Pattern pruning (low support), LRU cache, summarization

## Timeline Summary

```
Week 1: Pattern Mining Engine
Week 2: Knowledge Base Integration
Week 3: Deviation Classifier (Python)
Week 4: Strategy Synthesizer
Week 5: Integration & Testing

Total: 5 weeks
```

## Next Steps

1. ✅ Review this plan
2. ✅ Setup infrastructure (Neo4j, Python)
3. ✅ Start Week 1 implementation
4. ✅ Daily progress tracking
5. ✅ Weekly review meetings

---

**Document Version**: 1.0
**Last Updated**: 2026-01-25
**Status**: Ready to Implement
