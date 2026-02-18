//! `GeminiCliClient` — Provider LLM via Gemini CLI (OAuth Google AI Pro).
//!
//! Invoca `gemini -p "<prompt>" -o json` come sottoprocesso headless.
//! Nessuna API key necessaria: l'autenticazione è gestita dal CLI via OAuth.
//!
//! **Legalità**: ✅ Legale — Gemini CLI è Apache 2.0 open-source, la modalità
//! `--prompt` è esplicitamente progettata per scripting. L'utente usa la propria
//! sottoscrizione Google AI Pro. Non legale: rivendere l'accesso o condividere credenziali.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::time::Instant;
use tokio::process::Command;
use tokio::time::timeout;
use std::time::Duration;

use crate::llm_integration::{
    DocFormat, ExplanationStyle, ImprovementMetric, LLMClient, LLMContext, LLMSuggestion,
    LLMSuggestionType,
};
use sentinel_core::Uuid;

const CLI_TIMEOUT_SECS: u64 = 180;
const GEMINI_BIN: &str = "gemini";

/// Risposta JSON del Gemini CLI v0.28+
#[derive(Debug, Deserialize)]
struct GeminiCliResponse {
    response: String,
    #[serde(default)]
    stats: serde_json::Value,
}

/// Provider che usa il binary `gemini` CLI con OAuth Google.
///
/// Usa `gemini -3-flash-preview` o il modello scelto dalla sottoscrizione.
/// Non richiede API key: l'OAuth è già gestito dal CLI al primo avvio.
#[derive(Debug, Clone)]
pub struct GeminiCliClient {
    /// Modello da usare (es. "gemini-2.5-pro", None = default CLI)
    model: Option<String>,
    /// Path al binary gemini (default: "gemini" dal PATH)
    binary: String,
}

impl GeminiCliClient {
    /// Crea un client con rilevamento automatico del binary e modello default.
    pub fn new() -> Self {
        Self {
            model: None,
            binary: GEMINI_BIN.to_string(),
        }
    }

    /// Specifica un modello esplicito (es. "gemini-2.5-pro-preview").
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Usa un path specifico per il binary (utile in CI o path non standard).
    pub fn with_binary(mut self, path: impl Into<String>) -> Self {
        self.binary = path.into();
        self
    }

    /// Verifica che il binary sia raggiungibile e autenticato.
    ///
    /// Ritorna Ok(model_name) se disponibile, Err altrimenti.
    pub async fn probe(&self) -> Result<String> {
        let out = timeout(
            Duration::from_secs(10),
            Command::new(&self.binary)
                .arg("-p")
                .arg("respond with exactly: ok")
                .arg("-o")
                .arg("json")
                .output(),
        )
        .await
        .context("probe timeout")?
        .context("gemini binary not found or not executable")?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!("gemini probe failed: {}", stderr.trim());
        }

        let resp: GeminiCliResponse = serde_json::from_slice(&out.stdout)
            .context("probe response not valid JSON")?;

        // Estrai model name dagli stats se disponibile
        let model = resp
            .stats
            .get("models")
            .and_then(|m| m.as_object())
            .and_then(|m| m.keys().next().cloned())
            .unwrap_or_else(|| "gemini-cli".to_string());

        Ok(model)
    }

    /// Invoca il Gemini CLI con il prompt dato e ritorna la risposta grezza.
    async fn call(&self, prompt: &str) -> Result<(String, u32)> {
        let start = Instant::now();

        let mut cmd = Command::new(&self.binary);
        // Usa --output-format (flag corretto per v0.28+), non -o
        cmd.arg("-p").arg(prompt).arg("--output-format").arg("json");

        if let Some(model) = &self.model {
            cmd.arg("-m").arg(model);
        }

        let output = timeout(
            Duration::from_secs(CLI_TIMEOUT_SECS),
            cmd.output(),
        )
        .await
        .with_context(|| format!("gemini CLI timeout after {}s", CLI_TIMEOUT_SECS))?
        .context("gemini CLI failed to start")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("gemini CLI exited with error: {}", stderr.trim());
        }

        // L'output contiene righe di log prima del JSON (es. "Loaded cached credentials.")
        // Estrae solo il blocco JSON cercando la prima '{' e l'ultima '}'
        let raw_stdout = String::from_utf8_lossy(&output.stdout);
        let json_str = extract_json_object(&raw_stdout);

        if let Ok(parsed) = serde_json::from_str::<GeminiCliResponse>(&json_str) {
            // Estrae il conteggio token dagli stats
            let tokens = parsed
                .stats
                .get("models")
                .and_then(|m| m.as_object())
                .and_then(|m| m.values().next())
                .and_then(|v| v.get("tokens"))
                .and_then(|t| t.get("total"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as u32;

            tracing::debug!(
                "gemini CLI response: {} chars, {} tokens, {}ms",
                parsed.response.len(),
                tokens,
                start.elapsed().as_millis()
            );

            return Ok((parsed.response, tokens));
        }

        // Fallback: se il JSON non è parsabile, usa stdout come testo plain
        // (rimuovendo le righe di log che precedono il contenuto utile)
        let plain = raw_stdout
            .lines()
            .filter(|l| !l.starts_with("Loaded ") && !l.starts_with("Loading ") && !l.starts_with("Server ") && !l.starts_with("Error during") && !l.starts_with("Hook registry"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if plain.is_empty() {
            bail!("gemini CLI returned empty response. stdout: {}", raw_stdout.trim());
        }

        Ok((plain, 0))
    }

    fn make_suggestion(
        content: String,
        token_cost: u32,
        suggestion_type: LLMSuggestionType,
    ) -> LLMSuggestion {
        LLMSuggestion {
            id: Uuid::new_v4(),
            suggestion_type,
            llm_name: "GeminiCLI/GoogleAIPro".to_string(),
            content,
            estimated_quality: 0.88,
            goal_alignment: 0.85,
            confidence: 0.90,
            token_cost,
        }
    }
}

impl Default for GeminiCliClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LLMClient for GeminiCliClient {
    async fn generate_code(&self, prompt: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let full_prompt = build_code_prompt(prompt, context);
        let (content, tokens) = self.call(&full_prompt).await?;
        Ok(Self::make_suggestion(
            content.clone(),
            tokens,
            LLMSuggestionType::CodeGeneration {
                file_path: "generated.rs".to_string(),
                code: content,
                language: "rust".to_string(),
            },
        ))
    }

    async fn suggest_refactoring(
        &self,
        code: &str,
        context: &LLMContext,
    ) -> Result<LLMSuggestion> {
        let prompt = format!(
            "You are a senior Rust engineer. Suggest refactoring improvements for the following code.\n\
            Project goal: {}\n\nCode:\n```rust\n{}\n```\n\nRespond with refactored code and explanation.",
            context.goal_description, code
        );
        let (content, tokens) = self.call(&prompt).await?;
        Ok(Self::make_suggestion(
            content,
            tokens,
            LLMSuggestionType::Refactoring {
                file_path: "refactored.rs".to_string(),
                description: "Gemini CLI refactoring suggestion".to_string(),
                expected_improvement: ImprovementMetric::CodeQuality,
            },
        ))
    }

    async fn generate_documentation(
        &self,
        code: &str,
        context: &LLMContext,
    ) -> Result<LLMSuggestion> {
        let prompt = format!(
            "Generate Rust doc comments (///) for the following code.\n\
            Project goal: {}\n\nCode:\n```rust\n{}\n```\n\nReturn only doc comments.",
            context.goal_description, code
        );
        let (content, tokens) = self.call(&prompt).await?;
        Ok(Self::make_suggestion(
            content,
            tokens,
            LLMSuggestionType::Documentation {
                to_document: "code".to_string(),
                format: DocFormat::DocComments,
            },
        ))
    }

    async fn generate_tests(&self, code: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let prompt = format!(
            "Generate comprehensive Rust unit tests for the following code.\n\
            Project goal: {}\n\nCode:\n```rust\n{}\n```\n\nReturn a #[cfg(test)] module with tests.",
            context.goal_description, code
        );
        let (content, tokens) = self.call(&prompt).await?;
        Ok(Self::make_suggestion(
            content,
            tokens,
            LLMSuggestionType::TestCase {
                test_target: "generated_code".to_string(),
                test_type: "unit".to_string(),
            },
        ))
    }

    async fn explain_concept(&self, concept: &str, context: &LLMContext) -> Result<LLMSuggestion> {
        let prompt = format!(
            "You are a senior software engineer. Explain the following concept clearly.\n\
            Project context: {}\n\nConcept: {}",
            context.goal_description, concept
        );
        let (content, tokens) = self.call(&prompt).await?;
        Ok(Self::make_suggestion(
            content,
            tokens,
            LLMSuggestionType::ConceptExplanation {
                concept: concept.to_string(),
                style: ExplanationStyle::StepByStep,
            },
        ))
    }
}

/// Costruisce il prompt completo per la generazione di codice.
fn build_code_prompt(prompt: &str, context: &LLMContext) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "You are a senior software engineer expert in Rust, TypeScript and Python.\n\
        Project goal: {}\n\
        Context: {}",
        context.goal_description, context.context
    ));

    if !context.constraints.is_empty() {
        parts.push(format!("Constraints: {}", context.constraints.join(", ")));
    }

    if !context.previous_approaches.is_empty() {
        parts.push(format!(
            "Previous approaches tried (do not repeat): {}",
            context.previous_approaches.join("; ")
        ));
    }

    if !context.p2p_intelligence.is_empty() {
        parts.push(format!("P2P intelligence: {}", context.p2p_intelligence));
    }

    parts.push(format!("Task: {}", prompt));
    parts.join("\n\n")
}

/// Estrae il primo oggetto JSON `{...}` da una stringa che può contenere
/// righe di log prima del JSON (output tipico di Gemini CLI v0.28+).
fn extract_json_object(raw: &str) -> String {
    // Cerca la prima '{' e l'ultima '}' per isolare il blocco JSON
    if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}')) {
        if start < end {
            return raw[start..=end].to_string();
        }
    }
    raw.trim().to_string()
}

/// Controlla se il Gemini CLI è disponibile nel PATH (sincrono, per startup check).
pub fn is_gemini_cli_available() -> bool {
    std::process::Command::new(GEMINI_BIN)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_cli_client_creation() {
        let client = GeminiCliClient::new();
        assert_eq!(client.binary, "gemini");
        assert!(client.model.is_none());
    }

    #[test]
    fn test_gemini_cli_client_with_model() {
        let client = GeminiCliClient::new().with_model("gemini-2.5-pro");
        assert_eq!(client.model, Some("gemini-2.5-pro".to_string()));
    }

    #[test]
    fn test_build_code_prompt_includes_context() {
        use crate::llm_integration::LLMContext;
        let ctx = LLMContext {
            goal_description: "Build auth".to_string(),
            context: "Rust API".to_string(),
            p2p_intelligence: "".to_string(),
            constraints: vec!["no unsafe".to_string()],
            previous_approaches: vec![],
        };
        let prompt = build_code_prompt("implement JWT", &ctx);
        assert!(prompt.contains("Build auth"));
        assert!(prompt.contains("no unsafe"));
        assert!(prompt.contains("implement JWT"));
    }

    #[test]
    fn test_make_suggestion_fields() {
        let s = GeminiCliClient::make_suggestion(
            "fn main() {}".to_string(),
            1000,
            LLMSuggestionType::CodeGeneration {
                file_path: "main.rs".to_string(),
                code: "fn main() {}".to_string(),
                language: "rust".to_string(),
            },
        );
        assert_eq!(s.llm_name, "GeminiCLI/GoogleAIPro");
        assert_eq!(s.content, "fn main() {}");
        assert_eq!(s.token_cost, 1000);
        assert!(s.confidence > 0.8);
    }

    #[test]
    fn test_is_gemini_cli_available() {
        // Il test è informativo: su macchine senza gemini ritorna false (ok)
        let available = is_gemini_cli_available();
        println!("gemini CLI available: {}", available);
        // Non assert — deve solo non panic
    }

    #[tokio::test]
    #[ignore = "Requires gemini CLI authenticated with Google AI Pro"]
    async fn test_real_gemini_cli_call() {
        let client = GeminiCliClient::new();
        let ctx = crate::llm_integration::LLMContext {
            goal_description: "Test".to_string(),
            context: "Unit test".to_string(),
            p2p_intelligence: "".to_string(),
            constraints: vec![],
            previous_approaches: vec![],
        };
        let result = client.generate_code("Say HELLO in one word", &ctx).await;
        assert!(result.is_ok(), "Real call failed: {:?}", result.err());
        let suggestion = result.unwrap();
        println!("Response: {}", suggestion.content);
        assert!(!suggestion.content.is_empty());
    }
}
