//! Deterministic Agent Emergence
//!
//! Agents don't exist before the task - they emerge from goal analysis.
//! This module uses deterministic rule-based parsing to decide which agents
//! should exist for any given goal.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Domain classification for goals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Frontend,
    Backend,
    Database,
    Security,
    DevOps,
    Mobile,
    General,
}

/// Security criticality level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Detected patterns in goal
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetectedPattern {
    Authentication,
    Authorization,
    JWT,
    OAuth,
    API,
    REST,
    GraphQL,
    Database,
    Frontend,
    Testing,
    Documentation,
    Performance,
    Security,
    Realtime,
    Microservices,
    Serverless,
}

/// Agent types that can emerge from goal analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    AuthArchitect,
    SecurityAuditor,
    JWTCoder,
    APICoder,
    FrontendCoder,
    DatabaseArchitect,
    TestWriter,
    DocWriter,
    ReviewAgent,
    PerformanceOptimizer,
    DevOpsEngineer,
    Manager,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentType::AuthArchitect => "AuthArchitect",
            AgentType::SecurityAuditor => "SecurityAuditor",
            AgentType::JWTCoder => "JWTCoder",
            AgentType::APICoder => "APICoder",
            AgentType::FrontendCoder => "FrontendCoder",
            AgentType::DatabaseArchitect => "DatabaseArchitect",
            AgentType::TestWriter => "TestWriter",
            AgentType::DocWriter => "DocWriter",
            AgentType::ReviewAgent => "ReviewAgent",
            AgentType::PerformanceOptimizer => "PerformanceOptimizer",
            AgentType::DevOpsEngineer => "DevOpsEngineer",
            AgentType::Manager => "Manager",
        };
        write!(f, "{}", s)
    }
}

/// Analysis result that determines which agents emerge
#[derive(Debug, Clone)]
pub struct GoalAnalysis {
    /// Domain detected
    pub domain: Domain,

    /// Complexity score (0.0 - 1.0)
    pub complexity: f64,

    /// Security criticality
    pub security_level: SecurityLevel,

    /// Patterns detected
    pub patterns: Vec<DetectedPattern>,

    /// Agents that should emerge (in order of priority)
    pub required_agents: Vec<AgentType>,
}

/// Goal analyzer - deterministic rule-based parser
pub struct GoalAnalyzer;

impl GoalAnalyzer {
    /// Analyze goal and determine which agents should emerge
    ///
    /// This function is DETERMINISTIC: same goal → same analysis → same agents
    pub async fn analyze(goal: &str) -> Result<GoalAnalysis> {
        let goal_lower = goal.to_lowercase();
        let mut agents = Vec::new();
        let mut patterns = Vec::new();

        // === SECURITY DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "auth",
                "login",
                "jwt",
                "oauth",
                "password",
                "credential",
                "session",
                "token",
            ],
        ) {
            agents.push(AgentType::AuthArchitect);
            agents.push(AgentType::SecurityAuditor);
            patterns.push(DetectedPattern::Authentication);
            patterns.push(DetectedPattern::Security);

            if goal_lower.contains("jwt") {
                agents.push(AgentType::JWTCoder);
                patterns.push(DetectedPattern::JWT);
            }

            if goal_lower.contains("oauth") {
                patterns.push(DetectedPattern::OAuth);
            }
        }

        // === API DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "api", "endpoint", "rest", "graphql", "http", "server", "backend",
            ],
        ) {
            agents.push(AgentType::APICoder);
            patterns.push(DetectedPattern::API);

            if goal_lower.contains("rest") {
                patterns.push(DetectedPattern::REST);
            }

            if goal_lower.contains("graphql") {
                patterns.push(DetectedPattern::GraphQL);
            }
        }

        // === FRONTEND DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "frontend",
                "ui",
                "react",
                "vue",
                "angular",
                "html",
                "css",
                "component",
            ],
        ) {
            agents.push(AgentType::FrontendCoder);
            patterns.push(DetectedPattern::Frontend);
        }

        // === DATABASE DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "database", "db", "postgres", "mysql", "mongo", "redis", "sql", "schema",
            ],
        ) {
            agents.push(AgentType::DatabaseArchitect);
            patterns.push(DetectedPattern::Database);
        }

        // === TESTING DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "test",
                "spec",
                "jest",
                "pytest",
                "unit test",
                "integration test",
            ],
        ) {
            agents.push(AgentType::TestWriter);
            patterns.push(DetectedPattern::Testing);
        }

        // === PERFORMANCE DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "performance",
                "optimize",
                "cache",
                "speed",
                "fast",
                "slow",
                "bottleneck",
            ],
        ) {
            agents.push(AgentType::PerformanceOptimizer);
            patterns.push(DetectedPattern::Performance);
        }

        // === DEVOPS DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "deploy",
                "docker",
                "kubernetes",
                "k8s",
                "ci/cd",
                "pipeline",
                "serverless",
            ],
        ) {
            agents.push(AgentType::DevOpsEngineer);

            if goal_lower.contains("serverless") {
                patterns.push(DetectedPattern::Serverless);
            }
        }

        // === REALTIME DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &[
                "realtime",
                "websocket",
                "socket",
                "live",
                "streaming",
                "async",
            ],
        ) {
            patterns.push(DetectedPattern::Realtime);
        }

        // === MICROSERVICES DOMAIN DETECTION ===
        if Self::detect_pattern(
            &goal_lower,
            &["microservice", "service", "distributed", "grpc"],
        ) {
            patterns.push(DetectedPattern::Microservices);
        }

        // === ALWAYS ADD THESE FOR NON-TRIVIAL TASKS ===
        let complexity = Self::calculate_complexity(goal);

        if complexity > 0.2 {
            // Add reviewer for anything non-trivial
            if !agents.contains(&AgentType::ReviewAgent) {
                agents.push(AgentType::ReviewAgent);
            }
        }

        if complexity > 0.3 {
            // Add documentation for medium+ complexity
            if !agents.contains(&AgentType::DocWriter) {
                agents.push(AgentType::DocWriter);
            }
        }

        if complexity > 0.5 {
            // Add test writer for complex tasks if not already present
            if !agents.contains(&AgentType::TestWriter) {
                agents.push(AgentType::TestWriter);
            }
        }

        // Deduplicate while preserving order
        let mut seen = HashSet::new();
        let unique_agents: Vec<_> = agents.into_iter().filter(|a| seen.insert(*a)).collect();

        // Detect domain
        let domain = Self::detect_domain(&goal_lower, &patterns);

        // Assess security level
        let security_level = Self::assess_security(&goal_lower, &patterns);

        Ok(GoalAnalysis {
            domain,
            complexity,
            security_level,
            patterns,
            required_agents: unique_agents,
        })
    }

    /// Check if any pattern keyword exists in goal
    fn detect_pattern(goal: &str, keywords: &[&str]) -> bool {
        keywords.iter().any(|kw| goal.contains(kw))
    }

    /// Calculate complexity score (0.0 - 1.0)
    ///
    /// Deterministic factors:
    /// - Word count (more words = more complex)
    /// - Technical term density
    /// - Conjunction count (and, with, plus = more components)
    /// - Verb count (actions needed)
    pub fn calculate_complexity(goal: &str) -> f64 {
        let goal_lower = goal.to_lowercase();
        let words: Vec<_> = goal_lower.split_whitespace().collect();
        let word_count = words.len();

        // Technical terms (uppercase or specific keywords)
        let technical_terms = words
            .iter()
            .filter(|w| {
                w.chars().any(|c| c.is_uppercase())
                    || [
                        "api",
                        "database",
                        "auth",
                        "jwt",
                        "oauth",
                        "microservice",
                        "graphql",
                        "websocket",
                    ]
                    .contains(&w.to_lowercase().as_str())
            })
            .count();

        // Conjunctions indicate multiple components
        let conjunctions = ["and", "with", "plus", "also", "then", "or"]
            .iter()
            .map(|c| goal_lower.matches(c).count())
            .sum::<usize>();

        // Action verbs indicate work needed
        let action_verbs = [
            "build",
            "create",
            "implement",
            "design",
            "develop",
            "write",
            "generate",
            "make",
        ]
        .iter()
        .map(|v| goal_lower.matches(v).count())
        .sum::<usize>();

        // Calculate score (normalized)
        let word_score = (word_count as f64 * 0.02).min(0.4);
        let tech_score = (technical_terms as f64 * 0.1).min(0.3);
        let conj_score = (conjunctions as f64 * 0.05).min(0.2);
        let action_score = (action_verbs as f64 * 0.03).min(0.1);

        let total = word_score + tech_score + conj_score + action_score;
        total.min(1.0)
    }

    /// Detect domain from goal and patterns
    fn detect_domain(goal: &str, patterns: &[DetectedPattern]) -> Domain {
        // Check explicit mentions
        if goal.contains("frontend")
            || goal.contains("ui")
            || goal.contains("react")
            || goal.contains("vue")
        {
            return Domain::Frontend;
        }

        if goal.contains("backend") || goal.contains("api") || goal.contains("server") {
            return Domain::Backend;
        }

        if goal.contains("database") || goal.contains("db") || goal.contains("sql") {
            return Domain::Database;
        }

        if goal.contains("deploy") || goal.contains("docker") || goal.contains("kubernetes") {
            return Domain::DevOps;
        }

        if goal.contains("mobile")
            || goal.contains("ios")
            || goal.contains("android")
            || goal.contains("app")
        {
            return Domain::Mobile;
        }

        // Check patterns
        if patterns.contains(&DetectedPattern::Authentication)
            || patterns.contains(&DetectedPattern::Security)
        {
            return Domain::Security;
        }

        if patterns.contains(&DetectedPattern::Database) {
            return Domain::Database;
        }

        if patterns.contains(&DetectedPattern::Frontend) {
            return Domain::Frontend;
        }

        Domain::General
    }

    /// Assess security criticality
    fn assess_security(goal: &str, patterns: &[DetectedPattern]) -> SecurityLevel {
        let critical_terms = [
            "auth",
            "password",
            "security",
            "encrypt",
            "token",
            "credential",
            "secret",
            "key",
            "hash",
        ];
        let count = critical_terms.iter().filter(|t| goal.contains(*t)).count();

        if patterns.contains(&DetectedPattern::Authentication) || count >= 3 {
            SecurityLevel::Critical
        } else if count >= 2 {
            SecurityLevel::High
        } else if count >= 1 {
            SecurityLevel::Medium
        } else {
            SecurityLevel::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_goal() {
        let goal = "Build authentication system with JWT and password hashing";
        let analysis = GoalAnalyzer::analyze(goal).await.unwrap();

        assert!(analysis.required_agents.contains(&AgentType::AuthArchitect));
        assert!(analysis
            .required_agents
            .contains(&AgentType::SecurityAuditor));
        assert!(analysis.required_agents.contains(&AgentType::JWTCoder));
        assert!(analysis.patterns.contains(&DetectedPattern::Authentication));
        assert!(analysis.patterns.contains(&DetectedPattern::JWT));
    }

    #[tokio::test]
    async fn test_api_goal() {
        let goal = "Create REST API with database integration and tests";
        let analysis = GoalAnalyzer::analyze(goal).await.unwrap();

        assert!(analysis.required_agents.contains(&AgentType::APICoder));
        assert!(analysis
            .required_agents
            .contains(&AgentType::DatabaseArchitect));
        assert!(analysis.required_agents.contains(&AgentType::TestWriter));
        assert!(analysis.patterns.contains(&DetectedPattern::API));
        assert!(analysis.patterns.contains(&DetectedPattern::REST));
    }

    #[tokio::test]
    async fn test_determinism() {
        let goal = "Build auth system with JWT tokens";

        let analysis1 = GoalAnalyzer::analyze(goal).await.unwrap();
        let analysis2 = GoalAnalyzer::analyze(goal).await.unwrap();

        // Same goal → same agents (in same order)
        assert_eq!(analysis1.required_agents, analysis2.required_agents);
        assert_eq!(analysis1.patterns, analysis2.patterns);
        assert_eq!(analysis1.complexity, analysis2.complexity);
    }

    #[test]
    fn test_complexity_calculation() {
        let simple = "Build auth";
        let complex = "Build authentication system with JWT, OAuth integration, database storage, and comprehensive test suite";

        let simple_score = GoalAnalyzer::calculate_complexity(simple);
        let complex_score = GoalAnalyzer::calculate_complexity(complex);

        assert!(complex_score > simple_score);
        assert!(simple_score < 0.3);
        assert!(complex_score > 0.4);
    }
}
