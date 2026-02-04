use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm_integration::{
    DocFormat, ExplanationStyle, ImprovementMetric, LLMChatClient, LLMChatCompletion, LLMClient,
    LLMContext, LLMSuggestion, LLMSuggestionType,
};
use sentinel_core::Uuid;

#[derive(Debug, Clone)]
pub struct OpenAICompatibleClient {
    pub name: String,
    api_key: Option<String>,
    base_url: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
    headers: HashMap<String, String>,
    http_client: reqwest::Client,
}

impl OpenAICompatibleClient {
    pub fn new(
        name: impl Into<String>,
        api_key: Option<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            api_key,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            temperature: 0.3,
            max_tokens: 2048,
            headers: HashMap::new(),
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

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    async fn request_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(String, u32)> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };

        let mut builder = self
            .http_client
            .post(self.endpoint())
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        let response = builder
            .json(&request)
            .send()
            .await
            .context("Failed to send OpenAI-compatible request")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI-compatible API error ({}): {}", status, error_text);
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI-compatible response")?;

        let content = completion
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens = completion
            .usage
            .as_ref()
            .and_then(|u| u.total_tokens)
            .unwrap_or(0);

        Ok((content, tokens))
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
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    total_tokens: Option<u32>,
}

#[async_trait::async_trait]
impl LLMChatClient for OpenAICompatibleClient {
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
impl LLMClient for OpenAICompatibleClient {
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
