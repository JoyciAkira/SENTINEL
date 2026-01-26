use clap::{Parser, Subcommand};
use sentinel_core::Result;
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

#[derive(Subcommand)]
enum Commands {
    /// Inizializza un nuovo Goal Manifold
    Init {
        /// Descrizione dell'intento originale
        description: String,
    },

    /// Mostra lo stato attuale dell'allineamento
    Status,

    /// Avvia l'interfaccia TUI interattiva
    Ui,

    /// Analizza i pattern appresi dai progetti precedenti
    Learnings,
}

mod tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { description } => {
            println!("Inizializzazione nuovo manifold: {}", description);
        }
        Commands::Status => {
            println!("Recupero stato di allineamento...");
        }
        Commands::Ui => {
            tui::run_tui()?;
        }
        Commands::Learnings => {
            println!("Esplorazione Knowledge Base...");
        }
    }

    Ok(())
}
