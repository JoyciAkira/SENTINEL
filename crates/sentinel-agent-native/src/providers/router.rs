use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::llm_integration::{LLMChatClient, LLMChatCompletion};
use crate::openrouter::{OpenRouterClient, OpenRouterModel};
use crate::providers::anthropic::AnthropicClient;
use crate::providers::gemini::GeminiClient;
use crate::providers::openai_compatible::OpenAICompatibleClient;

#[derive(Debug, Deserialize)]
pub struct ProviderRouterConfig {
    pub default: Option<String>,
    pub fallbacks: Option<Vec<String>>,
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderConfig {
    OpenAIAuth {
        model: Option<String>,
        base_url: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    OpenRouter {
        api_key: Option<String>,
        api_key_env: Option<String>,
        model: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    OpenAI {
        api_key: Option<String>,
        api_key_env: Option<String>,
        model: Option<String>,
        base_url: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    Anthropic {
        api_key: Option<String>,
        api_key_env: Option<String>,
        model: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    Gemini {
        api_key: Option<String>,
        api_key_env: Option<String>,
        model: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
    },
    OpenAICompatible {
        name: Option<String>,
        api_key: Option<String>,
        api_key_env: Option<String>,
        base_url: String,
        model: String,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone)]
struct ProviderEntry {
    name: String,
    client: Arc<dyn LLMChatClient>,
}

#[derive(Debug, Clone)]
pub struct ProviderRouter {
    providers: Vec<ProviderEntry>,
}

impl ProviderRouter {
    fn normalize_openrouter_model(model: String) -> String {
        let trimmed = model.trim().trim_matches('"').trim_matches('\'');
        if trimmed.eq_ignore_ascii_case("deepseek/deepseek-r1:free") {
            return "deepseek/deepseek-r1-0528:free".to_string();
        }
        trimmed.to_string()
    }

    pub fn from_env() -> Result<Self> {
        if let Some(config) = Self::load_config_from_file()? {
            return Self::from_config(config);
        }

        Self::from_env_fallback()
    }

    pub fn from_config(config: ProviderRouterConfig) -> Result<Self> {
        let mut providers = Vec::new();
        let mut seen = HashSet::new();

        let mut order = Vec::new();
        if let Some(default) = config.default {
            order.push(default);
        }
        if let Some(fallbacks) = config.fallbacks {
            order.extend(fallbacks);
        }

        if order.is_empty() {
            order.extend(config.providers.keys().cloned());
        }

        for name in order {
            if seen.contains(&name) {
                continue;
            }
            let Some(config) = config.providers.get(&name) else {
                continue;
            };
            if let Some(entry) = Self::build_provider(&name, config)? {
                providers.push(entry);
                seen.insert(name);
            }
        }

        if providers.is_empty() {
            anyhow::bail!("No valid LLM providers configured.");
        }

        Ok(Self { providers })
    }

    fn from_env_fallback() -> Result<Self> {
        let mut providers = Vec::new();
        let mut seen = HashSet::new();

        let preferred = std::env::var("SENTINEL_LLM_PROVIDER").ok();
        let mut order = Vec::new();
        if let Some(provider) = preferred {
            order.push(provider.to_lowercase());
        }
        order.extend(vec![
            "openai_auth".to_string(),
            "openrouter".to_string(),
            "openai".to_string(),
            "anthropic".to_string(),
            "gemini".to_string(),
            "openai_compatible".to_string(),
        ]);

        for name in order {
            if seen.contains(&name) {
                continue;
            }
            if let Some(entry) = Self::build_provider_from_env(&name)? {
                providers.push(entry);
                seen.insert(name);
            }
        }

        if providers.is_empty() {
            anyhow::bail!(
                "No LLM providers found. Set OPENROUTER_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY, or SENTINEL_LLM_BASE_URL + SENTINEL_LLM_MODEL."
            );
        }

        Ok(Self { providers })
    }

    fn load_config_from_file() -> Result<Option<ProviderRouterConfig>> {
        let path = if let Ok(path) = std::env::var("SENTINEL_LLM_CONFIG") {
            PathBuf::from(path)
        } else {
            let default_path = PathBuf::from("sentinel_llm_config.json");
            if default_path.exists() {
                default_path
            } else {
                return Ok(None);
            }
        };

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read LLM config at {:?}", path))?;
        match serde_json::from_str::<ProviderRouterConfig>(&content) {
            Ok(config) => Ok(Some(config)),
            Err(error) => {
                tracing::warn!(
                    "Invalid LLM config JSON at {:?}: {}. Falling back to env-based provider configuration.",
                    path,
                    error
                );
                Ok(None)
            }
        }
    }

    fn build_provider(name: &str, config: &ProviderConfig) -> Result<Option<ProviderEntry>> {
        let entry = match config {
            ProviderConfig::OpenAIAuth {
                model,
                base_url,
                temperature,
                max_tokens,
            } => {
                let api_key = load_codex_api_key()?;
                let model = model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string());
                let base_url = base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let mut client =
                    OpenAICompatibleClient::new("OpenAI (ChatGPT)", Some(api_key), base_url, model);
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }
            }
            ProviderConfig::OpenRouter {
                api_key,
                api_key_env,
                model,
                temperature,
                max_tokens,
            } => {
                let api_key = resolve_api_key(api_key, api_key_env, "OPENROUTER_API_KEY")?;
                let model = model
                    .clone()
                    .or_else(|| std::env::var("OPENROUTER_MODEL").ok())
                    .unwrap_or_else(|| "deepseek/deepseek-r1-0528:free".to_string());
                let model = Self::normalize_openrouter_model(model);
                let mut client = OpenRouterClient::new(api_key, OpenRouterModel::Custom(model));
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }
            }
            ProviderConfig::OpenAI {
                api_key,
                api_key_env,
                model,
                base_url,
                temperature,
                max_tokens,
            } => {
                let api_key = resolve_api_key(api_key, api_key_env, "OPENAI_API_KEY")?;
                let model = model.clone().unwrap_or_else(|| "gpt-4o-mini".to_string());
                let base_url = base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                let mut client =
                    OpenAICompatibleClient::new("OpenAI", Some(api_key), base_url, model);
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }
            }
            ProviderConfig::Anthropic {
                api_key,
                api_key_env,
                model,
                temperature,
                max_tokens,
            } => {
                let api_key = resolve_api_key(api_key, api_key_env, "ANTHROPIC_API_KEY")?;
                let model = model
                    .clone()
                    .unwrap_or_else(|| "claude-3-5-sonnet-20240620".to_string());
                let mut client = AnthropicClient::new(api_key, model);
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }
            }
            ProviderConfig::Gemini {
                api_key,
                api_key_env,
                model,
                temperature,
                max_tokens,
            } => {
                let api_key = resolve_api_key(api_key, api_key_env, "GEMINI_API_KEY")?;
                let model = model
                    .clone()
                    .unwrap_or_else(|| "gemini-1.5-pro".to_string());
                let mut client = GeminiClient::new(api_key, model);
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }
            }
            ProviderConfig::OpenAICompatible {
                name,
                api_key,
                api_key_env,
                base_url,
                model,
                temperature,
                max_tokens,
                headers,
            } => {
                let api_key =
                    resolve_api_key_optional(api_key, api_key_env, "SENTINEL_LLM_API_KEY");
                let display_name = name
                    .clone()
                    .unwrap_or_else(|| "OpenAI-Compatible".to_string());
                let mut client = OpenAICompatibleClient::new(
                    display_name.clone(),
                    api_key,
                    base_url.clone(),
                    model.clone(),
                );
                if let Some(value) = *temperature {
                    client = client.with_temperature(value);
                }
                if let Some(value) = *max_tokens {
                    client = client.with_max_tokens(value);
                }
                if let Some(extra_headers) = headers {
                    for (key, value) in extra_headers {
                        client = client.with_header(key.clone(), value.clone());
                    }
                }
                ProviderEntry {
                    name: name
                        .clone()
                        .unwrap_or_else(|| "openai_compatible".to_string()),
                    client: Arc::new(client),
                }
            }
        };

        Ok(Some(entry))
    }

    fn build_provider_from_env(name: &str) -> Result<Option<ProviderEntry>> {
        match name {
            "openai_auth" => {
                let allow = std::env::var("SENTINEL_OPENAI_AUTH")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
                    || codex_auth_path().is_some();
                if !allow {
                    return Ok(None);
                }
                let api_key = match load_codex_api_key() {
                    Ok(key) => key,
                    Err(_) => return Ok(None),
                };
                let model =
                    std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
                let base_url = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                let client =
                    OpenAICompatibleClient::new("OpenAI (ChatGPT)", Some(api_key), base_url, model);
                Ok(Some(ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }))
            }
            "openrouter" => {
                let api_key = match std::env::var("OPENROUTER_API_KEY") {
                    Ok(key) => key,
                    Err(_) => return Ok(None),
                };
                let model = std::env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "deepseek/deepseek-r1-0528:free".to_string());
                let model = Self::normalize_openrouter_model(model);
                let client = OpenRouterClient::new(api_key, OpenRouterModel::Custom(model));
                Ok(Some(ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }))
            }
            "openai" => {
                let api_key = match std::env::var("OPENAI_API_KEY") {
                    Ok(key) => key,
                    Err(_) => return Ok(None),
                };
                let model =
                    std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
                let base_url = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                let client = OpenAICompatibleClient::new("OpenAI", Some(api_key), base_url, model);
                Ok(Some(ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }))
            }
            "anthropic" => {
                let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                    Ok(key) => key,
                    Err(_) => return Ok(None),
                };
                let model = std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-5-sonnet-20240620".to_string());
                let client = AnthropicClient::new(api_key, model);
                Ok(Some(ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }))
            }
            "gemini" => {
                let api_key = match std::env::var("GEMINI_API_KEY") {
                    Ok(key) => key,
                    Err(_) => return Ok(None),
                };
                let model =
                    std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-pro".to_string());
                let client = GeminiClient::new(api_key, model);
                Ok(Some(ProviderEntry {
                    name: name.to_string(),
                    client: Arc::new(client),
                }))
            }
            "openai_compatible" => {
                let base_url = match std::env::var("SENTINEL_LLM_BASE_URL") {
                    Ok(value) => value,
                    Err(_) => return Ok(None),
                };
                let model = match std::env::var("SENTINEL_LLM_MODEL") {
                    Ok(value) => value,
                    Err(_) => return Ok(None),
                };
                let name = std::env::var("SENTINEL_LLM_NAME")
                    .unwrap_or_else(|_| "OpenAI-Compatible".to_string());
                let api_key = std::env::var("SENTINEL_LLM_API_KEY").ok();
                let client = OpenAICompatibleClient::new(name, api_key, base_url, model);
                Ok(Some(ProviderEntry {
                    name: "openai_compatible".to_string(),
                    client: Arc::new(client),
                }))
            }
            _ => Ok(None),
        }
    }
}

fn resolve_api_key(
    explicit: &Option<String>,
    env_hint: &Option<String>,
    default_env: &str,
) -> Result<String> {
    if let Some(value) = explicit.clone() {
        return Ok(value);
    }
    if let Some(env_name) = env_hint {
        if let Ok(value) = std::env::var(env_name) {
            return Ok(value);
        }
    }
    std::env::var(default_env)
        .with_context(|| format!("Missing required API key env var {}", default_env))
}

fn resolve_api_key_optional(
    explicit: &Option<String>,
    env_hint: &Option<String>,
    default_env: &str,
) -> Option<String> {
    if let Some(value) = explicit.clone() {
        return Some(value);
    }
    if let Some(env_name) = env_hint {
        if let Ok(value) = std::env::var(env_name) {
            return Some(value);
        }
    }
    std::env::var(default_env).ok()
}

fn load_codex_api_key() -> Result<String> {
    let auth_path = codex_auth_path().context("Codex auth.json not found")?;
    let content = std::fs::read_to_string(&auth_path)
        .with_context(|| format!("Failed to read {:?}", auth_path))?;
    let json: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in {:?}", auth_path))?;

    if let Some(value) = json
        .get("tokens")
        .and_then(|t| t.get("access_token"))
        .and_then(|v| v.as_str())
    {
        return Ok(value.to_string());
    }

    if let Some(value) = json.get("api_key").and_then(|v| v.as_str()) {
        return Ok(value.to_string());
    }
    if let Some(value) = json.get("apiKey").and_then(|v| v.as_str()) {
        return Ok(value.to_string());
    }
    if let Some(value) = json.get("OPENAI_API_KEY").and_then(|v| v.as_str()) {
        return Ok(value.to_string());
    }

    if let Some(value) = find_api_key_in_json(&json) {
        return Ok(value);
    }

    anyhow::bail!("No OpenAI API key found in Codex auth.json");
}

fn find_api_key_in_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            if text.starts_with("sk-") {
                Some(text.to_string())
            } else {
                None
            }
        }
        serde_json::Value::Array(items) => items.iter().find_map(find_api_key_in_json),
        serde_json::Value::Object(map) => map.values().find_map(find_api_key_in_json),
        _ => None,
    }
}

fn codex_auth_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        let path = PathBuf::from(home).join("auth.json");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let path = PathBuf::from(home).join(".codex").join("auth.json");
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(home) = std::env::var("USERPROFILE") {
        let path = PathBuf::from(home).join(".codex").join("auth.json");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[async_trait::async_trait]
impl LLMChatClient for ProviderRouter {
    async fn chat_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<LLMChatCompletion> {
        let mut errors = Vec::new();

        for provider in &self.providers {
            match provider
                .client
                .chat_completion(system_prompt, user_prompt)
                .await
            {
                Ok(result) => {
                    return Ok(LLMChatCompletion {
                        llm_name: result.llm_name,
                        content: result.content,
                        token_cost: result.token_cost,
                    })
                }
                Err(err) => {
                    errors.push(format!("{}: {}", provider.name, err));
                }
            }
        }

        anyhow::bail!("All LLM providers failed:\n{}", errors.join("\n"));
    }
}
