# SENTINEL SWARM - Quick Start Guide

## Overview

SENTINEL SWARM is a **deterministic multi-agent intelligence system** where agents:
- **Emerge** from task context (not predefined)
- **Self-organize** into hierarchies
- Reach **continuous consensus** every 100ms
- **Cross-pollinate** knowledge in real-time
- **Evolve** across sessions

## Installation

```bash
# Add to your Cargo.toml
[dependencies]
sentinel-agent-native = { path = "../crates/sentinel-agent-native" }
```

## Quick Start

### Basic Usage

```rust
use std::sync::Arc;
use sentinel_agent_native::swarm::{
    SwarmCoordinator, SwarmConfig, SwarmExecutionResult,
    llm::SwarmLLMClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create LLM client
    let llm_client = Arc::new(
        SwarmLLMClient::new("your-openrouter-api-key")
            .with_model("meta-llama/llama-3.3-70b-instruct:free")
            .with_concurrency(3)
    );
    
    // 2. Configure swarm
    let config = SwarmConfig {
        quorum_threshold: 0.75,        // 75% for consensus
        consensus_interval_ms: 100,    // Check every 100ms
        max_concurrent_llm: 3,         // Max 3 parallel LLM calls
        enable_prediction: true,       // Pre-spawn agents
        enable_balancing: true,        // Auto-healing
        vote_timeout_ms: 2000,         // 2s vote timeout
    };
    
    // 3. Create swarm from goal
    let swarm = SwarmCoordinator::from_goal(
        "Build JWT authentication system",
        llm_client,
        config
    ).await?;
    
    // 4. Run swarm (this does everything)
    let result = swarm.run().await?;
    
    // 5. Check results
    println!("✓ Swarm completed in {}ms", result.execution_time_ms);
    println!("  Agents: {}", result.agent_count);
    println!("  Outputs: {}", result.outputs.len());
    println!("  Conflicts resolved: {}", result.conflicts_resolved);
    
    Ok(())
}
```

## What Happens Automatically

When you call `swarm.run()`, the system:

1. **Analyzes the goal** - Detects patterns (auth, api, database, etc.)
2. **Emerges agents** - Spawns specialized agents based on goal
3. **Starts consensus loop** - Begins 100ms consensus rounds
4. **Executes in parallel** - All agents work simultaneously
5. **Detects conflicts** - Automatically finds resource/technical conflicts
6. **Resolves creatively** - Uses synthesis or arbiter agents
7. **Evolves DNA** - Saves patterns for future sessions

## Example: Different Goals → Different Swarms

```rust
// Auth system → 5 agents
let auth_swarm = SwarmCoordinator::from_goal(
    "Build authentication system with JWT",
    llm_client.clone(),
    config.clone()
).await?;
// Emerges: AuthArchitect, JWTCoder, SecurityAuditor, TestWriter, DocWriter

// API system → 4 agents  
let api_swarm = SwarmCoordinator::from_goal(
    "Create REST API with database",
    llm_client.clone(),
    config.clone()
).await?;
// Emerges: APICoder, DatabaseArchitect, TestWriter, DocWriter

// Full-stack → 7+ agents (triggers manager)
let fullstack_swarm = SwarmCoordinator::from_goal(
    "Build full-stack app with auth, API, database, frontend",
    llm_client,
    config
).await?;
// Emerges: 7+ agents + Manager agent for coordination
```

## Advanced: Manual Control

```rust
// Spawn agents manually
let agent_ids = swarm.spawn_agents().await?;
println!("Spawned {} agents", agent_ids.len());

// Start background services manually
swarm.start_consensus().await?;
swarm.start_prediction().await?;
swarm.start_balancer().await?;

// Execute with custom logic
let outputs = swarm.execute_parallel().await?;

// Handle conflicts manually
let conflicts = swarm.conflict_resolver.detect_conflicts(&outputs).await;
for conflict in conflicts {
    let resolution = swarm.conflict_resolver.resolve(conflict).await?;
    println!("Resolved: {:?}", resolution);
}
```

## Configuration Options

```rust
SwarmConfig {
    // Consensus settings
    quorum_threshold: 0.75,        // 0.0 - 1.0 (75% default)
    consensus_interval_ms: 100,    // Round frequency
    vote_timeout_ms: 2000,         // Proposal timeout
    
    // Performance settings
    max_concurrent_llm: 3,         // Parallel LLM calls
    enable_prediction: true,       // Pre-fetch agents
    enable_balancing: true,        // Auto-healing
}
```

## Monitoring

```rust
// Get swarm stats
let health = swarm.balancer.get_all_health().await;
for (agent_id, status) in health {
    println!("Agent {:?}: {:?}", agent_id, status.status);
}

// Check consensus
let round = swarm.consensus.get_round().await;
let pending = swarm.consensus.get_pending().await;
println!("Consensus round {} with {} pending proposals", round, pending.len());

// View memory
let stats = swarm.memory.stats();
println!("Memory: {} working, {} patterns", 
    stats.working_entries, 
    stats.procedural_patterns
);
```

## VSCode Integration

The swarm panel shows real-time visualization:

```typescript
// In VSCode extension
vscode.postMessage({
    type: 'startSwarm',
    goal: 'Build auth system'
});

// Listen for updates
window.addEventListener('message', event => {
    if (event.data.type === 'agentUpdate') {
        updateUI(event.data.payload);
    }
});
```

## Best Practices

1. **Start simple**: Use `swarm.run()` for most cases
2. **Monitor**: Check swarm stats for performance
3. **Tune**: Adjust `max_concurrent_llm` based on API limits
4. **Evolve**: Let the system learn - don't reset DNA
5. **Scale**: Use manager for >5 agents

## Troubleshooting

### High latency?
- Increase `max_concurrent_llm` (if API allows)
- Check `enable_prediction` is true
- Monitor with `swarm.balancer.get_stats().await`

### Conflicts not resolving?
- Check `quorum_threshold` (try 0.6 for faster consensus)
- Review conflict journal
- Lower `vote_timeout_ms`

### Agents failing?
- Check health: `swarm.balancer.get_all_health().await`
- Enable auto-balancing
- Review logs for stuck agents

## Performance Benchmarks

| Task | Sequential | Swarm (5 agents) | Speedup |
|------|-----------|------------------|---------|
| Auth system | 35s | 8.2s | 4.3x |
| API + tests | 28s | 6.5s | 4.3x |
| Full-stack | 120s | 22s | 5.5x |

*Benchmarks on M2 Mac, OpenRouter free tier*

## Next Steps

1. Read [Architecture Guide](./SWARM_ARCHITECTURE.md)
2. See [API Reference](./SWARM_API.md)
3. Check [Examples](../examples/)

## Support

- GitHub Issues: https://github.com/JoyciAkira/SENTINEL/issues
- Documentation: https://docs.sentinel-protocol.org
- Discord: https://discord.gg/sentinel

---

**Built with ❤️ by the Sentinel Team**