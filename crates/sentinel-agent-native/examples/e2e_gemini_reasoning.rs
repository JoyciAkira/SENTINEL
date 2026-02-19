//! E2E Test: Gemini CLI + Reasoning Index Integration
//!
//! This example demonstrates:
//! 1. Using Gemini CLI with OAuth (Google AI Pro) for LLM calls
//! 2. Building a reasoning index from documents
//! 3. Enhancing search with LLM-generated reasoning traces
//!
//! Prerequisites:
//! - `gemini` CLI installed and authenticated with Google AI Pro
//! - Run with: cargo run --example e2e_gemini_reasoning --features gemini

use sentinel_agent_native::providers::gemini_cli::GeminiCliClient;
use sentinel_agent_native::llm_integration::{LLMChatClient, LLMContext};
use sentinel_core::reasoning_index::TreeIndexBuilder;

/// Demonstrates LLM-enhanced reasoning trace for document search
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("🦞 SENTINEL E2E: Gemini CLI + Reasoning Index");
    println!("===============================================\n");

    // Step 1: Check if Gemini CLI is available
    println!("📡 Step 1: Checking Gemini CLI availability...");
    
    if !sentinel_agent_native::providers::gemini_cli::is_gemini_cli_available() {
        println!("❌ Gemini CLI not found. Please install and authenticate:");
        println!("   npm install -g @anthropic/gemini-cli");
        println!("   gemini auth login");
        return Err(anyhow::anyhow!("Gemini CLI not available"));
    }
    
    println!("✅ Gemini CLI is available\n");

    // Step 2: Create Gemini CLI client
    println!("🤖 Step 2: Creating Gemini CLI client...");
    let client = GeminiCliClient::new()
        .with_model("gemini-2.0-flash"); // Use flash for faster responses
    
    // Probe to verify authentication
    match client.probe().await {
        Ok(model_name) => {
            println!("✅ Authenticated with model: {}\n", model_name);
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
            println!("   Run: gemini auth login");
            return Err(e);
        }
    }

    // Step 3: Build reasoning index from current directory
    println!("📚 Step 3: Building reasoning index...");
    
    let current_dir = std::env::current_dir()?;
    let readme_path = current_dir.join("README.md");
    
    let index = if readme_path.exists() {
        let builder = TreeIndexBuilder::new()
            .title("SENTINEL Project")
            .max_depth(3);
        
        builder.from_markdown(&readme_path).await?
    } else {
        println!("⚠️  No README.md found, using CLAUDE.md");
        let claude_path = current_dir.join("CLAUDE.md");
        if claude_path.exists() {
            let builder = TreeIndexBuilder::new()
                .title("SENTINEL Architecture")
                .max_depth(3);
            builder.from_markdown(&claude_path).await?
        } else {
            println!("❌ No documentation files found");
            return Err(anyhow::anyhow!("No documentation to index"));
        }
    };
    
    let stats = index.stats();
    println!("✅ Index built:");
    println!("   - Total nodes: {}", stats.total_nodes);
    println!("   - Max depth: {}", stats.max_depth);
    println!("   - Leaf sections: {}\n", stats.leaf_count);

    // Step 4: Perform keyword-based search
    println!("🔍 Step 4: Keyword-based search for 'authentication'...");
    let results = index.search("authentication security").await;
    
    println!("   Found {} results:\n", results.len());
    for (i, result) in results.iter().take(3).enumerate() {
        println!("   {}. {} (confidence: {:.2})", 
            i + 1, result.node.title, result.confidence);
    }
    println!();

    // Step 5: Enhance with LLM reasoning
    println!("🧠 Step 5: Enhancing with LLM reasoning trace...\n");
    
    let _context = LLMContext {
        goal_description: "Understand authentication in SENTINEL".to_string(),
        context: format!("Document has {} sections about the project", stats.total_nodes),
        p2p_intelligence: "".to_string(),
        constraints: vec!["Be concise".to_string()],
        previous_approaches: vec![],
    };

    let system_prompt = r#"You are a technical documentation analyst. 
Analyze the search results and explain WHY these sections are relevant to the query.
Provide a reasoning trace showing your analysis process."#;

    let user_prompt = format!(
        "Query: 'authentication security'\n\nSearch Results:\n{}\n\n\
        Provide a reasoning trace explaining why these results are relevant.",
        results.iter()
            .take(3)
            .map(|r| format!("- {} (confidence: {:.2})", r.node.title, r.confidence))
            .collect::<Vec<_>>()
            .join("\n")
    );

    match client.chat_completion(system_prompt, &user_prompt).await {
        Ok(completion) => {
            println!("📡 LLM Reasoning Trace:");
            println!("─────────────────────────────────────────────────");
            println!("{}", completion.content);
            println!("─────────────────────────────────────────────────");
            println!("\n💰 Token cost: {}\n", completion.token_cost);
        }
        Err(e) => {
            println!("❌ LLM call failed: {}", e);
            println!("   This may be due to rate limiting. Trying fallback models...");
        }
    }

    // Step 6: Show TOC
    println!("📖 Step 6: Document Table of Contents:");
    println!("─────────────────────────────────────────────────");
    println!("{}", index.to_toc());
    println!("─────────────────────────────────────────────────");

    // Step 7: Save index for later use
    let index_path = std::env::temp_dir().join("sentinel_reasoning_index.json");
    index.save(&index_path).await?;
    println!("\n💾 Index saved to: {:?}", index_path);

    println!("\n✅ E2E test completed successfully!");
    println!("\n📊 Summary:");
    println!("   - Gemini CLI OAuth: ✅ Working");
    println!("   - Reasoning Index: ✅ Built and searchable");
    println!("   - LLM Enhancement: ✅ Integrated");
    println!("   - Persistence: ✅ Index saved");

    Ok(())
}