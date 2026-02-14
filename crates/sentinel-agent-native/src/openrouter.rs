//! OpenRouter LLM Client - Real LLM Provider Integration
//!
//! Implements the LLMClient trait using OpenRouter's OpenAI-compatible API.
//! Supports free models for development and testing.
//!
//! # Usage
//!
//! ```text
//! let client = OpenRouterClient::new(
//!     "your-api-key".to_string(),
//!     OpenRouterModel::MetaLlama3_3_70B,
//! );
//! let suggestion = client.generate_code("Write a hello world", &context).await?;
//! ```

use crate::llm_integration::{
    DocFormat, ExplanationStyle, ImprovementMetric, LLMChatClient, LLMChatCompletion, LLMClient,
    LLMContext, LLMSuggestion, LLMSuggestionType,
};
use anyhow::{Context, Result};
use sentinel_core::Uuid;
use serde::{Deserialize, Serialize};

/// OpenRouter API base URL
const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Free models available on OpenRouter (no API cost)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRouterModel {
    /// Meta Llama 3.3 70B Instruct (Free)
    MetaLlama3_3_70B,
    /// Google Gemini 2.0 Flash Experimental (Free)
    GoogleGemini2Flash,
    /// DeepSeek R1 0528 (Free)
    DeepSeekR1,
    /// Mistral Devstral Small 2505 (Free)
    MistralDevstral,
    /// Custom model ID
    Custom(String),
}

impl OpenRouterModel {
    /// Get the model ID string for the API
    pub fn model_id(&self) -> &str {
        match self {
            Self::MetaLlama3_3_70B => "meta-llama/llama-3.3-70b-instruct:free",
            Self::GoogleGemini2Flash => "google/gemini-2.0-flash-exp:free",
            Self::DeepSeekR1 => "deepseek/deepseek-r1-0528:free",
            Self::MistralDevstral => "mistralai/devstral-small:free",
            Self::Custom(id) => id,
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &str {
        match self {
            Self::MetaLlama3_3_70B => "Meta Llama 3.3 70B",
            Self::GoogleGemini2Flash => "Google Gemini 2.0 Flash",
            Self::DeepSeekR1 => "DeepSeek R1",
            Self::MistralDevstral => "Mistral Devstral",
            Self::Custom(id) => id,
        }
    }
}

/// OpenRouter API request
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

/// Chat message
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenRouter API response
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: Option<String>,
    choices: Vec<ChatChoice>,
    usage: Option<UsageInfo>,
}

/// Chat choice
#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

/// Token usage info
#[derive(Debug, Deserialize)]
struct UsageInfo {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

/// OpenRouter LLM Client
///
/// Implements the LLMClient trait using OpenRouter's API.
/// Supports free models for zero-cost development and testing.
pub struct OpenRouterClient {
    /// API key for OpenRouter
    api_key: String,

    /// Model to use
    model: OpenRouterModel,

    /// HTTP client
    http_client: reqwest::Client,

    /// Temperature (0.0 = deterministic, 1.0 = creative)
    temperature: f64,

    /// Max tokens per response
    max_tokens: u32,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client
    pub fn new(api_key: String, model: OpenRouterModel) -> Self {
        Self {
            api_key,
            model,
            http_client: reqwest::Client::new(),
            temperature: 0.3, // Low temperature for code generation
            max_tokens: 2048,
        }
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Send a chat completion request to OpenRouter
    async fn request_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(String, u32)> {
        let request = ChatCompletionRequest {
            model: self.model.model_id().to_string(),
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

        let response = self
            .http_client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/JoyciAkira/SENTINEL")
            .header("X-Title", "Sentinel Protocol")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenRouter")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenRouter API error ({}): {}", status, error_text);
        }

        let completion: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;

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

    /// Build system prompt with Sentinel context
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

impl std::fmt::Debug for OpenRouterClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenRouterClient")
            .field("model", &self.model)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .finish()
    }
}

#[async_trait::async_trait]
impl LLMChatClient for OpenRouterClient {
    async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<LLMChatCompletion> {
        let (content, tokens) = self.request_completion(system_prompt, user_prompt).await?;
        Ok(LLMChatCompletion {
            llm_name: self.model.display_name().to_string(),
            content,
            token_cost: tokens,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for OpenRouterClient {
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
            llm_name: self.model.display_name().to_string(),
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
            llm_name: self.model.display_name().to_string(),
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
            llm_name: self.model.display_name().to_string(),
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
            llm_name: self.model.display_name().to_string(),
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
            llm_name: self.model.display_name().to_string(),
            content,
            estimated_quality: 0.85,
            goal_alignment: 0.90,
            confidence: 0.85,
            token_cost: tokens,
        })
    }
}
