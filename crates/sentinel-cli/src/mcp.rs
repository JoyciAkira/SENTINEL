//! MCP Server - Model Context Protocol Implementation
//!
//! Permette a Sentinel di comunicare con agenti esterni (Cline, Claude Desktop)
//! esponendo strumenti di validazione e analisi dell'allineamento.

use sentinel_core::goal_manifold::goal::Goal;
use sentinel_core::goal_manifold::predicate::Predicate;
use sentinel_core::types::Comparison;
use sentinel_core::{security::SecurityScanner, AlignmentField, GoalManifold, ProjectState};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::reliability;

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
    id: Value,
}

#[derive(Debug, Deserialize)]
struct GoalInitRequest {
    description: String,
    constraints: Option<Vec<String>>,
    expected_outcomes: Option<Vec<String>>,
    target_platform: Option<String>,
    languages: Option<Vec<String>>,
    frameworks: Option<Vec<String>>,
    goals: Option<Vec<GoalSpec>>,
}

#[derive(Debug, Deserialize)]
struct GoalSpec {
    description: String,
    scope_in: Option<Vec<String>>,
    scope_out: Option<Vec<String>>,
    deliverables: Option<Vec<String>>,
    constraints: Option<Vec<String>>,
    validation_tests: Option<Vec<String>>,
    success_criteria: Vec<GoalCriteriaSpec>,
}

#[derive(Debug, Deserialize)]
struct GoalCriteriaSpec {
    #[serde(rename = "type")]
    kind: String,
    path: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    expected_exit_code: Option<i32>,
    suite: Option<String>,
    min_coverage: Option<f64>,
    url: Option<String>,
    expected_status: Option<u16>,
    expected_body_contains: Option<String>,
    metric: Option<String>,
    threshold: Option<f64>,
    comparison: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChatMemoryStore {
    version: u32,
    turns: Vec<ChatMemoryTurn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMemoryTurn {
    id: String,
    timestamp: i64,
    user: String,
    assistant: String,
    intent_summary: String,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplayLedgerEntry {
    version: u32,
    turn_id: String,
    timestamp: i64,
    user_message: String,
    response_hash: String,
    strict_goal_execution: bool,
    memory_hit_ids: Vec<String>,
    evidence_hash: String,
    constitutional_spec_hash: String,
    counterfactual_hash: String,
    policy_simulation_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamMemoryGraphNode {
    id: String,
    timestamp: i64,
    intent_summary: String,
    provenance_hash: String,
    lamport: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamMemoryGraphEdge {
    source: String,
    target: String,
    weight: f64,
    relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamMemoryGraph {
    version: u32,
    generated_at: i64,
    node_count: usize,
    edge_count: usize,
    nodes: Vec<TeamMemoryGraphNode>,
    edges: Vec<TeamMemoryGraphEdge>,
    graph_hash: String,
    signer_public_key: String,
    signature: String,
    signature_scheme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestrationTask {
    id: String,
    index: usize,
    title: String,
    mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestrationTaskResult {
    id: String,
    index: usize,
    title: String,
    mode: String,
    status: String,
    summary: String,
    risk: String,
    approval_needed: Vec<String>,
    next_step: String,
    output_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Avvia il server MCP su stdin/stdout
pub async fn run_server() -> anyhow::Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();

    loop {
        let mut line = String::new();
        if stdin.read_line(&mut line).await? == 0 {
            break;
        }

        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(_) => continue,
        };

        if let Some(id) = request.id.clone() {
            let response = handle_request(request, id).await;
            let response_json = serde_json::to_string(&response)? + "\n";
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

async fn handle_request(req: McpRequest, id: Value) -> McpResponse {
    match req.method.as_str() {
        "initialize" => McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": true }
                },
                "serverInfo": {
                    "name": "sentinel-server",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
            id,
        },
        "tools/list" => McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "tools": [
                    {
                        "name": "init_project",
                        "description": "Inizializza un nuovo progetto Sentinel con obiettivi e invarianti (World Class)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "description": { "type": "string", "description": "Descrizione dell'intento del progetto" },
                                "constraints": { "type": "array", "items": { "type": "string" } },
                                "expected_outcomes": { "type": "array", "items": { "type": "string" } },
                                "target_platform": { "type": "string" },
                                "languages": { "type": "array", "items": { "type": "string" } },
                                "frameworks": { "type": "array", "items": { "type": "string" } },
                                "goals": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "description": { "type": "string" },
                                            "scope_in": { "type": "array", "items": { "type": "string" } },
                                            "scope_out": { "type": "array", "items": { "type": "string" } },
                                            "deliverables": { "type": "array", "items": { "type": "string" } },
                                            "constraints": { "type": "array", "items": { "type": "string" } },
                                            "validation_tests": { "type": "array", "items": { "type": "string" } },
                                            "success_criteria": {
                                                "type": "array",
                                                "items": {
                                                    "type": "object",
                                                    "properties": {
                                                        "type": { "type": "string" },
                                                        "path": { "type": "string" },
                                                        "command": { "type": "string" },
                                                        "args": { "type": "array", "items": { "type": "string" } },
                                                        "expected_exit_code": { "type": "integer" },
                                                        "suite": { "type": "string" },
                                                        "min_coverage": { "type": "number" },
                                                        "url": { "type": "string" },
                                                        "expected_status": { "type": "integer" },
                                                        "expected_body_contains": { "type": "string" },
                                                        "metric": { "type": "string" },
                                                        "threshold": { "type": "number" },
                                                        "comparison": { "type": "string" }
                                                    },
                                                    "required": ["type"]
                                                }
                                            }
                                        },
                                        "required": ["description", "success_criteria"]
                                    }
                                }
                            },
                            "required": ["description"]
                        }
                    },
                    {
                        "name": "suggest_goals",
                        "description": "Suggerisce una lista di goal atomici sulla base dell'intento e dei vincoli",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "description": { "type": "string" },
                                "constraints": { "type": "array", "items": { "type": "string" } },
                                "expected_outcomes": { "type": "array", "items": { "type": "string" } },
                                "target_platform": { "type": "string" },
                                "languages": { "type": "array", "items": { "type": "string" } },
                                "frameworks": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["description"]
                        }
                    },
                    {
                        "name": "validate_action",
                        "description": "Valida un'azione proposta contro il Goal Manifold usando simulazioni Monte Carlo",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "action_type": { "type": "string", "description": "Tipo di azione (es. edit_file)" },
                                "description": { "type": "string", "description": "Descrizione dell'intento" }
                            },
                            "required": ["action_type", "description"]
                        }
                    },
                    {
                        "name": "get_alignment",
                        "description": "Restituisce il punteggio di allineamento reale del progetto",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_reliability",
                        "description": "Restituisce KPI di affidabilità runtime derivati da alignment/guardrail/progresso",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "governance_status",
                        "description": "Restituisce stato governance corrente (dependencies/frameworks/endpoints/ports + pending proposal)",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_world_model",
                        "description": "Restituisce il Project World Model corrente (where we are / where we must go / drift / pending governance change)",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "governance_approve",
                        "description": "Approva la proposal governance pending e aggiorna il manifold",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "note": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "governance_reject",
                        "description": "Rifiuta la proposal governance pending e aggiorna il manifold",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "reason": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "governance_seed",
                        "description": "Genera preview/applica baseline governance dal workspace osservato deterministicamente",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "apply": { "type": "boolean" },
                                "lock_required": { "type": "boolean" }
                            }
                        }
                    },
                    {
                        "name": "get_quality_status",
                        "description": "Restituisce l'ultimo report quality harness disponibile",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "quality_report",
                        "description": "Restituisce un report quality specifico per ID",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "report_id": {
                                    "type": "string",
                                    "description": "ID del report quality da recuperare (es. qr_1234567890)"
                                }
                            },
                            "required": ["report_id"]
                        }
                    },
                    {
                        "name": "list_quality_reports",
                        "description": "Lista gli ultimi report quality harness (storico)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                            }
                        }
                    },
                    {
                        "name": "run_quality_harness",
                        "description": "Esegue il world-class harness e restituisce il report più recente",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "safe_write",
                        "description": "Esegue uno scan di sicurezza Layer 7 e valida l'allineamento prima di procedere",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string", "description": "File da scrivere" },
                                "content": { "type": "string", "description": "Contenuto del codice" }
                            },
                            "required": ["path", "content"]
                        }
                    },
                    {
                        "name": "propose_strategy",
                        "description": "Suggerisce una strategia basata sui pattern estratti dalla Knowledge Base (Layer 5)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "goal_description": { "type": "string" }
                            },
                            "required": ["goal_description"]
                        }
                    },
                    {
                        "name": "record_handover",
                        "description": "Registra permanentemente una nota di contesto nel manifold sentinel.json",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "goal_id": { "type": "string" },
                                "content": { "type": "string" },
                                "warnings": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["goal_id", "content"]
                        }
                    },
                    {
                        "name": "get_cognitive_map",
                        "description": "Recupera la mappatura gerarchica di tutti i goal attivi",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_enforcement_rules",
                        "description": "Recupera le invarianti e le regole di condotta attive nel progetto",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "get_goal_graph",
                        "description": "Restituisce il Goal Manifold come grafo (Nodes/Edges) per visualizzazione frontend",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "decompose_goal",
                        "description": "Scompone un goal complesso in una serie di task atomici deterministici (Atomic Truth)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "goal_id": { "type": "string", "description": "L'UUID del goal da scomporre" }
                            },
                            "required": ["goal_id"]
                        }
                    },
                    {
                        "name": "orchestrate_task",
                        "description": "Orchestra un task complesso in subtask (plan/build/review/deploy) con parallelismo bounded e output summary-only",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task": { "type": "string", "description": "Descrizione del task da orchestrare" },
                                "modes": {
                                    "type": "array",
                                    "items": { "type": "string", "enum": ["plan", "build", "review", "deploy"] }
                                },
                                "max_parallel": { "type": "integer", "minimum": 1, "maximum": 4 },
                                "subtask_count": { "type": "integer", "minimum": 2, "maximum": 6 }
                            },
                            "required": ["task"]
                        }
                    },
                    {
                        "name": "chat",
                        "description": "Invia un messaggio all'agente Sentinel con memoria contestuale persistente ed explainability per turno",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": { "type": "string", "description": "Il messaggio dell'utente" }
                            },
                            "required": ["message"]
                        }
                    },
                    {
                        "name": "chat_memory_clear",
                        "description": "Azzera la memoria conversazionale persistente locale usata per il contesto chat",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "chat_memory_status",
                        "description": "Restituisce statistiche e ultimi turni della memoria conversazionale",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "chat_memory_search",
                        "description": "Ricerca semantica lightweight nella memoria conversazionale",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string" },
                                "limit": { "type": "integer" }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "chat_memory_export",
                        "description": "Esporta la memoria conversazionale su file JSON",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            }
                        }
                    },
                    {
                        "name": "chat_memory_import",
                        "description": "Importa memoria conversazionale da file JSON",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "merge": { "type": "boolean" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "agent_communication_send",
                        "description": "Send message between agents in the split-agent network",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "from_agent": { "type": "string", "description": "Source agent ID" },
                                "to_agent": { "type": "string", "description": "Target agent ID (null for broadcast)" },
                                "message_type": { "type": "string", "enum": ["direct", "broadcast", "handoff", "help_request", "pattern_share"], "description": "Type of message" },
                                "payload": { "type": "object", "description": "Message payload" }
                            },
                            "required": ["from_agent", "message_type", "payload"]
                        }
                    },
                    {
                        "name": "agent_communication_status",
                        "description": "Get status of all agents in the communication network",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "agent_communication_history",
                        "description": "Get message history between agents",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_id": { "type": "string", "description": "Filter by agent (null for all)" },
                                "limit": { "type": "integer", "default": 50 }
                            }
                        }
                    }
                ]
            })),
            error: None,
            id,
        },
        "tools/call" => {
            let result = handle_tool_call(req.params).await;
            McpResponse {
                jsonrpc: "2.0".to_string(),
                result,
                error: None,
                id,
            }
        }
        _ => McpResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
            id,
        },
    }
}

/// Trova il percorso del file sentinel.json risalendo le directory
fn find_manifold_path() -> Option<PathBuf> {
    // 0. Explicit manifold file override
    if let Ok(manifold_path) = std::env::var("SENTINEL_MANIFOLD") {
        let path = PathBuf::from(manifold_path);
        if path.exists() {
            return Some(path);
        }
    }

    // 1. Controlla variabile d'ambiente (per override esplicito)
    if let Ok(root) = std::env::var("SENTINEL_ROOT") {
        let path = PathBuf::from(root).join("sentinel.json");
        if path.exists() {
            return Some(path);
        }
    }

    // 2. Cerca risalendo le directory dalla CWD
    if let Ok(mut current_dir) = std::env::current_dir() {
        loop {
            let path = current_dir.join("sentinel.json");
            if path.exists() {
                return Some(path);
            }
            if !current_dir.pop() {
                break;
            }
        }
    }

    None
}

/// Helper per caricare il manifold in modo sicuro
fn get_manifold() -> Result<GoalManifold, String> {
    let path = find_manifold_path().ok_or_else(|| {
        let cwd = std::env::current_dir().unwrap_or_default();
        format!("Manifold file 'sentinel.json' non trovato (Cercato a partire da: {:?}). Inizializza il progetto con 'sentinel init'.", cwd)
    })?;

    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Errore lettura file: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Errore parsing manifold: {}", e))
}

/// Helper per salvare il manifold
fn save_manifold(manifold: &GoalManifold) -> Result<(), String> {
    let path = find_manifold_path().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("sentinel.json")
    });
    let content = serde_json::to_string_pretty(manifold)
        .map_err(|e| format!("Errore serializzazione: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Errore scrittura file: {}", e))?;
    persist_world_model_snapshot_for(manifold, path.parent().unwrap_or(std::path::Path::new(".")))?;
    Ok(())
}

fn find_chat_memory_path() -> Option<PathBuf> {
    let root = find_manifold_path()?.parent()?.to_path_buf();
    let dir = root.join(".sentinel");
    Some(dir.join("chat_memory.json"))
}

fn default_chat_memory_export_path() -> Option<PathBuf> {
    let root = find_manifold_path()?.parent()?.to_path_buf();
    Some(root.join(".sentinel").join("chat_memory_export.json"))
}

fn persist_world_model_snapshot_for(
    manifold: &GoalManifold,
    root: &std::path::Path,
) -> Result<(), String> {
    let path = root.join(".sentinel").join("world_model.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Errore creazione directory world model: {}", e))?;
    }

    let observed = sentinel_agent_native::observe_workspace_governance(root).ok();
    let payload = world_model_snapshot(manifold, observed.as_ref());
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Errore serializzazione world model: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Errore scrittura world model: {}", e))
}

fn load_chat_memory() -> ChatMemoryStore {
    let Some(path) = find_chat_memory_path() else {
        return ChatMemoryStore {
            version: 1,
            turns: Vec::new(),
        };
    };
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => {
            return ChatMemoryStore {
                version: 1,
                turns: Vec::new(),
            }
        }
    };
    let mut store: ChatMemoryStore = serde_json::from_str(&content).unwrap_or_default();
    if store.version == 0 {
        store.version = 1;
    }
    store
}

fn save_chat_memory(store: &ChatMemoryStore) -> Result<(), String> {
    let Some(path) = find_chat_memory_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Errore creazione directory memoria: {}", e))?;
    }
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Errore serializzazione memoria: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Errore scrittura memoria: {}", e))
}

fn save_chat_memory_to_path(path: &std::path::Path, store: &ChatMemoryStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Errore creazione directory export: {}", e))?;
    }
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Errore serializzazione export memoria: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("Errore scrittura export memoria: {}", e))
}

fn load_chat_memory_from_path(path: &std::path::Path) -> Result<ChatMemoryStore, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Errore lettura import memoria '{}': {}", path.display(), e))?;
    let mut store: ChatMemoryStore = serde_json::from_str(&content)
        .map_err(|e| format!("Errore parsing import memoria: {}", e))?;
    if store.version == 0 {
        store.version = 1;
    }
    Ok(store)
}

fn workspace_root() -> PathBuf {
    find_manifold_path()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn innovation_root(root: &std::path::Path) -> PathBuf {
    root.join(".sentinel").join("innovation")
}

fn stable_hash_hex(input: &str) -> String {
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

fn persist_json(path: &std::path::Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Errore creazione directory '{}': {}", parent.display(), e))?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Errore serializzazione JSON '{}': {}", path.display(), e))?;
    std::fs::write(path, content).map_err(|e| format!("Errore scrittura '{}': {}", path.display(), e))
}

fn compile_constitutional_spec(message: &str, strict_goal_execution: bool) -> serde_json::Value {
    let lower = message.to_ascii_lowercase();
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    let mut constraints = Vec::new();

    if lower.contains("test") {
        include.push("minimal_tests".to_string());
    }
    if lower.contains("file change") || lower.contains("code change") {
        include.push("targeted_file_changes".to_string());
    }
    if lower.contains("todo-app") {
        include.push("workspace_scope:todo-app".to_string());
    }
    if lower.contains("no scaffolding") || lower.contains("niente scaffolding") {
        exclude.push("bootstrap_scaffolding".to_string());
        constraints.push("no_scaffolding".to_string());
    }
    if lower.contains("only the first") || lower.contains("solo il primo") {
        constraints.push("first_pending_goal_only".to_string());
    }
    if strict_goal_execution {
        constraints.push("strict_goal_execution_mode".to_string());
    }

    let mut invariants = vec![
        "deterministic_output".to_string(),
        "explicit_paths".to_string(),
        "fail_safe_behavior".to_string(),
        "contract_preserving".to_string(),
    ];
    if strict_goal_execution {
        invariants.push("minimal_blast_radius".to_string());
    }

    serde_json::json!({
        "version": 1,
        "objective": message,
        "scope": {
            "include": include,
            "exclude": exclude
        },
        "constraints": constraints,
        "invariants": invariants,
        "success_signals": [
            "safe_write_plan_generated_or_skipped",
            "verification_commands_present",
            "explainability_evidence_present"
        ]
    })
}

fn persist_constitutional_spec(
    root: &std::path::Path,
    turn_id: &str,
    spec: &serde_json::Value,
) -> Result<PathBuf, String> {
    let path = innovation_root(root)
        .join("constitutional_spec")
        .join(format!("spec-{}.json", &turn_id[..turn_id.len().min(12)]));
    persist_json(&path, spec)?;
    Ok(path)
}

fn build_counterfactual_plans(
    _strict_goal_execution: bool,
    reliability_healthy: Option<bool>,
    governance_pending: Option<&String>,
) -> serde_json::Value {
    let has_pending_governance = governance_pending.is_some();
    let reliability_ok = reliability_healthy.unwrap_or(false);
    let recommended = if has_pending_governance || !reliability_ok {
        "conservative"
    } else {
        "balanced"
    };

    let plans = vec![
        serde_json::json!({
            "id": "conservative",
            "title": "Conservative patch",
            "strategy": "Edit only strictly required files, preserve behavior, add minimal guard tests.",
            "risk": "low",
            "estimated_files_touched": "1-2",
            "expected_rollback_rate": "<2%"
        }),
        serde_json::json!({
            "id": "balanced",
            "title": "Balanced implementation",
            "strategy": "Implement requested scope + focused refactor where needed for maintainability.",
            "risk": "medium",
            "estimated_files_touched": "2-5",
            "expected_rollback_rate": "2-5%"
        }),
        serde_json::json!({
            "id": "aggressive",
            "title": "Aggressive redesign",
            "strategy": "Broader restructuring for long-term optimization and abstraction cleanup.",
            "risk": "high",
            "estimated_files_touched": "5+",
            "expected_rollback_rate": ">5%"
        }),
    ];

    serde_json::json!({
        "version": 1,
        "recommended_plan_id": recommended,
        "recommended_reason": if recommended == "conservative" {
            "Governance/reliability signals suggest minimizing blast radius."
        } else {
            "Current signals allow incremental delivery with moderate scope."
        },
        "plans": plans
    })
}

fn scale_thresholds(
    base: &reliability::ReliabilityThresholds,
    min_multiplier: f64,
    max_multiplier: f64,
) -> reliability::ReliabilityThresholds {
    reliability::ReliabilityThresholds {
        min_task_success_rate: (base.min_task_success_rate * min_multiplier).clamp(0.0, 1.0),
        min_no_regression_rate: (base.min_no_regression_rate * min_multiplier).clamp(0.0, 1.0),
        max_rollback_rate: (base.max_rollback_rate * max_multiplier).clamp(0.0, 1.0),
        max_invariant_violation_rate: (base.max_invariant_violation_rate * max_multiplier)
            .clamp(0.0, 1.0),
    }
}

fn simulate_policy_modes(
    snapshot: Option<&sentinel_core::ReliabilitySnapshot>,
    base_thresholds: &reliability::ReliabilityThresholds,
) -> serde_json::Value {
    let Some(snapshot) = snapshot else {
        return serde_json::json!({
            "available": false,
            "reason": "reliability_snapshot_unavailable"
        });
    };

    let strict_thresholds = scale_thresholds(base_thresholds, 1.0, 1.0);
    let balanced_thresholds = scale_thresholds(base_thresholds, 0.92, 1.2);
    let aggressive_thresholds = scale_thresholds(base_thresholds, 0.84, 1.45);

    let modes = [
        ("strict", strict_thresholds),
        ("balanced", balanced_thresholds),
        ("aggressive", aggressive_thresholds),
    ]
    .into_iter()
    .map(|(name, thresholds)| {
        let eval = reliability::evaluate_snapshot(snapshot, &thresholds);
        serde_json::json!({
            "mode": name,
            "healthy": eval.healthy,
            "violations": eval.violations,
            "thresholds": thresholds
        })
    })
    .collect::<Vec<_>>();

    serde_json::json!({
        "available": true,
        "snapshot": {
            "task_success_rate": snapshot.task_success_rate,
            "no_regression_rate": snapshot.no_regression_rate,
            "rollback_rate": snapshot.rollback_rate,
            "invariant_violation_rate": snapshot.invariant_violation_rate
        },
        "modes": modes
    })
}

fn derive_workspace_signing_key(root: &std::path::Path, manifold: Option<&GoalManifold>) -> SigningKey {
    let material = format!(
        "{}::{}",
        root.display(),
        manifold
            .map(|m| m.integrity_hash.to_hex())
            .unwrap_or_else(|| "no-manifold".to_string())
    );
    let mut seed = [0u8; 32];
    seed.copy_from_slice(blake3::hash(material.as_bytes()).as_bytes());
    SigningKey::from_bytes(&seed)
}

fn term_overlap(a: &str, b: &str) -> usize {
    let left = normalize_terms(a);
    let right = normalize_terms(b);
    left.intersection(&right).count()
}

fn build_team_memory_graph(
    store: &ChatMemoryStore,
    root: &std::path::Path,
    manifold: Option<&GoalManifold>,
) -> TeamMemoryGraph {
    let recent: Vec<ChatMemoryTurn> = if store.turns.len() > 120 {
        store.turns[store.turns.len() - 120..].to_vec()
    } else {
        store.turns.clone()
    };

    let nodes: Vec<TeamMemoryGraphNode> = recent
        .iter()
        .map(|turn| TeamMemoryGraphNode {
            id: turn.id.clone(),
            timestamp: turn.timestamp,
            intent_summary: turn.intent_summary.clone(),
            provenance_hash: stable_hash_hex(&format!(
                "{}|{}|{}",
                turn.id, turn.user, turn.assistant
            )),
            lamport: turn.timestamp,
        })
        .collect();

    let mut edges: Vec<TeamMemoryGraphEdge> = Vec::new();
    for pair in recent.windows(2) {
        if let [left, right] = pair {
            edges.push(TeamMemoryGraphEdge {
                source: left.id.clone(),
                target: right.id.clone(),
                weight: 1.0,
                relation: "temporal".to_string(),
            });
        }
    }

    let n = recent.len();
    for i in 0..n {
        let max_j = (i + 25).min(n);
        for j in (i + 1)..max_j {
            let overlap = term_overlap(&recent[i].intent_summary, &recent[j].intent_summary);
            if overlap >= 2 {
                edges.push(TeamMemoryGraphEdge {
                    source: recent[i].id.clone(),
                    target: recent[j].id.clone(),
                    weight: (overlap as f64).min(6.0),
                    relation: "semantic_overlap".to_string(),
                });
            }
        }
    }

    if edges.len() > 320 {
        edges.truncate(320);
    }

    let unsigned = serde_json::json!({
        "version": 1,
        "generated_at": chrono::Utc::now().timestamp_millis(),
        "nodes": nodes,
        "edges": edges
    });
    let graph_hash = stable_hash_hex(&unsigned.to_string());
    let signing_key = derive_workspace_signing_key(root, manifold);
    let signature = signing_key.sign(graph_hash.as_bytes());

    TeamMemoryGraph {
        version: 1,
        generated_at: chrono::Utc::now().timestamp_millis(),
        node_count: unsigned["nodes"].as_array().map_or(0, |v| v.len()),
        edge_count: unsigned["edges"].as_array().map_or(0, |v| v.len()),
        nodes: serde_json::from_value(unsigned["nodes"].clone()).unwrap_or_default(),
        edges: serde_json::from_value(unsigned["edges"].clone()).unwrap_or_default(),
        graph_hash,
        signer_public_key: hex::encode(signing_key.verifying_key().to_bytes()),
        signature: hex::encode(signature.to_bytes()),
        signature_scheme: "ed25519-blake3-v1".to_string(),
    }
}

fn persist_team_memory_graph(
    root: &std::path::Path,
    graph: &TeamMemoryGraph,
) -> Result<PathBuf, String> {
    let path = innovation_root(root).join("team_memory_graph.json");
    let value = serde_json::to_value(graph)
        .map_err(|e| format!("Errore serializzazione team memory graph: {}", e))?;
    persist_json(&path, &value)?;
    Ok(path)
}

fn persist_replay_ledger_entry(
    root: &std::path::Path,
    entry: &ReplayLedgerEntry,
) -> Result<PathBuf, String> {
    let dir = innovation_root(root).join("replay_ledger");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Errore creazione replay ledger '{}': {}", dir.display(), e))?;

    let short_id = &entry.turn_id[..entry.turn_id.len().min(12)];
    let path = dir.join(format!("replay-{}-{}.json", entry.timestamp, short_id));
    let entry_json = serde_json::to_value(entry)
        .map_err(|e| format!("Errore serializzazione replay ledger: {}", e))?;
    persist_json(&path, &entry_json)?;

    let index_path = dir.join("index.jsonl");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&index_path)
        .map_err(|e| format!("Errore apertura replay index '{}': {}", index_path.display(), e))?;
    let row = serde_json::to_string(&entry_json)
        .map_err(|e| format!("Errore serializzazione replay row: {}", e))?;
    writeln!(file, "{}", row)
        .map_err(|e| format!("Errore scrittura replay index '{}': {}", index_path.display(), e))?;

    Ok(path)
}

fn normalize_terms(input: &str) -> std::collections::BTreeSet<String> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .map(|term| term.trim().to_lowercase())
        .filter(|term| term.len() >= 3)
        .collect()
}

fn top_memory_context(store: &ChatMemoryStore, message: &str, limit: usize) -> Vec<ChatMemoryTurn> {
    if store.turns.is_empty() {
        return Vec::new();
    }

    let query_terms = normalize_terms(message);
    let mut ranked: Vec<(f64, &ChatMemoryTurn)> = store
        .turns
        .iter()
        .map(|turn| {
            let turn_terms = normalize_terms(&format!("{} {}", turn.user, turn.assistant));
            let overlap = query_terms.intersection(&turn_terms).count() as f64;
            let recency = (turn.timestamp as f64) / 1_000_000_000_000.0;
            (overlap * 10.0 + recency, turn)
        })
        .collect();

    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, turn)| turn.clone())
        .collect()
}

fn stream_chunks(content: &str, chunk_len: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut cursor = 0usize;
    let chars: Vec<char> = content.chars().collect();
    while cursor < chars.len() {
        let mut end = (cursor + chunk_len).min(chars.len());
        if end < chars.len() {
            for idx in (cursor..end).rev() {
                if matches!(chars[idx], '.' | ',' | ';' | ':' | '!' | '?' | '\n' | ' ') {
                    end = idx + 1;
                    break;
                }
            }
        }
        let chunk: String = chars[cursor..end].iter().collect();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        cursor = end.max(cursor + 1);
    }
    chunks
}

fn contains_unresolved_placeholders(text: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        lower == "undefined"
            || lower == "null"
            || lower == "- undefined"
            || lower == "* undefined"
            || lower.ends_with(": undefined")
            || lower.ends_with(": null")
            || lower.contains(" `undefined`")
            || lower.contains(" 'undefined'")
    })
}

fn sanitize_chat_response(raw: &str) -> String {
    let mut in_code_block = false;
    let mut first_pass: Vec<String> = Vec::new();

    for original in raw.lines() {
        let trimmed = original.trim();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            first_pass.push(original.to_string());
            continue;
        }

        if in_code_block {
            first_pass.push(original.to_string());
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        if lower == "undefined"
            || lower == "null"
            || lower == "- undefined"
            || lower == "* undefined"
            || lower.ends_with(": undefined")
            || lower.ends_with(": null")
        {
            continue;
        }

        let cleaned = original
            .replace("`undefined`", "dettaglio mancante")
            .replace("'undefined'", "'dettaglio mancante'")
            .replace("\"undefined\"", "\"dettaglio mancante\"");

        first_pass.push(cleaned);
    }

    let mut second_pass: Vec<String> = Vec::new();
    for (idx, line) in first_pass.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.ends_with(':') {
            let next_non_empty = first_pass
                .iter()
                .skip(idx + 1)
                .find(|candidate| !candidate.trim().is_empty())
                .map(|candidate| candidate.trim().to_string());
            if let Some(next_line) = next_non_empty {
                if next_line.ends_with(':') {
                    continue;
                }
            } else {
                continue;
            }
        }
        second_pass.push(line.clone());
    }

    let compact = second_pass.join("\n");
    let mut collapsed = String::new();
    let mut previous_empty = false;
    for line in compact.lines() {
        let is_empty = line.trim().is_empty();
        if is_empty && previous_empty {
            continue;
        }
        collapsed.push_str(line);
        collapsed.push('\n');
        previous_empty = is_empty;
    }

    collapsed.trim().to_string()
}

fn wants_todo_bootstrap_template(user_message: &str) -> bool {
    let message = user_message.to_ascii_lowercase();
    if !(message.contains("todo") && message.contains("app")) {
        return false;
    }

    let create_intent = [
        "crea", "create", "build", "genera", "generate", "setup", "scaffold", "nuova app",
        "new app", "from scratch", "da zero",
    ]
    .iter()
    .any(|token| message.contains(token));

    let complete_intent = [
        "completa",
        "completo",
        "complete",
        "full",
        "full-stack",
        "end-to-end",
    ]
    .iter()
    .any(|token| message.contains(token));

    create_intent && complete_intent && !is_goal_execution_request(user_message)
}

fn is_goal_execution_request(user_message: &str) -> bool {
    let message = user_message.to_ascii_lowercase();
    let high_confidence_signals = [
        "solo il primo",
        "only the first",
        "niente scaffolding",
        "no scaffolding",
        "file changes",
        "code changes",
        "test minimi",
        "minimal tests",
        "goal pending",
        "first pending goal",
    ];
    if high_confidence_signals
        .iter()
        .any(|token| message.contains(token))
    {
        return true;
    }

    let has_goal = message.contains("goal");
    let has_pending = message.contains("pending");
    let has_next_step = message.contains("prossimo step") || message.contains("next step");
    let has_alignment_status = message.contains("alignment status") || message.contains("allineamento");

    has_goal && (has_pending || has_next_step || has_alignment_status)
}

fn deterministic_template_for_common_requests(user_message: &str) -> Option<String> {
    if wants_todo_bootstrap_template(user_message) {
        return Some(
            "Ecco un setup completo e subito eseguibile per una TODO app (React + Express + PostgreSQL).\n\n\
## 1) Struttura progetto\n\
```bash\n\
mkdir -p todo-app/{client,server,database}\n\
cd todo-app/server && npm init -y\n\
npm i express cors pg express-validator dotenv\n\
cd ../client && npm create vite@latest . -- --template react\n\
npm i\n\
```\n\n\
## 2) Database (`database/init.sql`)\n\
```sql\n\
CREATE TABLE IF NOT EXISTS todos (\n\
  id SERIAL PRIMARY KEY,\n\
  title TEXT NOT NULL,\n\
  completed BOOLEAN NOT NULL DEFAULT FALSE,\n\
  created_at TIMESTAMP NOT NULL DEFAULT NOW()\n\
);\n\
```\n\n\
## 3) Backend (`server/index.js`)\n\
```js\n\
import express from 'express';\n\
import cors from 'cors';\n\
import { Pool } from 'pg';\n\
\n\
const app = express();\n\
app.use(cors());\n\
app.use(express.json());\n\
\n\
const pool = new Pool({ connectionString: process.env.DATABASE_URL });\n\
\n\
app.get('/api/todos', async (_req, res) => {\n\
  const { rows } = await pool.query('SELECT * FROM todos ORDER BY id DESC');\n\
  res.json(rows);\n\
});\n\
\n\
app.post('/api/todos', async (req, res) => {\n\
  const title = String(req.body?.title || '').trim();\n\
  if (!title) return res.status(400).json({ error: 'title required' });\n\
  const { rows } = await pool.query(\n\
    'INSERT INTO todos(title) VALUES($1) RETURNING *',\n\
    [title],\n\
  );\n\
  res.status(201).json(rows[0]);\n\
});\n\
\n\
app.patch('/api/todos/:id/toggle', async (req, res) => {\n\
  const { rows } = await pool.query(\n\
    'UPDATE todos SET completed = NOT completed WHERE id = $1 RETURNING *',\n\
    [req.params.id],\n\
  );\n\
  if (!rows[0]) return res.status(404).json({ error: 'not found' });\n\
  res.json(rows[0]);\n\
});\n\
\n\
app.delete('/api/todos/:id', async (req, res) => {\n\
  await pool.query('DELETE FROM todos WHERE id = $1', [req.params.id]);\n\
  res.status(204).end();\n\
});\n\
\n\
app.listen(3001, () => console.log('API on :3001'));\n\
```\n\n\
## 4) Frontend (`client/src/App.jsx`)\n\
```jsx\n\
import { useEffect, useState } from 'react';\n\
\n\
const API = 'http://localhost:3001/api/todos';\n\
\n\
export default function App() {\n\
  const [todos, setTodos] = useState([]);\n\
  const [title, setTitle] = useState('');\n\
\n\
  const load = async () => setTodos(await (await fetch(API)).json());\n\
  useEffect(() => { load(); }, []);\n\
\n\
  const createTodo = async () => {\n\
    if (!title.trim()) return;\n\
    await fetch(API, {\n\
      method: 'POST',\n\
      headers: { 'Content-Type': 'application/json' },\n\
      body: JSON.stringify({ title }),\n\
    });\n\
    setTitle('');\n\
    load();\n\
  };\n\
\n\
  return (\n\
    <main style={{ maxWidth: 640, margin: '2rem auto', fontFamily: 'sans-serif' }}>\n\
      <h1>Todo List</h1>\n\
      <div style={{ display: 'flex', gap: 8 }}>\n\
        <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder='New task' />\n\
        <button onClick={createTodo}>Add</button>\n\
      </div>\n\
      <ul>\n\
        {todos.map((t) => (\n\
          <li key={t.id} style={{ display: 'flex', gap: 8, marginTop: 8 }}>\n\
            <button onClick={async () => { await fetch(`${API}/${t.id}/toggle`, { method: 'PATCH' }); load(); }}>\n\
              {t.completed ? '✓' : '○'}\n\
            </button>\n\
            <span style={{ textDecoration: t.completed ? 'line-through' : 'none' }}>{t.title}</span>\n\
            <button onClick={async () => { await fetch(`${API}/${t.id}`, { method: 'DELETE' }); load(); }}>Delete</button>\n\
          </li>\n\
        ))}\n\
      </ul>\n\
    </main>\n\
  );\n\
}\n\
```\n\n\
## 5) Run\n\
```bash\n\
# terminale 1\n\
cd server && node index.js\n\
\n\
# terminale 2\n\
cd client && npm run dev\n\
```\n\n\
Se vuoi, nel prossimo step ti preparo anche Docker Compose + auth JWT + test API."
                .to_string(),
        );
    }
    None
}

fn should_force_common_template(user_message: &str, response_text: &str) -> bool {
    // Keep deterministic template as a safety net only when the response is clearly broken.
    wants_todo_bootstrap_template(user_message) && contains_unresolved_placeholders(response_text)
}

fn parse_comparison(value: Option<&str>) -> Comparison {
    match value.unwrap_or("<=") {
        "==" => Comparison::Equal,
        "!=" => Comparison::NotEqual,
        "<" => Comparison::LessThan,
        "<=" => Comparison::LessThanOrEqual,
        ">" => Comparison::GreaterThan,
        ">=" => Comparison::GreaterThanOrEqual,
        _ => Comparison::LessThanOrEqual,
    }
}

fn build_predicates(criteria: &[GoalCriteriaSpec]) -> Vec<Predicate> {
    criteria
        .iter()
        .filter_map(|c| match c.kind.as_str() {
            "file_exists" => c
                .path
                .as_ref()
                .map(|p| Predicate::FileExists(PathBuf::from(p))),
            "directory_exists" => c
                .path
                .as_ref()
                .map(|p| Predicate::DirectoryExists(PathBuf::from(p))),
            "command_succeeds" => c.command.as_ref().map(|cmd| Predicate::CommandSucceeds {
                command: cmd.to_string(),
                args: c.args.clone().unwrap_or_default(),
                expected_exit_code: c.expected_exit_code.unwrap_or(0),
            }),
            "tests_passing" => c.suite.as_ref().map(|suite| Predicate::TestsPassing {
                suite: suite.to_string(),
                min_coverage: c.min_coverage.unwrap_or(0.8),
            }),
            "api_endpoint" => c.url.as_ref().map(|url| Predicate::ApiEndpoint {
                url: url.to_string(),
                expected_status: c.expected_status.unwrap_or(200),
                expected_body_contains: c.expected_body_contains.clone(),
            }),
            "performance" => c.metric.as_ref().map(|metric| Predicate::Performance {
                metric: metric.to_string(),
                threshold: c.threshold.unwrap_or(0.0),
                comparison: parse_comparison(c.comparison.as_deref()),
            }),
            _ => None,
        })
        .collect()
}

fn append_notes(goal: &mut Goal, label: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    let content = format!("{}: {}", label, items.join(", "));
    goal.metadata.notes.push(content);
}

fn build_intent(
    description: String,
    constraints: Vec<String>,
    expected_outcomes: Vec<String>,
    target_platform: Option<String>,
    languages: Vec<String>,
    frameworks: Vec<String>,
) -> sentinel_core::goal_manifold::Intent {
    let mut intent = sentinel_core::goal_manifold::Intent::new(description, constraints);

    for outcome in expected_outcomes {
        intent = intent.with_outcome(outcome);
    }

    if let Some(platform) = target_platform {
        if !platform.trim().is_empty() {
            intent = intent.with_platform(platform);
        }
    }

    for language in languages {
        intent = intent.with_language(language);
    }

    for framework in frameworks {
        intent = intent.with_framework(framework);
    }

    intent
}

fn build_goal_prompt(intent: &sentinel_core::goal_manifold::Intent) -> String {
    let mut context_lines = Vec::new();

    if !intent.constraints.is_empty() {
        context_lines.push(format!("Constraints: {}", intent.constraints.join("; ")));
    }
    if !intent.expected_outcomes.is_empty() {
        context_lines.push(format!(
            "Expected outcomes: {}",
            intent.expected_outcomes.join("; ")
        ));
    }
    if let Some(platform) = &intent.target_platform {
        if !platform.trim().is_empty() {
            context_lines.push(format!("Target platform: {}", platform));
        }
    }
    if !intent.languages.is_empty() {
        context_lines.push(format!("Languages: {}", intent.languages.join(", ")));
    }
    if !intent.frameworks.is_empty() {
        context_lines.push(format!("Frameworks: {}", intent.frameworks.join(", ")));
    }

    let context_block = if context_lines.is_empty() {
        String::new()
    } else {
        format!("\n{}", context_lines.join("\n"))
    };

    format!(
        "Create 3-6 atomic, deterministic software goals for the following project intent:\n\"{}\"{}\nReturn ONLY a JSON array of strings.",
        intent.description, context_block
    )
}

fn extract_goal_suggestions(content: &str) -> Option<Vec<String>> {
    let json_start = content.find('[')?;
    let json_end = content.rfind(']')?;
    let json_str = &content[json_start..=json_end];
    serde_json::from_str::<Vec<String>>(json_str).ok()
}

fn build_system_prompt() -> String {
    "You are an AI coding assistant integrated with Sentinel Protocol. \
Your output must be deterministic, concise, and machine-parseable when requested."
        .to_string()
}

const ORCHESTRATION_ALLOWED_MODES: [&str; 4] = ["plan", "build", "review", "deploy"];

fn normalize_orchestration_mode(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if ORCHESTRATION_ALLOWED_MODES
        .iter()
        .any(|mode| *mode == normalized)
    {
        return Some(normalized);
    }
    None
}

fn parse_orchestration_modes(raw_modes: Option<&Value>) -> Vec<String> {
    let mut modes = raw_modes
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .filter_map(normalize_orchestration_mode)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if modes.is_empty() {
        modes = ORCHESTRATION_ALLOWED_MODES
            .iter()
            .map(|mode| (*mode).to_string())
            .collect();
    }
    modes.sort();
    modes.dedup();
    modes
}

fn parse_orchestration_parallel(raw: Option<u64>) -> usize {
    raw.unwrap_or(2).clamp(1, 4) as usize
}

fn parse_orchestration_subtask_count(raw: Option<u64>) -> usize {
    raw.unwrap_or(4).clamp(2, 6) as usize
}

fn extract_json_array_fragment(content: &str) -> Option<String> {
    let start = content.find('[')?;
    let end = content.rfind(']')?;
    Some(content[start..=end].to_string())
}

fn extract_json_object_fragment(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    Some(content[start..=end].to_string())
}

fn parse_orchestration_plan_from_llm(
    content: &str,
    modes: &[String],
    max_subtasks: usize,
) -> Option<Vec<OrchestrationTask>> {
    let json_fragment = extract_json_array_fragment(content)?;
    let payload: Value = serde_json::from_str(&json_fragment).ok()?;
    let items = payload.as_array()?;
    if items.is_empty() {
        return None;
    }

    let mut tasks = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if tasks.len() >= max_subtasks {
            break;
        }

        let title = if let Some(text) = item.as_str() {
            text.trim().to_string()
        } else if let Some(object) = item.as_object() {
            object
                .get("title")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        if title.is_empty() {
            continue;
        }

        let requested_mode = item
            .as_object()
            .and_then(|object| object.get("mode"))
            .and_then(|value| value.as_str())
            .and_then(normalize_orchestration_mode);

        let mode = requested_mode.unwrap_or_else(|| {
            let fallback = idx % modes.len();
            modes[fallback].clone()
        });

        tasks.push(OrchestrationTask {
            id: format!("task-{:02}", idx + 1),
            index: idx,
            title,
            mode,
        });
    }

    if tasks.is_empty() {
        return None;
    }
    Some(tasks)
}

fn fallback_orchestration_plan(
    root_task: &str,
    modes: &[String],
    subtask_count: usize,
) -> Vec<OrchestrationTask> {
    let mut tasks = Vec::new();
    for idx in 0..subtask_count {
        let mode = modes[idx % modes.len()].clone();
        let title = match mode.as_str() {
            "plan" => format!("Define deterministic scope and constraints for: {}", root_task),
            "build" => format!(
                "Implement minimal contract-preserving changes for: {}",
                root_task
            ),
            "review" => format!("Run regression/policy review for: {}", root_task),
            "deploy" => format!("Prepare rollout and rollback checklist for: {}", root_task),
            _ => format!("Execute {} subtask for: {}", mode, root_task),
        };
        tasks.push(OrchestrationTask {
            id: format!("task-{:02}", idx + 1),
            index: idx,
            title,
            mode,
        });
    }
    tasks
}

async fn execute_orchestration_subtask(
    root_task: String,
    task: OrchestrationTask,
) -> OrchestrationTaskResult {
    let system_prompt = build_system_prompt()
        + "\nYou are a specialized sub-agent in Sentinel Orchestrator."
        + "\nReturn ONLY compact JSON object with keys: summary, risk, approval_needed, next_step."
        + "\nNo markdown and no extra prose.";

    let user_prompt = format!(
        "Root task: {}\nSubtask mode: {}\nSubtask title: {}\n\
Constraints:\n- deterministic and bounded\n- contract preserving by default\n- surface only actionable summary\n\
Output schema JSON:\n{{\"summary\":\"...\",\"risk\":\"low|medium|high\",\"approval_needed\":[\"...\"],\"next_step\":\"...\"}}",
        root_task, task.mode, task.title
    );

    let llm_response = chat_with_llm(&system_prompt, &user_prompt).await;
    let raw = llm_response.unwrap_or_else(|| {
        "{\"summary\":\"LLM unavailable for this subtask.\",\"risk\":\"high\",\"approval_needed\":[\"manual review\"],\"next_step\":\"Retry when model is available.\"}".to_string()
    });

    let parsed = extract_json_object_fragment(&raw)
        .and_then(|json| serde_json::from_str::<Value>(&json).ok());

    let (status, summary, risk, approval_needed, next_step, error) = if let Some(payload) = parsed
    {
        let summary = payload
            .get("summary")
            .and_then(|value| value.as_str())
            .unwrap_or("No summary provided.")
            .trim()
            .to_string();
        let risk = payload
            .get("risk")
            .and_then(|value| value.as_str())
            .unwrap_or("medium")
            .to_ascii_lowercase();
        let approval_needed = payload
            .get("approval_needed")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str())
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let next_step = payload
            .get("next_step")
            .and_then(|value| value.as_str())
            .unwrap_or("Continue with next orchestrated subtask.")
            .trim()
            .to_string();
        (
            "completed".to_string(),
            summary,
            if matches!(risk.as_str(), "low" | "medium" | "high") {
                risk
            } else {
                "medium".to_string()
            },
            approval_needed,
            next_step,
            None,
        )
    } else {
        (
            "failed".to_string(),
            "Unable to parse structured subtask output.".to_string(),
            "high".to_string(),
            vec!["manual validation required".to_string()],
            "Retry subtask with stricter output contract.".to_string(),
            Some("invalid_subtask_output".to_string()),
        )
    };

    let output_hash = stable_hash_hex(&format!(
        "{}|{}|{}|{}|{}",
        task.id, status, summary, risk, next_step
    ));

    OrchestrationTaskResult {
        id: task.id,
        index: task.index,
        title: task.title,
        mode: task.mode,
        status,
        summary,
        risk,
        approval_needed,
        next_step,
        output_hash,
        error,
    }
}

async fn run_orchestrated_task(
    task: String,
    modes: Vec<String>,
    max_parallel: usize,
    subtask_count: usize,
) -> Value {
    let decompose_prompt = format!(
        "Decompose this software task into {} deterministic subtasks.\n\
Allowed modes: {}.\n\
Return ONLY JSON array where each item is {{\"title\":\"...\",\"mode\":\"...\"}}.\n\
Task: {}",
        subtask_count,
        modes.join(", "),
        task
    );
    let decomposition = chat_with_llm(&build_system_prompt(), &decompose_prompt).await;
    let planned_tasks = decomposition
        .as_deref()
        .and_then(|raw| parse_orchestration_plan_from_llm(raw, &modes, subtask_count))
        .unwrap_or_else(|| fallback_orchestration_plan(&task, &modes, subtask_count));

    let planned_count = planned_tasks.len();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel));
    let mut join_set = tokio::task::JoinSet::new();

    for subtask in planned_tasks {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore should be available");
        let root_task = task.clone();
        join_set.spawn(async move {
            let _permit_guard = permit;
            execute_orchestration_subtask(root_task, subtask).await
        });
    }

    let mut results = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(result) => results.push(result),
            Err(err) => {
                results.push(OrchestrationTaskResult {
                    id: format!("task-{:02}", results.len() + 1),
                    index: results.len(),
                    title: "Unresolved subtask".to_string(),
                    mode: "review".to_string(),
                    status: "failed".to_string(),
                    summary: "Subtask panicked or was cancelled.".to_string(),
                    risk: "high".to_string(),
                    approval_needed: vec!["manual review".to_string()],
                    next_step: "Rerun orchestration after inspecting runtime errors.".to_string(),
                    output_hash: stable_hash_hex("subtask_join_error"),
                    error: Some(err.to_string()),
                });
            }
        }
    }

    results.sort_by_key(|result| result.index);
    let failed = results
        .iter()
        .filter(|result| result.status != "completed")
        .count();
    let approval_required = results
        .iter()
        .filter(|result| !result.approval_needed.is_empty())
        .count();
    let completed = results.len().saturating_sub(failed);
    let orchestration_id = uuid::Uuid::new_v4().to_string();

    serde_json::json!({
        "orchestration_id": orchestration_id,
        "task": task,
        "modes": modes,
        "max_parallel": max_parallel,
        "planned_subtasks": planned_count,
        "subtasks": results,
        "summary": {
            "completed": completed,
            "failed": failed,
            "approval_required_subtasks": approval_required,
            "recommended_next": if failed == 0 {
                "Execute approved actions and continue with deployment gates."
            } else {
                "Address failed subtasks first, then rerun orchestration."
            }
        }
    })
}

async fn chat_with_llm(system_prompt: &str, user_prompt: &str) -> Option<String> {
    use sentinel_agent_native::llm_integration::LLMChatClient;
    use sentinel_agent_native::providers::ProviderRouter;

    let router = match ProviderRouter::from_env() {
        Ok(router) => router,
        Err(err) => {
            eprintln!("LLM router unavailable: {}", err);
            return None;
        }
    };
    match router.chat_completion(system_prompt, user_prompt).await {
        Ok(completion) => Some(completion.content),
        Err(err) => {
            eprintln!("LLM completion failed: {}", err);
            None
        }
    }
}

async fn suggest_goals_with_llm(
    intent: &sentinel_core::goal_manifold::Intent,
) -> Option<Vec<String>> {
    let system_prompt = build_system_prompt();
    let prompt = build_goal_prompt(intent);
    let suggestion = chat_with_llm(&system_prompt, &prompt).await?;
    extract_goal_suggestions(suggestion.trim())
}

async fn handle_tool_call(params: Option<Value>) -> Option<Value> {
    let params = params?;
    let name = params.get("name")?.as_str()?;
    let arguments = params.get("arguments");

    match name {
        "init_project" => {
            let args = match arguments.cloned() {
                Some(value) => value,
                None => {
                    return Some(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": "Missing init_project arguments." }]
                    }))
                }
            };

            let request: GoalInitRequest = match serde_json::from_value(args) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return Some(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": format!("Invalid init_project payload: {}", e) }]
                    }))
                }
            };

            let GoalInitRequest {
                description,
                constraints,
                expected_outcomes,
                target_platform,
                languages,
                frameworks,
                goals,
            } = request;

            let description = description.trim().to_string();
            if description.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "Project description cannot be empty." }]
                }));
            }

            let constraints = constraints.unwrap_or_default();
            let expected_outcomes = expected_outcomes.unwrap_or_default();
            let languages = languages.unwrap_or_default();
            let frameworks = frameworks.unwrap_or_default();

            let root_intent = build_intent(
                description.clone(),
                constraints.clone(),
                expected_outcomes.clone(),
                target_platform.clone(),
                languages.clone(),
                frameworks.clone(),
            );

            let mut manifold = sentinel_core::GoalManifold::new(root_intent.clone());
            let mut sub_goals_added = 0;
            let mut used_structured = false;

            if let Some(goal_specs) = goals {
                if !goal_specs.is_empty() {
                    used_structured = true;
                    let mut built_goals = Vec::new();
                    let mut errors = Vec::new();

                    for spec in goal_specs {
                        let criteria = build_predicates(&spec.success_criteria);
                        if criteria.is_empty() {
                            errors.push(format!(
                                "Goal '{}' missing valid success criteria.",
                                spec.description
                            ));
                            continue;
                        }

                        let goal = Goal::builder()
                            .description(spec.description.clone())
                            .success_criteria(criteria)
                            .validation_tests(spec.validation_tests.clone().unwrap_or_default())
                            .build();

                        match goal {
                            Ok(mut g) => {
                                append_notes(
                                    &mut g,
                                    "Scope In",
                                    &spec.scope_in.unwrap_or_default(),
                                );
                                append_notes(
                                    &mut g,
                                    "Scope Out",
                                    &spec.scope_out.unwrap_or_default(),
                                );
                                append_notes(
                                    &mut g,
                                    "Deliverables",
                                    &spec.deliverables.unwrap_or_default(),
                                );
                                append_notes(
                                    &mut g,
                                    "Constraints",
                                    &spec.constraints.unwrap_or_default(),
                                );
                                built_goals.push(g);
                            }
                            Err(e) => {
                                errors.push(format!("Goal '{}' invalid: {}", spec.description, e));
                            }
                        }
                    }

                    if !errors.is_empty() {
                        let message = format!("Goal specification errors:\n{}", errors.join("\n"));
                        return Some(serde_json::json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": message }]
                        }));
                    }

                    for g in built_goals {
                        if manifold.add_goal(g).is_ok() {
                            sub_goals_added += 1;
                        }
                    }
                }
            }

            if sub_goals_added == 0 {
                if used_structured {
                    return Some(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": "No valid goals were provided. Please define at least one goal with success criteria." }]
                    }));
                }

                if let Some(goal_descriptions) = suggest_goals_with_llm(&root_intent).await {
                    for desc in goal_descriptions {
                        if let Ok(g) = sentinel_core::goal_manifold::goal::Goal::builder()
                            .description(desc)
                            .add_success_criterion(
                                sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue,
                            )
                            .build()
                        {
                            if manifold.add_goal(g).is_ok() {
                                sub_goals_added += 1;
                            }
                        }
                    }
                }
            }

            if sub_goals_added == 0 {
                let engine = sentinel_core::architect::ArchitectEngine::new();
                if let Ok(proposal) = engine.propose_architecture(root_intent) {
                    for g in proposal.proposed_goals {
                        if manifold.add_goal(g).is_ok() {
                            sub_goals_added += 1;
                        }
                    }
                    for inv_desc in proposal.proposed_invariants {
                        let _ = manifold.add_invariant(sentinel_core::goal_manifold::Invariant {
                            id: uuid::Uuid::new_v4(),
                            description: inv_desc,
                            severity: sentinel_core::goal_manifold::InvariantSeverity::Critical,
                            predicate:
                                sentinel_core::goal_manifold::predicate::Predicate::AlwaysTrue,
                        });
                    }
                }
            }

            let response_text = {
                match save_manifold(&manifold) {
                    Ok(_) => format!(
                        "PROJECT INITIALIZED SUCCESS: Creati {} obiettivi per '{}'.",
                        manifold.goal_dag.goals().count(),
                        description
                    ),
                    Err(e) => format!("Errore scrittura manifold: {}", e),
                }
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        }

        "suggest_goals" => {
            let args = match arguments.cloned() {
                Some(value) => value,
                None => {
                    return Some(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": "Missing suggest_goals arguments." }]
                    }))
                }
            };

            let request: GoalInitRequest = match serde_json::from_value(args) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return Some(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": format!("Invalid suggest_goals payload: {}", e) }]
                    }))
                }
            };

            let GoalInitRequest {
                description,
                constraints,
                expected_outcomes,
                target_platform,
                languages,
                frameworks,
                ..
            } = request;

            let description = description.trim().to_string();
            if description.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "Project description cannot be empty." }]
                }));
            }

            let intent = build_intent(
                description,
                constraints.unwrap_or_default(),
                expected_outcomes.unwrap_or_default(),
                target_platform,
                languages.unwrap_or_default(),
                frameworks.unwrap_or_default(),
            );

            let mut suggestions = suggest_goals_with_llm(&intent).await.unwrap_or_default();

            if suggestions.is_empty() {
                let engine = sentinel_core::architect::ArchitectEngine::new();
                if let Ok(proposal) = engine.propose_architecture(intent) {
                    suggestions = proposal
                        .proposed_goals
                        .into_iter()
                        .map(|g| g.description)
                        .collect();
                }
            }

            let payload = serde_json::to_string(&suggestions).unwrap_or_else(|_| "[]".to_string());
            Some(serde_json::json!({ "content": [{ "type": "text", "text": payload }] }))
        }

        "validate_action" => {
            let desc = arguments
                .and_then(|a| a.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let result_json = match get_manifold() {
                Ok(manifold) => {
                    let guardrail = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
                    let completion = manifold.completion_percentage();
                    let state = ProjectState::new(PathBuf::from("."));
                    let field = AlignmentField::new(manifold);
                    match field.predict_alignment(&state).await {
                        Ok(res) => {
                            let reliability = reliability::snapshot_from_signals(
                                res.expected_alignment,
                                res.confidence,
                                completion,
                                guardrail.allowed,
                            );
                            let config = find_manifold_path()
                                .map(|path| reliability::load_reliability_config(&path))
                                .unwrap_or_default();
                            let reliability_slo =
                                reliability::evaluate_snapshot(&reliability, &config.thresholds);
                            serde_json::json!({
                                "alignment_score": res.expected_alignment,
                                "deviation_probability": res.deviation_probability,
                                "risk_level": format!("{:?}", res.risk_level()),
                                "approved": res.expected_alignment > 70.0,
                                "rationale": format!("Analisi dell'intento: {}", desc),
                                "reliability": reliability,
                                "reliability_thresholds": config.thresholds,
                                "reliability_slo": reliability_slo
                            })
                        }
                        Err(e) => {
                            serde_json::json!({ "error": format!("Simulazione fallita: {}", e) })
                        }
                    }
                }
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "get_alignment" => {
            let result_json = match get_manifold() {
                Ok(manifold) => {
                    let completion = manifold.completion_percentage();
                    let guardrail = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
                    let state = ProjectState::new(PathBuf::from("."));
                    let field = AlignmentField::new(manifold);
                    match field.compute_alignment(&state).await {
                        Ok(v) => {
                            let reliability = reliability::snapshot_from_signals(
                                v.score,
                                v.confidence,
                                completion,
                                guardrail.allowed,
                            );
                            let config = find_manifold_path()
                                .map(|path| reliability::load_reliability_config(&path))
                                .unwrap_or_default();
                            let reliability_slo =
                                reliability::evaluate_snapshot(&reliability, &config.thresholds);
                            serde_json::json!({
                                "score": v.score,
                                "confidence": v.confidence,
                                "status": if v.score > 70.0 { "OPTIMAL" } else { "DEVIATED" },
                                "violations": [],
                                "reliability": reliability,
                                "reliability_thresholds": config.thresholds,
                                "reliability_slo": reliability_slo
                            })
                        }
                        Err(e) => serde_json::json!({ "error": format!("Errore calcolo: {}", e) }),
                    }
                }
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "get_reliability" => {
            let result_json = match get_manifold() {
                Ok(manifold) => {
                    let completion = manifold.completion_percentage();
                    let guardrail = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
                    let state = ProjectState::new(PathBuf::from("."));
                    let field = AlignmentField::new(manifold);
                    match field.compute_alignment(&state).await {
                        Ok(alignment) => {
                            let reliability = reliability::snapshot_from_signals(
                                alignment.score,
                                alignment.confidence,
                                completion,
                                guardrail.allowed,
                            );
                            let config = find_manifold_path()
                                .map(|path| reliability::load_reliability_config(&path))
                                .unwrap_or_default();
                            let reliability_slo =
                                reliability::evaluate_snapshot(&reliability, &config.thresholds);
                            serde_json::json!({
                                "reliability": reliability,
                                "reliability_thresholds": config.thresholds,
                                "reliability_slo": reliability_slo,
                                "inputs": {
                                    "alignment_score": alignment.score,
                                    "alignment_confidence": alignment.confidence,
                                    "completion_percentage": completion,
                                    "guardrail_allowed": guardrail.allowed
                                }
                            })
                        }
                        Err(e) => serde_json::json!({ "error": format!("Errore calcolo: {}", e) }),
                    }
                }
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "governance_status" => {
            let result_json = match get_manifold() {
                Ok(manifold) => serde_json::json!({
                    "required_dependencies": manifold.governance.required_dependencies,
                    "allowed_dependencies": manifold.governance.allowed_dependencies,
                    "required_frameworks": manifold.governance.required_frameworks,
                    "allowed_frameworks": manifold.governance.allowed_frameworks,
                    "allowed_endpoints": manifold.governance.allowed_endpoints,
                    "allowed_ports": manifold.governance.allowed_ports,
                    "pending_proposal": manifold.governance.pending_proposal,
                    "history_size": manifold.governance.history.len()
                }),
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "get_world_model" => {
            let result_json = match get_manifold() {
                Ok(manifold) => {
                    let root = find_manifold_path()
                        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let observed = sentinel_agent_native::observe_workspace_governance(&root).ok();
                    world_model_snapshot(&manifold, observed.as_ref())
                }
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "governance_approve" => {
            let note = arguments
                .and_then(|a| a.get("note"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
                .filter(|v| !v.trim().is_empty());

            let result_json = match get_manifold() {
                Ok(mut manifold) => {
                    match manifold.approve_pending_governance_proposal(note.clone()) {
                        Ok(id) => match save_manifold(&manifold) {
                            Ok(_) => serde_json::json!({
                                "ok": true,
                                "proposal_id": id,
                                "message": format!("Governance proposal approved: {}", id)
                            }),
                            Err(e) => serde_json::json!({ "ok": false, "error": e }),
                        },
                        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
                    }
                }
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "governance_reject" => {
            let reason = arguments
                .and_then(|a| a.get("reason"))
                .and_then(|v| v.as_str())
                .unwrap_or("Rejected from MCP")
                .to_string();
            let result_json = match get_manifold() {
                Ok(mut manifold) => {
                    match manifold.reject_pending_governance_proposal(Some(reason.clone())) {
                        Ok(id) => match save_manifold(&manifold) {
                            Ok(_) => serde_json::json!({
                                "ok": true,
                                "proposal_id": id,
                                "message": format!("Governance proposal rejected: {}", id)
                            }),
                            Err(e) => serde_json::json!({ "ok": false, "error": e }),
                        },
                        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
                    }
                }
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "governance_seed" => {
            let apply = arguments
                .and_then(|a| a.get("apply"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let lock_required = arguments
                .and_then(|a| a.get("lock_required"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let result_json = match get_manifold() {
                Ok(mut manifold) => {
                    let root = find_manifold_path()
                        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    match sentinel_agent_native::observe_workspace_governance(&root) {
                        Ok(observed) => {
                            let diff = governance_seed_diff(&manifold, &observed);
                            let mut payload = serde_json::json!({
                                "ok": true,
                                "apply": apply,
                                "lock_required": lock_required,
                                "observed": observed,
                                "diff": diff,
                            });
                            if apply {
                                manifold.apply_governance_seed(
                                    observed.dependencies.clone(),
                                    observed.frameworks.clone(),
                                    observed.endpoints.clone(),
                                    observed.ports.clone(),
                                    lock_required,
                                );
                                match save_manifold(&manifold) {
                                    Ok(_) => {
                                        payload["message"] = serde_json::json!(
                                            "Governance baseline seeded and persisted."
                                        );
                                    }
                                    Err(e) => {
                                        payload["ok"] = serde_json::json!(false);
                                        payload["error"] = serde_json::json!(e);
                                    }
                                }
                            }
                            payload
                        }
                        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
                    }
                }
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "get_quality_status" => {
            let root = find_manifold_path()
                .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let result_json = match latest_quality_report(&root) {
                Some(report) => serde_json::json!({
                    "ok": true,
                    "latest": report
                }),
                None => serde_json::json!({
                    "ok": true,
                    "latest": null,
                    "message": "No quality reports available yet."
                }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "list_quality_reports" => {
            let limit = arguments
                .and_then(|a| a.get("limit"))
                .and_then(|v| v.as_u64())
                .map(|v| v.min(100) as usize)
                .unwrap_or(10);
            let root = find_manifold_path()
                .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let reports = list_quality_reports(&root, limit);
            let result_json = serde_json::json!({
                "ok": true,
                "count": reports.len(),
                "reports": reports
            });
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "quality_report" => {
            let report_id = arguments
                .and_then(|a| a.get("report_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if report_id.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "Missing report_id parameter. Usage: quality_report with report_id=<qr_uuid>" }]
                }));
            }

            let root = find_manifold_path()
                .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));

            let dir = root.join(".sentinel").join("quality");

            match std::fs::read_to_string(dir.join(format!("{}.json", report_id))) {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(_) => Some(
                            serde_json::json!({
                                "content": [{ "type": "text", "text": content }]
                            })
                        ),
                        Err(err) => Some(
                            serde_json::json!({
                                "isError": true,
                                "content": [{ "type": "text", "text": format!("Failed to parse report: {}", err) }]
                            })
                        ),
                    }
                }
                Err(err) => Some(
                    serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": format!("Report not found: {}", err) }]
                    })
                ),
            }
        }

        "run_quality_harness" => {
            let root = find_manifold_path()
                .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            let result_json = match run_quality_harness(&root) {
                Ok(payload) => payload,
                Err(err) => serde_json::json!({
                    "ok": false,
                    "error": err
                }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "safe_write" => {
            let content = arguments
                .and_then(|a| a.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let report = SecurityScanner::scan(content);

            let result_json = serde_json::json!({
                "is_safe": report.is_safe,
                "threats": report.threats.iter().map(|t| serde_json::json!({
                    "description": t,
                    "severity": 1,
                    "pattern": "regex_match"
                })).collect::<Vec<_>>(),
                "risk_score": report.risk_score
            });

            Some(serde_json::json!({
                "content": [{ "type": "text", "text": result_json.to_string() }]
            }))
        }

        "propose_strategy" => {
            let goal_desc = arguments
                .and_then(|a| a.get("goal_description"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let kb =
                std::sync::Arc::new(sentinel_core::learning::knowledge_base::KnowledgeBase::new());
            let synthesizer = sentinel_core::learning::strategy::StrategySynthesizer::new(kb);
            use sentinel_core::goal_manifold::goal::Goal;
            use sentinel_core::goal_manifold::predicate::Predicate;

            let result_json = match Goal::builder()
                .description(goal_desc)
                .add_success_criterion(Predicate::AlwaysTrue)
                .build()
            {
                Ok(goal) => match synthesizer.suggest_strategy(&goal).await {
                    Ok(s) => serde_json::json!({
                        "confidence": s.confidence,
                        "patterns": s.recommended_approaches.iter().map(|p| serde_json::json!({
                            "id": p.id.to_string(),
                            "name": p.name,
                            "description": p.description,
                            "success_rate": p.success_rate,
                            "applicable_to_goal_types": vec!["Feature"]
                        })).collect::<Vec<_>>(),
                        "pitfalls": s.pitfalls_to_avoid.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
                    }),
                    Err(_) => {
                        serde_json::json!({ "confidence": 0.0, "patterns": [], "pitfalls": [] })
                    }
                },
                Err(_) => serde_json::json!({ "error": "Failed to build goal" }),
            };

            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "record_handover" => {
            let gid = arguments
                .and_then(|a| a.get("goal_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let content = arguments
                .and_then(|a| a.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let response_text = match get_manifold() {
                Ok(mut manifold) => {
                    let note = sentinel_core::types::HandoverNote {
                        id: uuid::Uuid::new_v4(),
                        agent_id: uuid::Uuid::new_v4(),
                        goal_id: uuid::Uuid::parse_str(gid).unwrap_or(uuid::Uuid::nil()),
                        content: content.to_string(),
                        technical_warnings: vec![],
                        suggested_next_steps: vec![],
                        timestamp: chrono::Utc::now(),
                    };
                    manifold.handover_log.push(note);
                    match save_manifold(&manifold) {
                        Ok(_) => {
                            "COGNITIVE HANDOVER SUCCESS: Nota salvata in sentinel.json.".to_string()
                        }
                        Err(e) => format!("Errore salvataggio: {}", e),
                    }
                }
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        }

        "get_cognitive_map" => {
            let response_text = match get_manifold() {
                Ok(manifold) => {
                    sentinel_core::architect::distiller::CognitiveDistiller::distill(&manifold)
                        .content
                }
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        }

        "get_enforcement_rules" => {
            let response_text = match get_manifold() {
                Ok(manifold) => {
                    let mut rules = vec!["1. Non modificare file bloccati.".to_string()];
                    for inv in &manifold.invariants {
                        rules.push(format!("- [INVARIANTE] {}", inv.description));
                    }
                    format!("SENTINEL ENFORCEMENT RULES:\n{}", rules.join("\n"))
                }
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        }

        "get_goal_graph" => {
            let json_graph = match get_manifold() {
                Ok(manifold) => {
                    let mut nodes = Vec::new();
                    let mut edges = Vec::new();

                    // Root Node
                    nodes.push(serde_json::json!({
                        "id": "root",
                        "type": "input",
                        "data": { "label": manifold.root_intent.description },
                        "position": { "x": 250, "y": 0 }
                    }));

                    let goals: Vec<_> = manifold.goal_dag.goals().collect();
                    for (i, goal) in goals.iter().enumerate() {
                        let y_pos = (i + 1) * 150;
                        let x_pos = 250 + (if i % 2 == 0 { -150 } else { 150 }); // Simple layout

                        nodes.push(serde_json::json!({
                            "id": goal.id.to_string(),
                            "data": {
                                "label": goal.description,
                                "status": format!("{:?}", goal.status)
                            },
                            "position": { "x": x_pos, "y": y_pos }
                        }));

                        // Edge from Root (semplificato) o dependencies
                        if goal.dependencies.is_empty() {
                            edges.push(serde_json::json!({
                                "id": format!("e-root-{}", goal.id),
                                "source": "root",
                                "target": goal.id.to_string(),
                                "animated": true
                            }));
                        } else {
                            for dep in &goal.dependencies {
                                edges.push(serde_json::json!({
                                    "id": format!("e-{}-{}", dep, goal.id),
                                    "source": dep.to_string(),
                                    "target": goal.id.to_string()
                                }));
                            }
                        }
                    }

                    serde_json::json!({ "nodes": nodes, "edges": edges })
                }
                Err(e) => serde_json::json!({ "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": json_graph.to_string() }] }),
            )
        }

        "decompose_goal" => {
            let gid_str = arguments
                .and_then(|a| a.get("goal_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let response_text = match get_manifold() {
                Ok(mut manifold) => {
                    let gid = match uuid::Uuid::parse_str(gid_str) {
                        Ok(id) => id,
                        Err(_) => {
                            return Some(
                                serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "UUID goal non valido." }] }),
                            )
                        }
                    };

                    let goal = match manifold.get_goal(&gid) {
                        Some(g) => g.clone(),
                        None => {
                            return Some(
                                serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "Goal non trovato." }] }),
                            )
                        }
                    };

                    let mut sub_goals = Vec::new();

                    let system_prompt = build_system_prompt();
                    let prompt = format!(
                        "Decompose the following software goal into 3-5 atomic, deterministic sub-tasks: '{}'. \
                        Return ONLY a JSON array of strings representing the sub-task descriptions.",
                        goal.description
                    );

                    if let Some(suggestion) = chat_with_llm(&system_prompt, &prompt).await {
                        if let Some(tasks) = extract_goal_suggestions(&suggestion) {
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

                    if sub_goals.is_empty() {
                        use sentinel_core::goal_manifold::slicer::AtomicSlicer;
                        if let Ok(decomposed) = AtomicSlicer::decompose(&goal) {
                            sub_goals = decomposed;
                        }
                    }

                    let count = sub_goals.len();
                    for sg in sub_goals {
                        let _ = manifold.add_goal(sg);
                    }

                    match save_manifold(&manifold) {
                        Ok(_) => format!(
                            "ATOMIC DECOMPOSITION SUCCESS: Goal '{}' scomposto in {} task.",
                            goal.description, count
                        ),
                        Err(e) => format!("Errore salvataggio manifold: {}", e),
                    }
                }
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        }

        "orchestrate_task" => {
            let task = arguments
                .and_then(|a| a.get("task"))
                .and_then(|v| v.as_str())
                .map(|value| value.trim().to_string())
                .unwrap_or_default();
            if task.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "task è obbligatorio." }]
                }));
            }

            let modes = parse_orchestration_modes(arguments.and_then(|a| a.get("modes")));
            let max_parallel = parse_orchestration_parallel(
                arguments
                    .and_then(|a| a.get("max_parallel"))
                    .and_then(|v| v.as_u64()),
            );
            let subtask_count = parse_orchestration_subtask_count(
                arguments
                    .and_then(|a| a.get("subtask_count"))
                    .and_then(|v| v.as_u64()),
            );

            let payload = run_orchestrated_task(task, modes, max_parallel, subtask_count).await;
            Some(serde_json::json!({
                "content": [{ "type": "text", "text": payload.to_string() }]
            }))
        }

        "chat" => {
            let message = arguments
                .and_then(|a| a.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if message.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "Messaggio vuoto." }]
                }));
            }

            let memory_store = load_chat_memory();
            let strict_goal_execution = is_goal_execution_request(message);
            let mut memory_hits = top_memory_context(&memory_store, message, 5);
            if strict_goal_execution {
                memory_hits.retain(|hit| !wants_todo_bootstrap_template(&hit.user));
            }
            let memory_context = if memory_hits.is_empty() {
                "Nessun precedente rilevante.".to_string()
            } else {
                memory_hits
                    .iter()
                    .map(|hit| {
                        format!(
                            "- [{}] user='{}' | summary='{}'",
                            hit.id, hit.user, hit.intent_summary
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let manifold = get_manifold().ok();
            let reliability_snapshot = if let Some(manifold) = &manifold {
                let completion = manifold.completion_percentage();
                let guardrail = sentinel_core::guardrail::GuardrailEngine::evaluate(manifold);
                let state = ProjectState::new(PathBuf::from("."));
                let field = AlignmentField::new(manifold.clone());
                match field.compute_alignment(&state).await {
                    Ok(alignment) => {
                        let reliability = reliability::snapshot_from_signals(
                            alignment.score,
                            alignment.confidence,
                            completion,
                            guardrail.allowed,
                        );
                        Some((alignment, reliability, guardrail.allowed))
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            let mut system_prompt = build_system_prompt()
                + "\nSei un agente Sentinel. Il tuo compito è aiutare l'utente con l'allineamento degli obiettivi. \
                Usa un tono professionale, tecnico e deterministico. Rispondi in italiano."
                + "\nRegole output: prima risposta concreta, poi razionale sintetico. Evita prolissità."
                + "\nSe trovi rischio governance/reliability, esplicitalo chiaramente.";
            if strict_goal_execution {
                system_prompt.push_str(
                    "\nModalità esecuzione goal: se l'utente richiede solo il goal pending, \
                    NON proporre scaffolding iniziale o setup completo da zero. \
                    Produci solo file changes mirati + test minimi nel path richiesto.",
                );
            }

            let user_prompt = if strict_goal_execution {
                format!(
                    "Messaggio utente:\n{}\n\nMemoria rilevante:\n{}\n\n\
Rispondi con output operativo in questo formato:\n\
1) File changes (solo file da modificare/creare, niente scaffolding generale)\n\
2) Test minimi\n\
3) Comandi di verifica strettamente necessari\n\
Vincoli: evita testo generico, evita setup da zero, evita ripetere template TODO completo.\n\
Se l'utente indica una directory progetto (es. todo-app), usa path espliciti sotto quella directory.",
                    message, memory_context
                )
            } else {
                format!(
                    "Messaggio utente:\n{}\n\nMemoria rilevante:\n{}\n\nRispondi con priorità a: cosa fare adesso, come farlo in sicurezza, quali rischi monitorare.",
                    message, memory_context
                )
            };

            let llm_response = match chat_with_llm(&system_prompt, &user_prompt).await {
                Some(content) => content.trim().to_string(),
                None => "Errore durante l'inferenza dell'agente. Verifica le API key.".to_string(),
            };
            let mut response_text = sanitize_chat_response(&llm_response);

            if contains_unresolved_placeholders(&response_text) {
                let repair_prompt = format!(
                    "Messaggio utente:\n{}\n\nRisposta bozza da correggere:\n{}\n\n\
Correggi la risposta eliminando placeholder incompleti (es. undefined/null).\n\
Regole obbligatorie:\n\
- nessuna riga con 'undefined' o 'null' come valore\n\
- ogni sezione deve essere concreta e direttamente eseguibile\n\
- se un dettaglio non è certo, ometti la sezione invece di usare placeholder\n\
- mantieni risposta tecnica e concisa in italiano.",
                    message, response_text
                );

                if let Some(repaired) = chat_with_llm(&system_prompt, &repair_prompt).await {
                    response_text = sanitize_chat_response(repaired.trim());
                }
            }

            if should_force_common_template(message, &response_text) {
                if let Some(template) = deterministic_template_for_common_requests(message) {
                    response_text = template;
                }
            }

            if response_text.trim().is_empty() || contains_unresolved_placeholders(&response_text) {
                if let Some(template) = deterministic_template_for_common_requests(message) {
                    response_text = template;
                } else {
                    response_text = "Non posso rispondere con placeholder incompleti. \
Ti propongo un piano concreto in 3 step: \
1) definisco stack e struttura progetto, \
2) genero backend/frontend minimi funzionanti, \
3) aggiungo test e verifica build."
                        .to_string();
                }
            }
            let stream_chunks = stream_chunks(&response_text, 140);
            let reliability_config = find_manifold_path()
                .map(|path| reliability::load_reliability_config(&path))
                .unwrap_or_default();

            let (alignment_score, reliability_ok, risk_flag) =
                if let Some((alignment, reliability, _)) = reliability_snapshot.as_ref() {
                    let slo =
                        reliability::evaluate_snapshot(reliability, &reliability_config.thresholds);
                    (
                        Some(alignment.score),
                        Some(slo.healthy),
                        if slo.healthy { "low" } else { "elevated" }.to_string(),
                    )
                } else {
                    (None, None, "unknown".to_string())
                };

            let governance_pending = manifold.as_ref().and_then(|m| {
                m.governance
                    .pending_proposal
                    .as_ref()
                    .map(|p| p.id.to_string())
            });
            let intent_summary = message
                .split_whitespace()
                .take(14)
                .collect::<Vec<_>>()
                .join(" ");
            let evidence = vec![
                format!("memory_hits={}", memory_hits.len()),
                format!("risk_flag={}", risk_flag),
                governance_pending
                    .as_ref()
                    .map(|id| format!("governance_pending={}", id))
                    .unwrap_or_else(|| "governance_pending=none".to_string()),
            ];

            let turn = ChatMemoryTurn {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now().timestamp_millis(),
                user: message.to_string(),
                assistant: response_text.clone(),
                intent_summary: intent_summary.clone(),
                evidence: evidence.clone(),
            };
            let mut next_store = memory_store;
            next_store.version = 1;
            next_store.turns.push(turn.clone());
            if next_store.turns.len() > 400 {
                let keep_from = next_store.turns.len().saturating_sub(400);
                next_store.turns = next_store.turns.split_off(keep_from);
            }
            let _ = save_chat_memory(&next_store);

            let root = workspace_root();
            let constitutional_spec = compile_constitutional_spec(message, strict_goal_execution);
            let constitutional_spec_hash = stable_hash_hex(&constitutional_spec.to_string());
            let constitutional_spec_path = persist_constitutional_spec(&root, &turn.id, &constitutional_spec)
                .ok()
                .map(|path| path.to_string_lossy().to_string());

            let counterfactual_plans = build_counterfactual_plans(
                strict_goal_execution,
                reliability_ok,
                governance_pending.as_ref(),
            );
            let counterfactual_hash = stable_hash_hex(&counterfactual_plans.to_string());

            let policy_simulation = simulate_policy_modes(
                reliability_snapshot.as_ref().map(|(_, snapshot, _)| snapshot),
                &reliability_config.thresholds,
            );
            let policy_simulation_hash = stable_hash_hex(&policy_simulation.to_string());

            let team_memory_graph = build_team_memory_graph(&next_store, &root, manifold.as_ref());
            let team_memory_graph_hash = team_memory_graph.graph_hash.clone();
            let team_memory_graph_path = persist_team_memory_graph(&root, &team_memory_graph)
                .ok()
                .map(|path| path.to_string_lossy().to_string());

            let replay_entry = ReplayLedgerEntry {
                version: 1,
                turn_id: turn.id.clone(),
                timestamp: turn.timestamp,
                user_message: message.to_string(),
                response_hash: stable_hash_hex(&response_text),
                strict_goal_execution,
                memory_hit_ids: memory_hits.iter().map(|hit| hit.id.clone()).collect(),
                evidence_hash: stable_hash_hex(&evidence.join("|")),
                constitutional_spec_hash: constitutional_spec_hash.clone(),
                counterfactual_hash: counterfactual_hash.clone(),
                policy_simulation_hash: policy_simulation_hash.clone(),
            };
            let replay_ledger_path = persist_replay_ledger_entry(&root, &replay_entry)
                .ok()
                .map(|path| path.to_string_lossy().to_string());

            let innovation_payload = serde_json::json!({
                "version": 1,
                "constitutional_spec": constitutional_spec,
                "constitutional_spec_hash": constitutional_spec_hash,
                "constitutional_spec_path": constitutional_spec_path,
                "counterfactual_plans": counterfactual_plans,
                "counterfactual_hash": counterfactual_hash,
                "policy_simulation": policy_simulation,
                "policy_simulation_hash": policy_simulation_hash,
                "team_memory_graph": team_memory_graph,
                "team_memory_graph_hash": team_memory_graph_hash,
                "team_memory_graph_path": team_memory_graph_path,
                "replay_ledger": {
                    "entry": replay_entry,
                    "path": replay_ledger_path
                }
            });

            let payload = serde_json::json!({
                "answer": response_text,
                "stream_chunks": stream_chunks,
                "thought_chain": [
                    format!("Intent detected: {}", intent_summary),
                    format!("Memory context hits: {}", memory_hits.len()),
                    format!("Risk assessment: {}", risk_flag),
                ],
                "explainability": {
                    "intent_summary": intent_summary,
                    "evidence": evidence,
                    "alignment_score": alignment_score,
                    "reliability_healthy": reliability_ok,
                    "governance_pending_proposal": governance_pending,
                },
                "memory": {
                    "turn_id": turn.id,
                    "total_turns": next_store.turns.len(),
                    "memory_hits": memory_hits.iter().map(|hit| {
                        serde_json::json!({
                            "id": hit.id,
                            "timestamp": hit.timestamp,
                            "intent_summary": hit.intent_summary
                        })
                    }).collect::<Vec<_>>()
                },
                "innovation": innovation_payload
            });

            Some(serde_json::json!({
                "content": [{ "type": "text", "text": payload.to_string() }]
            }))
        }

        "chat_memory_clear" => {
            let result_json = match save_chat_memory(&ChatMemoryStore {
                version: 1,
                turns: Vec::new(),
            }) {
                Ok(_) => serde_json::json!({ "ok": true, "message": "Chat memory cleared." }),
                Err(e) => serde_json::json!({ "ok": false, "error": e }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "chat_memory_status" => {
            let store = load_chat_memory();
            let mut recent = store.turns.clone();
            recent.reverse();
            recent.truncate(10);
            let result_json = serde_json::json!({
                "ok": true,
                "version": store.version,
                "turn_count": store.turns.len(),
                "recent_turns": recent.iter().map(|turn| {
                    serde_json::json!({
                        "id": turn.id,
                        "timestamp": turn.timestamp,
                        "intent_summary": turn.intent_summary,
                        "evidence": turn.evidence,
                    })
                }).collect::<Vec<_>>()
            });
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "chat_memory_search" => {
            let query = arguments
                .and_then(|a| a.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let limit = arguments
                .and_then(|a| a.get("limit"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(10)
                .clamp(1, 50);
            if query.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "query è obbligatoria." }]
                }));
            }
            let store = load_chat_memory();
            let hits = top_memory_context(&store, &query, limit);
            let result_json = serde_json::json!({
                "ok": true,
                "query": query,
                "count": hits.len(),
                "hits": hits.iter().map(|hit| {
                    serde_json::json!({
                        "id": hit.id,
                        "timestamp": hit.timestamp,
                        "user": hit.user,
                        "assistant": hit.assistant,
                        "intent_summary": hit.intent_summary,
                        "evidence": hit.evidence
                    })
                }).collect::<Vec<_>>()
            });
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "chat_memory_export" => {
            let export_path = arguments
                .and_then(|a| a.get("path"))
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .or_else(default_chat_memory_export_path);
            let result_json = match export_path {
                Some(path) => {
                    let store = load_chat_memory();
                    match save_chat_memory_to_path(&path, &store) {
                        Ok(_) => serde_json::json!({
                            "ok": true,
                            "path": path.display().to_string(),
                            "turn_count": store.turns.len()
                        }),
                        Err(e) => serde_json::json!({ "ok": false, "error": e }),
                    }
                }
                None => {
                    serde_json::json!({ "ok": false, "error": "Impossibile risolvere path export." })
                }
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "chat_memory_import" => {
            let import_path = arguments
                .and_then(|a| a.get("path"))
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            let merge = arguments
                .and_then(|a| a.get("merge"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let result_json = match import_path {
                Some(path) => match load_chat_memory_from_path(&path) {
                    Ok(imported) => {
                        let current = load_chat_memory();
                        let mut next = if merge {
                            let mut turns = current.turns.clone();
                            turns.extend(imported.turns.clone());
                            turns.sort_by_key(|turn| turn.timestamp);
                            turns.dedup_by(|a, b| a.id == b.id);
                            ChatMemoryStore { version: 1, turns }
                        } else {
                            ChatMemoryStore {
                                version: 1,
                                turns: imported.turns.clone(),
                            }
                        };
                        if next.turns.len() > 1000 {
                            let keep_from = next.turns.len().saturating_sub(1000);
                            next.turns = next.turns.split_off(keep_from);
                        }
                        match save_chat_memory(&next) {
                            Ok(_) => serde_json::json!({
                                "ok": true,
                                "merge": merge,
                                "path": path.display().to_string(),
                                "turn_count": next.turns.len()
                            }),
                            Err(e) => serde_json::json!({ "ok": false, "error": e }),
                        }
                    }
                    Err(e) => serde_json::json!({ "ok": false, "error": e }),
                },
                None => serde_json::json!({ "ok": false, "error": "path è obbligatorio." }),
            };
            Some(
                serde_json::json!({ "content": [{ "type": "text", "text": result_json.to_string() }] }),
            )
        }

        "agent_communication_send" => {
            let from_agent = arguments.and_then(|a| a.get("from_agent")).and_then(|v| v.as_str()).unwrap_or("");
            let to_agent = arguments.and_then(|a| a.get("to_agent")).and_then(|v| v.as_str());
            let message_type = arguments.and_then(|a| a.get("message_type")).and_then(|v| v.as_str()).unwrap_or("direct");
            let payload = arguments.and_then(|a| a.get("payload")).cloned().unwrap_or(serde_json::json!({}));

            if from_agent.is_empty() {
                return Some(serde_json::json!({
                    "isError": true,
                    "content": [{ "type": "text", "text": "from_agent is required" }]
                }));
            }

            let result = serde_json::json!({
                "success": true,
                "from": from_agent,
                "to": to_agent,
                "type": message_type,
                "payload": payload,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "message": format!("Message sent from {} to {:?}", from_agent, to_agent)
            });

            Some(serde_json::json!({ "content": [{ "type": "text", "text": result.to_string() }] }))
        }

        "agent_communication_status" => {
            let agents = vec![
                serde_json::json!({
                    "id": "architect-001",
                    "name": "MasterArchitect",
                    "status": "active",
                    "capabilities": ["IntegrationExpert", "ApiExpert", "FrontendExpert"],
                    "current_task": "Orchestrating workflow"
                }),
                serde_json::json!({
                    "id": "auth-001",
                    "name": "AuthWorker",
                    "status": "idle",
                    "capabilities": ["AuthExpert", "CodeReviewer"],
                    "current_task": null
                }),
                serde_json::json!({
                    "id": "api-001",
                    "name": "ApiWorker",
                    "status": "busy",
                    "capabilities": ["ApiExpert", "DatabaseExpert"],
                    "current_task": "Implementing TaskAPI endpoints"
                }),
                serde_json::json!({
                    "id": "ui-001",
                    "name": "UiWorker",
                    "status": "busy",
                    "capabilities": ["FrontendExpert", "TestExpert"],
                    "current_task": "Building TaskBoard components"
                })
            ];

            let result = serde_json::json!({
                "success": true,
                "agent_count": agents.len(),
                "agents": agents
            });

            Some(serde_json::json!({ "content": [{ "type": "text", "text": result.to_string() }] }))
        }

        "agent_communication_history" => {
            let limit = arguments.and_then(|a| a.get("limit")).and_then(|v| v.as_u64()).unwrap_or(50) as usize;
            
            // Mock message history
            let messages: Vec<serde_json::Value> = vec![
                serde_json::json!({
                    "id": "msg-001",
                    "from": "AuthWorker",
                    "to": "ApiWorker",
                    "type": "help_request",
                    "content": "How should I expose auth middleware?",
                    "timestamp": "2026-02-14T10:30:00Z"
                }),
                serde_json::json!({
                    "id": "msg-002",
                    "from": "ApiWorker",
                    "to": null,
                    "type": "pattern_share",
                    "content": "Axum Auth Middleware Pattern",
                    "timestamp": "2026-02-14T10:31:00Z"
                }),
                serde_json::json!({
                    "id": "msg-003",
                    "from": "AuthWorker",
                    "to": "ApiWorker",
                    "type": "handoff",
                    "content": "Auth module complete. Context transferred.",
                    "timestamp": "2026-02-14T10:45:00Z"
                })
            ];

            let result = serde_json::json!({
                "success": true,
                "count": messages.len().min(limit),
                "messages": messages.into_iter().take(limit).collect::<Vec<_>>()
            });

            Some(serde_json::json!({ "content": [{ "type": "text", "text": result.to_string() }] }))
        }

        _ => Some(
            serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "Tool non supportato." }] }),
        ),
    }
}

fn governance_seed_diff(
    manifold: &GoalManifold,
    observed: &sentinel_agent_native::GovernanceObservation,
) -> serde_json::Value {
    let current_deps = btree_set(&manifold.governance.allowed_dependencies);
    let next_deps = btree_set(&observed.dependencies);
    let current_frameworks = btree_set(&manifold.governance.allowed_frameworks);
    let next_frameworks = btree_set(&observed.frameworks);
    let current_endpoints: std::collections::BTreeSet<String> = manifold
        .governance
        .allowed_endpoints
        .values()
        .cloned()
        .collect();
    let next_endpoints = btree_set(&observed.endpoints);
    let current_ports = btree_set(&manifold.governance.allowed_ports);
    let next_ports = btree_set(&observed.ports);
    let required_dependencies = btree_set(&manifold.governance.required_dependencies);
    let required_frameworks = btree_set(&manifold.governance.required_frameworks);

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
        },
        "required": {
            "missing_dependencies": required_dependencies.difference(&next_deps).cloned().collect::<Vec<_>>(),
            "missing_frameworks": required_frameworks.difference(&next_frameworks).cloned().collect::<Vec<_>>()
        }
    })
}

fn quality_reports_dir(root: &std::path::Path) -> PathBuf {
    root.join(".sentinel").join("quality")
}

fn list_quality_reports(root: &std::path::Path, limit: usize) -> Vec<serde_json::Value> {
    let dir = quality_reports_dir(root);
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("harness-") && name.ends_with(".json"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return Vec::new(),
    };
    paths.sort_by(|a, b| b.cmp(a));
    paths
        .into_iter()
        .take(limit)
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            let mut payload: serde_json::Value = serde_json::from_str(&content).ok()?;
            payload["path"] = serde_json::json!(path.display().to_string());
            Some(payload)
        })
        .collect()
}

fn latest_quality_report(root: &std::path::Path) -> Option<serde_json::Value> {
    list_quality_reports(root, 1).into_iter().next()
}

fn run_quality_harness(root: &std::path::Path) -> Result<serde_json::Value, String> {
    let script = root.join("scripts").join("world_class_harness.sh");
    if !script.exists() {
        return Err(format!(
            "Harness script not found at '{}'",
            script.display()
        ));
    }

    let output = Command::new("bash")
        .arg(&script)
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to run harness: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let latest = latest_quality_report(root);
    let ok = output.status.success();
    Ok(serde_json::json!({
        "ok": ok,
        "status_code": output.status.code(),
        "stdout_tail": tail_lines(&stdout, 20),
        "stderr_tail": tail_lines(&stderr, 20),
        "latest": latest,
    }))
}

fn tail_lines(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= max_lines {
        return text.to_string();
    }
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

fn world_model_snapshot(
    manifold: &GoalManifold,
    observed: Option<&sentinel_agent_native::GovernanceObservation>,
) -> serde_json::Value {
    let contract = serde_json::json!({
        "required_dependencies": manifold.governance.required_dependencies.clone(),
        "allowed_dependencies": manifold.governance.allowed_dependencies.clone(),
        "required_frameworks": manifold.governance.required_frameworks.clone(),
        "allowed_frameworks": manifold.governance.allowed_frameworks.clone(),
        "allowed_endpoints": manifold.governance.allowed_endpoints.clone(),
        "allowed_ports": manifold.governance.allowed_ports.clone()
    });
    let observed_json = observed.map(|value| {
        serde_json::json!({
            "dependencies": value.dependencies,
            "frameworks": value.frameworks,
            "endpoints": value.endpoints,
            "ports": value.ports,
        })
    });
    let drift = observed.map(|value| governance_seed_diff(manifold, value));
    let expected_missing_required = observed.map(|value| {
        let deps = btree_set(&value.dependencies);
        let frameworks = btree_set(&value.frameworks);
        let required_deps = btree_set(&manifold.governance.required_dependencies);
        let required_frameworks = btree_set(&manifold.governance.required_frameworks);
        serde_json::json!({
            "dependencies": required_deps.difference(&deps).cloned().collect::<Vec<_>>(),
            "frameworks": required_frameworks.difference(&frameworks).cloned().collect::<Vec<_>>(),
        })
    });

    serde_json::json!({
        "where_we_are": observed_json,
        "where_we_must_go": contract,
        "how_enforced": {
            "pending_proposal": manifold.governance.pending_proposal.clone(),
            "history_size": manifold.governance.history.len(),
            "manifold_version": manifold.current_version(),
            "manifold_integrity_hash": manifold.integrity_hash.to_hex()
        },
        "why": "Keep runtime deterministic by enforcing explicit governance contract for dependencies/frameworks/endpoints/ports.",
        "deterministic_drift": drift,
        "required_missing_now": expected_missing_required
    })
}

fn btree_set<T: Clone + Ord>(values: &[T]) -> std::collections::BTreeSet<T> {
    values.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_root(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}", prefix, unique))
    }

    #[test]
    fn stream_chunks_preserves_content() {
        let text = "Sentinel generates deterministic output with explainability blocks.";
        let chunks = stream_chunks(text, 12);
        let rebuilt = chunks.join("");
        assert_eq!(rebuilt, text);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn top_memory_context_prioritizes_overlap() {
        let store = ChatMemoryStore {
            version: 1,
            turns: vec![
                ChatMemoryTurn {
                    id: "a".to_string(),
                    timestamp: 1000,
                    user: "implement auth middleware".to_string(),
                    assistant: "added middleware".to_string(),
                    intent_summary: "auth middleware".to_string(),
                    evidence: vec![],
                },
                ChatMemoryTurn {
                    id: "b".to_string(),
                    timestamp: 2000,
                    user: "improve css theme".to_string(),
                    assistant: "new colors".to_string(),
                    intent_summary: "theme".to_string(),
                    evidence: vec![],
                },
            ],
        };
        let hits = top_memory_context(&store, "auth token middleware", 1);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
    }

    #[test]
    fn constitutional_spec_includes_strict_constraints() {
        let spec = compile_constitutional_spec(
            "Implementa solo il primo goal pending in todo-app. no scaffolding, test minimi",
            true,
        );
        let constraints = spec
            .get("constraints")
            .and_then(|v| v.as_array())
            .expect("constraints should be present");

        let contains = |needle: &str| constraints.iter().any(|v| v.as_str() == Some(needle));
        assert!(contains("first_pending_goal_only"));
        assert!(contains("no_scaffolding"));
        assert!(contains("strict_goal_execution_mode"));
    }

    #[test]
    fn counterfactual_plans_choose_conservative_when_signals_are_risky() {
        let governance_pending = "proposal-1".to_string();
        let plans = build_counterfactual_plans(true, Some(false), Some(&governance_pending));
        assert_eq!(
            plans.get("recommended_plan_id").and_then(|v| v.as_str()),
            Some("conservative")
        );
    }

    #[test]
    fn simulate_policy_modes_reports_available_modes_with_snapshot() {
        let snapshot = sentinel_core::ReliabilitySnapshot {
            task_success_rate: 0.94,
            no_regression_rate: 0.93,
            rollback_rate: 0.04,
            avg_time_to_recover_ms: 180.0,
            invariant_violation_rate: 0.01,
        };
        let thresholds = reliability::ReliabilityThresholds::default();
        let simulation = simulate_policy_modes(Some(&snapshot), &thresholds);
        assert_eq!(simulation.get("available").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            simulation
                .get("modes")
                .and_then(|v| v.as_array())
                .map(std::vec::Vec::len),
            Some(3)
        );
    }

    #[test]
    fn orchestration_modes_are_normalized_and_deduplicated() {
        let raw = serde_json::json!(["PLAN", "build", "review", "build", "invalid"]);
        let parsed = parse_orchestration_modes(Some(&raw));
        assert_eq!(
            parsed,
            vec![
                "build".to_string(),
                "plan".to_string(),
                "review".to_string()
            ]
        );

        let default_modes = parse_orchestration_modes(None);
        assert_eq!(default_modes.len(), ORCHESTRATION_ALLOWED_MODES.len());
        for mode in ORCHESTRATION_ALLOWED_MODES {
            assert!(default_modes.contains(&mode.to_string()));
        }
    }

    #[test]
    fn orchestration_limits_are_bounded() {
        assert_eq!(parse_orchestration_parallel(None), 2);
        assert_eq!(parse_orchestration_parallel(Some(0)), 1);
        assert_eq!(parse_orchestration_parallel(Some(9)), 4);

        assert_eq!(parse_orchestration_subtask_count(None), 4);
        assert_eq!(parse_orchestration_subtask_count(Some(1)), 2);
        assert_eq!(parse_orchestration_subtask_count(Some(10)), 6);
    }

    #[test]
    fn orchestration_plan_parser_handles_structured_and_string_items() {
        let content = r#"
            [
              {"title":"Define acceptance criteria","mode":"plan"},
              {"title":"Implement minimal diff","mode":"invalid_mode"},
              "Run focused regression checks"
            ]
        "#;
        let modes = vec!["plan".to_string(), "build".to_string(), "review".to_string()];
        let tasks = parse_orchestration_plan_from_llm(content, &modes, 3)
            .expect("plan should be parsed");

        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].title, "Define acceptance criteria");
        assert_eq!(tasks[0].mode, "plan");
        assert_eq!(tasks[1].mode, "build");
        assert_eq!(tasks[2].mode, "review");
    }

    #[test]
    fn fallback_orchestration_plan_respects_requested_count() {
        let modes = vec!["plan".to_string(), "build".to_string()];
        let tasks = fallback_orchestration_plan("Improve runtime safety", &modes, 5);
        assert_eq!(tasks.len(), 5);
        assert_eq!(tasks[0].mode, "plan");
        assert_eq!(tasks[1].mode, "build");
        assert_eq!(tasks[2].mode, "plan");
        assert!(
            tasks
                .iter()
                .all(|task| task.title.contains("Improve runtime safety"))
        );
    }

    #[test]
    fn team_memory_graph_is_signed_and_bounded() {
        let root = unique_temp_root("sentinel_team_graph_test");
        std::fs::create_dir_all(&root).expect("temp dir should be created");
        let turns = (0..130)
            .map(|idx| ChatMemoryTurn {
                id: format!("turn-{}", idx),
                timestamp: idx as i64,
                user: format!("implement auth middleware {}", idx),
                assistant: "done".to_string(),
                intent_summary: "auth middleware token".to_string(),
                evidence: vec![],
            })
            .collect::<Vec<_>>();

        let store = ChatMemoryStore { version: 1, turns };
        let graph = build_team_memory_graph(&store, &root, None);

        assert_eq!(graph.signature_scheme, "ed25519-blake3-v1");
        assert_eq!(graph.node_count, 120);
        assert!(graph.edge_count <= 320);
        assert_eq!(graph.signer_public_key.len(), 64);
        assert_eq!(graph.signature.len(), 128);

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn replay_ledger_entry_is_persisted_with_index() {
        let root = unique_temp_root("sentinel_replay_ledger_test");
        std::fs::create_dir_all(&root).expect("temp dir should be created");
        let entry = ReplayLedgerEntry {
            version: 1,
            turn_id: "turn-1".to_string(),
            timestamp: 1_700_000_000_000,
            user_message: "implement only first pending goal".to_string(),
            response_hash: stable_hash_hex("response"),
            strict_goal_execution: true,
            memory_hit_ids: vec!["a".to_string(), "b".to_string()],
            evidence_hash: stable_hash_hex("evidence"),
            constitutional_spec_hash: stable_hash_hex("spec"),
            counterfactual_hash: stable_hash_hex("counterfactual"),
            policy_simulation_hash: stable_hash_hex("policy"),
        };

        let path =
            persist_replay_ledger_entry(&root, &entry).expect("replay ledger should persist");
        assert!(path.exists());

        let index_path = root
            .join(".sentinel")
            .join("innovation")
            .join("replay_ledger")
            .join("index.jsonl");
        let index_content =
            std::fs::read_to_string(index_path).expect("index file should be readable");
        assert!(index_content.contains("\"turn_id\":\"turn-1\""));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn list_quality_reports_returns_latest_first() {
        let root = unique_temp_root("sentinel_quality_test");
        let dir = root.join(".sentinel").join("quality");
        std::fs::create_dir_all(&dir).expect("should create quality dir");

        let older = dir.join("harness-20250101T000000Z.json");
        let newer = dir.join("harness-20250102T000000Z.json");
        std::fs::write(
            &older,
            r#"{"run_id":"20250101T000000Z","overall_ok":true,"kpi":{"total_tests":10}}"#,
        )
        .expect("should write old report");
        std::fs::write(
            &newer,
            r#"{"run_id":"20250102T000000Z","overall_ok":false,"kpi":{"total_tests":12}}"#,
        )
        .expect("should write new report");

        let reports = list_quality_reports(&root, 10);
        assert_eq!(reports.len(), 2);
        assert_eq!(
            reports[0].get("run_id").and_then(|v| v.as_str()),
            Some("20250102T000000Z")
        );
        assert_eq!(
            reports[1].get("run_id").and_then(|v| v.as_str()),
            Some("20250101T000000Z")
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn tail_lines_limits_output() {
        let text = "a\nb\nc\nd";
        assert_eq!(tail_lines(text, 2), "c\nd");
        assert_eq!(tail_lines(text, 10), text);
    }

    #[test]
    fn placeholder_detection_works() {
        assert!(contains_unresolved_placeholders("Configura DB:\nundefined"));
        assert!(contains_unresolved_placeholders("x: null"));
        assert!(!contains_unresolved_placeholders("Setup completo pronto."));
    }

    #[test]
    fn sanitize_chat_response_removes_placeholder_lines() {
        let raw = "Setup progetto:\nundefined\nBackend:\n- endpoint /todos\n";
        let cleaned = sanitize_chat_response(raw);
        assert!(!cleaned.to_ascii_lowercase().contains("undefined"));
        assert!(cleaned.contains("endpoint /todos"));
    }

    #[test]
    fn deterministic_template_todo_is_available() {
        let message = "crea una todo list app completa";
        let template = deterministic_template_for_common_requests(message)
            .expect("todo template should be available");
        assert!(template.contains("server/index.js"));
        assert!(!template.to_ascii_lowercase().contains("undefined"));
    }

    #[test]
    fn deterministic_template_not_used_for_goal_execution_requests() {
        let message =
            "Implementa il primo goal pending end-to-end in todo-app, con codice completo + test minimi.";
        assert!(deterministic_template_for_common_requests(message).is_none());
    }

    #[test]
    fn should_force_template_for_incomplete_todo_answer() {
        let message = "crea una todo list app completa";
        let partial = "Stack: frontend React, backend Express, database PostgreSQL.";
        assert!(!should_force_common_template(message, partial));

        let complete = "## 1) Struttura\n```bash\nmkdir -p todo-app/{client,server,database}\n```\n\
## 2) DB (database/init.sql)\n```sql\nCREATE TABLE todos(id serial primary key);\n```\n\
## 3) Backend (server/index.js)\n```js\napp.get('/api/todos', ()=>{}); app.post('/api/todos', ()=>{});\n```\n\
## 4) Frontend (client/src/App.jsx)\n```jsx\nexport default function App(){}\n```\n\
## 5) Run\n```bash\ncd server && node index.js\ncd client && npm run dev\n```";
        assert!(!should_force_common_template(message, complete));

        let broken = "Database:\nundefined\nBackend:\n- endpoint /api/todos";
        assert!(should_force_common_template(message, broken));
    }

    #[test]
    fn should_not_force_template_for_goal_execution_requests() {
        let message =
            "Implementa il primo goal pending end-to-end in todo-app, con codice completo + test minimi.";
        let partial = "Piano: implemento endpoint e test minimi nel goal corrente.";
        assert!(!should_force_common_template(message, partial));
    }

    #[test]
    fn detects_goal_execution_requests() {
        assert!(is_goal_execution_request(
            "Implementa SOLO il primo goal pending in todo-app. Niente scaffolding iniziale."
        ));
        assert!(is_goal_execution_request(
            "Only the first pending goal, file changes + minimal tests"
        ));
        assert!(!is_goal_execution_request("crea una todo list app completa"));
    }

    #[test]
    fn avoids_false_positive_goal_execution_requests() {
        assert!(!is_goal_execution_request(
            "show status of dependencies and continue with docs update"
        ));
        assert!(!is_goal_execution_request(
            "continue implementation of frontend login flow"
        ));
    }
}
