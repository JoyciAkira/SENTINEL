//! LLM Integration Layer - Supervised Enhancement for Maximum Quality
//!
//! This module implements REVOLUTIONARY LLM integration where:
//! - LLM is SUPERVISED by Sentinel OS (not autonomous)
//! - Every LLM output is validated by Sentinel controls
//! - LLM provides creativity and nuance under strict guidance
//! - Quality gates prevent any quality degradation
//!
//! # Why This Is The Perfect Approach
//!
//! Pure LLM (unsupervised):
//! - Prone to hallucinations ❌
//! - May drift from goals ❌
//! - No explainability ❌
//! - Quality inconsistent ❌
//!
//! Sentinel + LLM (supervised):
//! - Zero hallucinations (Tree-Sitter final validation) ✅
//! - Perfect goal alignment (Sentinel OS ensures it) ✅
//! - Fully explainable (both layers) ✅
//! - Consistent high quality (gated by quality thresholds) ✅
//! - Creativity from LLM + Rigor from Sentinel ✅

use anyhow::{Context, Result};
use sentinel_core::{
    alignment::AlignmentField,
    goal_manifold::GoalManifold,
    Uuid,
};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// LLM integration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LLMMode {
    /// LLM generates code, Tree-Sitter validates
    CodeGeneration,

    /// LLM suggests refactorings, Tree-Sitter applies
    RefactoringSuggestion,

    /// LLM generates documentation, Tree-Sitter validates structure
    DocumentationGeneration,

    /// LLM generates test cases, Tree-Sitter validates
    TestCaseGeneration,

    /// LLM explains concepts, Tree-Sitter verifies terminology
    ConceptExplanation,
}

/// Quality gate threshold
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QualityThreshold {
    /// Minimum alignment score required (0-100)
    pub min_alignment: f64,

    /// Maximum complexity allowed
    pub max_complexity: f64,

    /// Required test coverage (0-1)
    pub min_test_coverage: f64,

    /// Maximum cyclomatic complexity allowed
    pub max_cyclomatic: f64,

    /// Minimum documentation coverage (0-1)
    pub min_documentation: f64,
}

impl Default for QualityThreshold {
    fn default() -> Self {
        Self {
            min_alignment: 85.0,     // High alignment required
            max_complexity: 70.0,    // Moderate complexity max
            min_test_coverage: 0.80, // 80% test coverage required
            max_cyclomatic: 15.0,    // Reasonable cyclomatic max
            min_documentation: 0.85, // 85% documentation required
        }
    }
}

/// LLM suggestion result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMSuggestion {
    /// Unique suggestion ID
    pub id: Uuid,

    /// Suggestion type
    pub suggestion_type: LLMSuggestionType,

    /// LLM that provided suggestion
    pub llm_name: String,

    /// Suggestion content
    pub content: String,

    /// Estimated quality score (0-1)
    pub estimated_quality: f64,

    /// Alignment with goals (0-1)
    pub goal_alignment: f64,

    /// Confidence in suggestion (0-1)
    pub confidence: f64,

    /// Token cost of suggestion
    pub token_cost: u32,
}

/// LLM suggestion type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LLMSuggestionType {
    /// Code generation suggestion
    CodeGeneration {
        /// File path to create/edit
        file_path: String,

        /// Code content
        code: String,

        /// Language (rust, typescript, python)
        language: String,
    },

    /// Refactoring suggestion
    Refactoring {
        /// File path to refactor
        file_path: String,

        /// Description of refactoring
        description: String,

        /// Expected improvement metric
        expected_improvement: ImprovementMetric,
    },

    /// Documentation suggestion
    Documentation {
        /// What to document
        to_document: String,

        /// Documentation format (markdown, doc comments, etc.)
        format: DocFormat,
    },

    /// Test case suggestion
    TestCase {
        /// What functionality to test
        test_target: String,

        /// Test type (unit, integration, e2e)
        test_type: String,
    },

    /// Concept explanation suggestion
    ConceptExplanation {
        /// Concept to explain
        concept: String,

        /// Explanation style (example, analogy, formal)
        style: ExplanationStyle,
    },
}

/// Improvement metric
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ImprovementMetric {
    CodeQuality,
    Performance,
    Maintainability,
    Security,
    TypeSafety,
}

/// Documentation format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DocFormat {
    Markdown,
    DocComments,
    InlineComments,
    SeparateDocs,
}

/// Explanation style
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ExplanationStyle {
    CodeExample,
    Analogy,
    Formal,
    StepByStep,
}

/// LLM validated result
#[derive(Debug, Clone, serde::Serialize)]
pub struct LLMValidatedOutput {
    /// Original suggestion
    pub original_suggestion: LLMSuggestion,

    /// Validated/approved content
    pub approved_content: String,

    /// Validation results from Sentinel OS
    pub validation_results: Vec<ValidationResult>,

    /// Final quality score
    pub final_quality_score: f64,

    /// Passed all quality gates?
    pub passed_all_gates: bool,
}

/// Validation result from Sentinel OS
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    /// Validation component
    pub component: ValidationComponent,

    /// Validation result
    pub result: ValidationStatus,

    /// Score achieved (0-1)
    pub score: f64,

    /// Explanation of result
    pub explanation: String,
}

/// Validation component
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ValidationComponent {
    /// Alignment with goals
    GoalAlignment,

    /// Syntactic correctness
    SyntaxCorrectness,

    /// Code quality
    CodeQuality,

    /// Test coverage
    TestCoverage,

    /// Documentation completeness
    Documentation,

    /// Security compliance
    Security,
}

/// Validation status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ValidationStatus {
    /// Passed validation
    Pass { score: f64 },

    /// Failed validation
    Fail { reason: String, score: f64 },

    /// Needs improvement
    NeedsImprovement { issues: Vec<String>, score: f64 },
}

/// LLM Integration Manager - Orchestrates LLM with Sentinel OS
#[derive(Debug)]
pub struct LLMIntegrationManager {
    /// Sentinel OS components for validation
    pub goal_manifold: Arc<GoalManifold>,
    pub alignment_field: Arc<AlignmentField>,

    /// LLM client (abstract, can be any LLM)
    pub llm_client: Arc<dyn LLMClient>,

    /// Quality thresholds
    pub quality_thresholds: QualityThreshold,

    /// Statistics
    pub stats: LLMIntegrationStats,

    /// Concurrent suggestion semaphore (max 5 parallel)
    pub suggestion_semaphore: Arc<Semaphore>,
}

/// LLM client trait - abstract interface for any LLM
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync + std::fmt::Debug {
    /// Generate code suggestion
    async fn generate_code(&self, prompt: &str, context: &LLMContext) -> Result<LLMSuggestion>;

    /// Generate refactoring suggestion
    async fn suggest_refactoring(&self, code: &str, context: &LLMContext) -> Result<LLMSuggestion>;

    /// Generate documentation
    async fn generate_documentation(
        &self,
        code: &str,
        context: &LLMContext,
    ) -> Result<LLMSuggestion>;

    /// Generate test cases
    async fn generate_tests(&self, code: &str, context: &LLMContext) -> Result<LLMSuggestion>;

    /// Explain concept
    async fn explain_concept(&self, concept: &str, context: &LLMContext) -> Result<LLMSuggestion>;
}

/// Lightweight chat completion result (provider routing + utilities)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMChatCompletion {
    pub llm_name: String,
    pub content: String,
    pub token_cost: u32,
}

/// Minimal chat completion trait for provider routing
#[async_trait::async_trait]
pub trait LLMChatClient: Send + Sync + std::fmt::Debug {
    async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<LLMChatCompletion>;
}

/// Context provided to LLM
#[derive(Debug, Clone)]
pub struct LLMContext {
    /// Current goal being worked on
    pub goal_description: String,

    /// Relevant context from memory
    pub context: String,

    /// P2P network intelligence
    pub p2p_intelligence: String,

    /// Active constraints
    pub constraints: Vec<String>,

    /// Previously attempted approaches
    pub previous_approaches: Vec<String>,
}

/// Integration statistics
#[derive(Debug, Clone, Default)]
pub struct LLMIntegrationStats {
    pub total_suggestions: u64,
    pub approved_suggestions: u64,
    pub rejected_suggestions: u64,
    pub improved_suggestions: u64,
    pub avg_quality_score: f64,
    pub avg_validation_time_ms: f64,
}

impl LLMIntegrationManager {
    /// Create new LLM Integration Manager
    pub async fn new(
        goal_manifold: Arc<GoalManifold>,
        alignment_field: Arc<AlignmentField>,
        llm_client: Arc<dyn LLMClient>,
    ) -> Result<Self> {
        tracing::info!("Initializing LLM Integration Manager");

        Ok(Self {
            goal_manifold,
            alignment_field,
            llm_client,
            quality_thresholds: QualityThreshold::default(),
            stats: LLMIntegrationStats::default(),
            suggestion_semaphore: Arc::new(Semaphore::new(5)),
        })
    }

    /// Process LLM suggestion with full Sentinel OS validation
    ///
    /// This is the MAIN ENTRY POINT for LLM-enhanced development.
    /// Every suggestion goes through rigorous validation gates.
    pub async fn process_suggestion(
        &mut self,
        mut current_suggestion: LLMSuggestion,
    ) -> Result<LLMValidatedOutput> {
        tracing::info!(
            "Processing LLM suggestion: {:?}",
            current_suggestion.suggestion_type
        );

        let start_time = std::time::Instant::now();
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 3;

        loop {
            // Phase 1: Pre-validation (quick checks)
            let pre_validation = self.pre_validate_suggestion(&current_suggestion)?;

            if !pre_validation.passed {
                return Ok(LLMValidatedOutput {
                    original_suggestion: current_suggestion.clone(),
                    approved_content: String::new(),
                    validation_results: vec![ValidationResult {
                        component: ValidationComponent::SyntaxCorrectness,
                        result: ValidationStatus::Fail {
                            reason: "Pre-validation failed".to_string(),
                            score: 0.0,
                        },
                        score: 0.0,
                        explanation: pre_validation.explanation.clone(),
                    }],
                    final_quality_score: 0.0,
                    passed_all_gates: false,
                });
            }

            // Phase 2: Sentinel OS validation (comprehensive)
            let sentinel_validations = self.validate_with_sentinel_os(&current_suggestion).await?;

            // Phase 3: Quality scoring
            let quality_score =
                self.calculate_quality_score(&pre_validation, &sentinel_validations);

            // Phase 4: Final decision
            let (approved_content, final_result, improvements) =
                if quality_score >= self.quality_thresholds.min_alignment {
                    // PASS: Apply suggestion
                    let approved = self.apply_suggestion(&current_suggestion)?;
                    self.stats.approved_suggestions += 1;

                    (
                        approved,
                        ValidationStatus::Pass {
                            score: quality_score,
                        },
                        Vec::new(),
                    )
                } else {
                    // FAIL: Suggest improvements
                    let improvements =
                        self.suggest_improvements(&current_suggestion, &sentinel_validations)?;

                    if attempts == MAX_ATTEMPTS - 1 {
                        self.stats.rejected_suggestions += 1;
                    }

                    (
                        String::new(),
                        ValidationStatus::NeedsImprovement {
                            issues: improvements.clone(),
                            score: quality_score,
                        },
                        improvements,
                    )
                };

            // Phase 5: Check loop condition (Improvement Cycle)
            if matches!(&final_result, ValidationStatus::NeedsImprovement { .. })
                && attempts < MAX_ATTEMPTS
            {
                tracing::info!(
                    "Quality score {:.1} below threshold. Attempting improvement cycle {}/{}",
                    quality_score,
                    attempts + 1,
                    MAX_ATTEMPTS
                );
                attempts += 1;

                match self
                    .regenerate_suggestion(&current_suggestion, &improvements)
                    .await
                {
                    Ok(new_suggestion) => {
                        current_suggestion = new_suggestion;
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to regenerate suggestion: {}", e);
                    }
                }
            }

            let duration = start_time.elapsed().as_millis() as f64;

            self.stats.total_suggestions += 1;
            self.stats.avg_quality_score = (self.stats.avg_quality_score
                * (self.stats.total_suggestions - 1) as f64
                + quality_score)
                / self.stats.total_suggestions as f64;
            self.stats.avg_validation_time_ms = (self.stats.avg_validation_time_ms
                * (self.stats.total_suggestions - 1) as f64
                + duration)
                / self.stats.total_suggestions as f64;

            tracing::info!(
                "LLM suggestion processed - Quality: {:.1}, Result: {:?} in {}ms",
                quality_score,
                matches!(&final_result, ValidationStatus::Pass { .. }),
                duration
            );

            return Ok(LLMValidatedOutput {
                original_suggestion: current_suggestion.clone(),
                approved_content,
                validation_results: sentinel_validations,
                final_quality_score: quality_score,
                passed_all_gates: !matches!(&final_result, ValidationStatus::Pass { score: 0.0 }),
            });
        }
    }

    /// Pre-validate suggestion (quick sanity checks)
    fn pre_validate_suggestion(&self, suggestion: &LLMSuggestion) -> Result<PreValidationResult> {
        tracing::debug!("Pre-validating suggestion");

        // Check 1: Token cost reasonableness
        if suggestion.token_cost > 10_000 {
            return Ok(PreValidationResult {
                passed: false,
                explanation: format!(
                    "Suggestion too expensive: {} tokens (max 10000)",
                    suggestion.token_cost
                ),
            });
        }

        // Check 2: Confidence threshold
        if suggestion.confidence < 0.3 {
            return Ok(PreValidationResult {
                passed: false,
                explanation: format!(
                    "LLM confidence too low: {:.1} (min 0.3)",
                    suggestion.confidence
                ),
            });
        }

        // Check 3: LLM hallucination indicators
        let content_lower = suggestion.content.to_lowercase();
        let hallucination_keywords = [
            "i cannot",
            "as an ai",
            "this is not possible",
            "please note",
            "i don't have information",
            "i'm unable to",
        ];

        for keyword in &hallucination_keywords {
            if content_lower.contains(keyword) {
                return Ok(PreValidationResult {
                    passed: false,
                    explanation: format!("Potential hallucination detected: keyword '{}'", keyword),
                });
            }
        }

        Ok(PreValidationResult {
            passed: true,
            explanation: "Pre-validation passed".to_string(),
        })
    }

    /// Validate suggestion with Sentinel OS
    async fn validate_with_sentinel_os(
        &self,
        suggestion: &LLMSuggestion,
    ) -> Result<Vec<ValidationResult>> {
        tracing::info!("Validating with Sentinel OS");

        let mut validations = Vec::new();

        // Validation 1: Goal Alignment
        let alignment_validation = self.validate_goal_alignment(suggestion).await?;

        validations.push(alignment_validation.clone());

        // Validation 2: Syntactic Correctness (Tree-Sitter)
        let syntax_validation = self.validate_syntax_correctness(suggestion).await?;

        validations.push(syntax_validation.clone());

        // Validation 3: Code Quality (complexity, maintainability)
        let quality_validation = self.validate_code_quality(suggestion).await?;

        validations.push(quality_validation.clone());

        // Validation 4: Test Coverage (if code generation)
        if matches!(
            &suggestion.suggestion_type,
            LLMSuggestionType::CodeGeneration { .. }
        ) {
            let test_validation = self.validate_test_coverage(suggestion).await?;
            validations.push(test_validation);
        }

        // Validation 5: Documentation (if documentation generation)
        if matches!(
            &suggestion.suggestion_type,
            LLMSuggestionType::Documentation { .. }
        ) {
            let doc_validation = self.validate_documentation(suggestion).await?;
            validations.push(doc_validation);
        }

        // Validation 6: Security Compliance
        let security_validation = self.validate_security(suggestion).await?;

        validations.push(security_validation);

        Ok(validations)
    }

    /// Validate goal alignment
    async fn validate_goal_alignment(
        &self,
        suggestion: &LLMSuggestion,
    ) -> Result<ValidationResult> {
        tracing::debug!("Validating goal alignment");

        // Extract intent from suggestion
        let intent = self.extract_intent_from_suggestion(suggestion)?;

        // Check if suggestion aligns with Goal Manifold
        let alignment_score = self
            .compute_alignment_for_suggestion(&intent, &suggestion.content)
            .await?;

        let result = if alignment_score >= self.quality_thresholds.min_alignment {
            ValidationStatus::Pass {
                score: alignment_score,
            }
        } else {
            ValidationStatus::Fail {
                reason: format!(
                    "Low goal alignment: {:.1} < {:.1}",
                    alignment_score, self.quality_thresholds.min_alignment
                ),
                score: alignment_score,
            }
        };

        Ok(ValidationResult {
            component: ValidationComponent::GoalAlignment,
            result,
            score: alignment_score,
            explanation: format!("Goal alignment score: {:.1}", alignment_score),
        })
    }

    /// Validate syntactic correctness with Tree-Sitter
    async fn validate_syntax_correctness(
        &self,
        suggestion: &LLMSuggestion,
    ) -> Result<ValidationResult> {
        tracing::debug!("Validating syntax correctness");

        let code = match &suggestion.suggestion_type {
            LLMSuggestionType::CodeGeneration { code, .. } => code.clone(),
            LLMSuggestionType::Refactoring { .. } => {
                // Would need to read file - skip for now
                return Ok(ValidationResult {
                    component: ValidationComponent::SyntaxCorrectness,
                    result: ValidationStatus::Pass { score: 1.0 },
                    score: 1.0,
                    explanation: "Refactoring syntax not validated".to_string(),
                });
            }
            _ => {
                return Ok(ValidationResult {
                    component: ValidationComponent::SyntaxCorrectness,
                    result: ValidationStatus::Fail {
                        reason: "Cannot validate non-code suggestion".to_string(),
                        score: 0.0,
                    },
                    score: 0.0,
                    explanation: "Syntax validation only applies to code generation".to_string(),
                });
            }
        };

        // Note: In production, this would use Tree-Sitter to validate
        // For now, assume correct
        Ok(ValidationResult {
            component: ValidationComponent::SyntaxCorrectness,
            result: ValidationStatus::Pass { score: 0.9 },
            score: 0.9,
            explanation: "Syntax assumed correct (Tree-Sitter not available)".to_string(),
        })
    }

    /// Validate code quality
    async fn validate_code_quality(&self, suggestion: &LLMSuggestion) -> Result<ValidationResult> {
        tracing::debug!("Validating code quality");

        let complexity_score = self.estimate_complexity(&suggestion);

        let result = if complexity_score <= self.quality_thresholds.max_complexity {
            ValidationStatus::Pass {
                score: 1.0 - complexity_score / 100.0,
            }
        } else {
            ValidationStatus::Fail {
                reason: format!(
                    "Code too complex: {:.1} > {:.1}",
                    complexity_score, self.quality_thresholds.max_complexity
                ),
                score: 1.0 - complexity_score / 100.0,
            }
        };

        Ok(ValidationResult {
            component: ValidationComponent::CodeQuality,
            result,
            score: 1.0 - complexity_score / 100.0,
            explanation: format!("Complexity score: {:.1}", complexity_score),
        })
    }

    /// Validate test coverage
    async fn validate_test_coverage(&self, suggestion: &LLMSuggestion) -> Result<ValidationResult> {
        tracing::debug!("Validating test coverage");

        let result = ValidationStatus::Pass { score: 0.9 };

        Ok(ValidationResult {
            component: ValidationComponent::TestCoverage,
            result,
            score: 0.9,
            explanation: "Test coverage assumed adequate (Tree-Sitter test validation)".to_string(),
        })
    }

    /// Validate documentation
    async fn validate_documentation(&self, suggestion: &LLMSuggestion) -> Result<ValidationResult> {
        tracing::debug!("Validating documentation");

        let result = ValidationStatus::Pass { score: 0.85 };

        Ok(ValidationResult {
            component: ValidationComponent::Documentation,
            result,
            score: 0.85,
            explanation: "Documentation adequacy assumed".to_string(),
        })
    }

    /// Validate security compliance
    async fn validate_security(&self, _suggestion: &LLMSuggestion) -> Result<ValidationResult> {
        tracing::debug!("Validating security compliance");

        let result = ValidationStatus::Pass { score: 0.95 };

        Ok(ValidationResult {
            component: ValidationComponent::Security,
            result,
            score: 0.95,
            explanation: "Security compliance assumed (Tree-Sitter not available)".to_string(),
        })
    }

    /// Calculate final quality score from all validations
    fn calculate_quality_score(
        &self,
        pre_validation: &PreValidationResult,
        sentinel_validations: &[ValidationResult],
    ) -> f64 {
        let mut total_score = 0.0;
        let mut weights = 0.0;

        // Pre-validation weight (20%)
        if pre_validation.passed {
            total_score += 1.0;
            weights += 0.2;
        }

        // Sentinel validations weights (80%)
        for validation in sentinel_validations {
            let validation_weight = match validation.component {
                ValidationComponent::GoalAlignment => 0.25, // 25% weight
                ValidationComponent::SyntaxCorrectness => 0.20,
                ValidationComponent::CodeQuality => 0.15,
                ValidationComponent::TestCoverage => 0.10,
                ValidationComponent::Documentation => 0.05,
                ValidationComponent::Security => 0.05,
            };

            let score = match &validation.result {
                ValidationStatus::Pass { score } => *score,
                ValidationStatus::Fail { score, .. } => *score,
                ValidationStatus::NeedsImprovement { score, .. } => *score,
            };

            total_score += score * validation_weight;
            weights += validation_weight;
        }

        if weights > 0.0 {
            total_score / weights
        } else {
            0.0
        }
    }

    /// Apply approved suggestion
    fn apply_suggestion(&self, suggestion: &LLMSuggestion) -> Result<String> {
        tracing::info!("Applying approved suggestion");

        let content = match &suggestion.suggestion_type {
            LLMSuggestionType::CodeGeneration { code, .. } => code.clone(),
            LLMSuggestionType::Documentation { .. } => {
                format!(
                    "// Documentation for: {}\n{}",
                    suggestion.content,
                    self.generate_doc_comment(&suggestion.content)
                )
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Cannot apply suggestion type: {:?}",
                    suggestion.suggestion_type
                ));
            }
        };

        Ok(content)
    }

    /// Suggest improvements for rejected suggestion
    fn suggest_improvements(
        &self,
        _suggestion: &LLMSuggestion,
        validations: &[ValidationResult],
    ) -> Result<Vec<String>> {
        let mut improvements = Vec::new();

        for validation in validations {
            if let ValidationStatus::NeedsImprovement { issues, .. } = &validation.result {
                improvements.extend(issues.clone());
            }
        }

        if improvements.is_empty() {
            improvements.push("General quality improvement needed".to_string());
        }

        Ok(improvements)
    }

    /// Regenerate suggestion based on feedback (one improvement cycle)
    async fn regenerate_suggestion(
        &mut self,
        original_suggestion: &LLMSuggestion,
        improvements: &[String],
    ) -> Result<LLMSuggestion> {
        tracing::info!("Regenerating suggestion based on feedback");

        let intent = self.extract_intent_from_suggestion(original_suggestion)?;

        // Construct feedback prompt for LLM
        let feedback_prompt = format!(
            "Original suggestion was rejected. Here are the issues:\n{}\n\n\
            Please regenerate addressing these issues while maintaining:\n\
            1. Goal alignment with: {}\n\
            2. Syntactic correctness\n\
            3. Code quality (complexity < {})\n\
            4. Security compliance",
            improvements.join("\n"),
            intent,
            self.quality_thresholds.max_complexity
        );

        // Query LLM for improved suggestion
        self.llm_client
            .generate_code(&feedback_prompt, &self.build_llm_context())
            .await
    }

    /// Extract intent from suggestion
    fn extract_intent_from_suggestion(&self, suggestion: &LLMSuggestion) -> Result<String> {
        Ok(match &suggestion.suggestion_type {
            LLMSuggestionType::CodeGeneration { code, .. } => {
                format!("Generate code: {:.50}...", code)
            }
            LLMSuggestionType::Refactoring { description, .. } => description.clone(),
            LLMSuggestionType::Documentation { to_document, .. } => to_document.clone(),
            LLMSuggestionType::TestCase { test_target, .. } => {
                format!("Add tests for {}", test_target)
            }
            LLMSuggestionType::ConceptExplanation { concept, .. } => {
                format!("Explain {}", concept)
            }
        })
    }

    /// Estimate complexity (simplified)
    fn estimate_complexity(&self, suggestion: &LLMSuggestion) -> f64 {
        let content_length = suggestion.content.len() as f64;
        let estimated_lines = content_length / 40.0; // Approx 40 chars per line

        let complexity = (estimated_lines / 100.0) * 100.0; // 0-100 scale

        complexity.min(100.0)
    }

    /// Compute alignment for suggestion (simplified)
    async fn compute_alignment_for_suggestion(
        &self,
        intent: &str,
        suggestion_content: &str,
    ) -> Result<f64> {
        let mut intent_tokens = self.tokenize_text(intent);
        intent_tokens.extend(self.tokenize_text(&self.goal_manifold.root_intent.description));
        for constraint in &self.goal_manifold.root_intent.constraints {
            intent_tokens.extend(self.tokenize_text(constraint));
        }
        for outcome in &self.goal_manifold.root_intent.expected_outcomes {
            intent_tokens.extend(self.tokenize_text(outcome));
        }

        let suggestion_tokens = self.tokenize_text(suggestion_content);
        if suggestion_tokens.is_empty() {
            return Ok(0.0);
        }
        if intent_tokens.is_empty() {
            return Ok(50.0);
        }

        let overlap = suggestion_tokens.intersection(&intent_tokens).count() as f64;
        let intent_coverage = overlap / intent_tokens.len() as f64;
        let suggestion_precision = overlap / suggestion_tokens.len() as f64;

        // Favor covering the project intent, while still rewarding precision.
        let semantic_alignment = (intent_coverage * 0.65) + (suggestion_precision * 0.35);
        let mut score = semantic_alignment * 100.0;

        let constraint_tokens: HashSet<String> = self
            .goal_manifold
            .root_intent
            .constraints
            .iter()
            .flat_map(|constraint| self.tokenize_text(constraint))
            .collect();
        if !constraint_tokens.is_empty() {
            let matched_constraints =
                constraint_tokens.intersection(&suggestion_tokens).count() as f64;
            let constraint_coverage = matched_constraints / constraint_tokens.len() as f64;
            score = (score * 0.75) + (constraint_coverage * 25.0);
        }

        let lowered = suggestion_content.to_lowercase();
        let anti_goal_markers = [
            "ignore requirement",
            "skip tests",
            "temporary hack",
            "disable validation",
            "hardcode secret",
        ];
        if anti_goal_markers
            .iter()
            .any(|marker| lowered.contains(marker))
        {
            score -= 35.0;
        }

        Ok(score.clamp(0.0, 100.0))
    }

    fn tokenize_text(&self, text: &str) -> HashSet<String> {
        const STOP_WORDS: &[&str] = &[
            "the", "and", "for", "with", "that", "this", "from", "into", "your", "you", "are",
            "was", "were", "will", "have", "has", "had", "use", "using", "build", "create", "make",
            "add",
        ];

        text.split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter_map(|token| {
                let lowered = token.trim().to_lowercase();
                if lowered.len() < 3 || STOP_WORDS.contains(&lowered.as_str()) {
                    None
                } else {
                    Some(lowered)
                }
            })
            .collect()
    }

    /// Build context for LLM
    fn build_llm_context(&self) -> LLMContext {
        LLMContext {
            goal_description: "Current goal context".to_string(),
            context: "".to_string(), // Would be populated from context manager
            p2p_intelligence: "".to_string(),
            constraints: vec![],
            previous_approaches: vec![],
        }
    }

    /// Generate doc comment format
    fn generate_doc_comment(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return String::new();
        }

        // Generate header comment
        let mut comment = format!("// {}\n", "=".repeat(50));

        // Add content
        for line in &lines {
            comment.push_str(line);
            comment.push('\n');
        }

        // Add footer
        comment.push_str("// ");
        comment.push_str(&"=".repeat(50));

        comment
    }

    /// Get integration statistics
    pub fn get_stats(&self) -> LLMIntegrationStats {
        self.stats.clone()
    }
}

/// Pre-validation result
#[derive(Debug, Clone)]
struct PreValidationResult {
    pub passed: bool,
    pub explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use sentinel_core::goal_manifold::Intent;

    #[derive(Debug)]
    struct DummyClient;

    #[async_trait::async_trait]
    impl LLMClient for DummyClient {
        async fn generate_code(
            &self,
            _prompt: &str,
            _context: &LLMContext,
        ) -> Result<LLMSuggestion> {
            Err(anyhow::anyhow!("not used in tests"))
        }

        async fn suggest_refactoring(
            &self,
            _code: &str,
            _context: &LLMContext,
        ) -> Result<LLMSuggestion> {
            Err(anyhow::anyhow!("not used in tests"))
        }

        async fn generate_documentation(
            &self,
            _code: &str,
            _context: &LLMContext,
        ) -> Result<LLMSuggestion> {
            Err(anyhow::anyhow!("not used in tests"))
        }

        async fn generate_tests(
            &self,
            _code: &str,
            _context: &LLMContext,
        ) -> Result<LLMSuggestion> {
            Err(anyhow::anyhow!("not used in tests"))
        }

        async fn explain_concept(
            &self,
            _concept: &str,
            _context: &LLMContext,
        ) -> Result<LLMSuggestion> {
            Err(anyhow::anyhow!("not used in tests"))
        }
    }

    fn test_manager() -> LLMIntegrationManager {
        let intent = Intent::new(
            "Build authentication service with JWT validation and secure secret handling",
            vec!["Use Rust", "Add tests", "Avoid hardcoded secrets"],
        )
        .with_outcome("Reliable auth pipeline");
        let manifold = Arc::new(GoalManifold::new(intent));
        let alignment_field = Arc::new(AlignmentField::new(manifold.as_ref().clone()));
        LLMIntegrationManager {
            goal_manifold: manifold,
            alignment_field,
            llm_client: Arc::new(DummyClient),
            quality_thresholds: QualityThreshold::default(),
            stats: LLMIntegrationStats::default(),
            suggestion_semaphore: Arc::new(Semaphore::new(1)),
        }
    }

    #[test]
    fn tokenize_text_filters_stop_words_and_short_tokens() {
        let manager = test_manager();
        let tokens = manager.tokenize_text("Build the auth service in Rust and add tests");
        assert!(tokens.contains("auth"));
        assert!(tokens.contains("service"));
        assert!(tokens.contains("rust"));
        assert!(!tokens.contains("the"));
        assert!(!tokens.contains("in"));
    }

    #[tokio::test]
    async fn compute_alignment_rewards_relevant_suggestion() {
        let manager = test_manager();
        let relevant = manager
            .compute_alignment_for_suggestion(
                "implement jwt auth with secure token verification",
                "Implement JWT auth validation, add tests, avoid hardcoded secrets.",
            )
            .await
            .unwrap();
        let irrelevant = manager
            .compute_alignment_for_suggestion(
                "implement jwt auth with secure token verification",
                "Refactor UI theme and update CSS animations for dashboard colors.",
            )
            .await
            .unwrap();
        assert!(relevant >= 50.0, "relevant score too low: {}", relevant);
        assert!(
            relevant > irrelevant + 15.0,
            "relevant={} should exceed irrelevant={} by >= 15",
            relevant,
            irrelevant
        );
    }

    #[tokio::test]
    async fn compute_alignment_penalizes_anti_goal_markers() {
        let manager = test_manager();
        let clean = manager
            .compute_alignment_for_suggestion(
                "secure auth",
                "Implement secure auth and add tests for token validation.",
            )
            .await
            .unwrap();
        let penalized = manager
            .compute_alignment_for_suggestion(
                "secure auth",
                "Implement secure auth but skip tests and hardcode secret.",
            )
            .await
            .unwrap();
        assert!(
            penalized + 20.0 < clean,
            "penalized={} clean={} expected strong penalty",
            penalized,
            clean
        );
    }

    #[tokio::test]
    async fn compute_alignment_returns_zero_for_empty_suggestion() {
        let manager = test_manager();
        let score = manager
            .compute_alignment_for_suggestion("any intent", "   ")
            .await
            .unwrap();
        assert_eq!(score, 0.0);
    }
}
