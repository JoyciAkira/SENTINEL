//! Agent Marketplace
//!
//! Registry of pre-configured agent templates that can be instantiated
//! for common tasks and patterns.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::swarm::agent::AgentPersonality;
use crate::swarm::emergence::AgentType;

/// Marketplace of agent templates
pub struct AgentMarketplace {
    /// Available templates
    templates: HashMap<String, AgentTemplate>,
    /// Categories
    categories: HashMap<String, Vec<String>>,
}

/// Template for creating an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub agent_type: AgentType,
    pub personality: PersonalityTemplate,
    pub capabilities: Vec<String>,
    pub system_prompt: String,
    pub example_tasks: Vec<String>,
    pub version: String,
    pub author: String,
    pub downloads: u64,
    pub rating: f64,
}

/// Personality configuration template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTemplate {
    pub creativity: f64,
    pub thoroughness: f64,
    pub risk_tolerance: f64,
    pub collaboration_style: CollaborationStyle,
}

/// Collaboration style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationStyle {
    Leader,
    Contributor,
    Specialist,
    Reviewer,
}

impl AgentMarketplace {
    /// Create new marketplace with built-in templates
    pub fn new() -> Self {
        let mut marketplace = Self {
            templates: HashMap::new(),
            categories: HashMap::new(),
        };

        marketplace.load_builtin_templates();
        marketplace
    }

    /// Load built-in templates
    fn load_builtin_templates(&mut self) {
        // Rust Developer
        self.add_template(AgentTemplate {
            id: "rust-developer".to_string(),
            name: "Rust Developer".to_string(),
            description: "Expert in Rust programming with focus on performance and safety".to_string(),
            category: "backend".to_string(),
            agent_type: AgentType::AuthArchitect,
            personality: PersonalityTemplate {
                creativity: 0.4,
                thoroughness: 0.9,
                risk_tolerance: 0.2,
                collaboration_style: CollaborationStyle::Specialist,
            },
            capabilities: vec!["rust".to_string(), "systems".to_string(), "async".to_string()],
            system_prompt: "You are an expert Rust developer. Write idiomatic, safe, and performant code. Always handle errors properly and use strong typing.".to_string(),
            example_tasks: vec!["Implement async API".to_string(), "Optimize hot path".to_string()],
            version: "1.0.0".to_string(),
            author: "Sentinel Team".to_string(),
            downloads: 15420,
            rating: 4.8,
        });

        // React Frontend Developer
        self.add_template(AgentTemplate {
            id: "react-developer".to_string(),
            name: "React Frontend Developer".to_string(),
            description: "Frontend specialist with React, TypeScript, and modern UI patterns".to_string(),
            category: "frontend".to_string(),
            agent_type: AgentType::FrontendCoder,
            personality: PersonalityTemplate {
                creativity: 0.8,
                thoroughness: 0.7,
                risk_tolerance: 0.5,
                collaboration_style: CollaborationStyle::Contributor,
            },
            capabilities: vec!["react".to_string(), "typescript".to_string(), "ui".to_string()],
            system_prompt: "You are a React expert. Create modern, accessible, and performant UI components. Use TypeScript and follow best practices.".to_string(),
            example_tasks: vec!["Build dashboard".to_string(), "Create form components".to_string()],
            version: "1.0.0".to_string(),
            author: "Sentinel Team".to_string(),
            downloads: 12890,
            rating: 4.7,
        });

        // Security Auditor
        self.add_template(AgentTemplate {
            id: "security-auditor".to_string(),
            name: "Security Auditor".to_string(),
            description: "Security specialist focused on identifying vulnerabilities".to_string(),
            category: "security".to_string(),
            agent_type: AgentType::SecurityAuditor,
            personality: PersonalityTemplate {
                creativity: 0.2,
                thoroughness: 1.0,
                risk_tolerance: 0.0,
                collaboration_style: CollaborationStyle::Reviewer,
            },
            capabilities: vec!["security".to_string(), "auditing".to_string(), "vulnerabilities".to_string()],
            system_prompt: "You are a security expert. Audit code for vulnerabilities, check for OWASP issues, and ensure secure practices.".to_string(),
            example_tasks: vec!["Audit auth system".to_string(), "Check SQL injection risks".to_string()],
            version: "1.0.0".to_string(),
            author: "Sentinel Team".to_string(),
            downloads: 8932,
            rating: 4.9,
        });

        // DevOps Engineer
        self.add_template(AgentTemplate {
            id: "devops-engineer".to_string(),
            name: "DevOps Engineer".to_string(),
            description: "Infrastructure and deployment automation specialist".to_string(),
            category: "devops".to_string(),
            agent_type: AgentType::DevOpsEngineer,
            personality: PersonalityTemplate {
                creativity: 0.5,
                thoroughness: 0.9,
                risk_tolerance: 0.3,
                collaboration_style: CollaborationStyle::Specialist,
            },
            capabilities: vec!["docker".to_string(), "kubernetes".to_string(), "ci-cd".to_string()],
            system_prompt: "You are a DevOps expert. Create Dockerfiles, K8s manifests, and CI/CD pipelines. Follow best practices for reliability.".to_string(),
            example_tasks: vec!["Setup CI/CD pipeline".to_string(), "Create K8s deployment".to_string()],
            version: "1.0.0".to_string(),
            author: "Sentinel Team".to_string(),
            downloads: 7654,
            rating: 4.6,
        });

        // Database Architect
        self.add_template(AgentTemplate {
            id: "database-architect".to_string(),
            name: "Database Architect".to_string(),
            description: "Database design and optimization expert".to_string(),
            category: "database".to_string(),
            agent_type: AgentType::DatabaseArchitect,
            personality: PersonalityTemplate {
                creativity: 0.3,
                thoroughness: 0.95,
                risk_tolerance: 0.1,
                collaboration_style: CollaborationStyle::Specialist,
            },
            capabilities: vec!["sql".to_string(), "postgres".to_string(), "optimization".to_string()],
            system_prompt: "You are a database expert. Design schemas, optimize queries, and ensure data integrity.".to_string(),
            example_tasks: vec!["Design schema".to_string(), "Optimize slow queries".to_string()],
            version: "1.0.0".to_string(),
            author: "Sentinel Team".to_string(),
            downloads: 6432,
            rating: 4.8,
        });
    }

    /// Add template to marketplace
    fn add_template(&mut self, template: AgentTemplate) {
        let category = template.category.clone();
        let id = template.id.clone();

        self.templates.insert(id.clone(), template);

        self.categories
            .entry(category)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Get template by ID
    pub fn get_template(&self, id: &str) -> Option<&AgentTemplate> {
        self.templates.get(id)
    }

    /// Search templates
    pub fn search(&self, query: &str) -> Vec<&AgentTemplate> {
        let query_lower = query.to_lowercase();

        self.templates
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.capabilities
                        .iter()
                        .any(|c| c.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get templates by category
    pub fn get_by_category(&self, category: &str) -> Vec<&AgentTemplate> {
        self.categories
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.templates.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn get_categories(&self) -> Vec<&String> {
        self.categories.keys().collect()
    }

    /// Get popular templates
    pub fn get_popular(&self, limit: usize) -> Vec<&AgentTemplate> {
        let mut templates: Vec<_> = self.templates.values().collect();
        templates.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        templates.into_iter().take(limit).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_creation() {
        let marketplace = AgentMarketplace::new();
        assert!(!marketplace.get_categories().is_empty());
    }

    #[test]
    fn test_search_templates() {
        let marketplace = AgentMarketplace::new();
        let results = marketplace.search("rust");
        assert!(!results.is_empty());
    }
}
