//! Example: Multi-Provider Swarm Execution
//!
//! This example shows how SENTINEL SWARM automatically uses
//! multiple LLM providers with fallback support.

use std::sync::Arc;
use sentinel_agent_native::swarm::{
    SwarmCoordinator, SwarmConfig,
    llm::SwarmLLMClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-Provider Swarm Example\n");
    
    // Initialize LLM client with auto-detection
    // Supports: OpenRouter, OpenAI, Anthropic, Google, Ollama, Groq, etc.
    println!("📡 Initializing LLM client (auto-detecting providers)...");
    
    // This will:
    // 1. Check for sentinel_llm_config.json
    // 2. Check environment variables
    // 3. Auto-configure all available providers
    // 4. Set up fallback chain
    
    let llm_client = match SwarmLLMClient::from_env() {
        Ok(client) => {
            println!("   ✓ Providers configured successfully");
            Arc::new(client)
        }
        Err(e) => {
            println!("   ❌ No providers found: {}", e);
            println!("\n   Please set one of these environment variables:");
            println!("   - OPENROUTER_API_KEY");
            println!("   - OPENAI_API_KEY");
            println!("   - ANTHROPIC_API_KEY");
            println!("   - GEMINI_API_KEY");
            println!("   - SENTINEL_LLM_BASE_URL (for local/Ollama)");
            println!("\n   Or create sentinel_llm_config.json");
            return Err(e.into());
        }
    };
    
    // Configure swarm
    let config = SwarmConfig {
        quorum_threshold: 0.75,
        consensus_interval_ms: 100,
        max_concurrent_llm: 3,
        enable_prediction: true,
        enable_balancing: true,
        vote_timeout_ms: 5000,
    };
    
    // Example 1: Simple task
    println!("\n🎯 Example 1: Simple authentication task");
    let goal1 = "Create a function to validate email addresses";
    
    let swarm1 = SwarmCoordinator::from_goal(goal1, llm_client.clone(), config.clone()).await?;
    let result1 = swarm1.run().await?;
    
    println!("   ✓ Completed in {}ms", result1.execution_time_ms);
    println!("   ✓ {} agents worked in parallel", result1.agent_count);
    
    // Example 2: Complex task with multiple providers
    println!("\n🎯 Example 2: Complex full-stack task");
    let goal2 = "Build JWT authentication with refresh tokens, password hashing, and comprehensive tests";
    
    let swarm2 = SwarmCoordinator::from_goal(goal2, llm_client.clone(), config.clone()).await?;
    let result2 = swarm2.run().await?;
    
    println!("   ✓ Completed in {}ms", result2.execution_time_ms);
    println!("   ✓ {} agents worked in parallel", result2.agent_count);
    println!("   ✓ {} conflicts auto-resolved", result2.conflicts_resolved);
    
    // Show provider usage
    println!("\n📊 Provider Usage:");
    let stats = llm_client.get_stats().await;
    println!("   Total requests: {}", stats.total_requests);
    println!("   Successful: {}", stats.successful_requests);
    println!("   Failed: {}", stats.failed_requests);
    println!("   Retries: {}", stats.retry_count);
    println!("   Avg response time: {:.0}ms", stats.avg_response_time_ms);
    
    println!("\n✨ Done! Swarm used available providers with automatic fallback.");
    println!("   If one provider failed, others were used automatically.");
    
    Ok(())
}

/*
Expected output with multiple providers configured:

🚀 Multi-Provider Swarm Example

📡 Initializing LLM client (auto-detecting providers)...
   ✓ Providers configured successfully
   Found: openrouter, openai, anthropic

🎯 Example 1: Simple authentication task
   ✓ Completed in 3200ms
   ✓ 3 agents worked in parallel

🎯 Example 2: Complex full-stack task  
   ✓ Completed in 8500ms
   ✓ 5 agents worked in parallel
   ✓ 1 conflicts auto-resolved

📊 Provider Usage:
   Total requests: 8
   Successful: 8
   Failed: 0
   Retries: 1 (OpenRouter timeout, fallback to OpenAI)
   Avg response time: 850ms

✨ Done! Swarm used available providers with automatic fallback.
   If one provider failed, others were used automatically.

Fallback chain worked:
  1. OpenRouter (primary) - timeout after 5s
  2. OpenAI (fallback 1) - success ✓
*/