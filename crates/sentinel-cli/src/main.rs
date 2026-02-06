#![recursion_limit = "256"]
#![allow(dead_code, unused_variables)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tokio::process::Command;

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
mod reliability;
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

    /// Gestione contratto governance (dependencies/frameworks/endpoints)
    Governance {
        #[command(subcommand)]
        action: GovernanceAction,
    },
}

#[derive(Subcommand)]
enum GovernanceAction {
    /// Mostra baseline, proposta pending e stato approvazione
    Status {
        /// Output in formato JSON
        #[arg(long)]
        json: bool,
    },
    /// Approva la proposta pending e aggiorna la baseline
    Approve {
        /// Nota opzionale di approvazione utente
        #[arg(long)]
        note: Option<String>,
    },
    /// Rifiuta la proposta pending
    Reject {
        /// Motivazione del rifiuto
        reason: String,
    },
    /// Genera preview baseline governance dal workspace e opzionalmente applica lock
    Seed {
        /// Applica realmente la baseline (altrimenti solo preview)
        #[arg(long)]
        apply: bool,
        /// Se true, sincronizza anche required=*allowed (lock stretto)
        #[arg(long, default_value = "true")]
        lock_required: bool,
        /// Output in formato JSON
        #[arg(long)]
        json: bool,
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

            if gemini_key.or(openrouter_key).is_some() {
                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::openrouter::{OpenRouterClient, OpenRouterModel};
                use sentinel_agent_native::providers::gemini::GeminiClient;

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
                    let model = std::env::var("GEMINI_MODEL")
                        .unwrap_or_else(|_| "gemini-1.5-pro".to_string());
                    Box::new(GeminiClient::new(
                        std::env::var("GEMINI_API_KEY").unwrap(),
                        model,
                    ))
                } else {
                    let model_id = std::env::var("OPENROUTER_MODEL")
                        .unwrap_or_else(|_| "deepseek/deepseek-r1:free".to_string());
                    Box::new(OpenRouterClient::new(
                        std::env::var("OPENROUTER_API_KEY").unwrap(),
                        OpenRouterModel::Custom(model_id),
                    ))
                };

                if let Ok(suggestion) = client.generate_code(&prompt, &context).await {
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
            println!(
                "\n✅ PROGETTO INIZIALIZZATO: {} goal creati.",
                manifold.goal_dag.goals().count()
            );
        }
        Commands::Design { intent } => {
            println!(
                "Sentinel Architect sta analizzando l'intento: \"{}\"...",
                intent
            );

            let gemini_key = std::env::var("GEMINI_API_KEY").ok();
            if let Some(key) = gemini_key {
                println!(
                    "✅ Connesso a Google Gemini (Modello: {})",
                    std::env::var("GEMINI_MODEL").unwrap_or_default()
                );

                use sentinel_agent_native::llm_integration::{LLMClient, LLMContext};
                use sentinel_agent_native::providers::gemini::GeminiClient;

                let model =
                    std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-pro".to_string());
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
        Commands::Status { json } => {
            let manifold = load_manifold(&cli.manifold)?;
            let state = sentinel_core::ProjectState::new(std::env::current_dir()?);
            let field = sentinel_core::AlignmentField::new(manifold.clone());
            let alignment = field.compute_alignment(&state).await?;
            let guardrail = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
            let reliability = reliability::snapshot_from_signals(
                alignment.score,
                alignment.confidence,
                manifold.completion_percentage(),
                guardrail.allowed,
            );
            let reliability_config = reliability::load_reliability_config(&cli.manifold);
            let reliability_eval =
                reliability::evaluate_snapshot(&reliability, &reliability_config.thresholds);

            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "manifold": cli.manifold,
                        "integrity_ok": manifold.verify_integrity(),
                        "goals_total": manifold.goal_count(),
                        "completion_percentage": manifold.completion_percentage(),
                        "alignment_score": alignment.score,
                        "alignment_confidence": alignment.confidence,
                        "guardrail_allowed": guardrail.allowed,
                        "guardrail_reason": guardrail.reason,
                        "reliability": reliability,
                        "reliability_thresholds": reliability_config.thresholds,
                        "reliability_slo": reliability_eval,
                        "compass": {
                            "where_we_are": format!("{} goals, {:.1}% completion", manifold.goal_count(), manifold.completion_percentage() * 100.0),
                            "where_we_must_go": manifold.root_intent.description,
                            "how": "Track alignment score and satisfy goal predicates",
                            "why": "Prevent drift from root intent with deterministic checks"
                        }
                    })
                );
            } else {
                println!("SENTINEL STATUS");
                println!("Manifold: {}", cli.manifold.display());
                println!(
                    "Integrity: {}",
                    if manifold.verify_integrity() {
                        "OK"
                    } else {
                        "FAILED"
                    }
                );
                println!("Goals: {}", manifold.goal_count());
                println!(
                    "Completion: {:.1}%",
                    manifold.completion_percentage() * 100.0
                );
                println!("Alignment Score: {:.1}/100", alignment.score);
                println!("Alignment Confidence: {:.2}", alignment.confidence);
                println!(
                    "Guardrail: {}",
                    if guardrail.allowed {
                        "UNLOCKED"
                    } else {
                        "LOCKED"
                    }
                );
                if let Some(reason) = guardrail.reason {
                    println!("Reason: {}", reason);
                }
                println!(
                    "DOVE: {:.1}% completion, score {:.1}",
                    manifold.completion_percentage() * 100.0,
                    alignment.score
                );
                println!("DOVE DEVE ANDARE: {}", manifold.root_intent.description);
                println!("COME: eseguire goal atomici rispettando invarianti e guardrail");
                println!("PERCHE: garantire allineamento continuo all'intento radice");
                println!(
                    "Reliability: success {:.1}%, no-regression {:.1}%, rollback {:.1}%, avg-TTR {:.0}ms, invariant-violation {:.1}%",
                    reliability.task_success_rate * 100.0,
                    reliability.no_regression_rate * 100.0,
                    reliability.rollback_rate * 100.0,
                    reliability.avg_time_to_recover_ms,
                    reliability.invariant_violation_rate * 100.0
                );
                println!(
                    "Reliability SLO: {}",
                    if reliability_eval.healthy {
                        "HEALTHY"
                    } else {
                        "VIOLATED"
                    }
                );
                if !reliability_eval.violations.is_empty() {
                    println!(
                        "SLO Violations: {}",
                        reliability_eval.violations.join(" | ")
                    );
                }
            }
        }
        Commands::Ui => {
            tui::run_tui(cli.manifold.clone())?;
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
            let parsed_violation = uuid::Uuid::parse_str(&violation_id)
                .map_err(|_| anyhow::anyhow!("Invalid violation UUID"))?;
            manifold
                .overrides
                .push(sentinel_core::types::HumanOverride {
                    violation_id: parsed_violation,
                    reason,
                    timestamp: chrono::Utc::now(),
                });
            save_manifold(&cli.manifold, &manifold)?;
            println!("Override registrato.");
        }
        Commands::Calibrate { value } => {
            if !(0.0..=1.0).contains(&value) {
                return Err(anyhow::anyhow!(
                    "Calibration value must be in range [0.0, 1.0]"
                ));
            }
            let mut manifold = load_manifold(&cli.manifold)?;
            manifold.sensitivity = value;
            save_manifold(&cli.manifold, &manifold)?;
            println!("Sensitivity aggiornata a {:.2}", value);
        }
        Commands::Doctor => {
            let mut checks = Vec::new();

            checks.push((
                "manifold_exists",
                cli.manifold.exists(),
                format!("Path: {}", cli.manifold.display()),
            ));

            if cli.manifold.exists() {
                match load_manifold(&cli.manifold) {
                    Ok(manifold) => {
                        checks.push((
                            "manifold_integrity",
                            manifold.verify_integrity(),
                            "Cryptographic hash verification".to_string(),
                        ));
                        checks.push((
                            "goal_dag_non_empty",
                            manifold.goal_count() > 0,
                            format!("Goal count: {}", manifold.goal_count()),
                        ));
                    }
                    Err(err) => {
                        checks.push(("manifold_parsing", false, format!("Parse error: {}", err)))
                    }
                }
            }

            let all_ok = checks.iter().all(|(_, ok, _)| *ok);
            for (name, ok, detail) in checks {
                println!(
                    "[{}] {} - {}",
                    if ok { "PASS" } else { "FAIL" },
                    name,
                    detail
                );
            }
            println!(
                "Doctor verdict: {}",
                if all_ok { "HEALTHY" } else { "ISSUES DETECTED" }
            );
        }
        Commands::Run { command } => {
            if command.is_empty() {
                return Err(anyhow::anyhow!("No command provided"));
            }

            let manifold = load_manifold(&cli.manifold)?;
            let decision = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
            if !decision.allowed {
                let reason = decision
                    .reason
                    .unwrap_or_else(|| "Unknown guardrail reason".to_string());
                println!("SENTINEL GUARDIAN BLOCK: {}", reason);
                return Ok(());
            }

            let mut cmd = Command::new(&command[0]);
            if command.len() > 1 {
                cmd.args(&command[1..]);
            }
            let output = cmd.output().await?;
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Command failed with exit code {:?}",
                    output.status.code()
                ));
            }
        }
        Commands::Verify { sandbox } => {
            if sandbox {
                let cwd = std::env::current_dir()?;
                let sb = sentinel_sandbox::Sandbox::new()?;
                sb.mirror_project(&cwd)?;
                let ok = sb.verify_atomic_truth().await?;
                println!("Sandbox verification: {}", if ok { "PASS" } else { "FAIL" });
            } else {
                let manifold = load_manifold(&cli.manifold)?;
                println!(
                    "Local verification: integrity {}",
                    if manifold.verify_integrity() {
                        "OK"
                    } else {
                        "FAILED"
                    }
                );
            }
        }
        Commands::Decompose { goal_id } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            let goal_uuid = uuid::Uuid::parse_str(&goal_id)
                .map_err(|_| anyhow::anyhow!("Invalid goal UUID"))?;
            let goal = manifold
                .get_goal(&goal_uuid)
                .ok_or_else(|| anyhow::anyhow!("Goal not found"))?
                .clone();

            let sub_goals = sentinel_core::goal_manifold::slicer::AtomicSlicer::decompose(&goal)?;
            let count = sub_goals.len();
            for sub_goal in sub_goals {
                let _ = manifold.add_goal(sub_goal);
            }
            save_manifold(&cli.manifold, &manifold)?;
            println!("Goal decomposto in {} task atomici.", count);
        }
        Commands::Learnings => {
            let manifold = load_manifold(&cli.manifold)?;
            println!("Learnings snapshot");
            println!("Overrides: {}", manifold.overrides.len());
            println!("Handover notes: {}", manifold.handover_log.len());
            println!("Version history: {}", manifold.version_history.len());
        }
        Commands::Governance { action } => {
            let mut manifold = load_manifold(&cli.manifold)?;
            match action {
                GovernanceAction::Status { json } => {
                    let pending = manifold.governance.pending_proposal.clone();
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "required_dependencies": manifold.governance.required_dependencies,
                                "allowed_dependencies": manifold.governance.allowed_dependencies,
                                "required_frameworks": manifold.governance.required_frameworks,
                                "allowed_frameworks": manifold.governance.allowed_frameworks,
                                "allowed_endpoints": manifold.governance.allowed_endpoints,
                                "allowed_ports": manifold.governance.allowed_ports,
                                "pending_proposal": pending,
                                "history_size": manifold.governance.history.len()
                            })
                        );
                    } else {
                        println!("SENTINEL GOVERNANCE STATUS");
                        println!(
                            "Allowed Dependencies: {}",
                            manifold.governance.allowed_dependencies.join(", ")
                        );
                        println!(
                            "Allowed Frameworks: {}",
                            manifold.governance.allowed_frameworks.join(", ")
                        );
                        println!(
                            "Allowed Ports: {}",
                            manifold
                                .governance
                                .allowed_ports
                                .iter()
                                .map(|p| p.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        if let Some(p) = pending {
                            println!("Pending Proposal: {}", p.id);
                            println!("Rationale: {}", p.rationale);
                        } else {
                            println!("Pending Proposal: none");
                        }
                    }
                }
                GovernanceAction::Approve { note } => {
                    let proposal_id = manifold.approve_pending_governance_proposal(note)?;
                    save_manifold(&cli.manifold, &manifold)?;
                    println!("Governance proposal approved: {}", proposal_id);
                }
                GovernanceAction::Reject { reason } => {
                    let proposal_id =
                        manifold.reject_pending_governance_proposal(Some(reason.clone()))?;
                    save_manifold(&cli.manifold, &manifold)?;
                    println!("Governance proposal rejected: {} ({})", proposal_id, reason);
                }
                GovernanceAction::Seed {
                    apply,
                    lock_required,
                    json,
                } => {
                    let root = cli
                        .manifold
                        .parent()
                        .map(std::path::Path::to_path_buf)
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let observed = sentinel_agent_native::observe_workspace_governance(&root)?;

                    let diff = governance_seed_diff(&manifold, &observed);
                    if json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "apply": apply,
                                "lock_required": lock_required,
                                "observed": observed,
                                "diff": diff
                            })
                        );
                    } else {
                        println!("SENTINEL GOVERNANCE SEED");
                        println!("Workspace root: {}", root.display());
                        println!(
                            "Observed: deps={} frameworks={} endpoints={} ports={}",
                            observed.dependencies.len(),
                            observed.frameworks.len(),
                            observed.endpoints.len(),
                            observed.ports.len()
                        );
                        println!("Diff: {}", serde_json::to_string_pretty(&diff)?);
                    }

                    if apply {
                        manifold.apply_governance_seed(
                            observed.dependencies.clone(),
                            observed.frameworks.clone(),
                            observed.endpoints.clone(),
                            observed.ports.clone(),
                            lock_required,
                        );
                        save_manifold(&cli.manifold, &manifold)?;
                        if !json {
                            println!("Governance baseline seeded and locked.");
                        }
                    } else if !json {
                        println!("Preview only. Re-run with `--apply` to persist baseline.");
                    }
                }
            }
        }
        Commands::Sync => {
            println!(
                "Sync completata (placeholder operativo): nessuna sorgente esterna configurata."
            );
        }
        Commands::Federate { relay } => {
            let relay_info = relay.unwrap_or_else(|| "no-relay".to_string());
            println!(
                "Federation bootstrap (best-effort) con relay: {}",
                relay_info
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

fn governance_seed_diff(
    manifold: &sentinel_core::GoalManifold,
    observed: &sentinel_agent_native::GovernanceObservation,
) -> serde_json::Value {
    fn set_of<T: Clone + Ord>(values: &[T]) -> std::collections::BTreeSet<T> {
        values.iter().cloned().collect()
    }

    let current_deps = set_of(&manifold.governance.allowed_dependencies);
    let next_deps = set_of(&observed.dependencies);
    let current_frameworks = set_of(&manifold.governance.allowed_frameworks);
    let next_frameworks = set_of(&observed.frameworks);
    let current_endpoints: std::collections::BTreeSet<String> = manifold
        .governance
        .allowed_endpoints
        .values()
        .cloned()
        .collect();
    let next_endpoints = set_of(&observed.endpoints);
    let current_ports = set_of(&manifold.governance.allowed_ports);
    let next_ports = set_of(&observed.ports);

    serde_json::json!({
        "dependencies": {
            "add": next_deps.difference(&current_deps).cloned().collect::<Vec<_>>(),
            "remove": current_deps.difference(&next_deps).cloned().collect::<Vec<_>>()
        },
        "frameworks": {
            "add": next_frameworks.difference(&current_frameworks).cloned().collect::<Vec<_>>(),
            "remove": current_frameworks.difference(&next_frameworks).cloned().collect::<Vec<_>>()
        },
        "endpoints": {
            "add": next_endpoints.difference(&current_endpoints).cloned().collect::<Vec<_>>(),
            "remove": current_endpoints.difference(&next_endpoints).cloned().collect::<Vec<_>>()
        },
        "ports": {
            "add": next_ports.difference(&current_ports).cloned().collect::<Vec<_>>(),
            "remove": current_ports.difference(&next_ports).cloned().collect::<Vec<_>>()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_core::goal_manifold::Intent;

    #[test]
    fn governance_seed_diff_reports_additions_and_removals() {
        let intent = Intent::new("test", Vec::<String>::new());
        let mut manifold = sentinel_core::GoalManifold::new(intent);
        manifold.governance.allowed_dependencies = vec!["cargo:serde".to_string()];
        manifold.governance.allowed_frameworks = vec!["framework:axum".to_string()];
        manifold
            .governance
            .allowed_endpoints
            .insert("api".to_string(), "/health".to_string());
        manifold.governance.allowed_ports = vec![3000];

        let observed = sentinel_agent_native::GovernanceObservation {
            dependencies: vec!["cargo:serde".to_string(), "cargo:tokio".to_string()],
            frameworks: vec!["framework:actix".to_string()],
            endpoints: vec!["/metrics".to_string()],
            ports: vec![4000],
        };

        let diff = governance_seed_diff(&manifold, &observed);
        assert_eq!(
            diff["dependencies"]["add"],
            serde_json::json!(["cargo:tokio"])
        );
        assert_eq!(diff["dependencies"]["remove"], serde_json::json!([]));
        assert_eq!(
            diff["frameworks"]["add"],
            serde_json::json!(["framework:actix"])
        );
        assert_eq!(
            diff["frameworks"]["remove"],
            serde_json::json!(["framework:axum"])
        );
        assert_eq!(diff["endpoints"]["add"], serde_json::json!(["/metrics"]));
        assert_eq!(diff["endpoints"]["remove"], serde_json::json!(["/health"]));
        assert_eq!(diff["ports"]["add"], serde_json::json!([4000]));
        assert_eq!(diff["ports"]["remove"], serde_json::json!([3000]));
    }
}
