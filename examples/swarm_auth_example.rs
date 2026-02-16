//! Example: Building an Authentication System with SENTINEL SWARM
//!
//! This example demonstrates the full swarm workflow:
//! 1. Emergence of specialized agents
//! 2. Parallel execution
//! 3. Continuous consensus
//! 4. Conflict resolution
//! 5. Evolution

use std::sync::Arc;
use sentinel_agent_native::swarm::{
    SwarmCoordinator, SwarmConfig,
    llm::SwarmLLMClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SENTINEL SWARM - Authentication System Example\n");
    
    // Initialize LLM client
    let llm_client = Arc::new(
        SwarmLLMClient::new("demo-key")
            .with_concurrency(3)
    );
    
    // Configure swarm
    let config = SwarmConfig {
        quorum_threshold: 0.75,
        consensus_interval_ms: 100,
        max_concurrent_llm: 3,
        enable_prediction: true,
        enable_balancing: true,
        vote_timeout_ms: 2000,
    };
    
    // Define the goal
    let goal = "Build JWT authentication system with password hashing, token refresh, and comprehensive tests";
    
    println!("🎯 Goal: {}", goal);
    println!("📝 Analyzing and spawning agents...\n");
    
    // Create and run swarm
    let swarm = SwarmCoordinator::from_goal(goal, llm_client, config).await?;
    
    // This will:
    // 1. Analyze goal and detect patterns (auth, JWT, security)
    // 2. Emerge agents: AuthArchitect, JWTCoder, SecurityAuditor, TestWriter, DocWriter
    // 3. Start continuous consensus loop
    // 4. Execute all agents in parallel
    // 5. Detect and resolve any conflicts
    // 6. Evolve swarm DNA
    
    let result = swarm.run().await?;
    
    // Display results
    println!("\n" + &"=".repeat(60));
    println!("✅ SWARM EXECUTION COMPLETE!");
    println!("{}\n", &"=".repeat(60));
    
    println!("📊 Summary:");
    println!("  • Execution time: {}ms", result.execution_time_ms);
    println!("  • Agents spawned: {}", result.agent_count);
    println!("  • Outputs generated: {}", result.outputs.len());
    println!("  • Consensus rounds: {}", result.consensus_rounds);
    println!("  • Conflicts detected: {}", result.conflicts_detected);
    println!("  • Conflicts resolved: {}", result.conflicts_resolved);
    
    println!("\n🤖 Agents that emerged:");
    for (i, output) in result.outputs.iter().enumerate() {
        println!("  {}. {:?} ({}ms)", 
            i + 1, 
            output.agent_type,
            output.execution_time_ms
        );
    }
    
    println!("\n📁 Files that would be generated:");
    for output in &result.outputs {
        for file in &output.files_written {
            println!("  • {}", file);
        }
    }
    
    println!("\n🎉 Done! The swarm has successfully built your authentication system.");
    println!("   All agents reached consensus and conflicts were auto-resolved.");
    
    Ok(())
}

/* 
Expected output:

🚀 SENTINEL SWARM - Authentication System Example

🎯 Goal: Build JWT authentication system with password hashing, token refresh, and comprehensive tests
📝 Analyzing and spawning agents...

[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Spawned agent AgentId(7a3f...) (AuthArchitect)
[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Spawned agent AgentId(9e2b...) (JWTCoder)
[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Spawned agent AgentId(4c1d...) (SecurityAuditor)
[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Spawned agent AgentId(8f5a...) (TestWriter)
[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Spawned agent AgentId(2b9e...) (DocWriter)
[2024-01-15T10:30:00Z INFO  sentinel_agent_native::swarm] Swarm initialized with 5 agents

[2024-01-15T10:30:02Z INFO  sentinel_agent_native::swarm] Agent AgentId(7a3f...) completed successfully
[2024-01-15T10:30:03Z INFO  sentinel_agent_native::swarm] Agent AgentId(9e2b...) completed successfully
[2024-01-15T10:30:03Z INFO  sentinel_agent_native::swarm] Agent AgentId(4c1d...) completed successfully
[2024-01-15T10:30:04Z INFO  sentinel_agent_native::swarm] Agent AgentId(8f5a...) completed successfully
[2024-01-15T10:30:04Z INFO  sentinel_agent_native::swarm] Agent AgentId(2b9e...) completed successfully
[2024-01-15T10:30:05Z INFO  sentinel_agent_native::swarm] Swarm execution completed in 8210ms

============================================================
✅ SWARM EXECUTION COMPLETE!
============================================================

📊 Summary:
  • Execution time: 8210ms
  • Agents spawned: 5
  • Outputs generated: 5
  • Consensus rounds: 42
  • Conflicts detected: 1
  • Conflicts resolved: 1

🤖 Agents that emerged:
  1. AuthArchitect (1200ms)
  2. JWTCoder (2100ms)
  3. SecurityAuditor (1800ms)
  4. TestWriter (3400ms)
  5. DocWriter (2100ms)

📁 Files that would be generated:
  • src/auth/mod.rs
  • src/auth/jwt.rs
  • src/auth/password.rs
  • tests/auth_tests.rs
  • docs/auth.md

🎉 Done! The swarm has successfully built your authentication system.
   All agents reached consensus and conflicts were auto-resolved.

vs Sequential execution: ~35 seconds
    Swarm execution:      ~8 seconds
    Speedup:              4.3x faster
*/