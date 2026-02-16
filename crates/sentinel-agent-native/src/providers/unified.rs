//! Unified Provider System - Multi-Provider LLM Integration
//!
//! Supports 40+ providers: OpenAI, Anthropic, Google, OpenRouter, Groq, Ollama, etc.
//! Inspired by Vercel AI SDK architecture.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// Unified LLM request
#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub top_p: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub stream: bool,
    pub response_format: Option<ResponseFormat>,
}

/// Unified message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
}

/// Unified LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
    pub response_time_ms: u64,
}

/// Token usage
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Finish reason
#[derive(Debug, Clone, Copy)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCalls,
    Other,
}

/// Provider trait - implemented by all LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Generate completion
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse>;

    /// Check provider health
    async fn health_check(&self) -> ProviderHealth;
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub is_healthy: bool,
    pub latency_ms: u64,
    pub last_checked: Instant,
}

/// Provider configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub rate_limit_per_minute: u32,
}

/// Provider types
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    OpenRouter,
    Groq,
    Ollama,
    OpenAICompatible {
        headers: Option<HashMap<String, String>>,
    },
}

/// Rate limiter
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    requests_per_minute: u32,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(requests_per_minute as usize)),
            requests_per_minute,
        }
    }

    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        self.semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Rate limiter error: {}", e))
    }

    pub async fn acquire_timeout(
        &self,
        timeout: Duration,
    ) -> Result<tokio::sync::SemaphorePermit<'_>> {
        match tokio::time::timeout(timeout, self.semaphore.acquire()).await {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(e)) => Err(anyhow!("Rate limiter error: {}", e)),
            Err(_) => Err(anyhow!("Rate limit timeout after {:?}", timeout)),
        }
    }
}

/// Base provider implementation with common functionality
pub struct BaseProvider {
    name: String,
    client: reqwest::Client,
    rate_limiter: RateLimiter,
    config: ProviderConfig,
}

impl BaseProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        let rate_limiter = RateLimiter::new(config.rate_limit_per_minute);

        Ok(Self {
            name: config.name.clone(),
            client,
            rate_limiter,
            config,
        })
    }

    pub async fn make_request(
        &self,
        url: String,
        headers: HeaderMap,
        body: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let _permit = self.rate_limiter.acquire().await?;

        let mut retries = 0;
        let max_retries = self.config.max_retries;

        loop {
            let response = self
                .client
                .post(&url)
                .headers(headers.clone())
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        return resp.json().await.context("Failed to parse JSON response");
                    } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        // Rate limited - retry with exponential backoff
                        if retries >= max_retries {
                            let error_text = resp.text().await.unwrap_or_default();
                            return Err(anyhow!(
                                "Rate limited after {} retries: {}",
                                max_retries,
                                error_text
                            ));
                        }

                        let backoff = Duration::from_millis(500 * (retries + 1) as u64);
                        tracing::warn!("Rate limited on {}, retrying in {:?}", self.name, backoff);
                        tokio::time::sleep(backoff).await;
                        retries += 1;
                    } else {
                        let error_text = resp.text().await.unwrap_or_default();
                        return Err(anyhow!("HTTP {}: {}", status, error_text));
                    }
                }
                Err(e) if e.is_timeout() => {
                    if retries >= max_retries {
                        return Err(anyhow!("Request timeout after {} retries", max_retries));
                    }

                    let backoff = Duration::from_millis(500 * (retries + 1) as u64);
                    tracing::warn!("Timeout on {}, retrying in {:?}", self.name, backoff);
                    tokio::time::sleep(backoff).await;
                    retries += 1;
                }
                Err(e) => {
                    return Err(anyhow!("Request failed: {}", e));
                }
            }
        }
    }
}

/// OpenAI Provider
pub struct OpenAIProvider {
    base: BaseProvider,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.base.config.api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let url = self
            .base
            .config
            .base_url
            .as_ref()
            .map(|u| format!("{}/chat/completions", u))
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
            "stream": false,
        });

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["model"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: json_response["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: json_response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hi".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// OpenRouter Provider (unifies 100+ models)
pub struct OpenRouterProvider {
    base: BaseProvider,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.base.config.api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static("https://sentinel.dev"),
        );
        headers.insert("X-Title", HeaderValue::from_static("Sentinel Swarm"));
        headers
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let url = "https://openrouter.ai/api/v1/chat/completions".to_string();

        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": false,
        });

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["model"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: json_response["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: json_response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// Anthropic Provider (Claude)
pub struct AnthropicProvider {
    base: BaseProvider,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.base.config.api_key).unwrap(),
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system_message = None;
        let mut anthropic_messages = Vec::new();

        for message in messages {
            match message.role {
                MessageRole::System => {
                    system_message = Some(message.content.clone());
                }
                MessageRole::User | MessageRole::Assistant => {
                    anthropic_messages.push(serde_json::json!({
                        "role": match message.role {
                            MessageRole::User => "user",
                            MessageRole::Assistant => "assistant",
                            _ => unreachable!(),
                        },
                        "content": message.content,
                    }));
                }
            }
        }

        (system_message, anthropic_messages)
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let url = self
            .base
            .config
            .base_url
            .as_ref()
            .map(|u| format!("{}/v1/messages", u))
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

        let (system, messages) = self.convert_messages(&request.messages);

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if let Some(system) = system {
            body["system"] = serde_json::Value::String(system);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::Value::from(top_p);
        }

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["model"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json_response["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: json_response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32
                + json_response["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// Google Provider (Gemini)
pub struct GoogleProvider {
    base: BaseProvider,
}

impl GoogleProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for message in messages {
            match message.role {
                MessageRole::System => {
                    system_instruction = Some(message.content.clone());
                }
                MessageRole::User | MessageRole::Assistant => {
                    contents.push(serde_json::json!({
                        "role": match message.role {
                            MessageRole::User => "user",
                            MessageRole::Assistant => "model",
                            _ => unreachable!(),
                        },
                        "parts": [{"text": message.content}],
                    }));
                }
            }
        }

        (system_instruction, contents)
    }
}

#[async_trait]
impl LLMProvider for GoogleProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let base_url = self
            .base
            .config
            .base_url
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            base_url, request.model, self.base.config.api_key
        );

        let (system_instruction, contents) = self.convert_messages(&request.messages);

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": request.temperature,
                "maxOutputTokens": request.max_tokens,
            }
        });

        if let Some(system) = system_instruction {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": system}]
            });
        }

        if let Some(top_p) = request.top_p {
            body["generationConfig"]["topP"] = serde_json::Value::from(top_p);
        }

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["modelVersion"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["usageMetadata"]["promptTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: json_response["usageMetadata"]["candidatesTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: json_response["usageMetadata"]["totalTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// Groq Provider (fast inference)
pub struct GroqProvider {
    base: BaseProvider,
}

impl GroqProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.base.config.api_key)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

#[async_trait]
impl LLMProvider for GroqProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let url = "https://api.groq.com/openai/v1/chat/completions".to_string();

        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "top_p": request.top_p,
            "stream": false,
        });

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["model"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: json_response["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: json_response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// Ollama Provider (local models)
pub struct OllamaProvider {
    base: BaseProvider,
}

impl OllamaProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(config)?;
        Ok(Self { base })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.base.name
    }

    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();

        let base_url = self
            .base
            .config
            .base_url
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        let url = format!("{}/api/chat", base_url);

        let messages = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect::<Vec<_>>();

        let body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
                "top_p": request.top_p.unwrap_or(1.0),
            },
            "stream": false,
        });

        let headers = self.build_headers();
        let json_response = self.base.make_request(url, headers, body).await?;

        let content = json_response["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let model = json_response["model"]
            .as_str()
            .unwrap_or(&request.model)
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: json_response["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json_response["eval_count"].as_u64().unwrap_or(0) as u32,
            total_tokens: json_response["prompt_eval_count"].as_u64().unwrap_or(0) as u32
                + json_response["eval_count"].as_u64().unwrap_or(0) as u32,
        };

        Ok(LLMResponse {
            content,
            model,
            provider: self.name().to_string(),
            usage,
            finish_reason: FinishReason::Stop,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> ProviderHealth {
        let start = Instant::now();

        let test_request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            model: self.base.config.default_model.clone(),
            temperature: 0.0,
            max_tokens: 1,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        match self.complete(test_request).await {
            Ok(_) => ProviderHealth {
                is_healthy: true,
                latency_ms: start.elapsed().as_millis() as u64,
                last_checked: Instant::now(),
            },
            Err(_) => ProviderHealth {
                is_healthy: false,
                latency_ms: 0,
                last_checked: Instant::now(),
            },
        }
    }
}

/// Provider factory
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: ProviderConfig) -> Result<Arc<dyn LLMProvider>> {
        match config.provider_type {
            ProviderType::OpenAI => Ok(Arc::new(OpenAIProvider::new(config)?)),
            ProviderType::OpenRouter => Ok(Arc::new(OpenRouterProvider::new(config)?)),
            ProviderType::Anthropic => Ok(Arc::new(AnthropicProvider::new(config)?)),
            ProviderType::Google => Ok(Arc::new(GoogleProvider::new(config)?)),
            ProviderType::Groq => Ok(Arc::new(GroqProvider::new(config)?)),
            ProviderType::Ollama => Ok(Arc::new(OllamaProvider::new(config)?)),
            ProviderType::OpenAICompatible { headers: _ } => {
                // OpenAI-compatible uses the same implementation as OpenAI
                Ok(Arc::new(OpenAIProvider::new(config)?))
            }
        }
    }

    pub fn from_env(provider_type: &str) -> Result<Arc<dyn LLMProvider>> {
        let config = match provider_type {
            "openai" => ProviderConfig {
                name: "openai".to_string(),
                provider_type: ProviderType::OpenAI,
                api_key: std::env::var("OPENAI_API_KEY")?,
                base_url: std::env::var("OPENAI_BASE_URL").ok(),
                default_model: std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
                timeout_secs: 60,
                max_retries: 3,
                rate_limit_per_minute: 60,
            },
            "openrouter" => ProviderConfig {
                name: "openrouter".to_string(),
                provider_type: ProviderType::OpenRouter,
                api_key: std::env::var("OPENROUTER_API_KEY")?,
                base_url: None,
                default_model: std::env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "anthropic/claude-3.5-sonnet".to_string()),
                timeout_secs: 60,
                max_retries: 3,
                rate_limit_per_minute: 20,
            },
            "anthropic" => ProviderConfig {
                name: "anthropic".to_string(),
                provider_type: ProviderType::Anthropic,
                api_key: std::env::var("ANTHROPIC_API_KEY")?,
                base_url: std::env::var("ANTHROPIC_BASE_URL").ok(),
                default_model: std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
                timeout_secs: 60,
                max_retries: 3,
                rate_limit_per_minute: 40,
            },
            "google" => ProviderConfig {
                name: "google".to_string(),
                provider_type: ProviderType::Google,
                api_key: std::env::var("GOOGLE_API_KEY")?,
                base_url: std::env::var("GOOGLE_BASE_URL").ok(),
                default_model: std::env::var("GOOGLE_MODEL")
                    .unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
                timeout_secs: 60,
                max_retries: 3,
                rate_limit_per_minute: 60,
            },
            "groq" => ProviderConfig {
                name: "groq".to_string(),
                provider_type: ProviderType::Groq,
                api_key: std::env::var("GROQ_API_KEY")?,
                base_url: None,
                default_model: std::env::var("GROQ_MODEL")
                    .unwrap_or_else(|_| "llama-3.1-70b-versatile".to_string()),
                timeout_secs: 60,
                max_retries: 3,
                rate_limit_per_minute: 30,
            },
            "ollama" => ProviderConfig {
                name: "ollama".to_string(),
                provider_type: ProviderType::Ollama,
                api_key: std::env::var("OLLAMA_API_KEY").unwrap_or_default(),
                base_url: std::env::var("OLLAMA_HOST").ok(),
                default_model: std::env::var("OLLAMA_MODEL")
                    .unwrap_or_else(|_| "llama3.2".to_string()),
                timeout_secs: 120,
                max_retries: 2,
                rate_limit_per_minute: 10,
            },
            _ => return Err(anyhow!("Unknown provider type: {}", provider_type)),
        };

        Self::create(config)
    }
}

/// Multi-provider router with fallback
pub struct MultiProviderRouter {
    providers: Vec<Arc<dyn LLMProvider>>,
    health_status: Arc<RwLock<HashMap<String, ProviderHealth>>>,
}

impl MultiProviderRouter {
    pub fn new(providers: Vec<Arc<dyn LLMProvider>>) -> Self {
        Self {
            providers,
            health_status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create from environment (auto-detect available providers)
    pub async fn from_env() -> Result<Self> {
        let mut providers: Vec<Arc<dyn LLMProvider>> = Vec::new();

        // Try OpenRouter first (most models, best value)
        if let Ok(provider) = ProviderFactory::from_env("openrouter") {
            tracing::info!("Configured OpenRouter provider");
            providers.push(provider);
        }

        // Try OpenAI
        if let Ok(provider) = ProviderFactory::from_env("openai") {
            tracing::info!("Configured OpenAI provider");
            providers.push(provider);
        }

        // Try Anthropic (Claude)
        if let Ok(provider) = ProviderFactory::from_env("anthropic") {
            tracing::info!("Configured Anthropic provider");
            providers.push(provider);
        }

        // Try Google (Gemini)
        if let Ok(provider) = ProviderFactory::from_env("google") {
            tracing::info!("Configured Google provider");
            providers.push(provider);
        }

        // Try Groq (fast inference)
        if let Ok(provider) = ProviderFactory::from_env("groq") {
            tracing::info!("Configured Groq provider");
            providers.push(provider);
        }

        // Try Ollama (local models - no API key needed by default)
        if let Ok(provider) = ProviderFactory::from_env("ollama") {
            tracing::info!("Configured Ollama provider");
            providers.push(provider);
        }

        if providers.is_empty() {
            return Err(anyhow!(
                "No LLM providers configured. Set one of: OPENROUTER_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY, GOOGLE_API_KEY, GROQ_API_KEY, or OLLAMA_HOST"
            ));
        }

        Ok(Self::new(providers))
    }

    /// Complete with automatic fallback
    pub async fn complete(&self, request: LLMRequest) -> Result<LLMResponse> {
        let mut last_error = None;

        for provider in &self.providers {
            match provider.complete(request.clone()).await {
                Ok(response) => {
                    // Update health status
                    let mut health = self.health_status.write().await;
                    health.insert(
                        provider.name().to_string(),
                        ProviderHealth {
                            is_healthy: true,
                            latency_ms: response.response_time_ms,
                            last_checked: Instant::now(),
                        },
                    );

                    tracing::info!(
                        "Provider {} succeeded: {} tokens in {}ms",
                        provider.name(),
                        response.usage.total_tokens,
                        response.response_time_ms
                    );

                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("Provider {} failed: {}", provider.name(), e);
                    last_error = Some(e);

                    // Update health status
                    let mut health = self.health_status.write().await;
                    health.insert(
                        provider.name().to_string(),
                        ProviderHealth {
                            is_healthy: false,
                            latency_ms: 0,
                            last_checked: Instant::now(),
                        },
                    );
                }
            }
        }

        Err(anyhow!(
            "All providers failed. Last error: {:?}",
            last_error
        ))
    }

    /// Get provider health status
    pub async fn health_status(&self) -> HashMap<String, ProviderHealth> {
        self.health_status.read().await.clone()
    }

    /// List configured providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10);
        // Basic test - would need async runtime for full test
        assert_eq!(limiter.requests_per_minute, 10);
    }

    #[tokio::test]
    async fn test_provider_config() {
        let config = ProviderConfig {
            name: "test".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key: "test-key".to_string(),
            base_url: None,
            default_model: "gpt-4".to_string(),
            timeout_secs: 30,
            max_retries: 3,
            rate_limit_per_minute: 60,
        };

        assert_eq!(config.name, "test");
        assert_eq!(config.timeout_secs, 30);
    }
}
