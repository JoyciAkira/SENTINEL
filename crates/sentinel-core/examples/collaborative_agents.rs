//! Example: Collaborative Split Agents
//!
//! This example demonstrates how multiple agents communicate to build a project together.
//! Run with: cargo run --example collaborative_agents

use sentinel_core::outcome_compiler::{
    AgentCapability, AgentCommunicationBus, AgentMessage, CollaborativeAgentOrchestrator,
    HandoffReason, MessagePayload, ModuleImplementationStatus, UrgencyLevel,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Collaborative Split Agent Example\n");

    // Create communication bus
    let bus = AgentCommunicationBus::new();

    // Phase 1: Register specialized agents
    println!("📋 Phase 1: Registering Specialized Agents\n");

    let mut architect = bus.register_agent(
        "MasterArchitect",
        vec![
            AgentCapability::ApiExpert,
            AgentCapability::FrontendExpert,
            AgentCapability::DatabaseExpert,
            AgentCapability::IntegrationExpert,
        ],
    )?;

    let mut auth_worker = bus.register_agent(
        "AuthWorker",
        vec![AgentCapability::AuthExpert, AgentCapability::CodeReviewer],
    )?;

    let mut api_worker = bus.register_agent(
        "ApiWorker",
        vec![
            AgentCapability::ApiExpert,
            AgentCapability::DatabaseExpert,
            AgentCapability::PerformanceOptimizer,
        ],
    )?;

    let mut ui_worker = bus.register_agent(
        "UiWorker",
        vec![
            AgentCapability::FrontendExpert,
            AgentCapability::CodeReviewer,
            AgentCapability::TestExpert,
        ],
    )?;

    println!("✓ MasterArchitect registered (integration expert)");
    println!("✓ AuthWorker registered (auth specialist)");
    println!("✓ ApiWorker registered (API specialist)");
    println!("✓ UiWorker registered (frontend specialist)\n");

    // Phase 2: Architect broadcasts initial plan
    println!("🏗️ Phase 2: Architect Broadcasting Plan\n");

    architect.broadcast(MessagePayload::PatternShare {
        title: "Project Structure".to_string(),
        description: "We will build a task board with 3 modules".to_string(),
        code_snippet: "modules: [Auth, API, UI]".to_string(),
        applicable_to: vec!["all".to_string()],
    })?;

    println!("✓ Architect shared project structure\n");

    // Phase 3: Workers start implementing and communicate
    println!("👷 Phase 3: Workers Implementing with Communication\n");

    // AuthWorker asks ApiWorker about auth middleware
    println!("🔍 AuthWorker needs info from ApiWorker about middleware integration:");
    auth_worker.request_help(
        AgentCapability::ApiExpert,
        "How should I expose the auth middleware for API routes?",
        "Building AuthModule with JWT validation",
        UrgencyLevel::Medium,
    )?;

    println!("  → Sent request to all API experts\n");

    // ApiWorker responds with pattern
    println!("💡 ApiWorker shares pattern:");
    api_worker.share_pattern(
        "Axum Auth Middleware",
        "Middleware layer for JWT validation in Axum",
        r#"
pub async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, AuthError> {
    let token = extract_token(&req)?;
    let claims = validate_jwt(&state.jwt_secret, token)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
        "#.to_string(),
        vec!["AuthModule".to_string(), "TaskAPI".to_string()],
    )?;

    println!("  → Broadcasted pattern to all agents\n");

    // AuthWorker learns and applies
    let patterns = auth_worker.learn_from_history();
    println!("📚 AuthWorker learned {} patterns from communication\n", patterns.len());

    // Phase 4: AuthWorker shares completion
    println!("✅ Phase 4: AuthWorker Shares Completion\n");

    auth_worker.update_status(sentinel_core::outcome_compiler::AgentStatus::Idle);

    auth_worker.broadcast(MessagePayload::StatusUpdate {
        module_id: "AuthModule".to_string(),
        status: ModuleImplementationStatus::Completed,
        completion_percentage: 100.0,
        blockers: vec![],
    })?;

    println!("✓ AuthModule completed and broadcasted\n");

    // Phase 5: Handoff from AuthWorker to ApiWorker
    println!("🤝 Phase 5: Handoff from AuthWorker to ApiWorker\n");

    let mut handoff_state = HashMap::new();
    handoff_state.insert(
        "auth_middleware_path".to_string(),
        serde_json::json!("src/auth/middleware.rs"),
    );
    handoff_state.insert(
        "jwt_secret_config".to_string(),
        serde_json::json!({
            "algorithm": "HS256",
            "expires_in": "1h",
        }),
    );

    auth_worker.handoff_to(
        &api_worker.id,
        "Auth module complete. Use the middleware from src/auth/middleware.rs. JWT secret is in config.",
        handoff_state,
        HandoffReason::Completed,
    )?;

    println!("✓ Handoff complete: AuthWorker → ApiWorker");
    println!("  Context transferred:");
    println!("    - auth_middleware_path: src/auth/middleware.rs");
    println!("    - jwt_secret_config: {{ algorithm: HS256, expires_in: 1h }}\n");

    // Phase 6: Validation and feedback loop
    println!("🔍 Phase 6: Validation & Feedback Loop\n");

    // ApiWorker validates AuthWorker's code
    api_worker.share_validation(
        "AuthModule",
        true,
        vec![], // No issues
        vec!["Consider adding refresh token support".to_string()],
    )?;

    println!("✓ ApiWorker validated AuthModule: PASS");
    println!("  Suggestion: Consider adding refresh token support\n");

    // Phase 7: Lesson learned sharing
    println!("📖 Phase 7: Sharing Lessons Learned\n");

    auth_worker.broadcast(MessagePayload::LessonLearned {
        situation: "JWT secret rotation".to_string(),
        solution: "Use environment variables with fallback to config file".to_string(),
        prevention: "Add validation at startup to ensure secret is 256+ bits".to_string(),
    })?;

    println!("✓ AuthWorker shared lesson about JWT secret management\n");

    // Phase 8: Conflict resolution scenario
    println!("⚠️ Phase 8: Conflict Resolution\n");

    // UiWorker encounters issue and asks for help
    ui_worker.request_help(
        AgentCapability::IntegrationExpert,
        "API response format doesn't match my TypeScript interfaces",
        "Expected: { id, title, status }, Got: { task_id, task_title, task_status }",
        UrgencyLevel::High,
    )?;

    println!("⚠️ UiWorker reported API format mismatch (High urgency)\n");

    // Architect mediates and broadcasts decision
    architect.broadcast(MessagePayload::PatternShare {
        title: "API Response Standardization".to_string(),
        description: "All API responses must use camelCase matching frontend interfaces".to_string(),
        code_snippet: r#"
// Use serde rename
#[derive(Serialize)]
struct TaskResponse {
    #[serde(rename = "taskId")]
    id: String,
    #[serde(rename = "taskTitle")]
    title: String,
}
        "#.to_string(),
        applicable_to: vec!["TaskAPI".to_string(), "TaskBoardUI".to_string()],
    })?;

    println!("✓ Architect resolved conflict and broadcasted standardization rule\n");

    // Summary
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              COLLABORATIVE WORKFLOW COMPLETE                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Communication Summary:                                       ║");
    println!("║   • Messages exchanged: ~15                                  ║");
    println!("║   • Patterns shared: 3                                       ║");
    println!("║   • Help requests: 2                                         ║");
    println!("║   • Handoffs: 1                                              ║");
    println!("║   • Conflicts resolved: 1                                    ║");
    println!("║   • Lessons learned: 1                                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("🎯 Key Collaborative Features Demonstrated:");
    println!("   1. ✓ Direct message passing between agents");
    println!("   2. ✓ Broadcast patterns to all agents");
    println!("   3. ✓ Request help based on capabilities");
    println!("   4. ✓ Handoff with context preservation");
    println!("   5. ✓ Peer code validation");
    println!("   6. ✓ Lesson sharing across agents");
    println!("   7. ✓ Conflict resolution via architect mediation");
    println!("   8. ✓ Learning from message history\n");

    Ok(())
}
