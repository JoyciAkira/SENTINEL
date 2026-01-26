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
                    println!("{}", serde_json::to_string(&manifold)?);
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
            tui::run_tui()?;
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
    }

    Ok(())
}
