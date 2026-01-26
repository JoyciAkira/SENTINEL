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

mod tui;
mod mcp;
mod lsp;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { description } => {
            let intent = sentinel_core::goal_manifold::Intent::new(description, Vec::<String>::new());
            let manifold = sentinel_core::GoalManifold::new(intent);
            let content = serde_json::to_string_pretty(&manifold)?;
            std::fs::write(&cli.manifold, content)?;
            println!("Inizializzato nuovo manifold in: {:?}", cli.manifold);
        }
        Commands::Status { json } => {
            if cli.manifold.exists() {
                let content = std::fs::read_to_string(&cli.manifold)?;
                let manifold: sentinel_core::GoalManifold = serde_json::from_str(&content)?;
                
                if json {
                    let mut watcher = sentinel_core::external::DependencyWatcher::new(std::path::PathBuf::from("."));
                    let _ = watcher.scan_dependencies().await;
                    let alerts = watcher.run_security_audit();
                    
                    let status_report = serde_json::json!({
                        "manifold": manifold,
                        "external": {
                            "risk_level": watcher.check_alignment_risk(),
                            "alerts": alerts,
                            "dependency_count": manifold.root_intent.infrastructure_map.len()
                        }
                    });
                    println!("{}", serde_json::to_string(&status_report)?);
                } else {
                    println!("GOAL MANIFOLD: {}", manifold.root_intent.description);
                    println!("COMPLETAMENTO: {:.1}%", manifold.completion_percentage() * 100.0);
                    println!("VERSIONE: {}", manifold.current_version());
                }
            } else {
                if json {
                    println!("{{ \"error\": \"Manifold not found\" }}");
                } else {
                    println!("Errore: File {:?} non trovato. Esegui 'init' prima.", cli.manifold);
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
        Commands::Override { violation_id, reason } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            let vid = uuid::Uuid::parse_str(&violation_id)?;
            
            manifold.overrides.push(sentinel_core::types::HumanOverride {
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
            let mut watcher = sentinel_core::external::DependencyWatcher::new(std::path::PathBuf::from("."));
            
            // Scansione dipendenze
            match watcher.scan_dependencies().await {
                Ok(deps) => {
                    println!("Trovate {} dipendenze nel progetto.", deps.len());
                    for dep in deps {
                        println!("- {} ({})", dep.name, dep.version);
                    }
                },
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
