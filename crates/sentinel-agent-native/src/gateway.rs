use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;

/// Unified LLM Request
#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub required_capabilities: Vec<String>, // e.g. "json_mode", "vision"
}

/// Unified LLM Response
#[derive(Debug, Clone)]
pub struct GatewayResponse {
    pub content: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: u32,
    pub cached: bool,
}

/// Abstract LLM Provider Interface (The "Poliglotta")
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate(&self, req: &GatewayRequest) -> Result<GatewayResponse>;
    fn name(&self) -> String;
    fn supports(&self, capability: &str) -> bool;
}

/// The Intelligence Gateway (The "Heimdall")
/// Handles routing, caching, and failover.
pub struct IntelligenceGateway {
    providers: Vec<Arc<dyn LLMProvider>>,
    cache: Arc<Mutex<HashMap<String, GatewayResponse>>>,
    fallback_strategy: FallbackStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum FallbackStrategy {
    Sequential, // Try 1, then 2, then 3
    BestMatch,  // Try provider with best capabilities
}

impl IntelligenceGateway {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            fallback_strategy: FallbackStrategy::Sequential,
        }
    }

    pub fn register_provider(&mut self, provider: Arc<dyn LLMProvider>) {
        self.providers.push(provider);
    }

    /// Primary Entry Point: Ask the collective intelligence
    pub async fn ask(&self, req: GatewayRequest) -> Result<GatewayResponse> {
        // 1. Check Cache (Semantic Caching Simulation)
        let cache_key = self.hash_request(&req);
        {
            let cache = self.cache.lock().await;
            if let Some(cached_resp) = cache.get(&cache_key) {
                tracing::info!("Gateway: Cache hit for query");
                let mut resp = cached_resp.clone();
                resp.cached = true;
                return Ok(resp);
            }
        }

        // 2. Select Provider & Failover Loop
        let mut last_error = anyhow::anyhow!("No providers available");

        for provider in &self.providers {
            // Check capabilities
            if !req.required_capabilities.iter().all(|cap| provider.supports(cap)) {
                continue;
            }

            match provider.generate(&req).await {
                Ok(response) => {
                    // 3. Cache Success
                    let mut cache = self.cache.lock().await;
                    cache.insert(cache_key, response.clone());
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("Provider {} failed: {}. Trying next...", provider.name(), e);
                    last_error = e;
                }
            }
        }

        Err(last_error.context("All providers failed"))
    }

    fn hash_request(&self, req: &GatewayRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        req.prompt.hash(&mut hasher);
        req.system_prompt.hash(&mut hasher);
        // Simple hash for demo purposes. In prod, use semantic embeddings.
        format!("{:x}", hasher.finish())
    }
}

// --- MOCK PROVIDERS FOR TESTING ---

pub struct MockProvider {
    name: String,
    should_fail: bool,
}

impl MockProvider {
    pub fn new(name: &str, should_fail: bool) -> Self {
        Self { name: name.to_string(), should_fail }
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    async fn generate(&self, req: &GatewayRequest) -> Result<GatewayResponse> {
        if self.should_fail {
            return Err(anyhow::anyhow!("Simulated failure"));
        }
        
        Ok(GatewayResponse {
            content: format!("Processed by {}: {}", self.name, req.prompt),
            provider: self.name.clone(),
            model: "mock-model".to_string(),
            tokens_used: 10,
            cached: false,
        })
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn supports(&self, _capability: &str) -> bool {
        true
    }
}
