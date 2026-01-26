//! LSP Server - Language Server Protocol Implementation
//!
//! Fornisce diagnostica di allineamento in tempo reale negli IDE (VS Code, Cursor).
//! Segnala deviazioni dal Goal Manifold direttamente come errori o avvertimenti nel codice.

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
            .log_message(MessageType::INFO, format!("File opened: {}", params.text_document.uri))
            .await;
        self.validate_alignment(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        if let Some(content) = params.content_changes.pop() {
            self.validate_alignment(params.text_document.uri, content.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("File saved: {}", params.text_document.uri))
            .await;
    }
}

impl Backend {
    /// Valida l'allineamento del codice rispetto al Goal Manifold
    async fn validate_alignment(&self, uri: Url, text: String) {
        let mut diagnostics = Vec::new();

        // LOGICA DI ALLINEAMENTO (Mock per ora)
        // In un'implementazione completa, qui chiameremmo sentinel_core::AlignmentField
        if text.contains("TODO") || text.contains("FIXME") {
            diagnostics.push(Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("alignment_drift".to_string())),
                source: Some("Sentinel".to_string()),
                message: "Possibile deviazione rilevata: task incompleto trovato in area critica.".to_string(),
                ..Default::default()
            });
        }

        // Se l'allineamento fosse calcolato dal Layer 2:
        // let score = self.core.compute_alignment(text).await;
        // if score < 0.7 { ... generate error diagnostic ... }

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

/// Avvia il server LSP su stdin/stdout
pub async fn run_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
