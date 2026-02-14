//! Outcome Interpreter - Intent parsing and validation
//!
//! Converts natural language intent into machine-readable OutcomeEnvelope.

use crate::error::{Result, SentinelError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Outcome Envelope v1.0 - machine-readable intent
///
/// This is the canonical representation of a user's desired outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeEnvelope {
    pub schema_version: String,
    pub outcome_id: String,
    pub intent: IntentEnvelope,
    pub constraints: ConstraintEnvelope,
    pub quality_metrics: Vec<QualityMetric>,
    pub acceptance_criteria: Vec<String>,
    pub assumptions: Vec<String>,
    pub risk_register: Vec<RiskEntry>,
}

/// Intent envelope - the core objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEnvelope {
    pub goal: String,
    pub target_domain: TargetDomain,
    pub success_narrative: String,
}

/// Target domain classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetDomain {
    WebApp,
    BackendService,
    Other,
}

/// Constraint envelope - limitations and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEnvelope {
    pub non_negotiables: Vec<String>,
    pub tech_constraints: Vec<String>,
    pub time_budget: String,
    pub cost_budget: String,
}

/// Quality metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    pub name: String,
    pub measurement: String,
    pub target: String,
    pub gate: GateType,
}

/// Gate type - hard or soft requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateType {
    Hard,
    Soft,
}

/// Risk register entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskEntry {
    pub risk: String,
    pub mitigation: String,
}

/// Context for interpretation
#[derive(Debug, Clone)]
pub struct InterpretContext {
    pub assumptions: Vec<String>,
    pub workspace_type: Option<String>,
    pub existing_tech_stack: Vec<String>,
}

impl Default for InterpretContext {
    fn default() -> Self {
        Self {
            assumptions: Vec::new(),
            workspace_type: None,
            existing_tech_stack: Vec::new(),
        }
    }
}

/// Extracted intent components
#[derive(Debug, Clone)]
pub struct ExtractedIntent {
    pub goal: String,
    pub success_narrative: String,
}

/// Intent validator
pub struct IntentValidator;

impl IntentValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate raw intent input
    pub fn validate_raw(&self, raw: &str) -> Result<()> {
        if raw.trim().is_empty() {
            return Err(SentinelError::InvalidInput(
                "Intent cannot be empty".to_string(),
            ));
        }
        if raw.len() < 10 {
            return Err(SentinelError::InvalidInput(
                "Intent too short (min 10 chars)".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for IntentValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome Interpreter - validates and normalizes intent
pub struct OutcomeInterpreter {
    validator: IntentValidator,
}

impl OutcomeInterpreter {
    pub fn new() -> Self {
        Self {
            validator: IntentValidator::new(),
        }
    }

    /// Parse natural language intent into OutcomeEnvelope
    pub fn interpret(
        &self,
        raw_intent: &str,
        context: &InterpretContext,
    ) -> Result<OutcomeEnvelope> {
        // 1. Validate input
        self.validator.validate_raw(raw_intent)?;

        // 2. Extract structured components
        let intent = self.extract_intent(raw_intent)?;
        let domain = self.classify_domain(raw_intent, context)?;
        let constraints = self.extract_constraints(raw_intent, context)?;
        let quality_metrics = self.extract_quality_requirements(raw_intent)?;
        let acceptance_criteria = self.extract_acceptance_criteria(raw_intent)?;

        // 3. Generate risk register
        let risk_register = self.assess_risks(&intent, &constraints, &domain)?;

        Ok(OutcomeEnvelope {
            schema_version: "1.0".to_string(),
            outcome_id: Uuid::new_v4().to_string(),
            intent: IntentEnvelope {
                goal: intent.goal,
                target_domain: domain,
                success_narrative: intent.success_narrative,
            },
            constraints: ConstraintEnvelope {
                non_negotiables: constraints.non_negotiables,
                tech_constraints: constraints.tech_constraints,
                time_budget: constraints.time_budget,
                cost_budget: constraints.cost_budget,
            },
            quality_metrics,
            acceptance_criteria,
            assumptions: context.assumptions.clone(),
            risk_register,
        })
    }

    fn extract_intent(&self, raw: &str) -> Result<ExtractedIntent> {
        // Use LLM or pattern matching to extract:
        // - goal: what to build
        // - success_narrative: what "done" looks like
        //
        // For now, use simple extraction
        let goal = raw.to_string();
        let success_narrative = format!("When complete, {}", raw.to_lowercase());

        Ok(ExtractedIntent {
            goal,
            success_narrative,
        })
    }

    fn classify_domain(&self, raw: &str, _ctx: &InterpretContext) -> Result<TargetDomain> {
        // Use semantic analysis or LLM to classify
        let raw_lower = raw.to_lowercase();

        // Check for backend-specific patterns first (more specific)
        if raw_lower.contains("api")
            || raw_lower.contains("backend service")
            || raw_lower.contains("backend")
            || raw_lower.contains("server")
        {
            return Ok(TargetDomain::BackendService);
        }

        // Check for frontend patterns (more specific than just "ui")
        if raw_lower.contains("web app")
            || raw_lower.contains("webapp")
            || raw_lower.contains("web interface")
            || raw_lower.contains("frontend")
        {
            return Ok(TargetDomain::WebApp);
        }

        Ok(TargetDomain::Other)
    }

    fn extract_constraints(&self, raw: &str, ctx: &InterpretContext) -> Result<ConstraintEnvelope> {
        // Extract constraints from context and raw intent
        let mut non_negotiables = Vec::new();
        let mut tech_constraints = ctx.existing_tech_stack.clone();

        // Look for common constraint patterns
        if raw.to_lowercase().contains("auth") || raw.to_lowercase().contains("authentication") {
            non_negotiables.push("Authentication required".to_string());
        }

        if raw.to_lowercase().contains("secure") {
            non_negotiables.push("Security best practices".to_string());
        }

        // Add existing tech stack as constraints
        for tech in &ctx.existing_tech_stack {
            tech_constraints.push(format!("Use {}", tech));
        }

        Ok(ConstraintEnvelope {
            non_negotiables,
            tech_constraints,
            time_budget: "2w".to_string(), // Default
            cost_budget: "medium".to_string(), // Default
        })
    }

    fn extract_quality_requirements(&self, _raw: &str) -> Result<Vec<QualityMetric>> {
        // Default quality metrics
        Ok(vec![
            QualityMetric {
                name: "test_coverage".to_string(),
                measurement: "percent".to_string(),
                target: ">=80".to_string(),
                gate: GateType::Soft,
            },
            QualityMetric {
                name: "no_critical_vulnerabilities".to_string(),
                measurement: "boolean".to_string(),
                target: "true".to_string(),
                gate: GateType::Hard,
            },
        ])
    }

    fn extract_acceptance_criteria(&self, raw: &str) -> Result<Vec<String>> {
        // Extract acceptance criteria from intent
        let mut criteria = Vec::new();

        // Basic criteria
        criteria.push(format!("{} is functional", raw));

        Ok(criteria)
    }

    fn assess_risks(
        &self,
        _intent: &ExtractedIntent,
        _constraints: &ConstraintEnvelope,
        _domain: &TargetDomain,
    ) -> Result<Vec<RiskEntry>> {
        // Assess common risks
        Ok(vec![
            RiskEntry {
                risk: "Scope creep through feature additions".to_string(),
                mitigation: "Strict atomic module boundaries".to_string(),
            },
            RiskEntry {
                risk: "Tech stack incompatibility".to_string(),
                mitigation: "Validate constraints early".to_string(),
            },
        ])
    }
}

impl Default for OutcomeInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret_basic_web_app() {
        let interpreter = OutcomeInterpreter::new();
        let context = InterpretContext {
            assumptions: vec!["Users have modern browsers".to_string()],
            workspace_type: Some("web".to_string()),
            existing_tech_stack: vec!["TypeScript".to_string(), "React".to_string()],
            ..Default::default()
        };

        let result = interpreter
            .interpret("Build a task board web app", &context)
            .unwrap();

        assert_eq!(result.schema_version, "1.0");
        assert!(!result.outcome_id.is_empty());
        assert_eq!(result.intent.goal, "Build a task board web app");
        assert!(matches!(result.intent.target_domain, TargetDomain::WebApp));
    }

    #[test]
    fn test_classify_domain_web_app() {
        let interpreter = OutcomeInterpreter::new();
        let context = InterpretContext::default();

        let result = interpreter
            .interpret("Build a web interface for the dashboard", &context)
            .unwrap();

        assert!(matches!(result.intent.target_domain, TargetDomain::WebApp));
    }

    #[test]
    fn test_classify_domain_backend() {
        let interpreter = OutcomeInterpreter::new();
        let context = InterpretContext::default();

        let result = interpreter
            .interpret("Build a backend API server for billing", &context)
            .unwrap();

        assert!(matches!(
            result.intent.target_domain,
            TargetDomain::BackendService
        ));
    }

    #[test]
    fn test_validator_rejects_empty() {
        let validator = IntentValidator::new();
        assert!(validator.validate_raw("").is_err());
        assert!(validator.validate_raw("   ").is_err());
    }

    #[test]
    fn test_validator_rejects_too_short() {
        let validator = IntentValidator::new();
        assert!(validator.validate_raw("hi").is_err());
    }

    #[test]
    fn test_validator_accepts_valid() {
        let validator = IntentValidator::new();
        assert!(validator.validate_raw("Build a web app").is_ok());
    }

    #[test]
    fn test_auth_constraint_extraction() {
        let interpreter = OutcomeInterpreter::new();
        let context = InterpretContext::default();

        let result = interpreter
            .interpret("Build a web app with authentication", &context)
            .unwrap();

        assert!(result
            .constraints
            .non_negotiables
            .iter()
            .any(|c| c.contains("Authentication")));
    }
}
