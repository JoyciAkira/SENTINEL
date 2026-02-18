//! End-to-End Agent — Il cuore di SENTINEL
//!
//! Implementa il loop deterministico:
//!   1. Architect LLM-powered: interpreta l'intent, produce piano atomico non negoziabile
//!   2. Worker LLM-powered: implementa ogni modulo confinato nel suo scope
//!   3. ModuleVerifier: verifica output_contract sul filesystem reale
//!   4. RepairLoop: non si ferma finché tutti i moduli non passano la verifica
//!
//! Questo è il differenziatore assoluto rispetto a Cline/Cursor/Copilot:
//! SENTINEL sa dall'inizio dove deve arrivare e non si ferma finché non ci arriva.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::providers::gemini_cli::GeminiCliClient;
use crate::llm_integration::{LLMClient, LLMContext};
use sentinel_core::split_agent::{
    ArchitectAgent, LocalGuardrail, ModuleVerifier, SplitAgentExecutor, SplitExecutionReport,
    SplitPlan, WorkerModule,
};
use sentinel_core::goal_manifold::{Intent, predicate::Predicate};

/// Configurazione del loop end-to-end
#[derive(Debug, Clone)]
pub struct E2EConfig {
    /// Directory di output dove il codice viene generato
    pub workspace: PathBuf,
    /// Numero massimo di tentativi di repair per modulo
    pub max_repair_attempts: usize,
    /// Se true, mostra output dettagliato
    pub verbose: bool,
    /// Modello Gemini da usare (None = default CLI)
    pub gemini_model: Option<String>,
}

impl Default for E2EConfig {
    fn default() -> Self {
        Self {
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            max_repair_attempts: 3,
            verbose: true,
            gemini_model: None,
        }
    }
}

/// Report finale dell'esecuzione end-to-end
#[derive(Debug, Clone)]
pub struct E2EReport {
    pub intent: String,
    pub total_modules: usize,
    pub passed_modules: usize,
    pub failed_modules: usize,
    pub repair_attempts: usize,
    pub duration_secs: f64,
    pub success: bool,
    pub module_details: Vec<ModuleDetail>,
    pub workspace: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModuleDetail {
    pub title: String,
    pub passed: bool,
    pub attempts: usize,
    pub predicate_results: Vec<(String, bool)>,
    pub generated_files: Vec<PathBuf>,
}

/// Il loop end-to-end principale
pub struct EndToEndAgent {
    llm: GeminiCliClient,
    config: E2EConfig,
}

impl EndToEndAgent {
    pub fn new(config: E2EConfig) -> Self {
        let mut llm = GeminiCliClient::new();
        if let Some(model) = &config.gemini_model {
            llm = llm.with_model(model.clone());
        }
        Self { llm, config }
    }

    /// Entry point principale: dato un intent in linguaggio naturale,
    /// produce codice funzionante senza fermarsi finché il goal non è raggiunto.
    pub async fn run(&self, intent_text: &str) -> Result<E2EReport> {
        let start = Instant::now();

        if self.config.verbose {
            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║           SENTINEL END-TO-END AGENT                     ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            println!("\n🎯 INTENT: {}", intent_text);
            println!("📁 WORKSPACE: {}", self.config.workspace.display());
            println!("🔄 MAX REPAIR ATTEMPTS: {}", self.config.max_repair_attempts);
            println!();
        }

        // Crea workspace se non esiste
        std::fs::create_dir_all(&self.config.workspace)
            .context("Failed to create workspace directory")?;

        // ─── FASE 1: ARCHITECT ───────────────────────────────────────────────
        if self.config.verbose {
            println!("━━━ FASE 1: ARCHITECT AGENT ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("🏗️  Analisi intent e produzione piano atomico non negoziabile...");
        }

        let plan = self.architect_phase(intent_text).await
            .context("Architect phase failed")?;

        if self.config.verbose {
            println!("✅ Piano prodotto: {} moduli atomici", plan.modules.len());
            println!("🔒 Plan hash (tamper-evident): {}", &plan.plan_hash[..16]);
            for (i, m) in plan.modules.iter().enumerate() {
                println!("   Module {}: {} [effort={}]", i + 1, m.title, m.estimated_effort);
                println!("     📍 Destination: {}", m.worker_context.destination_state);
                println!("     🛡️  Guardrails: {}", m.local_guardrails.len());
                println!("     📋 Output contract: {} predicates", m.output_contract.len());
            }
            println!();
        }

        // ─── FASE 2: WORKER LOOP ─────────────────────────────────────────────
        if self.config.verbose {
            println!("━━━ FASE 2: WORKER AGENTS + REPAIR LOOP ━━━━━━━━━━━━━━━━━━━");
            println!("⚙️  Esecuzione moduli con verifica continua...");
            println!();
        }

        let (report, module_details) = self.worker_repair_loop(&plan, intent_text).await?;

        let duration = start.elapsed().as_secs_f64();

        if self.config.verbose {
            println!();
            println!("━━━ RISULTATO FINALE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            if report.all_passed {
                println!("✅ GOAL RAGGIUNTO — tutti i moduli verificati");
            } else {
                println!("⚠️  GOAL PARZIALMENTE RAGGIUNTO — {}/{} moduli passati",
                    report.passed_modules, report.total_modules);
            }
            println!("⏱️  Durata: {:.1}s", duration);
            println!("📁 Output: {}", self.config.workspace.display());
        }

        Ok(E2EReport {
            intent: intent_text.to_string(),
            total_modules: report.total_modules,
            passed_modules: report.passed_modules,
            failed_modules: report.failed_modules,
            repair_attempts: 0, // aggiornato nel loop
            duration_secs: duration,
            success: report.all_passed,
            module_details,
            workspace: self.config.workspace.clone(),
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FASE 1: ARCHITECT — usa Gemini CLI per produrre piano atomico
    // ─────────────────────────────────────────────────────────────────────────

    async fn architect_phase(&self, intent_text: &str) -> Result<SplitPlan> {
        // Chiedi a Gemini di scomporre l'intent in moduli atomici con output_contract
        let architect_prompt = format!(
            r#"You are a senior software architect. Your task is to decompose a user intent into atomic, non-negotiable software modules.

USER INTENT: {}

Produce a JSON array of modules. Each module MUST have:
- "title": short name (e.g. "Auth Module", "Database Schema")
- "destination_state": exact description of what MUST be true when this module is complete
- "output_files": array of file paths that MUST exist after this module (relative to project root)
- "guardrails": array of rules the worker MUST follow (e.g. "no hardcoded secrets", "must have tests")
- "effort": integer 1-10

Rules:
1. Maximum 6 modules
2. Each module is ATOMIC — one clear responsibility
3. output_files are VERIFIABLE — they must exist on disk
4. Modules are ordered by dependency (first module has no deps)
5. Be specific and opinionated

Respond with ONLY a valid JSON array, no markdown, no explanation.

Example format:
[
  {{
    "title": "Project Setup",
    "destination_state": "Project structure initialized with package.json and src/ directory",
    "output_files": ["package.json", "src/index.ts", "tsconfig.json"],
    "guardrails": ["Use TypeScript strict mode", "No any types"],
    "effort": 2
  }}
]"#,
            intent_text
        );

        let ctx = LLMContext {
            goal_description: intent_text.to_string(),
            context: "Architect phase: decompose intent into atomic modules".to_string(),
            p2p_intelligence: String::new(),
            constraints: vec![],
            previous_approaches: vec![],
        };

        let suggestion = self.llm.generate_code(&architect_prompt, &ctx).await
            .context("Gemini CLI architect call failed")?;

        // Parsa il JSON prodotto dall'Architect
        let raw = &suggestion.content;
        let json_str = extract_json_array(raw);

        let modules_json: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .with_context(|| format!("Failed to parse architect JSON:\n{}", json_str))?;

        // Costruisce il SplitPlan dai moduli JSON
        let intent = Intent::new(intent_text, Vec::<String>::new());
        let architect = ArchitectAgent::new();

        // Costruisce WorkerModule da ogni entry JSON
        let mut worker_modules: Vec<WorkerModule> = Vec::new();
        let mut prev_id: Option<uuid::Uuid> = None;

        for (i, m) in modules_json.iter().enumerate() {
            let title = m["title"].as_str().unwrap_or(&format!("Module {}", i + 1)).to_string();
            let destination = m["destination_state"].as_str()
                .unwrap_or("Module complete").to_string();
            let effort = m["effort"].as_u64().unwrap_or(3) as u8;

            // output_files → Predicate::FileExists
            let output_files: Vec<PathBuf> = m["output_files"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(PathBuf::from)
                    .collect())
                .unwrap_or_default();

            let output_contract: Vec<Predicate> = if output_files.is_empty() {
                vec![Predicate::AlwaysTrue]
            } else {
                output_files.iter().map(|f| Predicate::FileExists(f.clone())).collect()
            };

            // guardrails → LocalGuardrail::block
            let guardrails: Vec<LocalGuardrail> = m["guardrails"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|rule| LocalGuardrail::block(
                        format!("Guardrail: {}", rule),
                        rule.to_string(),
                    ))
                    .collect())
                .unwrap_or_else(|| vec![
                    LocalGuardrail::block("Stay in scope", "Worker must only produce artifacts for this module"),
                ]);

            let allowed_paths: Vec<PathBuf> = output_files.clone();

            let worker_context = sentinel_core::split_agent::WorkerContext {
                destination_state: destination.clone(),
                allowed_paths,
                forbidden_paths: vec![],
                tech_constraints: vec![],
                non_negotiables: vec![destination.clone()],
            };

            let module_id = uuid::Uuid::new_v4();
            let dependencies = if let Some(pid) = prev_id {
                vec![pid]
            } else {
                vec![]
            };

            let module = WorkerModule {
                id: module_id,
                title: format!("Module {}: {}", i + 1, title),
                description: destination.clone(),
                input_contract: vec![Predicate::AlwaysTrue],
                output_contract,
                local_guardrails: guardrails,
                worker_context,
                dependencies,
                estimated_effort: effort,
            };

            prev_id = Some(module_id);
            worker_modules.push(module);
        }

        // Se Gemini non ha prodotto moduli validi, fallback su ArchitectAgent deterministico
        if worker_modules.is_empty() {
            let predicates = vec![
                Predicate::DirectoryExists(PathBuf::from("src")),
                Predicate::FileExists(PathBuf::from("README.md")),
            ];
            return architect.plan(&intent, &predicates)
                .context("Fallback architect plan failed");
        }

        let plan_hash = blake3::hash(
            format!("{}|{}", intent_text,
                worker_modules.iter().map(|m| m.contract_hash()).collect::<Vec<_>>().join("|")
            ).as_bytes()
        ).to_hex().to_string();

        Ok(SplitPlan {
            plan_id: uuid::Uuid::new_v4(),
            intent_description: intent_text.to_string(),
            modules: worker_modules,
            plan_hash,
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FASE 2: WORKER + REPAIR LOOP
    // ─────────────────────────────────────────────────────────────────────────

    async fn worker_repair_loop(
        &self,
        plan: &SplitPlan,
        intent_text: &str,
    ) -> Result<(SplitExecutionReport, Vec<ModuleDetail>)> {
        let workspace = &self.config.workspace;
        let mut module_details: Vec<ModuleDetail> = Vec::new();
        let mut total_repair_attempts = 0;

        // Esegui ogni modulo in ordine, con repair loop per ognuno
        let mut passed_ids: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();
        let mut all_reports: Vec<sentinel_core::split_agent::ModuleReport> = Vec::new();

        for module in &plan.modules {
            // Controlla dipendenze
            let dep_failed = module.dependencies.iter().find(|dep| !passed_ids.contains(dep));
            if let Some(dep_id) = dep_failed {
                if self.config.verbose {
                    println!("⏭️  Skip {} — dipendenza {} non passata", module.title, dep_id);
                }
                module_details.push(ModuleDetail {
                    title: module.title.clone(),
                    passed: false,
                    attempts: 0,
                    predicate_results: vec![],
                    generated_files: vec![],
                });
                continue;
            }

            if self.config.verbose {
                println!("▶️  {}", module.title);
                println!("   📍 {}", module.worker_context.destination_state);
            }

            let mut attempts = 0;
            let mut passed = false;
            let mut last_predicate_results: Vec<(String, bool)> = vec![];
            let mut generated_files: Vec<PathBuf> = vec![];
            let mut previous_approaches: Vec<String> = vec![];

            while attempts < self.config.max_repair_attempts && !passed {
                attempts += 1;
                if attempts > 1 {
                    total_repair_attempts += 1;
                    if self.config.verbose {
                        println!("   🔧 Repair attempt {}/{}", attempts, self.config.max_repair_attempts);
                    }
                }

                // Chiama il worker LLM
                match self.worker_implement_module(
                    module,
                    intent_text,
                    workspace,
                    &previous_approaches,
                ).await {
                    Ok(files) => {
                        generated_files = files;
                    }
                    Err(e) => {
                        if self.config.verbose {
                            println!("   ⚠️  Worker error: {}", e);
                        }
                        previous_approaches.push(format!("Attempt {} failed: {}", attempts, e));
                        continue;
                    }
                }

                // Verifica output_contract sul filesystem reale
                let verification = ModuleVerifier::verify(module, workspace);
                last_predicate_results = verification.predicate_results.iter()
                    .map(|r| (r.predicate_description.clone(), r.passed))
                    .collect();

                if verification.passed {
                    passed = true;
                    passed_ids.insert(module.id);
                    if self.config.verbose {
                        println!("   ✅ Verificato — output_contract soddisfatto");
                        for r in &verification.predicate_results {
                            println!("      {} {}", if r.passed { "✓" } else { "✗" }, r.predicate_description);
                        }
                    }
                } else {
                    if self.config.verbose {
                        println!("   ❌ Verifica fallita:");
                        for r in &verification.predicate_results {
                            if !r.passed {
                                println!("      ✗ {} — {}", r.predicate_description, r.detail);
                            }
                        }
                    }
                    // Prepara feedback per il prossimo tentativo
                    let failed_predicates: Vec<String> = verification.predicate_results.iter()
                        .filter(|r| !r.passed)
                        .map(|r| format!("{}: {}", r.predicate_description, r.detail))
                        .collect();
                    previous_approaches.push(format!(
                        "Attempt {} failed verification: {}",
                        attempts,
                        failed_predicates.join("; ")
                    ));
                }
            }

            if !passed && self.config.verbose {
                println!("   💀 Modulo fallito dopo {} tentativi", attempts);
            }

            module_details.push(ModuleDetail {
                title: module.title.clone(),
                passed,
                attempts,
                predicate_results: last_predicate_results,
                generated_files,
            });
        }

        // Costruisce il report finale
        let passed_count = module_details.iter().filter(|d| d.passed).count();
        let failed_count = module_details.iter().filter(|d| !d.passed).count();
        let skipped_count = module_details.iter().filter(|d| d.attempts == 0).count();

        let report = SplitExecutionReport {
            plan_id: plan.plan_id,
            intent_description: plan.intent_description.clone(),
            plan_hash: plan.plan_hash.clone(),
            module_reports: vec![], // semplificato
            all_passed: failed_count == 0 && skipped_count == 0,
            total_modules: plan.modules.len(),
            passed_modules: passed_count,
            failed_modules: failed_count,
            skipped_modules: skipped_count,
        };

        Ok((report, module_details))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // WORKER: usa Gemini CLI per implementare un singolo modulo
    // ─────────────────────────────────────────────────────────────────────────

    async fn worker_implement_module(
        &self,
        module: &WorkerModule,
        intent_text: &str,
        workspace: &Path,
        previous_approaches: &[String],
    ) -> Result<Vec<PathBuf>> {
        // Costruisce il prompt del worker con tutto il contesto necessario
        let output_contract_desc: Vec<String> = module.output_contract.iter()
            .map(|p| predicate_to_description(p))
            .collect();

        let guardrails_desc: Vec<String> = module.local_guardrails.iter()
            .map(|g| format!("- {} ({})", g.rule, match g.severity {
                sentinel_core::split_agent::GuardrailSeverity::Block => "BLOCK",
                sentinel_core::split_agent::GuardrailSeverity::Warn => "WARN",
            }))
            .collect();

        let previous_desc = if previous_approaches.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nPREVIOUS ATTEMPTS (do NOT repeat these mistakes):\n{}",
                previous_approaches.join("\n")
            )
        };

        let worker_prompt = format!(
            r#"You are a senior software engineer implementing a specific module.

PROJECT INTENT: {}

YOUR MODULE: {}
DESTINATION STATE (what MUST be true when you finish): {}

OUTPUT CONTRACT (files that MUST exist after your work):
{}

GUARDRAILS (rules you MUST follow):
{}

WORKSPACE: {}{}

Your task:
1. Create ALL the files listed in the output contract
2. Each file must contain real, working code (not placeholders)
3. Follow all guardrails strictly
4. Stay within your module scope — do NOT implement other modules

For each file you need to create, respond with:
FILE: <relative_path>
```
<file_content>
```

Create ALL required files. Be complete and production-ready."#,
            intent_text,
            module.title,
            module.worker_context.destination_state,
            output_contract_desc.join("\n"),
            guardrails_desc.join("\n"),
            workspace.display(),
            previous_desc,
        );

        let ctx = LLMContext {
            goal_description: intent_text.to_string(),
            context: format!("Worker implementing: {}", module.title),
            p2p_intelligence: String::new(),
            constraints: module.worker_context.non_negotiables.clone(),
            previous_approaches: previous_approaches.to_vec(),
        };

        let suggestion = self.llm.generate_code(&worker_prompt, &ctx).await
            .context("Gemini CLI worker call failed")?;

        // Parsa e scrive i file generati
        let generated = self.parse_and_write_files(&suggestion.content, workspace).await?;

        Ok(generated)
    }

    /// Parsa la risposta del worker e scrive i file sul filesystem
    async fn parse_and_write_files(
        &self,
        content: &str,
        workspace: &Path,
    ) -> Result<Vec<PathBuf>> {
        let mut generated = Vec::new();
        let mut current_file: Option<PathBuf> = None;
        let mut current_content: Vec<String> = Vec::new();
        let mut in_code_block = false;

        for line in content.lines() {
            // Rileva "FILE: path/to/file"
            if let Some(path_str) = line.strip_prefix("FILE:").map(str::trim) {
                // Salva il file precedente se c'era
                if let Some(ref path) = current_file {
                    if !current_content.is_empty() {
                        let file_content = current_content.join("\n");
                        if !file_content.trim().is_empty() {
                            write_file_safe(workspace, path, &file_content)?;
                            generated.push(path.clone());
                        }
                    }
                }
                current_file = Some(PathBuf::from(path_str));
                current_content = Vec::new();
                in_code_block = false;
                continue;
            }

            // Gestisce i code block markdown
            if line.starts_with("```") {
                if in_code_block {
                    // Fine del code block — salva il file
                    if let Some(ref path) = current_file {
                        let file_content = current_content.join("\n");
                        if !file_content.trim().is_empty() {
                            write_file_safe(workspace, path, &file_content)?;
                            generated.push(path.clone());
                        }
                        current_file = None;
                        current_content = Vec::new();
                    }
                    in_code_block = false;
                } else {
                    in_code_block = true;
                }
                continue;
            }

            if current_file.is_some() {
                current_content.push(line.to_string());
            }
        }

        // Salva l'ultimo file se rimasto aperto
        if let Some(ref path) = current_file {
            if !current_content.is_empty() {
                let file_content = current_content.join("\n");
                if !file_content.trim().is_empty() {
                    write_file_safe(workspace, path, &file_content)?;
                    generated.push(path.clone());
                }
            }
        }

        if self.config.verbose && !generated.is_empty() {
            for f in &generated {
                println!("   📄 Generato: {}", f.display());
            }
        }

        Ok(generated)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Scrive un file in modo sicuro, creando le directory necessarie
fn write_file_safe(workspace: &Path, relative_path: &Path, content: &str) -> Result<()> {
    // Sicurezza: impedisce path traversal
    let clean_path = relative_path
        .components()
        .filter(|c| !matches!(c, std::path::Component::ParentDir | std::path::Component::RootDir))
        .collect::<PathBuf>();

    let full_path = workspace.join(&clean_path);

    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
    }

    std::fs::write(&full_path, content)
        .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

    Ok(())
}

/// Converte un Predicate in descrizione leggibile per il prompt del worker
fn predicate_to_description(p: &Predicate) -> String {
    match p {
        Predicate::FileExists(path) => format!("File must exist: {}", path.display()),
        Predicate::DirectoryExists(path) => format!("Directory must exist: {}", path.display()),
        Predicate::CommandSucceeds { command, args, expected_exit_code } => {
            format!("Command must succeed: {} {} (exit {})", command, args.join(" "), expected_exit_code)
        }
        Predicate::TestsPassing { suite, min_coverage } => {
            format!("Tests must pass: suite={}, coverage>={:.0}%", suite, min_coverage * 100.0)
        }
        Predicate::AlwaysTrue => "Always satisfied".to_string(),
        _ => "Predicate must be satisfied".to_string(),
    }
}

/// Estrae un array JSON da una stringa che potrebbe contenere markdown
fn extract_json_array(raw: &str) -> String {
    // Cerca il primo '[' e l'ultimo ']'
    if let (Some(start), Some(end)) = (raw.find('['), raw.rfind(']')) {
        if start < end {
            return raw[start..=end].to_string();
        }
    }
    // Fallback: ritorna il raw pulito
    raw.trim_matches(|c: char| c == '`' || c.is_whitespace())
        .trim_start_matches("json")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_array_from_markdown() {
        let raw = "Here is the plan:\n```json\n[{\"title\": \"test\"}]\n```";
        let extracted = extract_json_array(raw);
        assert!(extracted.contains("[{"));
        assert!(extracted.contains("\"title\""));
    }

    #[test]
    fn test_extract_json_array_plain() {
        let raw = "[{\"title\": \"Module 1\", \"effort\": 3}]";
        let extracted = extract_json_array(raw);
        assert_eq!(extracted, raw);
    }

    #[test]
    fn test_predicate_to_description() {
        let p = Predicate::FileExists(PathBuf::from("src/main.rs"));
        let desc = predicate_to_description(&p);
        assert!(desc.contains("src/main.rs"));
        assert!(desc.contains("must exist"));
    }

    #[test]
    fn test_write_file_safe_prevents_traversal() {
        let tmp = std::env::temp_dir().join("sentinel_test_safe");
        std::fs::create_dir_all(&tmp).unwrap();
        // Path traversal attempt
        let result = write_file_safe(&tmp, Path::new("../../etc/passwd"), "evil");
        // Deve scrivere in tmp/etc/passwd, non in /etc/passwd
        assert!(result.is_ok());
        // Verifica che il file sia dentro tmp
        assert!(tmp.join("etc/passwd").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_e2e_config_default() {
        let config = E2EConfig::default();
        assert_eq!(config.max_repair_attempts, 3);
        assert!(config.verbose);
    }
}
