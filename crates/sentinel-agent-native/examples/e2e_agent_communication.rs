//! End-to-End Example: Multi-Agent Communication with Real LLM (OpenRouter)
//!
//! This example demonstrates how multiple LLM-powered agents communicate
//! to solve a coding problem collaboratively using OpenRouter (FREE models).
//!
//! Prerequisites:
//! - Create .env file with OPENROUTER_API_KEY (copy from .env.example)
//!
//! Run with:
//! ```bash
//! cargo run --example e2e_agent_communication
//! ```

use anyhow::Result;
use sentinel_agent_native::agent_communication_llm::LLMAgentOrchestrator;
use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};
use sentinel_agent_native::llm_integration::LLMChatClient;
use sentinel_core::outcome_compiler::agent_communication::{
    AgentCapability, MessagePayload, UrgencyLevel,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Mock LLM client for testing without real API
#[derive(Debug)]
struct MockLLMClient {
    model_name: String,
}

#[async_trait::async_trait]
impl LLMChatClient for MockLLMClient {
    async fn chat_completion(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
    ) -> anyhow::Result<sentinel_agent_native::llm_integration::LLMChatCompletion> {
        sleep(Duration::from_millis(50)).await;
        
        Ok(sentinel_agent_native::llm_integration::LLMChatCompletion {
            llm_name: self.model_name.clone(),
            content: "I acknowledge this message and will process it appropriately.".to_string(),
            token_cost: 25,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Multi-Agent Communication with LLM\n");

    // Check for API key
    let api_key_opt = std::env::var("OPENROUTER_API_KEY").ok();
    let model_name = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| "meta-llama/llama-3.3-70b-instruct:free".to_string());

    // Create LLM client (real or mock fallback)
    let llm_client: Arc<dyn LLMChatClient> = if let Some(api_key) = api_key_opt {
        // Check if key looks valid (starts with sk-or-)
        if api_key.starts_with("sk-or-") {
            println!("🤖 Initializing OpenRouter LLM client (REAL)...");
            println!("   Model: {}\n", model_name);
            
            let model = match model_name.as_str() {
                "meta-llama/llama-3.3-70b-instruct:free" => OpenRouterModel::MetaLlama3_3_70B,
                "google/gemini-2.0-flash-exp:free" => OpenRouterModel::GoogleGemini2Flash,
                "deepseek/deepseek-r1-0528:free" => OpenRouterModel::DeepSeekR1,
                "mistralai/devstral-small:free" => OpenRouterModel::MistralDevstral,
                _ => OpenRouterModel::Custom(model_name.clone()),
            };

            Arc::new(
                OpenRouterClient::new(api_key, model)
                    .with_temperature(0.7)
                    .with_max_tokens(1500)
            )
        } else {
            println!("⚠️  API key appears invalid (should start with 'sk-or-')");
            println!("   Using MOCK LLM for demonstration\n");
            Arc::new(MockLLMClient { model_name: format!("Mock ({})", model_name) })
        }
    } else {
        println!("⚠️  No OPENROUTER_API_KEY found in .env");
        println!("   Using MOCK LLM for demonstration\n");
        println!("   To use real LLM:");
        println!("   1. Get free API key at https://openrouter.ai/keys");
        println!("   2. Add to .env: OPENROUTER_API_KEY=sk-or-v1-...\n");
        Arc::new(MockLLMClient { model_name: format!("Mock ({})", model_name) })
    };

    // Create orchestrator
    println!("📡 Creating agent orchestrator...\n");
    let orchestrator = Arc::new(LLMAgentOrchestrator::new(llm_client));

    // Register specialized agents
    println!("👥 Registering specialized agents:\n");

    let (architect_handle, mut architect) = orchestrator
        .register_agent(
            "MasterArchitect",
            vec![
                AgentCapability::ApiExpert,
                AgentCapability::FrontendExpert,
                AgentCapability::IntegrationExpert,
            ],
            "You are the Master Architect. Your role is to design system architecture, \
             coordinate between specialists, and ensure the overall solution is coherent. \
             When you receive messages, analyze them and provide clear, actionable guidance. \
             Be concise but thorough. Focus on high-level design decisions.",
        )
        .await?;
    println!("  ✓ MasterArchitect registered (system designer)");

    let (auth_handle, mut auth_agent) = orchestrator
        .register_agent(
            "AuthSpecialist",
            vec![AgentCapability::AuthExpert, AgentCapability::CodeReviewer],
            "You are the Authentication Specialist. You focus on JWT, OAuth, session management, \
             and security best practices. When asked about auth, provide specific implementation \
             guidance with code examples. Be concise and code-focused. Prioritize security.",
        )
        .await?;
    println!("  ✓ AuthSpecialist registered (security expert)");

    let (api_handle, mut api_agent) = orchestrator
        .register_agent(
            "ApiSpecialist",
            vec![AgentCapability::ApiExpert, AgentCapability::DatabaseExpert],
            "You are the API Specialist. You design REST/GraphQL APIs, handle database models, \
             and optimize performance. Provide concrete API designs with endpoints and data models. \
             Be specific and include example requests/responses when helpful.",
        )
        .await?;
    println!("  ✓ ApiSpecialist registered (backend expert)\n");

    // Spawn orchestrator routing loop
    let orchestrator_clone = Arc::clone(&orchestrator);
    tokio::spawn(async move {
        orchestrator_clone.run().await.unwrap();
    });

    // Spawn agent processing loops
    let architect_id = architect_handle.id.clone();
    tokio::spawn(async move {
        if let Err(e) = architect.run().await {
            eprintln!("Architect error: {}", e);
        }
    });

    let auth_id = auth_handle.id.clone();
    tokio::spawn(async move {
        if let Err(e) = auth_agent.run().await {
            eprintln!("Auth agent error: {}", e);
        }
    });

    let api_id = api_handle.id.clone();
    tokio::spawn(async move {
        if let Err(e) = api_agent.run().await {
            eprintln!("API agent error: {}", e);
        }
    });

    // Give agents time to start
    sleep(Duration::from_millis(500)).await;

    // Scenario: Building a task management API with authentication
    println!("🏗️  Scenario: Building a Task Management API\n");
    println!("═══════════════════════════════════════════════════\n");

    // Phase 1: Architect broadcasts the project plan
    println!("📋 Phase 1: Architect broadcasts project plan");
    println!("   → Sending message to all agents (LLM call for each agent)...\n");
    
    let start = std::time::Instant::now();
    orchestrator
        .broadcast_from(
            &architect_id,
            MessagePayload::PatternShare {
                title: "Task Management API Architecture".to_string(),
                description: "We need to build a task management API with authentication. \
                             Components: 1) JWT Authentication, 2) Task CRUD API, 3) Database models. \
                             AuthSpecialist handles auth, ApiSpecialist handles API design."
                    .to_string(),
                code_snippet: "Stack: Rust + Axum + PostgreSQL + JWT".to_string(),
                applicable_to: vec!["AuthSpecialist".to_string(), "ApiSpecialist".to_string()],
            },
        )
        .await?;
    
    sleep(Duration::from_secs(3)).await;
    println!("   ✓ Broadcast complete ({:.1}s)\n", start.elapsed().as_secs_f32());

    // Phase 2: API Specialist asks Auth Specialist about JWT integration
    println!("🔍 Phase 2: API Specialist asks Auth Specialist about JWT middleware");
    println!("   → Direct message (1 LLM call for sender + 1 for receiver)...\n");
    
    let start = std::time::Instant::now();
    orchestrator
        .send_to(
            &auth_id,
            &api_id,
            MessagePayload::HelpRequest {
                question: "How should I structure the JWT middleware for Axum? \
                          I need to protect the task endpoints with authentication."
                    .to_string(),
                context: "Building REST API with Axum framework".to_string(),
                urgency: UrgencyLevel::High,
            },
        )
        .await?;
    
    sleep(Duration::from_secs(4)).await;
    println!("   ✓ Direct message complete ({:.1}s)\n", start.elapsed().as_secs_f32());

    // Phase 3: Auth Specialist shares JWT pattern
    println!("💡 Phase 3: Auth Specialist shares JWT implementation pattern");
    println!("   → Broadcasting to all agents...\n");
    
    let start = std::time::Instant::now();
    orchestrator
        .broadcast_from(
            &auth_id,
            MessagePayload::PatternShare {
                title: "Axum JWT Middleware Pattern".to_string(),
                description: "Production-ready JWT middleware for Axum with claims extraction and error handling".to_string(),
                code_snippet: r#"
// JWT Middleware for Axum
use axum::{extract::State, http::Request, middleware::Next, response::Response};
use jsonwebtoken::{decode, Validation, DecodingKey};

#[derive(Debug, serde::Deserialize)]
struct Claims {
    sub: String,    // User ID
    exp: usize,     // Expiration
    role: String,   // User role
}

pub async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, AuthError> {
    let token = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(AuthError::MissingToken)?;
    
    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &Validation::default(),
    ).map_err(|_| AuthError::InvalidToken)?;
    
    req.extensions_mut().insert(claims.claims);
    Ok(next.run(req).await)
}
"#.to_string(),
                applicable_to: vec!["ApiSpecialist".to_string()],
            },
        )
        .await?;
    
    sleep(Duration::from_secs(3)).await;
    println!("   ✓ Pattern shared ({:.1}s)\n", start.elapsed().as_secs_f32());

    // Phase 4: API Specialist shares Task API design
    println!("📝 Phase 4: API Specialist shares Task API design");
    println!("   → Broadcasting API endpoints...\n");
    
    let start = std::time::Instant::now();
    orchestrator
        .broadcast_from(
            &api_id,
            MessagePayload::PatternShare {
                title: "Task API Endpoints Design".to_string(),
                description: "RESTful endpoints for task management with JWT authentication".to_string(),
                code_snippet: r#"
// Task API Router with Auth Protection
let task_routes = Router::new()
    .route("/tasks", get(list_tasks).post(create_task))
    .route("/tasks/:id", get(get_task).put(update_task).delete(delete_task))
    .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

// Example handler with user from JWT claims
async fn create_task(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<Task>, AppError> {
    // claims.sub contains authenticated user ID
    let task = Task::new(payload.title, claims.sub);
    state.db.create_task(&task).await?;
    Ok(Json(task))
}
"#.to_string(),
                applicable_to: vec!["AuthSpecialist".to_string(), "MasterArchitect".to_string()],
            },
        )
        .await?;
    
    sleep(Duration::from_secs(3)).await;
    println!("   ✓ API design shared ({:.1}s)\n", start.elapsed().as_secs_f32());

    // Phase 5: Architect validates the collaboration
    println!("✅ Phase 5: Architect validates and provides feedback");
    println!("   → Final validation broadcast...\n");
    
    let start = std::time::Instant::now();
    orchestrator
        .broadcast_from(
            &architect_id,
            MessagePayload::ValidationResult {
                module_id: "task-api-auth-integration".to_string(),
                passed: true,
                issues: vec![],
                suggestions: vec![
                    "Consider adding rate limiting middleware".to_string(),
                    "Add refresh token mechanism for better UX".to_string(),
                    "Implement role-based access control (RBAC)".to_string(),
                ],
            },
        )
        .await?;
    
    sleep(Duration::from_secs(3)).await;
    println!("   ✓ Validation complete ({:.1}s)\n", start.elapsed().as_secs_f32());

    // Summary
    println!("═══════════════════════════════════════════════════");
    println!("\n✨ Collaboration Complete!\n");
    println!("📊 Summary:");
    println!("  • 3 agents collaborated to design Task Management API");
    println!("  • AuthSpecialist provided JWT middleware pattern");
    println!("  • ApiSpecialist designed RESTful endpoints");
    println!("  • MasterArchitect coordinated and validated");
    println!("  • All communication processed through REAL LLM\n");

    println!("👥 Active Agents:");
    let agents = orchestrator.list_agents().await;
    for agent in agents {
        println!(
            "  • {} - Capabilities: {:?}",
            agent.name,
            agent.capabilities
        );
    }

    println!("\n💰 Estimated API Costs:");
    println!("  • ~10 LLM calls (broadcasts + direct messages)");
    println!("  • Using FREE model: {}", model_name);
    println!("  • Total cost: $0.00 (free tier)\n");

    println!("🎉 End-to-End Agent Communication Test Complete!");
    println!("\n💡 Tips:");
    println!("  • Try different models by changing OPENROUTER_MODEL in .env");
    println!("  • Add more agents for complex projects");
    println!("  • Monitor agent responses in the logs above\n");

    Ok(())
}
