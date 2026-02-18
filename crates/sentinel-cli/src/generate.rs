//! `sentinel generate` — Autonomous code generation pipeline.
//!
//! Pipeline: GoalManifold → heuristic path mapping → TreeSitterGenerator::create_file()
//!           → AlignmentField validation → status report
//!
//! This is what makes SENTINEL a Cline alternative: it generates code
//! autonomously from the Goal Manifold without human intervention.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use sentinel_agent_native::codegen::TreeSitterGenerator;
use sentinel_core::types::GoalStatus;
use sentinel_core::{AlignmentField, GoalManifold, ProjectState};

/// Options for the `sentinel generate` command.
pub struct GenerateOptions {
    pub goal_id: Option<String>,
    pub output: PathBuf,
    pub llm: bool,
    pub dry_run: bool,
}

/// Report produced at the end of a generation run.
#[derive(Debug)]
pub struct GenerationReport {
    pub goals_processed: usize,
    pub files_created: usize,
    pub files_skipped: usize,
    pub alignment_score_before: f64,
    pub alignment_score_after: f64,
    pub errors: Vec<String>,
}

/// Run the autonomous code generation pipeline.
///
/// For each non-completed goal in the manifold, derives a target path and calls
/// `TreeSitterGenerator::create_file()` to produce syntactically-correct Rust code
/// validated by Tree-Sitter AST analysis.
pub async fn run(manifold_path: &Path, opts: GenerateOptions) -> Result<GenerationReport> {
    let manifold = load_manifold(manifold_path)?;

    // --- Alignment check BEFORE generation ---
    let state_before = ProjectState::new(opts.output.clone());
    let field = AlignmentField::new(manifold.clone());
    let alignment_before = field
        .compute_alignment(&state_before)
        .await
        .context("alignment check before generation")?;

    println!(
        "▶ Starting code generation for: {}",
        manifold.root_intent.description
    );
    println!(
        "  Alignment before: {:.1}/100  Goals: {}",
        alignment_before.score,
        manifold.goal_count()
    );
    println!();

    // --- Collect goals to generate ---
    let goals: Vec<_> = manifold
        .goal_dag
        .goals()
        .filter(|g| {
            if let Some(ref id) = opts.goal_id {
                g.id.to_string() == *id
            } else {
                !matches!(g.status, GoalStatus::Completed | GoalStatus::Deprecated)
            }
        })
        .cloned()
        .collect();

    if goals.is_empty() {
        println!("⚠  No goals to generate code for.");
        return Ok(GenerationReport {
            goals_processed: 0,
            files_created: 0,
            files_skipped: 0,
            alignment_score_before: alignment_before.score,
            alignment_score_after: alignment_before.score,
            errors: vec![],
        });
    }

    println!("📋 {} goals queued for generation", goals.len());
    println!();

    // TreeSitterGenerator requires `&mut self` (holds parsers)
    let mut generator = TreeSitterGenerator::new()?;
    let mut files_created = 0;
    let mut files_skipped = 0;
    let mut errors: Vec<String> = Vec::new();

    for (idx, goal) in goals.iter().enumerate() {
        println!(
            "→ [{}/{}] {}",
            idx + 1,
            goals.len(),
            goal.description
        );

        // Map goal description to a target file path
        let module_name = snake_case_name(&goal.description);
        let rel_path = format!("src/{}/mod.rs", module_name);
        let full_path = opts.output.join(&rel_path);

        // Don't overwrite existing files
        if full_path.exists() {
            println!("  ○ skip (exists): {}", rel_path);
            files_skipped += 1;
            continue;
        }

        if opts.dry_run {
            println!("  ○ [DRY-RUN] would create: {}", rel_path);
            files_skipped += 1;
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating dirs for {}", rel_path))?;
        }

        // Use absolute path so TreeSitterGenerator writes to the right location
        let abs_path = full_path
            .canonicalize()
            .unwrap_or_else(|_| full_path.clone());
        let abs_str = abs_path.to_string_lossy();

        // Generate with TreeSitter — takes description as "content" seed
        match generator
            .create_file(&abs_str, &goal.description)
            .await
        {
            Ok(result) => {
                if result.success {
                    println!("  ✓ created: {} ({} syntax errors)", rel_path, result.syntax_errors.len());
                    files_created += 1;
                } else {
                    let err = format!(
                        "'{}' generated with errors: {}",
                        rel_path,
                        result.syntax_errors.join("; ")
                    );
                    println!("  ⚠ {}", err);
                    errors.push(err);
                    files_created += 1; // still counted — file was created
                }
            }
            Err(e) => {
                let msg = format!("codegen failed for '{}': {}", goal.description, e);
                println!("  ✗ {}", msg);
                errors.push(msg);
                // Clean up empty file if it was created before error
                let _ = std::fs::remove_file(&full_path);
            }
        }
    }

    // --- Alignment check AFTER generation ---
    let state_after = ProjectState::new(opts.output.clone());
    let field_after = AlignmentField::new(manifold.clone());
    let alignment_after = field_after
        .compute_alignment(&state_after)
        .await
        .unwrap_or(alignment_before.clone());

    println!();
    println!("{}", "─".repeat(60));
    println!(
        "{} Generation complete: {} created, {} skipped, {} errors",
        if errors.is_empty() { "✅" } else { "⚠️" },
        files_created,
        files_skipped,
        errors.len()
    );
    println!(
        "  Alignment: {:.1} → {:.1}",
        alignment_before.score, alignment_after.score
    );

    if !errors.is_empty() {
        println!("\nErrors:");
        for e in &errors {
            println!("  • {}", e);
        }
    }

    Ok(GenerationReport {
        goals_processed: goals.len(),
        files_created,
        files_skipped,
        alignment_score_before: alignment_before.score,
        alignment_score_after: alignment_after.score,
        errors,
    })
}

/// Converts a goal description to a snake_case module name (max 4 words).
fn snake_case_name(description: &str) -> String {
    description
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("_")
}

fn load_manifold(path: &Path) -> Result<GoalManifold> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("reading manifold: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing manifold: {}", path.display()))
}
