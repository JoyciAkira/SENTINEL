//! Scaffolding emission - Generates production-ready code templates for atomic modules
//!
//! This module implements the ScaffoldGenerator that creates detailed, opinionated
//! code templates for each atomic module type with built-in guardrails.

use crate::error::Result;
use crate::outcome_compiler::compiler::AtomicModule;
use std::path::Path;

/// Module type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    ApiEndpoint,
    DatabaseModel,
    Service,
    FrontendComponent,
    AuthModule,
    Utility,
    Config,
    TestSuite,
}

impl ModuleType {
    pub fn detect(module_name: &str, objective: &str) -> Self {
        let name_lower = module_name.to_lowercase();
        let obj_lower = objective.to_lowercase();

        if name_lower.contains("auth")
            || name_lower.contains("login")
            || name_lower.contains("session")
        {
            ModuleType::AuthModule
        } else if name_lower.contains("test") || name_lower.contains("spec") {
            ModuleType::TestSuite
        } else if name_lower.contains("config") || name_lower.contains("settings") {
            ModuleType::Config
        } else if name_lower.contains("api")
            || name_lower.contains("route")
            || name_lower.contains("handler")
        {
            ModuleType::ApiEndpoint
        } else if name_lower.contains("db")
            || name_lower.contains("model")
            || name_lower.contains("repository")
        {
            ModuleType::DatabaseModel
        } else if name_lower.contains("service")
            || name_lower.contains("logic")
            || name_lower.contains("usecase")
        {
            ModuleType::Service
        } else if name_lower.contains("util") || name_lower.contains("helper") {
            ModuleType::Utility
        } else if obj_lower.contains("frontend")
            || obj_lower.contains("ui")
            || obj_lower.contains("component")
        {
            ModuleType::FrontendComponent
        } else {
            ModuleType::Service
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScaffoldGuardrail {
    pub rule_name: String,
    pub description: String,
    pub enforcement: GuardrailEnforcement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardrailEnforcement {
    Block,
    Warn,
    Audit,
}

#[derive(Debug, Clone)]
pub struct ScaffoldResult {
    pub module_id: String,
    pub module_name: String,
    pub files: Vec<ScaffoldFile>,
    pub guardrails: Vec<ScaffoldGuardrail>,
    pub metadata: ScaffoldMetadata,
}

#[derive(Debug, Clone)]
pub struct ScaffoldFile {
    pub path: String,
    pub content: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ScaffoldMetadata {
    pub language: String,
    pub framework: String,
    pub total_lines: usize,
    pub estimated_impl_time: String,
}

pub struct ScaffoldGenerator {
    language: String,
    framework: String,
}

impl ScaffoldGenerator {
    pub fn new(language: impl Into<String>, framework: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            framework: framework.into(),
        }
    }

    pub fn generate_scaffold(&self, module: &AtomicModule) -> Result<ScaffoldResult> {
        let module_type = ModuleType::detect(&module.module_name, &module.objective);
        let files = self.generate_files(module, module_type)?;
        let guardrails = self.generate_guardrails(module, module_type);

        let total_lines: usize = files.iter().map(|f| f.content.lines().count()).sum();

        Ok(ScaffoldResult {
            module_id: module.module_id.clone(),
            module_name: module.module_name.clone(),
            files,
            guardrails,
            metadata: ScaffoldMetadata {
                language: self.language.clone(),
                framework: self.framework.clone(),
                total_lines,
                estimated_impl_time: self.estimate_time(module_type, total_lines),
            },
        })
    }

    pub fn emit_scaffold(
        &self,
        result: &ScaffoldResult,
        output_dir: &Path,
    ) -> Result<Vec<std::path::PathBuf>> {
        let module_dir = output_dir.join("src");
        std::fs::create_dir_all(&module_dir)?;

        let mut emitted_paths = Vec::new();

        for file in &result.files {
            let file_path = module_dir.join(&file.path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, &file.content)?;
            emitted_paths.push(file_path);
        }

        Ok(emitted_paths)
    }

    fn generate_files(
        &self,
        module: &AtomicModule,
        module_type: ModuleType,
    ) -> Result<Vec<ScaffoldFile>> {
        let mut files = Vec::new();

        match module_type {
            ModuleType::ApiEndpoint => files.extend(self.generate_api_endpoint_files(module)?),
            ModuleType::DatabaseModel => files.extend(self.generate_database_model_files(module)?),
            ModuleType::Service => files.extend(self.generate_service_files(module)?),
            ModuleType::FrontendComponent => {
                files.extend(self.generate_frontend_component_files(module)?)
            }
            ModuleType::AuthModule => files.extend(self.generate_auth_module_files(module)?),
            ModuleType::Utility => files.extend(self.generate_utility_files(module)?),
            ModuleType::Config => files.extend(self.generate_config_files(module)?),
            ModuleType::TestSuite => files.extend(self.generate_test_files(module)?),
        }

        Ok(files)
    }

    fn generate_api_endpoint_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);
        let module_name_pascal = Self::to_pascal_case(&module.module_name);

        let in_scope = module.boundaries.in_scope.join(", ");
        let out_of_scope = module.boundaries.out_of_scope.join(", ");

        let handler_code = format!(
            "//! {} API Endpoint\n//! \n//! ## Guardrails\n//! - Input validation MUST pass before processing\n//! - Authentication REQUIRED for all mutations\n//!\n//! ## In-Scope: {}\n//! ## Out-of-Scope: {}\n\nuse axum::{{Json, extract::State, response::IntoResponse, http::StatusCode}};\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Deserialize)]\npub struct {}Request {{\n    // TODO: Define request fields\n}}\n\n#[derive(Debug, Serialize)]\npub struct {}Response {{\n    // TODO: Define response fields\n}}\n\npub async fn create_{}(\n    State(state): State<crate::AppState>,\n    Json(req): Json<{}Request>,\n) -> Result<impl IntoResponse, crate::Error> {{\n    // GUARDRAIL: Input validation\n    // TODO: Validate request\n    \n    Ok((StatusCode::CREATED, Json({}Response {{}})))\n}}\n",
            module_name_pascal,
            in_scope,
            out_of_scope,
            module_name_pascal,
            module_name_pascal,
            module_name_snake,
            module_name_pascal,
            module_name_pascal
        );

        let test_code = format!(
            "//! Tests for {} API\n\n#[tokio::test]\nasync fn test_create_{}_valid_input() {{\n    // TODO: Implement test\n}}\n",
            module_name_pascal,
            module_name_snake
        );

        Ok(vec![
            ScaffoldFile {
                path: format!("{}.rs", module_name_snake),
                content: handler_code,
                description: format!("API handlers for {}", module_name_pascal),
            },
            ScaffoldFile {
                path: format!("{}_test.rs", module_name_snake),
                content: test_code,
                description: format!("Tests for {} API", module_name_pascal),
            },
        ])
    }

    fn generate_database_model_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);
        let module_name_pascal = Self::to_pascal_case(&module.module_name);

        let in_scope = module.boundaries.in_scope.join(", ");
        let out_of_scope = module.boundaries.out_of_scope.join(", ");

        let model_code = format!(
            "//! {} Database Model\n//! \n//! ## Guardrails\n//! - Schema changes MUST be backward compatible\n//! - Soft delete ONLY (no hard deletes)\n//!\n//! ## In-Scope: {}\n//! ## Out-of-Scope: {}\n\nuse chrono::{{DateTime, Utc}};\nuse serde::{{Deserialize, Serialize}};\nuse sqlx::FromRow;\nuse uuid::Uuid;\n\n#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]\npub struct {} {{\n    pub id: Uuid,\n    pub created_at: DateTime<Utc>,\n    pub updated_at: DateTime<Utc>,\n    pub deleted_at: Option<DateTime<Utc>>,\n}}\n\nimpl {} {{\n    pub fn new() -> Self {{\n        Self {{\n            id: Uuid::new_v4(),\n            created_at: Utc::now(),\n            updated_at: Utc::now(),\n            deleted_at: None,\n        }}\n    }}\n}}\n",
            module_name_pascal,
            in_scope,
            out_of_scope,
            module_name_pascal,
            module_name_pascal
        );

        let migration_code = format!(
            "-- Migration: Create {} table\n\nCREATE TABLE IF NOT EXISTS {} (\n    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n    deleted_at TIMESTAMPTZ\n);\n\nCREATE INDEX IF NOT EXISTS idx_{}_created_at ON {}(created_at);\nCREATE INDEX IF NOT EXISTS idx_{}_deleted_at ON {}(deleted_at) WHERE deleted_at IS NULL;\n",
            module_name_snake,
            module_name_snake,
            module_name_snake,
            module_name_snake,
            module_name_snake,
            module_name_snake
        );

        Ok(vec![
            ScaffoldFile {
                path: format!("{}.rs", module_name_snake),
                content: model_code,
                description: format!("Database model for {}", module_name_pascal),
            },
            ScaffoldFile {
                path: format!("migrations/001_create_{}.sql", module_name_snake),
                content: migration_code,
                description: format!("Migration for {}", module_name_pascal),
            },
        ])
    }

    fn generate_service_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);
        let module_name_pascal = Self::to_pascal_case(&module.module_name);

        let in_scope = module.boundaries.in_scope.join(", ");
        let out_of_scope = module.boundaries.out_of_scope.join(", ");

        let service_code = format!(
            "//! {} Business Logic Service\n//! \n//! ## Guardrails\n//! - Input validation at entry point\n//! - Authorization check before mutations\n//!\n//! ## In-Scope: {}\n//! ## Out-of-Scope: {}\n\nuse crate::Result;\n\npub struct {}Service;\n\nimpl {}Service {{\n    pub fn new() -> Self {{\n        Self\n    }}\n    \n    pub async fn execute(&self) -> Result<()> {{\n        // GUARDRAIL: Authorization check\n        // TODO: Check permissions\n        \n        // GUARDRAIL: Business rule validation\n        // TODO: Validate\n        \n        Ok(())\n    }}\n}}\n",
            module_name_pascal,
            in_scope,
            out_of_scope,
            module_name_pascal,
            module_name_pascal
        );

        Ok(vec![ScaffoldFile {
            path: format!("{}.rs", module_name_snake),
            content: service_code,
            description: format!("Service for {}", module_name_pascal),
        }])
    }

    fn generate_frontend_component_files(
        &self,
        module: &AtomicModule,
    ) -> Result<Vec<ScaffoldFile>> {
        let module_name_pascal = Self::to_pascal_case(&module.module_name);

        let component_code = format!(
            "/**\n * {} Component\n */\n\nimport React from 'react';\n\nexport interface {}Props {{\n    // TODO: Define props\n}}\n\nexport const {}: React.FC<{}Props> = (props) => {{\n    return (\n        <div>\n            <h1>{}</h1>\n        </div>\n    );\n}};\n\nexport default {};\n",
            module_name_pascal,
            module_name_pascal,
            module_name_pascal,
            module_name_pascal,
            module_name_pascal,
            module_name_pascal
        );

        let test_code = format!(
            "import React from 'react';\nimport {{ render, screen }} from '@testing-library/react';\nimport {{ {} }} from './{}';\n\ndescribe('{}', () => {{\n    it('renders', () => {{\n        render(<{} />);\n        expect(screen.getByText('{}')).toBeInTheDocument();\n    }});\n}});\n",
            module_name_pascal,
            module_name_pascal,
            module_name_pascal,
            module_name_pascal,
            module_name_pascal
        );

        Ok(vec![
            ScaffoldFile {
                path: format!("{}.tsx", module_name_pascal),
                content: component_code,
                description: format!("Component for {}", module_name_pascal),
            },
            ScaffoldFile {
                path: format!("{}.test.tsx", module_name_pascal),
                content: test_code,
                description: format!("Tests for {}", module_name_pascal),
            },
        ])
    }

    fn generate_auth_module_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);

        let auth_code = format!(
            "//! Authentication Module\n//! \n//! ## CRITICAL GUARDRAILS\n//! - Passwords MUST be hashed with Argon2\n//! - JWT secrets MUST be 256+ bits\n//! - Rate limiting: 5 req/min\n\npub struct AuthService;\n\nimpl AuthService {{\n    pub fn hash_password(password: &str) -> Result<String, argon2::Error> {{\n        // GUARDRAIL: Use Argon2\n        todo!()\n    }}\n    \n    pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::Error> {{\n        todo!()\n    }}\n}}\n"
        );

        Ok(vec![ScaffoldFile {
            path: format!("{}.rs", module_name_snake),
            content: auth_code,
            description: "Authentication module".to_string(),
        }])
    }

    fn generate_utility_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);

        let util_code = format!(
            "//! Utility Module\n\n/// TODO: Document\npub fn example_function(input: &str) -> String {{\n    if input.is_empty() {{\n        return String::new();\n    }}\n    input.to_string()\n}}\n\n#[cfg(test)]\nmod tests {{\n    use super::*;\n    \n    #[test]\n    fn test_example() {{\n        assert_eq!(example_function(\"test\"), \"test\");\n    }}\n}}\n"
        );

        Ok(vec![ScaffoldFile {
            path: format!("{}.rs", module_name_snake),
            content: util_code,
            description: "Utility functions".to_string(),
        }])
    }

    fn generate_config_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);
        let module_name_pascal = Self::to_pascal_case(&module.module_name);

        let config_code = format!(
            "//! {} Configuration\n\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {}Config {{\n    // TODO: Add fields\n}}\n\nimpl {}Config {{\n    pub fn from_env() -> Result<Self, std::env::VarError> {{\n        Ok(Self {{}})\n    }}\n}}\n",
            module_name_pascal,
            module_name_pascal,
            module_name_pascal
        );

        let env_example = "# Environment Variables\nDATABASE_URL=postgresql://user:pass@localhost/db\nAPI_PORT=8080\n";

        Ok(vec![
            ScaffoldFile {
                path: format!("{}.rs", module_name_snake),
                content: config_code,
                description: "Configuration".to_string(),
            },
            ScaffoldFile {
                path: ".env.example".to_string(),
                content: env_example.to_string(),
                description: "Example env".to_string(),
            },
        ])
    }

    fn generate_test_files(&self, module: &AtomicModule) -> Result<Vec<ScaffoldFile>> {
        let module_name_snake = Self::to_snake_case(&module.module_name);

        let test_code = format!(
            "//! Integration Tests\n\n#[tokio::test]\nasync fn test_happy_path() {{\n    // TODO: Implement\n}}\n\n#[tokio::test]\nasync fn test_error_handling() {{\n    // TODO: Implement\n}}\n"
        );

        Ok(vec![ScaffoldFile {
            path: format!("{}_test.rs", module_name_snake),
            content: test_code,
            description: "Integration tests".to_string(),
        }])
    }

    fn generate_guardrails(
        &self,
        module: &AtomicModule,
        module_type: ModuleType,
    ) -> Vec<ScaffoldGuardrail> {
        let mut guardrails = Vec::new();

        guardrails.push(ScaffoldGuardrail {
            rule_name: "invariant_preservation".to_string(),
            description: "Module invariants must be preserved".to_string(),
            enforcement: GuardrailEnforcement::Block,
        });

        guardrails.push(ScaffoldGuardrail {
            rule_name: "test_coverage".to_string(),
            description: "Minimum 80% test coverage required".to_string(),
            enforcement: GuardrailEnforcement::Block,
        });

        match module_type {
            ModuleType::ApiEndpoint => {
                guardrails.push(ScaffoldGuardrail {
                    rule_name: "input_validation".to_string(),
                    description: "All inputs must be validated".to_string(),
                    enforcement: GuardrailEnforcement::Block,
                });
            }
            ModuleType::DatabaseModel => {
                guardrails.push(ScaffoldGuardrail {
                    rule_name: "soft_delete".to_string(),
                    description: "Only soft deletes allowed".to_string(),
                    enforcement: GuardrailEnforcement::Block,
                });
            }
            ModuleType::AuthModule => {
                guardrails.push(ScaffoldGuardrail {
                    rule_name: "password_hashing".to_string(),
                    description: "Passwords must be hashed with Argon2".to_string(),
                    enforcement: GuardrailEnforcement::Block,
                });
            }
            _ => {}
        }

        guardrails
    }

    fn estimate_time(&self, module_type: ModuleType, lines: usize) -> String {
        let base_hours = match module_type {
            ModuleType::ApiEndpoint => 4,
            ModuleType::DatabaseModel => 6,
            ModuleType::Service => 8,
            ModuleType::FrontendComponent => 6,
            ModuleType::AuthModule => 12,
            ModuleType::Utility => 2,
            ModuleType::Config => 1,
            ModuleType::TestSuite => 4,
        };

        let complexity_factor = lines as f64 / 100.0;
        let total_hours = (base_hours as f64 * complexity_factor).max(1.0);

        if total_hours < 4.0 {
            format!("{}h", total_hours.round())
        } else {
            format!("{}d", (total_hours / 8.0).ceil())
        }
    }

    fn to_snake_case(s: &str) -> String {
        s.chars()
            .enumerate()
            .map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    format!("_{}", c.to_lowercase())
                } else {
                    c.to_lowercase().to_string()
                }
            })
            .collect()
    }

    fn to_pascal_case(s: &str) -> String {
        s.split(|c: char| c == '_' || c == '-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect()
    }
}

impl Default for ScaffoldGenerator {
    fn default() -> Self {
        Self::new("rust", "axum")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_module_type_api() {
        assert_eq!(
            ModuleType::detect("user_api_handler", ""),
            ModuleType::ApiEndpoint
        );
    }

    #[test]
    fn test_detect_module_type_auth() {
        assert_eq!(
            ModuleType::detect("auth_service", ""),
            ModuleType::AuthModule
        );
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(
            ScaffoldGenerator::to_snake_case("UserProfile"),
            "user_profile"
        );
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(
            ScaffoldGenerator::to_pascal_case("user_profile"),
            "UserProfile"
        );
    }
}
