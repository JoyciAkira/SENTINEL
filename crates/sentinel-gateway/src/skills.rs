//! Skills system for SENTINEL Gateway
//!
//! Configurable templates and guardrails for different agent behaviors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::{GatewayError, Result};

/// Skill template types (inspired by OpenClaw)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillTemplate {
    /// Agent behavior configuration
    Agents,
    /// Initial setup/bootstrap
    Bootstrap,
    /// Agent identity
    Identity,
    /// Agent personality/soul
    Soul,
    /// Available tools
    Tools,
    /// User preferences
    User,
}

impl std::fmt::Display for SkillTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillTemplate::Agents => write!(f, "AGENTS"),
            SkillTemplate::Bootstrap => write!(f, "BOOTSTRAP"),
            SkillTemplate::Identity => write!(f, "IDENTITY"),
            SkillTemplate::Soul => write!(f, "SOUL"),
            SkillTemplate::Tools => write!(f, "TOOLS"),
            SkillTemplate::User => write!(f, "USER"),
        }
    }
}

/// Guardrail rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guardrail {
    /// Guardrail name
    pub name: String,

    /// Guardrail type
    #[serde(rename = "type")]
    pub guardrail_type: GuardrailType,

    /// Parameters
    pub params: HashMap<String, serde_json::Value>,

    /// Is enabled
    pub enabled: bool,
}

impl Guardrail {
    pub fn new(name: impl Into<String>, guardrail_type: GuardrailType) -> Self {
        Self {
            name: name.into(),
            guardrail_type,
            params: HashMap::new(),
            enabled: true,
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardrailType {
    /// Block operation
    Block,
    /// Require confirmation
    Confirm,
    /// Limit by count
    Limit,
    /// Filter content
    Filter,
    /// Timeout after duration
    Timeout,
    /// Require specific condition
    Require,
}

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    pub description: String,

    /// Tool category
    pub category: ToolCategory,

    /// Is enabled
    pub enabled: bool,

    /// Required permissions
    pub permissions: Vec<String>,
}

impl Tool {
    pub fn new(name: impl Into<String>, description: impl Into<String>, category: ToolCategory) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            enabled: true,
            permissions: Vec::new(),
        }
    }

    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    FileSystem,
    Shell,
    Network,
    Database,
    Llm,
    Custom,
}

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name
    pub name: String,

    /// Skill description
    pub description: String,

    /// Template type
    pub template: SkillTemplate,

    /// Available tools
    pub tools: Vec<String>,

    /// Guardrails
    pub guardrails: Vec<Guardrail>,

    /// Custom configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Is enabled
    pub enabled: bool,

    /// Priority (higher = more important)
    pub priority: u32,
}

impl Skill {
    pub fn new(name: impl Into<String>, template: SkillTemplate) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            template,
            tools: Vec::new(),
            guardrails: Vec::new(),
            config: HashMap::new(),
            enabled: true,
            priority: 0,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }

    pub fn with_guardrail(mut self, guardrail: Guardrail) -> Self {
        self.guardrails.push(guardrail);
        self
    }

    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if a tool is allowed
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        self.enabled && self.tools.contains(&tool_name.to_string())
    }

    /// Validate against guardrails
    pub fn validate(&self, context: &serde_json::Value) -> Result<bool> {
        let context_str = serde_json::to_string(context).unwrap_or_default();
        
        for guardrail in &self.guardrails {
            if !guardrail.enabled {
                continue;
            }

            match &guardrail.guardrail_type {
                GuardrailType::Block => {
                    if let Some(blocked) = guardrail.params.get("blocked").and_then(|v| v.as_array()) {
                        for item in blocked {
                            let item_str = match item.as_str() {
                                Some(s) => s.to_string(),
                                None => item.to_string(),
                            };
                            if context_str.contains(&item_str) {
                                return Err(GatewayError::SecurityViolation(format!(
                                    "Blocked by guardrail: {}",
                                    guardrail.name
                                )));
                            }
                        }
                    }
                }
                GuardrailType::Require => {
                    if let Some(required) = guardrail.params.get("required").and_then(|v| v.as_array()) {
                        for item in required {
                            let item_str = match item.as_str() {
                                Some(s) => s.to_string(),
                                None => item.to_string(),
                            };
                            if !context_str.contains(&item_str) {
                                return Err(GatewayError::SecurityViolation(format!(
                                    "Missing required condition: {}",
                                    item
                                )));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(true)
    }
}

/// Skill registry
pub struct SkillRegistry {
    /// Registered skills
    skills: Arc<RwLock<HashMap<String, Skill>>>,

    /// Registered tools
    tools: Arc<RwLock<HashMap<String, Tool>>>,

    /// Default skill for unmatched requests
    default_skill: Arc<RwLock<Option<String>>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
            default_skill: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a skill
    pub fn register(&self, skill: Skill) {
        let name = skill.name.clone();
        let mut skills = self.skills.write();
        skills.insert(name.clone(), skill);
        tracing::info!("Skill registered: {}", name);
    }

    /// Unregister a skill
    pub fn unregister(&self, name: &str) -> Result<()> {
        let mut skills = self.skills.write();
        if skills.remove(name).is_some() {
            tracing::info!("Skill unregistered: {}", name);
            Ok(())
        } else {
            Err(GatewayError::SkillNotFound(name.to_string()))
        }
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<Skill> {
        let skills = self.skills.read();
        skills.get(name).cloned()
    }

    /// List all skills
    pub fn list(&self) -> Vec<Skill> {
        let skills = self.skills.read();
        skills.values().cloned().collect()
    }

    /// Register a tool
    pub fn register_tool(&self, tool: Tool) {
        let name = tool.name.clone();
        let mut tools = self.tools.write();
        tools.insert(name.clone(), tool);
        tracing::info!("Tool registered: {}", name);
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<Tool> {
        let tools = self.tools.read();
        tools.get(name).cloned()
    }

    /// List all tools
    pub fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read();
        tools.values().cloned().collect()
    }

    /// Set default skill
    pub fn set_default(&self, skill_name: impl Into<String>) {
        let mut default = self.default_skill.write();
        *default = Some(skill_name.into());
    }

    /// Get default skill
    pub fn get_default(&self) -> Option<Skill> {
        let default = self.default_skill.read();
        if let Some(name) = default.as_ref() {
            self.get(name)
        } else {
            None
        }
    }

    /// Load skills from JSON file
    pub fn load_from_file(&self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let skills: Vec<Skill> = serde_json::from_str(&content)?;
        for skill in skills {
            self.register(skill);
        }
        Ok(())
    }

    /// Save skills to JSON file
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let skills = self.list();
        let content = serde_json::to_string_pretty(&skills)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create default skills
pub fn create_default_skills() -> Vec<Skill> {
    vec![
        Skill::new("code_generator", SkillTemplate::Agents)
            .with_description("Code generation skill with file system access")
            .with_tool("file_write")
            .with_tool("file_read")
            .with_tool("shell_exec")
            .with_guardrail(
                Guardrail::new("no_delete_without_confirm", GuardrailType::Confirm)
                    .with_param("pattern", serde_json::json!("rm -rf"))
            )
            .with_guardrail(
                Guardrail::new("max_file_size", GuardrailType::Limit)
                    .with_param("max_bytes", serde_json::json!(1048576)) // 1MB
            )
            .with_priority(10),

        Skill::new("architect", SkillTemplate::Identity)
            .with_description("Architecture analysis skill - read only")
            .with_tool("file_read")
            .with_tool("diagram_gen")
            .with_priority(5),

        Skill::new("test_runner", SkillTemplate::Tools)
            .with_description("Test execution skill")
            .with_tool("shell_exec")
            .with_guardrail(
                Guardrail::new("test_timeout", GuardrailType::Timeout)
                    .with_param("seconds", serde_json::json!(300))
            )
            .with_priority(7),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new("test_skill", SkillTemplate::Agents)
            .with_description("Test skill")
            .with_tool("file_read");

        assert_eq!(skill.name, "test_skill");
        assert_eq!(skill.template, SkillTemplate::Agents);
        assert!(skill.is_tool_allowed("file_read"));
        assert!(!skill.is_tool_allowed("file_write"));
    }

    #[test]
    fn test_guardrail_block() {
        let skill = Skill::new("test", SkillTemplate::Agents)
            .with_guardrail(
                Guardrail::new("block_test", GuardrailType::Block)
                    .with_param("blocked", serde_json::json!(["rm -rf"]))
            );

        let context = serde_json::json!({"command": "rm -rf /"});
        let result = skill.validate(&context);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_registry() {
        let registry = SkillRegistry::new();

        let skill = Skill::new("test_skill", SkillTemplate::Agents);
        registry.register(skill);

        assert!(registry.get("test_skill").is_some());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_default_skills() {
        let skills = create_default_skills();
        assert!(!skills.is_empty());

        let code_gen = skills.iter().find(|s| s.name == "code_generator");
        assert!(code_gen.is_some());
        assert!(code_gen.unwrap().is_tool_allowed("file_write"));
    }

    #[test]
    fn test_tool_registration() {
        let registry = SkillRegistry::new();

        let tool = Tool::new("file_read", "Read file contents", ToolCategory::FileSystem);
        registry.register_tool(tool);

        assert!(registry.get_tool("file_read").is_some());
    }
}