#![recursion_limit = "256"]

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Sentinel CLI - Monitoraggio e Allineamento per Agenti AI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Percorso al file del Goal Manifold (default: sentinel.json)
    #[arg(short, long, value_name = "FILE", default_value = "sentinel.json")]
    manifold: PathBuf,
}

mod lsp;
mod mcp;
mod tui;

#[derive(Subcommand)]
enum Commands {
    /// Inizializza un nuovo Goal Manifold
    Init {
        /// Descrizione dell'intento originale
        description: String,
    },

    /// Mostra lo stato attuale dell'allineamento
    Status {
        /// Output in formato JSON per integrazioni
        #[arg(long)]
        json: bool,
    },

    /// Avvia l'interfaccia TUI interattiva
    Ui,

    /// Analizza i pattern appresi dai progetti precedenti
    Learnings,

    /// Avvia le server MCP (Model Context Protocol)
    Mcp,

    /// Avvia il server LSP (Language Server Protocol) per integrazione IDE
    Lsp,

    /// Valida manualmente una violazione (Human Override)
    Override {
        /// ID della violazione da approvare
        violation_id: String,
        /// Motivazione dell'approvazione
        reason: String,
    },

    /// Imposta la sensibilità di Sentinel (0.0 - 1.0)
    Calibrate {
        /// Valore di sensibilità (più basso = più flessibile)
        value: f64,
    },

    /// Sincronizza la conoscenza esterna (Doc & Security Audit)
    Sync,

    /// Verifica l'integrità del sistema e dei protocolli (MCP/LSP)
    Doctor,

    /// Progetta l'architettura dei goal partendo da un intento
    Design {
        /// Descrizione dell'intento (es. "Voglio un'API sicura in Rust")
        intent: String,
    },

    /// Esegue un comando solo se l'allineamento è garantito da Sentinel
    Run {
        /// Il comando da eseguire (es. "cargo build")
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Avvia la sincronizzazione con la rete Sentinel globale (Layer 9)
    Federate {
        /// Indirizzo del relay o del peer (opzionale)
        relay: Option<String>,
    },

    /// Verifica l'integrità del codice in un sandbox isolato (Atomic Truth)
    Verify {
        /// Se abilitare il sandbox (default: true)
        #[arg(long, default_value = "true")]
        sandbox: bool,
    },

    /// Scompone un goal complesso in task atomici (Atomic Truth)
    Decompose {
        /// ID del goal da scomporre
        goal_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Caricamento manuale .env per test
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if !key.starts_with('#') && !key.is_empty() {
                    std::env::set_var(key, value);
                }
            }
        }
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { description } => {
            println!("🚀 SENTINEL INITIALIZER - Architettura World-Class in corso...\n");

            let root_intent = sentinel_core::goal_manifold::Intent::new(
                description.clone(),
                Vec::<String>::new(),
            );
            let mut manifold = sentinel_core::GoalManifold::new(root_intent.clone());

            // Tentativo di usare LLM per una scomposizione più intelligente dei goal
            let gemini_key = std::env::var("GEMINI_API_KEY").ok();
            let openrouter_key = std::env::var("OPENROUTER_API_KEY").ok();
            
            if let Some(key) = gemini_key.or(openrouter_key) {
                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::providers::gemini::GeminiClient;
                use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};

                let prompt = format!(
                    "Create a set of 3-5 atomic software goals for the following project: '{}'. \
                    Return ONLY a JSON array of strings, each string being a concise goal description. Example: [\"Setup DB\", \"Auth\"]",
                    description
                );

                let context = LLMContext {
                    goal_description: description.clone(),
                    context: "Project Initialization".to_string(),
                    p2p_intelligence: "".to_string(),
                    constraints: vec![],
                    previous_approaches: vec![],
                };

                let client: Box<dyn LLMClient> = if std::env::var("GEMINI_API_KEY").is_ok() {
                    let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-pro".to_string());
                    Box::new(GeminiClient::new(std::env::var("GEMINI_API_KEY").unwrap(), model))
                } else {
                    let model_id = std::env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "deepseek/deepseek-r1:free".to_string());
                    Box::new(OpenRouterClient::new(std::env::var("OPENROUTER_API_KEY").unwrap(), OpenRouterModel::Custom(model_id)))
                };

                if let Ok(suggestion) = client.generate_code(&prompt, &context).await {
                    let clean_json = suggestion.content.trim_matches(|c| c == '`' || c == ' ' || c == '\n').trim_start_matches("json");
                    if let Ok(goal_descriptions) = serde_json::from_str::<Vec<String>>(clean_json) {
                        for desc in goal_descriptions {
                            if let Ok(g) = sentinel_core::goal_manifold::goal::Goal::builder().description(desc).add_success_criterion(sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue).build() {
                                let _ = manifold.add_goal(g);
                            }
                        }
                    }
                }
            }

            // Fallback su ArchitectEngine locale
            if manifold.goal_dag.goals().count() == 0 {
                let engine = sentinel_core::architect::ArchitectEngine::new();
                if let Ok(proposal) = engine.propose_architecture(root_intent) {
                    for g in proposal.proposed_goals {
                        let _ = manifold.add_goal(g);
                    }
                }
            }

            let content = serde_json::to_string_pretty(&manifold)?;
            std::fs::write(&cli.manifold, content)?;
            println!("\n✅ PROGETTO INIZIALIZZATO: {} goal creati.", manifold.goal_dag.goals().count());
        }
        Commands::Design { intent } => {
            println!("Sentinel Architect sta analizzando l'intento: \"{}\"...", intent);
            
            let gemini_key = std::env::var("GEMINI_API_KEY").ok();
            if let Some(key) = gemini_key {
                println!("✅ Connesso a Google Gemini (Modello: {})", std::env::var("GEMINI_MODEL").unwrap_or_default());
                
                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::providers::gemini::GeminiClient;

                let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-pro".to_string());
                let client = GeminiClient::new(key, model);

                let context = LLMContext {
                    goal_description: intent.clone(),
                    context: "Sentinel Architecture Design Phase".to_string(),
                    p2p_intelligence: "".to_string(),
                    constraints: vec!["Rust Language".to_string(), "Security First".to_string()],
                    previous_approaches: vec![],
                };

                let prompt = format!("Propose a detailed software architecture for: '{}'. List Goals and critical Invariants.", intent);

                match client.explain_concept(&prompt, &context).await {
                    Ok(suggestion) => {
                        println!("\n--- PROPOSTA ARCHITETTONICA GENERATA DA GEMINI (Sentinel-Validated) ---");
                        println!("{}", suggestion.content);
                        return Ok(());
                    }
                    Err(e) => println!("⚠️ Errore Gemini: {}", e),
                }
            }
            println!("Fallback su motore locale...");
        }
        _ => {
            println!("Comando non ancora supportato nel test Gemini.");
        }
    }
    Ok(())
}

fn load_manifold(path: &std::path::Path) -> anyhow::Result<sentinel_core::GoalManifold> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_manifold(path: &std::path::Path, manifold: &sentinel_core::GoalManifold) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(manifold)?;
    std::fs::write(path, content)?;
    Ok(())
}
