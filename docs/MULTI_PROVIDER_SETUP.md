# Multi-Provider LLM Support

SENTINEL SWARM now supports **40+ LLM providers** with automatic fallback, inspired by Vercel AI SDK architecture.

## Supported Providers

### Major Cloud Providers
- **OpenAI** (GPT-4, GPT-3.5, o1)
- **Anthropic** (Claude 3.5, Claude 3)
- **Google** (Gemini 1.5 Pro, Gemini Flash)
- **Google Gemini CLI** (OAuth-based, no API key required!)
- **Azure OpenAI**

### Inference Providers  
- **OpenRouter** (100+ models, unified API)
- **Groq** (ultra-fast inference)
- **Together AI**
- **Fireworks**
- **DeepInfra**
- **Cerebras**

### Local/Self-Hosted
- **Ollama** (local models)
- **LM Studio**
- **Any OpenAI-compatible API**

## Configuration

### Option 1: Environment Variables (Simple)

Set ONE of these:

```bash
# Option A: OpenRouter (recommended - supports 100+ models)
export OPENROUTER_API_KEY="sk-or-v1-..."
export OPENROUTER_MODEL="deepseek/deepseek-r1-0528:free"

# Option B: OpenAI
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o-mini"

# Option C: Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
export ANTHROPIC_MODEL="claude-3-5-sonnet-20240620"

# Option D: Google
export GEMINI_API_KEY="..."
export GEMINI_MODEL="gemini-1.5-pro"

# Option E: Ollama (local)
export SENTINEL_LLM_BASE_URL="http://localhost:11434/v1"
export SENTINEL_LLM_MODEL="llama3.2:latest"

# Option F: Groq (fast)
export GROQ_API_KEY="gsk_..."

# Option G: Gemini CLI (OAuth - no API key needed!)
# First install: npm install -g @anthropic-ai/gemini-cli
# Then authenticate: gemini auth login
# That's it! No API key required.
```

### Option 2: Configuration File (Advanced)

Create `sentinel_llm_config.json`:

```json
{
  "default": "openrouter",
  "fallbacks": ["openai", "anthropic"],
  
  "providers": {
    "openrouter": {
      "type": "openrouter",
      "api_key_env": "OPENROUTER_API_KEY",
      "model": "deepseek/deepseek-r1-0528:free",
      "temperature": 0.3,
      "max_tokens": 2048
    },
    
    "openai": {
      "type": "openai",
      "api_key_env": "OPENAI_API_KEY",
      "model": "gpt-4o-mini",
      "temperature": 0.3,
      "max_tokens": 2048
    }
  }
}
```

Set path:
```bash
export SENTINEL_LLM_CONFIG="/path/to/sentinel_llm_config.json"
```

## Usage

### Basic Usage (Auto-Detect)

```rust
use std::sync::Arc;
use sentinel_agent_native::swarm::{
    SwarmCoordinator, SwarmConfig,
    llm::SwarmLLMClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-detect providers from environment
    let llm_client = Arc::new(SwarmLLMClient::from_env()?);
    
    let config = SwarmConfig::default();
    
    let swarm = SwarmCoordinator::from_goal(
        "Build authentication system",
        llm_client,
        config
    ).await?;
    
    let result = swarm.run().await?;
    println!("Done! Used providers: {:?}", result);
    
    Ok(())
}
```

### Advanced: Specify Provider

```rust
use sentinel_agent_native::providers::router::ProviderRouter;

// Create router with specific provider
let router = ProviderRouter::from_env()?;
let llm_client = Arc::new(
    SwarmLLMClient::new(Arc::new(router))
        .with_provider("openai")  // Force OpenAI
        .with_concurrency(3)
);
```

### Multiple Providers with Fallback

```rust
// Configure multiple providers with automatic fallback
// If OpenRouter fails, automatically tries OpenAI, then Anthropic

let router = ProviderRouter::from_env()?;  // Reads sentinel_llm_config.json
let llm_client = Arc::new(SwarmLLMClient::new(Arc::new(router)));
```

## Provider Selection Priority

1. **Explicit config** (sentinel_llm_config.json)
2. **Environment variable**: `SENTINEL_LLM_PROVIDER=openai`
3. **Auto-detect** (checks env vars in order):
   - OPENROUTER_API_KEY
   - OPENAI_API_KEY
   - ANTHROPIC_API_KEY
   - GEMINI_API_KEY
   - SENTINEL_LLM_BASE_URL (for local)

## Cost Optimization

### Free Models
```json
{
  "providers": {
    "openrouter": {
      "type": "openrouter",
      "model": "deepseek/deepseek-r1-0528:free"
    }
  }
}
```

### Cost-Based Routing
Use different providers for different tasks:
- **Complex tasks**: GPT-4, Claude (expensive but accurate)
- **Simple tasks**: GPT-3.5, Llama (cheap)
- **Local tasks**: Ollama (free)

## Fallback Behavior

When a provider fails:
1. Logs the error
2. Automatically tries next provider in fallback list
3. Continues until success or all providers exhausted
4. Returns error only if ALL providers fail

Example:
```
[ERROR] OpenRouter timeout, trying fallback...
[INFO] Fallback to OpenAI successful
```

## Performance Comparison

| Provider | Latency | Cost | Quality |
|----------|---------|------|---------|
| Groq | ~100ms | $ | ⭐⭐⭐ |
| OpenAI GPT-4 | ~500ms | $$$ | ⭐⭐⭐⭐⭐ |
| Claude 3.5 | ~800ms | $$$ | ⭐⭐⭐⭐⭐ |
| Ollama (local) | ~2000ms | Free | ⭐⭐⭐ |
| OpenRouter Free | ~1500ms | Free | ⭐⭐⭐⭐ |

## Troubleshooting

### "No LLM providers found"
Set at least one API key:
```bash
export OPENROUTER_API_KEY="your-key"
```

### "All LLM providers failed"
Check:
1. API keys are valid
2. Internet connection
3. Rate limits not exceeded
4. Model names are correct

### Switch Provider
```bash
# Temporarily switch
export SENTINEL_LLM_PROVIDER="openai"

# Or edit config
vim sentinel_llm_config.json
```

## Migration from Single Provider

**Before (OpenRouter only):**
```rust
let client = SwarmLLMClient::new("sk-or-v1-...");
```

**After (Multi-provider):**
```rust
// Just set OPENROUTER_API_KEY in environment
// Or create sentinel_llm_config.json
let client = SwarmLLMClient::from_env()?;
```

Zero code changes needed - just configuration!

## Advanced: Custom Provider

Add any OpenAI-compatible API:

```json
{
  "providers": {
    "my_custom": {
      "type": "openai_compatible",
      "name": "My Custom LLM",
      "base_url": "https://api.mycustomllm.com/v1",
      "model": "my-model-v1",
      "api_key": "my-api-key"
    }
  }
}
```

## Security Best Practices

1. **Never commit API keys** - use environment variables
2. **Use separate keys** for dev/prod
3. **Rotate keys regularly**
4. **Monitor usage** in provider dashboards
5. **Set spending limits** on provider accounts

## New Unified Provider System (v2)

SENTINEL SWARM now includes a new **Unified Provider System** with enhanced features:

### Features
- **6 Built-in Providers**: OpenAI, Anthropic, Google, OpenRouter, Groq, Ollama
- **Automatic Fallback**: If one provider fails, automatically tries the next
- **Rate Limiting**: Built-in request throttling per provider
- **Health Monitoring**: Tracks provider health and latency
- **Unified API**: Same interface for all providers

### Quick Start with Unified System

```rust
use sentinel_agent_native::providers::unified::MultiProviderRouter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Auto-detect all configured providers
    let router = MultiProviderRouter::from_env().await?;
    
    // Use with SwarmLLMClient
    let client = SwarmLLMClient::with_unified_router(Arc::new(router));
    
    // Now you have automatic fallback across all providers!
    Ok(())
}
```

### Environment Variables for Unified System

```bash
# Set multiple providers for fallback
export OPENROUTER_API_KEY="sk-or-v1-..."
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."
export GROQ_API_KEY="gsk_..."
export OLLAMA_HOST="http://localhost:11434"  # Optional

# Run example
cargo run --example unified_provider_example
```

### Provider Priority

The unified system tries providers in this order:
1. **OpenRouter** (40+ models, best value)
2. **OpenAI** (reliable)
3. **Anthropic** (Claude - excellent reasoning)
4. **Google** (Gemini - good for code)
5. **Groq** (fast inference)
6. **Ollama** (local models)

### Running the Example

```bash
# 1. Set at least one API key
export OPENROUTER_API_KEY="sk-or-v1-..."

# 2. Run the example
cargo run --example unified_provider_example

# Output:
# ✅ Configured 1 provider(s): ["openrouter"]
# 🧪 Test 1: Direct Provider Call
# ✓ Response received in 1.2s
#   Provider: openrouter
#   Model: deepseek/deepseek-r1-0528:free
#   Tokens: 342 (prompt: 45, completion: 297)
```

### Direct Provider Usage

```rust
use sentinel_agent_native::providers::unified::{
    Message, MessageRole, MultiProviderRouter
};

let router = MultiProviderRouter::from_env().await?;

let messages = vec![
    Message {
        role: MessageRole::System,
        content: "You are a helpful assistant.".to_string(),
        name: None,
    },
    Message {
        role: MessageRole::User,
        content: "Hello!".to_string(),
        name: None,
    },
];

let request = sentinel_agent_native::providers::unified::LLMRequest {
    messages,
    model: "default".to_string(),
    temperature: 0.7,
    max_tokens: 500,
    top_p: None,
    frequency_penalty: None,
    presence_penalty: None,
    stream: false,
    response_format: None,
};

let response = router.complete(request).await?;
println!("Response: {}", response.content);
println!("Provider used: {}", response.provider);
println!("Tokens: {}", response.usage.total_tokens);
```

## Gemini CLI (OAuth - No API Key Required!)

Google Gemini CLI is a **special provider** that uses OAuth authentication instead of API keys. This means you can use Google AI Pro models without any API key!

### Installation

```bash
# Install Gemini CLI
npm install -g @anthropic-ai/gemini-cli

# Authenticate with Google
gemini auth login
```

### Configuration

Add to `sentinel_llm_config.json`:

```json
{
  "providers": {
    "gemini_cli": {
      "type": "gemini_cli",
      "model": null
    }
  }
}
```

Or use environment variable:
```bash
export SENTINEL_LLM_PROVIDER="gemini_cli"
```

### Features

- **No API Key Required**: Uses your Google account OAuth
- **Automatic Fallback**: If the primary model is rate-limited (HTTP 429), automatically tries fallback models
- **Fallback Models**: gemini-2.0-flash, gemini-2.0-flash-lite, gemini-1.5-flash, gemini-1.5-flash-8b
- **Free with Google AI Pro**: Uses your existing Google AI Pro subscription

### Usage

```rust
use sentinel_agent_native::providers::router::ProviderRouter;
use sentinel_agent_native::llm_integration::LLMChatClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = ProviderRouter::from_env()?;
    
    let result = router.chat_completion(
        "You are a helpful assistant.",
        "Hello, how are you?"
    ).await?;
    
    println!("Response: {}", result.content);
    println!("Provider: {}", result.llm_name);
    Ok(())
}
```

### Legal Note

✅ **Legal**: Gemini CLI is Apache 2.0 open-source. The `--prompt` mode is explicitly designed for scripting. You use your own Google AI Pro subscription.

❌ **Not Legal**: Reselling access or sharing credentials.

## Next Steps

1. Choose your provider(s) - see comparison above
2. Get API key from provider
3. Set environment variable or create config file
4. Run swarm - it auto-detects everything!

**Ready to use any LLM provider with SENTINEL SWARM! 🚀**