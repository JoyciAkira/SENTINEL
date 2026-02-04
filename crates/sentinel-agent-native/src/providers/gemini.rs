use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::llm_integration::{
    DocFormat, ExplanationStyle, ImprovementMetric, LLMChatClient, LLMChatCompletion, LLMClient,
    LLMContext, LLMSuggestion, LLMSuggestionType,
};
use sentinel_core::Uuid;

#[derive(Debug, Clone)]
pub struct GeminiClient {
    pub name: String,
    api_key: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
    http_client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: String, model: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            name: format!("Gemini {}", model),
            api_key,
            model,
            temperature: 0.3,
            max_tokens: 2048,
            http_client: reqwest::Client::new(),
        }
    }

    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
    }

    async fn request_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(String, u32)> {
        let request = GeminiRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: system_prompt.to_string(),
                }],
            }),
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: user_prompt.to_string(),
                }],
            }],
            generation_config: Some(GeminiGenerationConfig {
                temperature: self.temperature,
                max_output_tokens: self.max_tokens,
            }),
        };

        let response = self
            .http_client
            .post(self.endpoint())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error ({}): {}", status, error_text);
        }

        let completion: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini response")?;

        let content = completion
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok((content, 0))
    }

    fn build_system_prompt(&self, context: &LLMContext) -> String {
        let mut prompt = String::from(
            "You are an AI coding assistant integrated with Sentinel Protocol. \
             Your suggestions are validated by Sentinel OS for goal alignment, \
             code quality, and security compliance.\n\n",
        );

        if !context.goal_description.is_empty() {
            prompt.push_str(&format!("Current Goal: {}\n", context.goal_description));
        }

        if !context.constraints.is_empty() {
            prompt.push_str(&format!(
                "Constraints: {}\n",
                context.constraints.join(", ")
            ));
        }

        if !context.context.is_empty() {
            prompt.push_str(&format!("Context: {}\n", context.context));
        }

        prompt
    }
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemInstruction")]
    system_instruction: Option<GeminiSystemInstruction>,
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "temperature")]
    temperature: f64,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[async_trait::async_trait]
impl LLMChatClient for GeminiClient {
    async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<LLMChatCompletion> {
        let (content, tokens) = self.request_completion(system_prompt, user_prompt).await?;
        Ok(LLMChatCompletion {
            llm_name: self.name.clone(),
            content,
            token_cost: tokens,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for GeminiClient {
    async fn generate_code(&self, prompt: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let system = self.build_system_prompt(context);
        let user_prompt = format!(
            "Generate code for the following requirement. \
             Return ONLY the code, no explanations:\n\n{}",
            prompt
        );

        let (content, tokens) = self.request_completion(&system, &user_prompt).await?;

        Ok(LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: LLMSuggestionType::CodeGeneration {
                file_path: "generated.rs".to_string(),
                code: content.clone(),
                language: "rust".to_string(),
            },
            llm_name: self.name.clone(),
            content,
            estimated_quality: 0.85,
            goal_alignment: 0.90,
            confidence: 0.80,
            token_cost: tokens,
        })
    }

    async fn suggest_refactoring(&self, code: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let system = self.build_system_prompt(context);
        let user_prompt = format!(
            "Suggest refactoring improvements for this code. \
             Explain what to change and why:\n\n```\n{}\n```",
            code
        );

        let (content, tokens) = self.request_completion(&system, &user_prompt).await?;

        Ok(LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: LLMSuggestionType::Refactoring {
                file_path: "refactored.rs".to_string(),
                description: content.clone(),
                expected_improvement: ImprovementMetric::CodeQuality,
            },
            llm_name: self.name.clone(),
            content,
            estimated_quality: 0.80,
            goal_alignment: 0.85,
            confidence: 0.75,
            token_cost: tokens,
        })
    }

    async fn generate_documentation(
        &self,
        code: &str,
        context: &LLMContext,
    ) -> Result<LLMSuggestion> {
        let system = self.build_system_prompt(context);
        let user_prompt = format!(
            "Generate comprehensive documentation for this code:\n\n```\n{}\n```",
            code
        );

        let (content, tokens) = self.request_completion(&system, &user_prompt).await?;

        Ok(LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: LLMSuggestionType::Documentation {
                to_document: code[..code.len().min(100)].to_string(),
                format: DocFormat::DocComments,
            },
            llm_name: self.name.clone(),
            content,
            estimated_quality: 0.85,
            goal_alignment: 0.90,
            confidence: 0.85,
            token_cost: tokens,
        })
    }

    async fn generate_tests(&self, code: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let system = self.build_system_prompt(context);
        let user_prompt = format!(
            "Generate comprehensive test cases for this code. \
             Include unit tests and edge cases:\n\n```\n{}\n```",
            code
        );

        let (content, tokens) = self.request_completion(&system, &user_prompt).await?;

        Ok(LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: LLMSuggestionType::TestCase {
                test_target: code[..code.len().min(50)].to_string(),
                test_type: "unit".to_string(),
            },
            llm_name: self.name.clone(),
            content,
            estimated_quality: 0.80,
            goal_alignment: 0.85,
            confidence: 0.80,
            token_cost: tokens,
        })
    }

    async fn explain_concept(&self, concept: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let system = self.build_system_prompt(context);
        let user_prompt = format!("Explain this concept clearly with examples:\n\n{}", concept);

        let (content, tokens) = self.request_completion(&system, &user_prompt).await?;

        Ok(LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type: LLMSuggestionType::ConceptExplanation {
                concept: concept.to_string(),
                style: ExplanationStyle::StepByStep,
            },
            llm_name: self.name.clone(),
            content,
            estimated_quality: 0.85,
            goal_alignment: 0.90,
            confidence: 0.85,
            token_cost: tokens,
        })
    }
}
