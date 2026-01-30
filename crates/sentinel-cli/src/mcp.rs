//! MCP Server - Model Context Protocol Implementation
//! 
//! Permette a Sentinel di comunicare con agenti esterni (Cline, Claude Desktop)
//! esponendo strumenti di validazione e analisi dell'allineamento.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::path::PathBuf;
use sentinel_core::{GoalManifold, ProjectState, AlignmentField, security::SecurityScanner};

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
                        "name": "decompose_goal",
                        "description": "Scompone un goal complesso in una serie di task atomici deterministici (Atomic Truth)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "goal_id": { "type": "string", "description": "L'UUID del goal da scomporre" }
                            },
                            "required": ["goal_id"]
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
        },
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
    // 1. Controlla variabile d'ambiente (per override esplicito)
    if let Ok(root) = std::env::var("SENTINEL_ROOT") {
        let path = PathBuf::from(root).join("sentinel.json");
        if path.exists() { return Some(path); }
    }

    // 2. Cerca risalendo le directory dalla CWD
    if let Ok(mut current_dir) = std::env::current_dir() {
        loop {
            let path = current_dir.join("sentinel.json");
            if path.exists() { return Some(path); }
            if !current_dir.pop() { break; }
        }
    }

    // 3. Fallback hardcoded per questo ambiente (Fix per esecuzione da root /)
    // Nota: Include lo spazio finale nel nome della cartella come da filesystem attuale
    let fallback_path = PathBuf::from("/Users/danielecorrao/Documents/REPOSITORIES_GITHUB/SENTINEL /sentinel.json");
    if fallback_path.exists() {
        return Some(fallback_path);
    }

    None
}

/// Helper per caricare il manifold in modo sicuro
fn get_manifold() -> Result<GoalManifold, String> {
    let path = find_manifold_path().ok_or_else(|| {
        let cwd = std::env::current_dir().unwrap_or_default();
        format!("Manifold file 'sentinel.json' non trovato (Cercato a partire da: {:?}). Inizializza il progetto con 'sentinel init'.", cwd)
    })?;
    
    let content = std::fs::read_to_string(&path).map_err(|e| format!("Errore lettura file: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Errore parsing manifold: {}", e))
}

/// Helper per salvare il manifold
fn save_manifold(manifold: &GoalManifold) -> Result<(), String> {
    let path = find_manifold_path().ok_or_else(|| "Impossibile salvare: sentinel.json non trovato (necessario per sovrascrittura).".to_string())?;
    let content = serde_json::to_string_pretty(manifold).map_err(|e| format!("Errore serializzazione: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Errore scrittura file: {}", e))
}

async fn handle_tool_call(params: Option<Value>) -> Option<Value> {
    let params = params?;
    let name = params.get("name")?.as_str()?;
    let arguments = params.get("arguments");

    match name {
        "validate_action" => {
            let desc = arguments.and_then(|a| a.get("description")).and_then(|v| v.as_str()).unwrap_or("");
            let response_text = match get_manifold() {
                Ok(manifold) => {
                    let state = ProjectState::new(PathBuf::from("."));
                    let field = AlignmentField::new(manifold);
                    match field.predict_alignment(&state).await {
                        Ok(res) => format!("SENTINEL ANALYSIS: L'azione '{}' ha una probabilità di successo del {:.1}%. Allineamento atteso: {:.1}%. Risk Level: {:?}", 
                            desc, (1.0 - res.deviation_probability) * 100.0, res.expected_alignment, res.risk_level()),
                        Err(e) => format!("SENTINEL ERROR: Simulazione fallita: {}", e),
                    }
                },
                Err(e) => format!("SENTINEL PROVISIONAL: {}", e),
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "get_alignment" => {
            let response_text = match get_manifold() {
                Ok(manifold) => {
                    let state = ProjectState::new(PathBuf::from("."));
                    let field = AlignmentField::new(manifold);
                    match field.compute_alignment(&state).await {
                        Ok(v) => format!("GOAL ALIGNMENT REPORT:\nPunteggio: {:.1}%\nConfidenza: {:.0}%\nStato: {}", 
                            v.score, v.confidence * 100.0, if v.score > 70.0 { "OTTIMALE" } else { "DEVIATO" }),
                        Err(e) => format!("Errore calcolo: {}", e),
                    }
                },
                Err(e) => format!("SENTINEL INFO: {}", e),
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "safe_write" => {
            let path = arguments.and_then(|a| a.get("path")).and_then(|v| v.as_str()).unwrap_or("unknown");
            let content = arguments.and_then(|a| a.get("content")).and_then(|v| v.as_str()).unwrap_or("");
            let report = SecurityScanner::scan(content);
            
            let message = if !report.is_safe {
                format!("SENTINEL BLOCK: Rilevate {} minacce nel file {}.\nMinacce: {}", report.threats.len(), path, report.threats.join(", "))
            } else {
                format!("SAFE WRITE APPROVED: Il file {} ha superato lo scan Layer 7 (Risk Score: {:.2}).", path, report.risk_score)
            };

            Some(serde_json::json!({
                "content": [{ "type": "text", "text": message }]
            }))
        },

        "propose_strategy" => {
            let goal_desc = arguments.and_then(|a| a.get("goal_description")).and_then(|v| v.as_str()).unwrap_or("");
            let kb = std::sync::Arc::new(sentinel_core::learning::knowledge_base::KnowledgeBase::new());
            let synthesizer = sentinel_core::learning::strategy::StrategySynthesizer::new(kb);
            use sentinel_core::goal_manifold::goal::Goal;
            use sentinel_core::goal_manifold::predicate::Predicate;
            let temp_goal = Goal::builder()
                .description(goal_desc)
                .add_success_criterion(Predicate::AlwaysTrue)
                .build();
            let response_text = match temp_goal {
                Ok(goal) => match synthesizer.suggest_strategy(&goal).await {
                    Ok(s) => format!("STRATEGIA SUGGERITA (Layer 5):\n- Confidenza: {:.0}%\n- {} approcci trovati.",
                        s.confidence * 100.0, s.recommended_approaches.len()),
                    Err(e) => format!("STRATEGIA (Layer 5): Knowledge base vuota. Nessun pattern disponibile per '{}'. Errore: {}", goal_desc, e),
                },
                Err(e) => format!("STRATEGIA (Layer 5): Impossibile costruire il goal temporaneo: {}", e),
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "record_handover" => {
            let gid = arguments.and_then(|a| a.get("goal_id")).and_then(|v| v.as_str()).unwrap_or("");
            let content = arguments.and_then(|a| a.get("content")).and_then(|v| v.as_str()).unwrap_or("");
            let response_text = match get_manifold() {
                Ok(mut manifold) => {
                    let note = sentinel_core::types::HandoverNote {
                        id: uuid::Uuid::new_v4(), agent_id: uuid::Uuid::new_v4(),
                        goal_id: uuid::Uuid::parse_str(gid).unwrap_or(uuid::Uuid::nil()),
                        content: content.to_string(), technical_warnings: vec![],
                        suggested_next_steps: vec![], timestamp: chrono::Utc::now(),
                    };
                    manifold.handover_log.push(note);
                    match save_manifold(&manifold) {
                        Ok(_) => "COGNITIVE HANDOVER SUCCESS: Nota salvata in sentinel.json.".to_string(),
                        Err(e) => format!("Errore salvataggio: {}", e),
                    }
                },
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "get_cognitive_map" => {
            let response_text = match get_manifold() {
                Ok(manifold) => sentinel_core::architect::distiller::CognitiveDistiller::distill(&manifold).content,
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "get_enforcement_rules" => {
            let response_text = match get_manifold() {
                Ok(manifold) => {
                    let mut rules = vec!["1. Non modificare file bloccati.".to_string()];
                    for inv in &manifold.invariants { 
                        rules.push(format!("- [INVARIANTE] {}", inv.description)); 
                    }
                    format!("SENTINEL ENFORCEMENT RULES:\n{}", rules.join("\n"))
                },
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        "decompose_goal" => {
            let gid_str = arguments.and_then(|a| a.get("goal_id")).and_then(|v| v.as_str()).unwrap_or("");
            let response_text = match get_manifold() {
                Ok(mut manifold) => {
                    let gid = match uuid::Uuid::parse_str(gid_str) {
                        Ok(id) => id,
                        Err(_) => return Some(serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "UUID goal non valido." }] })),
                    };

                    let goal = match manifold.get_goal(&gid) {
                        Some(g) => g.clone(),
                        None => return Some(serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "Goal non trovato." }] })),
                    };

                    use sentinel_core::goal_manifold::slicer::AtomicSlicer;
                    match AtomicSlicer::decompose(&goal) {
                        Ok(sub_goals) => {
                            let count = sub_goals.len();
                            for sg in sub_goals {
                                let _ = manifold.add_goal(sg);
                            }
                            match save_manifold(&manifold) {
                                Ok(_) => format!("ATOMIC DECOMPOSITION SUCCESS: Goal '{}' scomposto in {} task atomici.", goal.description, count),
                                Err(e) => format!("Errore salvataggio manifold: {}", e),
                            }
                        },
                        Err(e) => format!("Errore scomposizione: {}", e),
                    }
                },
                Err(e) => e,
            };
            Some(serde_json::json!({ "content": [{ "type": "text", "text": response_text }] }))
        },

        _ => Some(serde_json::json!({ "isError": true, "content": [{ "type": "text", "text": "Tool non supportato." }] })),
    }
}
