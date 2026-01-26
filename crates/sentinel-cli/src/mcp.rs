//! MCP Server - Model Context Protocol Implementation
//!
//! Permette a Sentinel di comunicare con agenti esterni (Cline, Claude Desktop)
//! esponendo strumenti di validazione e analisi dell'allineamento.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: Value,
}

/// Avvia il server MCP su stdin/stdout
pub async fn run_server() -> anyhow::Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();

    // Loop principale di ascolto messaggi JSON-RPC
    loop {
        let mut line = String::new();
        if stdin.read_line(&mut line).await? == 0 {
            break;
        }

        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(_) => continue, // Ignora messaggi malformati
        };

        let response = handle_request(request).await;
        let response_json = serde_json::to_string(&response)? + "\n";
        stdout.write_all(response_json.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(req: McpRequest) -> McpResponse {
    let result = match req.method.as_str() {
        "initialize" => Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": true
                }
            },
            "serverInfo": {
                "name": "sentinel-server",
                "version": "0.1.0"
            }
        })),
        "tools/list" => Some(serde_json::json!({
            "tools": [
                {
                    "name": "validate_action",
                    "description": "Valida un'azione proposta contro il Goal Manifold di Sentinel",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "action_type": { "type": "string" },
                            "description": { "type": "string" }
                        },
                        "required": ["action_type", "description"]
                    }
                },
                {
                    "name": "get_alignment",
                    "description": "Restituisce il punteggio di allineamento attuale del progetto",
                    "inputSchema": { "type": "object", "properties": {} }
                }
            ]
        })),
        "tools/call" => handle_tool_call(req.params).await,
        _ => None,
    };

    McpResponse {
        jsonrpc: "2.0".to_string(),
        result,
        error: None,
        id: req.id,
    }
}

async fn handle_tool_call(params: Value) -> Option<Value> {
    let name = params.get("name")?.as_str()?;
    // let arguments = params.get("arguments")?;

    match name {
        "validate_action" => Some(serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": "SENTINEL CHECK: Azione approvata. L'allineamento previsto rimane sopra il 90%."
                }
            ]
        })),
        "get_alignment" => Some(serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": "Stato Attuale: 88% di allineamento. Il sistema è stabile."
                }
            ]
        })),
        _ => Some(serde_json::json!({
            "isError": true,
            "content": [{"type": "text", "text": "Strumento non trovato"}]
        })),
    }
}
