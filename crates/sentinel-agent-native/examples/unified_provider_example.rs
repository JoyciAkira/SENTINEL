//! Real-world example: Using Unified Provider System
//!
//! This example demonstrates how to use the new unified provider system
//! with automatic fallback across multiple LLM providers.
//!
//! # Usage
//!
//! ```bash
//! # Set at least one API key
//! export OPENROUTER_API_KEY="sk-or-v1-..."
//! # Optional: Set fallback providers
//! export OPENAI_API_KEY="sk-..."
//! export ANTHROPIC_API_KEY="sk-ant-..."
//!
//! cargo run --example unified_provider_example
//! ```

use std::sync::Arc;
use std::time::Instant;

use sentinel_agent_native::providers::unified::{
    Message, MessageRole, MultiProviderRouter,
};
use sentinel_agent_native::swarm::{
    llm::{LLMRequest, SwarmLLMClient},
    SwarmCoordinator, SwarmConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     SENTINEL SWARM - Unified Provider System Demo          ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create unified provider router from environment
    println!("📡 Configuring LLM providers from environment...\n");
    let router = match MultiProviderRouter::from_env().await {
        Ok(r) => {
            let providers = r.list_providers();
            println!("✅ Configured {} provider(s): {:?}\n", providers.len(), providers);
            Arc::new(r)
        }
        Err(e) => {
            eprintln!("❌ No LLM providers configured!");
            eprintln!("\nPlease set one of these environment variables:");
            eprintln!("  - OPENROUTER_API_KEY (recommended - 40+ models)");
            eprintln!("  - OPENAI_API_KEY");
            eprintln!("  - ANTHROPIC_API_KEY");
            eprintln!("  - GOOGLE_API_KEY");
            eprintln!("  - GROQ_API_KEY");
            eprintln!("  - OLLAMA_HOST (for local models)\n");
            return Err(e.into());
        }
    };

    // Test 1: Direct provider usage
    println!("🧪 Test 1: Direct Provider Call");
    println!("─────────────────────────────────");
    test_direct_provider(router.clone()).await?;

    // Test 2: Using SwarmLLMClient with unified router
    println!("\n🧪 Test 2: SwarmLLMClient with Unified Router");
    println!("─────────────────────────────────────────────");
    test_swarm_client(router.clone()).await?;

    // Test 3: Full swarm execution
    println!("\n🧪 Test 3: Full Swarm Execution");
    println!("────────────────────────────────");
    test_swarm_execution(router.clone()).await?;

    println!("\n✅ All tests completed successfully!");
    println!("\n💡 Tip: Set multiple API keys for automatic fallback:");
    println!("   export OPENROUTER_API_KEY=...");
    println!("   export OPENAI_API_KEY=...");
    println!("   export ANTHROPIC_API_KEY=...");

    Ok(())
}

async fn test_direct_provider(
    router: Arc<MultiProviderRouter>,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message {
            role: MessageRole::System,
            content: "You are a helpful Rust programming assistant.".to_string(),
            name: None,
        },
        Message {
            role: MessageRole::User,
            content: "Generate a simple function that calculates fibonacci numbers.".to_string(),
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

    let start = Instant::now();
    let response = router.complete(request).await?;
    let elapsed = start.elapsed();

    println!("✓ Response received in {:?}", elapsed);
    println!("  Provider: {}", response.provider);
    println!("  Model: {}", response.model);
    println!("  Tokens: {} (prompt: {}, completion: {})",
        response.usage.total_tokens,
        response.usage.prompt_tokens,
        response.usage.completion_tokens
    );
    println!("\n📝 Generated Code:\n{}", response.content);

    // Extract files from response
    let files = sentinel_agent_native::swarm::parser::parse_llm_response(&response.content);
    if !files.is_empty() {
        println!("📁 Extracted {} file(s):", files.len());
        for file in &files {
            println!("  - {} ({})", file.path, file.language);
        }
    }

    Ok(())
}

async fn test_swarm_client(
    router: Arc<MultiProviderRouter>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create SwarmLLMClient with unified router
    let client = SwarmLLMClient::with_unified_router(router).with_concurrency(2);

    let request = LLMRequest {
        system: "You are an expert TypeScript developer.".to_string(),
        user: "Create a type-safe event emitter with TypeScript.".to_string(),
        context: "The emitter should support typed events and handlers.".to_string(),
    };

    let start = Instant::now();
    let response = client.execute(request).await?;
    let elapsed = start.elapsed();

    println!("✓ Response received in {:?}", elapsed);
    println!("  Model: {}", response.model);
    println!("  Tokens: {}", response.tokens);
    println!("\n📝 Generated Code:\n{}", response.content);

    // Get stats
    let stats = client.get_stats().await;
    println!("\n📊 Client Stats:");
    println!("  Total requests: {}", stats.total_requests);
    println!("  Successful: {}", stats.successful_requests);
    println!("  Failed: {}", stats.failed_requests);
    println!("  Avg response time: {:.0}ms", stats.avg_response_time_ms);

    Ok(())
}

async fn test_swarm_execution(
    router: Arc<MultiProviderRouter>,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm_client = Arc::new(SwarmLLMClient::with_unified_router(router));
    let config = SwarmConfig::default();

    let goal = "Build a simple CLI tool in Rust that reads JSON and outputs formatted YAML";

    println!("🎯 Goal: {}", goal);
    println!("🚀 Initializing swarm...");

    let start = Instant::now();
    let swarm = SwarmCoordinator::from_goal(goal, llm_client, config)
        .await
        .map_err(|e| format!("Failed to create swarm: {}", e))?;

    println!("✓ Swarm initialized in {:?}", start.elapsed());

    // Spawn agents
    println!("👥 Spawning agents...");
    let agents = swarm.spawn_agents().await?;
    println!("✓ Spawned {} agent(s)", agents.len());

    // Show agent details
    for (i, agent_id) in agents.iter().enumerate() {
        println!("  Agent {}: {:?}", i + 1, &agent_id.0[..8]);
    }

    // Start consensus mechanism
    println!("\n⚡ Starting consensus mechanism...");
    swarm.start_consensus().await?;
    println!("✓ Consensus mechanism active");

    // Demonstrate agent communication
    println!("\n📡 Testing agent communication...");
    use sentinel_agent_native::swarm::communication::{SwarmMessage, UrgencyLevel};
    
    if let Some(first_agent) = agents.first() {
        let test_msg = SwarmMessage::HelpRequest {
            by: *first_agent,
            issue: "Testing unified provider system".to_string(),
            urgency: UrgencyLevel::Low,
        };
        
        swarm.communication_bus.broadcast(test_msg).await?;
        println!("✓ Broadcasted test message to all agents");
    }

    println!("\n📊 Swarm Status:");
    println!("  Swarm ID: {:?}", &swarm.id.0[..8]);
    println!("  Total agents: {}", agents.len());
    println!("  Goal hash: {:?}", &swarm.goal_hash.as_bytes()[..8]);

    Ok(())
}
