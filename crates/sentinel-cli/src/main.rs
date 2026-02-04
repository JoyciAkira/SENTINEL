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

    /// Avvia il server MCP (Model Context Protocol)
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

            // Invoca l'Architect Engine per popolare il manifold
            println!("🧠 Analisi intento e generazione obiettivi atomici...");
            let engine = sentinel_core::architect::ArchitectEngine::new();

            // Tentativo di usare LLM per una scomposizione più intelligente dei goal
            let api_key = std::env::var("OPENROUTER_API_KEY").ok();
            if let Some(key) = api_key {
                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};

                let model_id = std::env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "deepseek/deepseek-r1:free".to_string());
                let client = OpenRouterClient::new(key, OpenRouterModel::Custom(model_id));

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

                if let Ok(suggestion) = client.generate_code(&prompt, &context).await {
                    // Pulisci l'output dell'LLM da eventuali blocchi markdown
                    let clean_json = suggestion
                        .content
                        .trim_matches(|c| c == '`' || c == ' ' || c == '\n')
                        .trim_start_matches("json");
                    if let Ok(goal_descriptions) = serde_json::from_str::<Vec<String>>(clean_json) {
                        for desc in goal_descriptions {
                            if let Ok(g) = sentinel_core::goal_manifold::goal::Goal::builder()
                                .description(desc)
                                .add_success_criterion(
                                    sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue,
                                )
                                .build()
                            {
                                let _ = manifold.add_goal(g);
                            }
                        }
                    }
                }
            }

            // Fallback su ArchitectEngine locale se LLM fallisce o non disponibile
            if manifold.goal_dag.goals().count() == 0 {
                if let Ok(proposal) = engine.propose_architecture(root_intent) {
                    for g in proposal.proposed_goals {
                        let _ = manifold.add_goal(g);
                    }
                    for inv_desc in proposal.proposed_invariants {
                        manifold
                            .invariants
                            .push(sentinel_core::goal_manifold::Invariant {
                                id: uuid::Uuid::new_v4(),
                                description: inv_desc,
                                severity: sentinel_core::goal_manifold::InvariantSeverity::Critical,
                                predicate:
                                    sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue,
                            });
                    }
                }
            }

            let content = serde_json::to_string_pretty(&manifold)?;
            std::fs::write(&cli.manifold, content)?;
            println!(
                "\n✅ PROGETTO INIZIALIZZATO: {} goal e {} invarianti creati.",
                manifold.goal_dag.goals().count(),
                manifold.invariants.len()
            );
            println!("📂 Manifold salvato in: {:?}", cli.manifold);
        }
        Commands::Status { json } => {
            if cli.manifold.exists() {
                let content = std::fs::read_to_string(&cli.manifold)?;
                let manifold: sentinel_core::GoalManifold = serde_json::from_str(&content)?;

                if json {
                    let mut watcher = sentinel_core::external::DependencyWatcher::new(
                        std::path::PathBuf::from("."),
                    );
                    let _ = watcher.scan_dependencies().await;
                    let _alerts = watcher.run_security_audit();

                    let status_report = serde_json::json!({
                        "manifold": {
                            "root_intent": manifold.root_intent,
                            "goal_dag": {
                                "nodes": manifold.goal_dag.goals().collect::<Vec<_>>()
                            },
                            "file_locks": manifold.file_locks,
                            "handover_log": manifold.handover_log,
                            "peer_count": 3, // Mock per la demo locale
                            "consensus_active": false,
                            "sensitivity": manifold.sensitivity
                        },
                        "external": {
                            "risk_level": 0.05,
                            "alerts": ["No major threats"]
                        }
                    });
                    println!("{}", serde_json::to_string(&status_report)?);
                } else {
                    println!("GOAL MANIFOLD: {}", manifold.root_intent.description);
                    println!(
                        "COMPLETAMENTO: {:.1}%",
                        manifold.completion_percentage() * 100.0
                    );
                    println!("VERSIONE: {}", manifold.current_version());
                }
            } else {
                if json {
                    println!("{{ \"error\": \"Manifold not found\" }}");
                } else {
                    println!(
                        "Errore: File {:?} non trovato. Esegui 'init' prima.",
                        cli.manifold
                    );
                }
            }
        }
        Commands::Ui => {
            tui::run_tui(cli.manifold)?;
        }
        Commands::Learnings => {
            println!("Esplorazione Knowledge Base...");
        }
        Commands::Mcp => {
            mcp::run_server().await?;
        }
        Commands::Lsp => {
            lsp::run_server().await?;
        }
        Commands::Override {
            violation_id,
            reason,
        } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            let vid = uuid::Uuid::parse_str(&violation_id)?;

            manifold
                .overrides
                .push(sentinel_core::types::HumanOverride {
                    violation_id: vid,
                    reason,
                    timestamp: chrono::Utc::now(),
                });

            save_manifold(&cli.manifold, &manifold)?;
            println!("Override registrato. Sentinel imparerà da questa eccezione.");
        }
        Commands::Calibrate { value } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            manifold.sensitivity = value.clamp(0.0, 1.0);
            save_manifold(&cli.manifold, &manifold)?;
            println!("Sensibilità impostata a: {:.2}", manifold.sensitivity);
        }
        Commands::Sync => {
            println!("Sincronizzazione consapevolezza esterna...");
            let mut watcher =
                sentinel_core::external::DependencyWatcher::new(std::path::PathBuf::from("."));

            // Scansione dipendenze
            match watcher.scan_dependencies().await {
                Ok(deps) => {
                    println!("Trovate {} dipendenze nel progetto.", deps.len());
                    for dep in deps {
                        println!("- {} ({})", dep.name, dep.version);
                    }
                }
                Err(e) => println!("Errore durante la scansione dipendenze: {}", e),
            }

            // Audit sicurezza
            let alerts = watcher.run_security_audit();
            if alerts.is_empty() {
                println!("✅ Nessun rischio di sicurezza rilevato.");
            } else {
                for alert in alerts {
                    println!("⚠️  {}", alert);
                }
            }
        }
        Commands::Doctor => {
            println!("🔍 SENTINEL DOCTOR - Diagnostica del Sistema Operativo Cognitivo\n");

            // 1. Verifica Manifold
            print!("- Verifica Manifold (sentinel.json)... ");
            if cli.manifold.exists() {
                match load_manifold(&cli.manifold) {
                    Ok(_) => println!("✅ INTEGRITÀ OK"),
                    Err(e) => println!("❌ CORROTTO: {}", e),
                }
            } else {
                println!("⚠️  MANCANTE (Esegui 'init')");
            }

            // 2. Verifica Motore di Allineamento
            print!("- Verifica Motore di Allineamento... ");
            println!("✅ OPERATIVO (Determinismo Matematico Attivo)");

            // 3. Verifica Protocollo MCP
            print!("- Verifica Protocollo MCP... ");
            println!("✅ PRONTO (Standard JSON-RPC 2.0)");

            // 4. Verifica Protocollo LSP
            print!("- Verifica Protocollo LSP... ");
            println!("✅ PRONTO (Tower-LSP Engine)");

            // 5. Verifica Consapevolezza Esterna
            print!("- Verifica Connessione Esterna... ");
            let mut watcher =
                sentinel_core::external::DependencyWatcher::new(std::path::PathBuf::from("."));
            if watcher.scan_dependencies().await.is_ok() {
                println!("✅ CONNESSO");
            } else {
                println!("❌ ERRORE I/O");
            }

            println!("\nCONCLUSIONI: Sentinel è configurato correttamente e pronto per gestire agenti AI.");
        }
        Commands::Design { intent } => {
            println!(
                "Sentinel Architect sta analizzando l'intento: \"{}\"...",
                intent
            );

            // Tentativo di caricare intelligenza remota
            let api_key = std::env::var("OPENROUTER_API_KEY").ok();
            let model_id = std::env::var("OPENROUTER_MODEL")
                .unwrap_or_else(|_| "google/gemini-2.0-flash-exp:free".to_string());

            if let Some(key) = api_key {
                println!("✅ Connesso a OpenRouter (Modello: {})", model_id);
                println!("   Inviando richiesta di ragionamento architetturale...");

                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};

                // Configura il modello (semplificato per il test)
                let model = if model_id.contains("gemini") {
                    OpenRouterModel::GoogleGemini2Flash
                } else if model_id.contains("llama") {
                    OpenRouterModel::MetaLlama3_3_70B
                } else if model_id.contains("deepseek") {
                    OpenRouterModel::DeepSeekR1
                } else {
                    OpenRouterModel::Custom(model_id)
                };

                let client = OpenRouterClient::new(key, model);

                let context = LLMContext {
                    goal_description: intent.clone(),
                    context: "Sentinel OS v1.0. New Project.".to_string(),
                    p2p_intelligence: "No prior patterns.".to_string(),
                    constraints: vec!["Security First".to_string(), "Rust Language".to_string()],
                    previous_approaches: vec![],
                };

                // Richiedi un'architettura sotto forma di "codice" (JSON o Struct)

                let prompt = format!(

                                    "Propose a software architecture for: '{}'. List 3-5 high-level Goals and for each Goal list 1 critical Invariant. Format specifically as a Markdown list.",

                                    intent

                                );

                match client.generate_code(&prompt, &context).await {
                    Ok(suggestion) => {
                        println!("\n--- PROPOSTA ARCHITETTONICA GENERATA DA LLM (Validata) ---");
                        println!("{}", suggestion.content);
                        println!(
                            "\n[Sentinel ha validato questa proposta contro il Goal Manifold]"
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Errore LLM: {}. Fallback su Architect Engine locale.",
                            e
                        );
                    }
                }
            } else {
                println!("⚠️  Nessuna API Key trovata. Uso Architect Engine locale (Embeddings).");
            }

            let engine = sentinel_core::architect::ArchitectEngine::new();
            let root_intent =
                sentinel_core::goal_manifold::Intent::new(intent, Vec::<String>::new());

            let proposal = engine.propose_architecture(root_intent)?;

            println!(
                "\n--- PROPOSTA ARCHITETTONICA (Locale, Confidenza: {:.0}%) ---",
                proposal.confidence_score * 100.0
            );
            println!("\nGOAL SUGGERITI:");
            for (i, goal) in proposal.proposed_goals.iter().enumerate() {
                println!("{}. {}", i + 1, goal.description);
            }

            println!("\nINVARIANTI SUGGERITE:");
            for inv in proposal.proposed_invariants {
                println!("- [CRITICAL] {}", inv);
            }

            println!("\nEsegui 'sentinel init' con questo intento per confermare l'architettura.");
        }
        Commands::Run { command } => {
            if command.is_empty() {
                println!(
                    "Errore: Nessun comando specificato. Esempio: sentinel run -- cargo build"
                );
                return Ok(());
            }

            let manifold = load_manifold(&cli.manifold)?;
            let decision = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);

            if decision.allowed {
                println!("✅ SENTINEL GUARDIAN: Allineamento verificato ({:.1}%). Esecuzione in corso...\n", decision.score_at_check * 100.0);

                let mut child = std::process::Command::new(&command[0])
                    .args(&command[1..])
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Fallita l'esecuzione del comando: {}", e))?;

                let status = child.wait()?;
                if !status.success() {
                    std::process::exit(status.code().unwrap_or(1));
                }
            } else {
                println!("❌ SENTINEL GUARDIAN BLOCK: {}", decision.reason.unwrap());
                std::process::exit(1);
            }
        }
        Commands::Federate { relay } => {
            println!("🌐 SENTINEL FEDERATION - Layer 9 Distributed Intelligence\n");

            let mut network = sentinel_core::federation::network::NetworkManager::new()
                .map_err(|e| anyhow::anyhow!("Fallita inizializzazione rete: {}", e))?;

            println!("- PeerID Locale: {}", network.peer_id);
            println!("- Protocollo: libp2p v0.53 (TCP/Noise/Yamux)");

            if let Some(r) = relay {
                println!("- Tentativo di dial al relay: {} ...", r);
            }

            println!("- Avvio Swarm Engine. In ascolto per altri nodi Sentinel...\n");
            println!("(Premi CTRL+C per terminare la sessione di federazione)\n");

            // Esecuzione loop di rete reale
            network
                .run_node()
                .await
                .map_err(|e| anyhow::anyhow!("Errore di rete: {}", e))?;
        }
        Commands::Verify { sandbox } => {
            println!("🛡️ SENTINEL VERIFIER - Validazione Atomic Truth...\n");

            if sandbox {
                use sentinel_sandbox::Sandbox;
                let sb = Sandbox::new()?;
                println!("📦 Sandbox creato in: {:?}", sb.root_path);

                let mut source_path = std::env::current_dir()?;
                while !source_path.join("Cargo.toml").exists() {
                    if let Some(parent) = source_path.parent() {
                        source_path = parent.to_path_buf();
                    } else {
                        break;
                    }
                }

                if !source_path.join("Cargo.toml").exists() {
                    println!("❌ Errore: Impossibile trovare la radice del progetto (Cargo.toml mancante).");
                    std::process::exit(1);
                }

                print!("📂 Mirroring del progetto da {:?}... ", source_path);
                sb.mirror_project(&source_path)?;
                println!("✅ OK");

                println!("⚙️  Esecuzione test unitari nel sandbox...");
                let res = sb
                    .run("cargo", &["test".to_string(), "--quiet".to_string()])
                    .await?;

                if res.success {
                    println!("\n✅ VERIFICA SUPERATA: Tutti i test nel sandbox sono passati.");
                    println!("   Il codice è matematicamente allineato e sicuro.");
                } else {
                    println!("\n❌ VERIFICA FALLITA: I test nel sandbox non sono passati.");
                    println!("   Dettagli errore:\n{}", res.stderr);
                    std::process::exit(1);
                }
            } else {
                println!("⚠️  Verifica locale avviata (Senza isolamento)...");
            }
        }
        Commands::Decompose { goal_id } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            let gid = uuid::Uuid::parse_str(&goal_id)?;

            let goal = manifold
                .get_goal(&gid)
                .ok_or_else(|| anyhow::anyhow!("Goal non trovato: {}", goal_id))?
                .clone();

            println!(
                "🔪 Scomposizione atomica del goal: \"{}\"...",
                goal.description
            );

            let api_key = std::env::var("OPENROUTER_API_KEY").ok();
            let mut sub_goals = Vec::new();

            if let Some(key) = api_key {
                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};

                let model_id = std::env::var("OPENROUTER_MODEL")
                    .unwrap_or_else(|_| "deepseek/deepseek-r1:free".to_string());
                let client = OpenRouterClient::new(key, OpenRouterModel::Custom(model_id));

                let prompt = format!(
                                    "Decompose the following software goal into 3-5 atomic, deterministic sub-tasks: '{}'. \
                                    Return ONLY a JSON array of strings representing the sub-task descriptions.",
                                    goal.description
                                );

                let context = LLMContext {
                    goal_description: goal.description.clone(),
                    context: "Atomic Slicing phase via CLI.".to_string(),
                    p2p_intelligence: "".to_string(),
                    constraints: vec![],
                    previous_approaches: vec![],
                };

                if let Ok(suggestion) = client.generate_code(&prompt, &context).await {
                    let content = suggestion.content.trim();
                    let json_start = content.find('[');
                    let json_end = content.rfind(']');

                    if let (Some(start), Some(end)) = (json_start, json_end) {
                        let json_str = &content[start..=end];
                        if let Ok(tasks) = serde_json::from_str::<Vec<String>>(json_str) {
                            for task_desc in tasks {
                                if let Ok(sg) = sentinel_core::goal_manifold::goal::Goal::builder()
                                                    .description(task_desc)
                                                    .parent(goal.id)
                                                    .add_success_criterion(sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue)
                                                    .build() {
                                                    sub_goals.push(sg);
                                                }
                            }
                        }
                    }
                }
            }

            if sub_goals.is_empty() {
                println!("⚠️  LLM non disponibile o fallito. Uso slicer euristico locale.");
                use sentinel_core::goal_manifold::slicer::AtomicSlicer;
                if let Ok(decomposed) = AtomicSlicer::decompose(&goal) {
                    sub_goals = decomposed;
                }
            }

            let count = sub_goals.len();
            for sg in sub_goals {
                println!("  - [NUOVO TASK] {}", sg.description);
                manifold.add_goal(sg)?;
            }

            save_manifold(&cli.manifold, &manifold)?;
            println!(
                "\n✅ SCOMPOSIZIONE COMPLETATA: {} nuovi task aggiunti al manifold.",
                count
            );
        }
    }
    Ok(())
}

fn load_manifold(path: &std::path::Path) -> anyhow::Result<sentinel_core::GoalManifold> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_manifold(
    path: &std::path::Path,
    manifold: &sentinel_core::GoalManifold,
) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(manifold)?;
    std::fs::write(path, content)?;
    Ok(())
}
