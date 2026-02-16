//! LLM Integration for Swarm
//!
//! Multi-provider LLM client using ProviderRouter or UnifiedProviderSystem.
//! Supports OpenAI, Anthropic, Google, OpenRouter, Ollama, Groq, and more.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

use crate::llm_integration::LLMChatClient;
use crate::providers::router::ProviderRouter;
use crate::providers::unified::{Message as UnifiedMessage, MessageRole, MultiProviderRouter};

/// LLM client for swarm agents - supports both ProviderRouter and UnifiedProviderSystem
pub struct SwarmLLMClient {
    /// Provider router for multi-provider support (legacy)
    provider_router: Option<Arc<ProviderRouter>>,

    /// Unified multi-provider router (new)
    unified_router: Option<Arc<MultiProviderRouter>>,

    /// Rate limiter (max concurrent requests)
    semaphore: Arc<Semaphore>,

    /// Request timeout
    timeout: Duration,

    /// Max retries
    max_retries: u32,

    /// Request statistics
    stats: Arc<RwLock<LLMStats>>,

    /// Provider preference (optional)
    preferred_provider: Option<String>,
}

/// Statistics for LLM calls
#[derive(Debug, Clone, Default)]
pub struct LLMStats {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub retry_count: u32,
    pub total_tokens: u32,
    pub avg_response_time_ms: f64,
}

/// LLM request
#[derive(Debug, Clone, Serialize)]
pub struct LLMRequest {
    pub system: String,
    pub user: String,
    pub context: String,
}

/// LLM response
#[derive(Debug, Clone, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub tokens: u32,
    pub model: String,
    pub response_time_ms: u64,
}

/// OpenRouter API request body
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// OpenRouter API response
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}

impl SwarmLLMClient {
    /// Create new LLM client from ProviderRouter (legacy)
    pub fn new(provider_router: Arc<ProviderRouter>) -> Self {
        Self {
            provider_router: Some(provider_router),
            unified_router: None,
            semaphore: Arc::new(Semaphore::new(3)), // Max 3 concurrent requests
            timeout: Duration::from_secs(60),
            max_retries: 3,
            stats: Arc::new(RwLock::new(LLMStats::default())),
            preferred_provider: None,
        }
    }

    /// Create new LLM client from Unified MultiProviderRouter
    pub fn with_unified_router(unified_router: Arc<MultiProviderRouter>) -> Self {
        Self {
            provider_router: None,
            unified_router: Some(unified_router),
            semaphore: Arc::new(Semaphore::new(3)),
            timeout: Duration::from_secs(60),
            max_retries: 3,
            stats: Arc::new(RwLock::new(LLMStats::default())),
            preferred_provider: None,
        }
    }

    /// Create from environment using legacy ProviderRouter
    pub fn from_env() -> Result<Self> {
        let router = ProviderRouter::from_env()?;
        Ok(Self::new(Arc::new(router)))
    }

    /// Create from environment using Unified MultiProviderRouter (recommended)
    pub async fn from_env_unified() -> Result<Self> {
        let router = MultiProviderRouter::from_env().await?;
        Ok(Self::with_unified_router(Arc::new(router)))
    }

    /// Set preferred provider
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.preferred_provider = Some(provider.into());
        self
    }

    /// Set max concurrent requests
    pub fn with_concurrency(mut self, max: usize) -> Self {
        self.semaphore = Arc::new(Semaphore::new(max));
        self
    }

    /// Execute LLM call with retry logic
    pub async fn execute(&self, request: LLMRequest) -> Result<LLMResponse> {
        let start = Instant::now();
        let mut attempts = 0;

        loop {
            match self.call_llm(&request).await {
                Ok(response) => {
                    // Update stats
                    let mut stats = self.stats.write().await;
                    stats.total_requests += 1;
                    stats.successful_requests += 1;
                    stats.total_tokens += response.tokens;

                    let elapsed = start.elapsed().as_millis() as f64;
                    stats.avg_response_time_ms =
                        (stats.avg_response_time_ms * (stats.total_requests - 1) as f64 + elapsed)
                            / stats.total_requests as f64;

                    return Ok(response);
                }
                Err(e) => {
                    attempts += 1;

                    let mut stats = self.stats.write().await;
                    stats.retry_count += 1;
                    drop(stats);

                    if attempts >= self.max_retries {
                        let mut stats = self.stats.write().await;
                        stats.total_requests += 1;
                        stats.failed_requests += 1;

                        return Err(anyhow!(
                            "LLM call failed after {} retries: {}",
                            self.max_retries,
                            e
                        ));
                    }

                    // Exponential backoff
                    let backoff = Duration::from_millis(500 * attempts as u64);
                    tracing::warn!(
                        "LLM call failed (attempt {}/{}), retrying in {:?}: {}",
                        attempts,
                        self.max_retries,
                        backoff,
                        e
                    );

                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// Execute multiple requests in parallel
    pub async fn execute_parallel(&self, requests: Vec<LLMRequest>) -> Vec<Result<LLMResponse>> {
        let futures = requests.into_iter().map(|req| {
            let client = self;
            async move { client.execute(req).await }
        });

        futures::future::join_all(futures).await
    }

    /// Call LLM API using available provider router
    async fn call_llm(&self, request: &LLMRequest) -> Result<LLMResponse> {
        // Acquire rate limit permit
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| anyhow!("Failed to acquire semaphore: {}", e))?;

        let start = Instant::now();

        // Use unified router if available (preferred), otherwise fall back to legacy
        if let Some(ref unified) = self.unified_router {
            self.call_unified(unified, request, start).await
        } else if let Some(ref legacy) = self.provider_router {
            self.call_legacy(legacy, request, start).await
        } else {
            Err(anyhow!("No LLM provider configured"))
        }
    }

    /// Call using unified provider system
    async fn call_unified(
        &self,
        router: &MultiProviderRouter,
        request: &LLMRequest,
        start: Instant,
    ) -> Result<LLMResponse> {
        let messages = vec![
            UnifiedMessage {
                role: MessageRole::System,
                content: request.system.clone(),
                name: None,
            },
            UnifiedMessage {
                role: MessageRole::User,
                content: format!("{}\n\nContext:\n{}", request.user, request.context),
                name: None,
            },
        ];

        let unified_request = crate::providers::unified::LLMRequest {
            messages,
            model: "default".to_string(), // Router will use default model
            temperature: 0.7,
            max_tokens: 4000,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            response_format: None,
        };

        let completion = router
            .complete(unified_request)
            .await
            .map_err(|e| anyhow!("Unified provider error: {}", e))?;

        Ok(LLMResponse {
            content: completion.content,
            tokens: completion.usage.total_tokens,
            model: completion.model,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Call using legacy provider router
    async fn call_legacy(
        &self,
        router: &ProviderRouter,
        request: &LLMRequest,
        start: Instant,
    ) -> Result<LLMResponse> {
        let user_prompt = format!("{}\n\nContext:\n{}", request.user, request.context);

        let completion = router
            .chat_completion(&request.system, &user_prompt)
            .await
            .map_err(|e| anyhow!("Provider error: {}", e))?;

        Ok(LLMResponse {
            content: completion.content,
            tokens: completion.token_cost,
            model: completion.llm_name,
            response_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Get statistics
    pub async fn get_stats(&self) -> LLMStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = LLMStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::router::ProviderRouter;

    fn create_test_router() -> Arc<ProviderRouter> {
        // Create router from env or use a default
        // For tests, we need to ensure at least one provider is configured
        // or skip the test if none are available
        match ProviderRouter::from_env() {
            Ok(router) => Arc::new(router),
            Err(_) => {
                // For unit tests without env vars, we can't test the full flow
                // These tests should be run as integration tests with real providers
                panic!(
                    "No LLM providers configured for testing. Set OPENROUTER_API_KEY or similar."
                );
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires LLM provider API key"]
    async fn test_llm_execution() {
        let router = create_test_router();
        let client = SwarmLLMClient::new(router);

        let request = LLMRequest {
            system: "You are a Rust expert".to_string(),
            user: "Generate a simple hello world function".to_string(),
            context: "Testing provider integration".to_string(),
        };

        let response = client.execute(request).await;
        assert!(
            response.is_ok(),
            "LLM execution failed: {:?}",
            response.err()
        );

        let response = response.unwrap();
        assert!(!response.content.is_empty(), "Response content is empty");
        assert!(response.tokens > 0, "Token count should be > 0");
        assert!(!response.model.is_empty(), "Model name should not be empty");
    }

    #[tokio::test]
    #[ignore = "Requires LLM provider API key"]
    async fn test_parallel_execution() {
        let router = create_test_router();
        let client = SwarmLLMClient::new(router).with_concurrency(3);

        let requests = vec![
            LLMRequest {
                system: "Test".to_string(),
                user: "Say hello".to_string(),
                context: "".to_string(),
            },
            LLMRequest {
                system: "Test".to_string(),
                user: "Say world".to_string(),
                context: "".to_string(),
            },
        ];

        let start = Instant::now();
        let results = client.execute_parallel(requests).await;
        let elapsed = start.elapsed();

        // Parallel execution should be faster than sequential
        println!("Parallel execution took: {:?}", elapsed);
        assert_eq!(results.len(), 2, "Should have 2 results");

        // At least one should succeed (fallback might kick in)
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert!(success_count > 0, "At least one request should succeed");
    }

    #[tokio::test]
    #[ignore = "Requires LLM provider API key"]
    async fn test_stats_tracking() {
        let router = create_test_router();
        let client = SwarmLLMClient::new(router);

        let request = LLMRequest {
            system: "Test".to_string(),
            user: "Count: 1".to_string(),
            context: "".to_string(),
        };

        // Execute twice
        let _ = client.execute(request.clone()).await;
        let _ = client.execute(request.clone()).await;

        let stats = client.get_stats().await;
        assert!(
            stats.total_requests >= 2,
            "Should have at least 2 total requests"
        );
        println!("Stats: {:?}", stats);
    }

    #[tokio::test]
    async fn test_provider_fallback() {
        // Test that we can create a client with multiple providers
        // This doesn't make actual API calls, just verifies configuration
        if let Ok(router) = ProviderRouter::from_env() {
            let client = SwarmLLMClient::new(Arc::new(router));
            let stats = client.get_stats().await;

            // Initially should have zero requests
            assert_eq!(stats.total_requests, 0);
            assert_eq!(stats.successful_requests, 0);
            assert_eq!(stats.failed_requests, 0);
            println!(
                "Provider fallback test: Client created successfully with stats: {:?}",
                stats
            );
        } else {
            println!("Skipping test_provider_fallback - no providers configured");
        }
    }
}
