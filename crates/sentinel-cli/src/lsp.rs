//! LSP Server - Language Server Protocol Implementation
//!
//! Fornisce diagnostica di allineamento in tempo reale negli IDE (VS Code, Cursor).
//! Segnala deviazioni dal Goal Manifold direttamente come errori o avvertimenti nel codice.

use sentinel_core::{AlignmentField, GoalManifold, ProjectState};
use std::path::{Path, PathBuf};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Backend del Server LSP di Sentinel
pub struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "sentinel-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Sentinel LSP Initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("File opened: {}", params.text_document.uri),
            )
            .await;
        self.validate_alignment(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(content) = params.content_changes.pop() {
            self.validate_alignment(params.text_document.uri, content.text)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("File saved: {}", params.text_document.uri),
            )
            .await;
    }
}

impl Backend {
    /// Valida l'allineamento del codice rispetto al Goal Manifold
    async fn validate_alignment(&self, uri: Url, text: String) {
        let mut diagnostics = Vec::new();

        // Fast local heuristic for incomplete critical work.
        if text.contains("TODO") || text.contains("FIXME") {
            diagnostics.push(Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("alignment_drift".to_string())),
                source: Some("Sentinel".to_string()),
                message: "Possibile deviazione rilevata: task incompleto trovato in area critica."
                    .to_string(),
                ..Default::default()
            });
        }

        // Real alignment signal from Layer 2, bound to the current project manifold.
        if let Some(manifold_path) = find_manifold_path() {
            match load_manifold(&manifold_path) {
                Ok(manifold) => {
                    let working_directory = manifold_path
                        .parent()
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| PathBuf::from("."));
                    let state = ProjectState::new(working_directory);
                    let field = AlignmentField::new(manifold);

                    match field.compute_alignment(&state).await {
                        Ok(alignment) => {
                            if alignment.score < 70.0 {
                                diagnostics.push(Diagnostic {
                                    range: Range::default(),
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    code: Some(NumberOrString::String(
                                        "sentinel_low_alignment".to_string(),
                                    )),
                                    source: Some("Sentinel".to_string()),
                                    message: format!(
                                        "Allineamento basso ({:.1}/100). Rivedi obiettivi e azioni.",
                                        alignment.score
                                    ),
                                    ..Default::default()
                                });
                            } else if alignment.score < 85.0 {
                                diagnostics.push(Diagnostic {
                                    range: Range::default(),
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    code: Some(NumberOrString::String(
                                        "sentinel_alignment_watch".to_string(),
                                    )),
                                    source: Some("Sentinel".to_string()),
                                    message: format!(
                                        "Allineamento da monitorare ({:.1}/100).",
                                        alignment.score
                                    ),
                                    ..Default::default()
                                });
                            }
                        }
                        Err(err) => {
                            self.client
                                .log_message(
                                    MessageType::WARNING,
                                    format!("Sentinel alignment compute error: {}", err),
                                )
                                .await;
                        }
                    }
                }
                Err(err) => {
                    self.client
                        .log_message(
                            MessageType::WARNING,
                            format!("Sentinel manifold load error: {}", err),
                        )
                        .await;
                }
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn find_manifold_path() -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var("SENTINEL_MANIFOLD") {
        let explicit_path = PathBuf::from(explicit);
        if explicit_path.exists() {
            return Some(explicit_path);
        }
    }

    if let Ok(mut current_dir) = std::env::current_dir() {
        loop {
            let candidate = current_dir.join("sentinel.json");
            if candidate.exists() {
                return Some(candidate);
            }
            if !current_dir.pop() {
                break;
            }
        }
    }

    None
}

fn load_manifold(path: &Path) -> anyhow::Result<GoalManifold> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Avvia il server LSP su stdin/stdout
pub async fn run_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
