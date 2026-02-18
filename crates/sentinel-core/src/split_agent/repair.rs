//! RepairOracle — diagnoses failed module contracts and suggests minimal RepairActions.
//!
//! 250 IQ insight: after a ModuleVerifier reports failure, instead of just "failed",
//! we analyze WHICH predicate failed and WHY, then produce a concrete RepairAction
//! that a worker (human or AI) can execute to bring the workspace into compliance.
//!
//! This is the difference between a compiler that says "error" and one that says
//! "did you mean...?" — but for AI agent goals.

use crate::goal_manifold::predicate::Predicate;
use crate::split_agent::{ModuleVerifier, WorkerModule};
use std::path::{Path, PathBuf};

/// A single, minimal, executable action to repair a failed predicate.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairAction {
    /// Create an empty file at the given path (creating parent dirs as needed).
    CreateFile(PathBuf),
    /// Create a directory (and all parents) at the given path.
    CreateDirectory(PathBuf),
    /// Run a command with args in the workspace root.
    RunCommand {
        command: String,
        args: Vec<String>,
        rationale: String,
    },
    /// The predicate requires an external service — repair is manual.
    ManualAction {
        description: String,
        predicate_description: String,
    },
    /// The predicate is AlwaysFalse — structurally unrepaiable.
    Unrepaiable {
        reason: String,
    },
}

impl RepairAction {
    /// Human-readable description of this repair action.
    pub fn describe(&self) -> String {
        match self {
            RepairAction::CreateFile(p) => format!("create file: {}", p.display()),
            RepairAction::CreateDirectory(p) => format!("create directory: {}", p.display()),
            RepairAction::RunCommand { command, args, rationale } => {
                format!("run: {} {} ({})", command, args.join(" "), rationale)
            }
            RepairAction::ManualAction { description, .. } => {
                format!("manual: {}", description)
            }
            RepairAction::Unrepaiable { reason } => {
                format!("unrepaiable: {}", reason)
            }
        }
    }

    /// Returns true if this action can be applied automatically (no human needed).
    pub fn is_automatable(&self) -> bool {
        matches!(
            self,
            RepairAction::CreateFile(_)
                | RepairAction::CreateDirectory(_)
                | RepairAction::RunCommand { .. }
        )
    }

    /// Apply this repair action to the given workspace root.
    /// Returns Ok(()) if applied successfully, Err(String) with details on failure.
    pub fn apply(&self, workspace_root: &Path) -> Result<(), String> {
        match self {
            RepairAction::CreateFile(rel_path) => {
                let full = if rel_path.is_absolute() {
                    rel_path.clone()
                } else {
                    workspace_root.join(rel_path)
                };
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir -p {:?}: {}", parent, e))?;
                }
                if !full.exists() {
                    std::fs::write(&full, b"")
                        .map_err(|e| format!("create {:?}: {}", full, e))?;
                }
                Ok(())
            }
            RepairAction::CreateDirectory(rel_path) => {
                let full = if rel_path.is_absolute() {
                    rel_path.clone()
                } else {
                    workspace_root.join(rel_path)
                };
                std::fs::create_dir_all(&full)
                    .map_err(|e| format!("mkdir -p {:?}: {}", full, e))
            }
            RepairAction::RunCommand { command, args, .. } => {
                let output = std::process::Command::new(command)
                    .args(args)
                    .current_dir(workspace_root)
                    .output()
                    .map_err(|e| format!("spawn {}: {}", command, e))?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "{} failed (exit {:?}): {}",
                        command,
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    ))
                }
            }
            RepairAction::ManualAction { description, .. } => {
                Err(format!("manual action required: {}", description))
            }
            RepairAction::Unrepaiable { reason } => {
                Err(format!("unrepaiable: {}", reason))
            }
        }
    }
}

/// Diagnosis of a failed module: which predicates failed and what to do about them.
#[derive(Debug, Clone)]
pub struct RepairDiagnosis {
    pub module_id: uuid::Uuid,
    pub module_title: String,
    /// One RepairAction per failed predicate (in order).
    pub actions: Vec<RepairAction>,
    /// True if all actions are automatable (no manual intervention needed).
    pub fully_automatable: bool,
}

impl RepairDiagnosis {
    /// Apply all automatable repair actions. Returns count of applied, count of skipped.
    pub fn apply_automatable(&self, workspace_root: &Path) -> (usize, usize) {
        let mut applied = 0;
        let mut skipped = 0;
        for action in &self.actions {
            if action.is_automatable() {
                match action.apply(workspace_root) {
                    Ok(()) => applied += 1,
                    Err(_) => skipped += 1,
                }
            } else {
                skipped += 1;
            }
        }
        (applied, skipped)
    }
}

/// Analyzes a failed WorkerModule and produces a RepairDiagnosis.
pub struct RepairOracle;

impl RepairOracle {
    /// Diagnose a failed module: re-run verification, identify failed predicates,
    /// produce minimal RepairActions.
    pub fn diagnose(module: &WorkerModule, workspace_root: &Path) -> RepairDiagnosis {
        let outcome = ModuleVerifier::verify(module, workspace_root);

        let actions: Vec<RepairAction> = module
            .output_contract
            .iter()
            .zip(outcome.predicate_results.iter())
            .filter(|(_, result)| !result.passed)
            .map(|(predicate, _result)| repair_for_predicate(predicate, workspace_root))
            .collect();

        let fully_automatable = actions.iter().all(|a| a.is_automatable());

        RepairDiagnosis {
            module_id: module.id,
            module_title: module.title.clone(),
            fully_automatable,
            actions,
        }
    }

    /// Apply all automatable repairs and re-verify.
    /// Returns true if the module passes after repair.
    pub fn repair_and_verify(module: &WorkerModule, workspace_root: &Path) -> bool {
        let diagnosis = Self::diagnose(module, workspace_root);
        diagnosis.apply_automatable(workspace_root);
        // Re-verify after repairs
        ModuleVerifier::verify(module, workspace_root).passed
    }
}

fn repair_for_predicate(predicate: &Predicate, _workspace_root: &Path) -> RepairAction {
    match predicate {
        Predicate::FileExists(path) => RepairAction::CreateFile(path.clone()),
        Predicate::DirectoryExists(path) => RepairAction::CreateDirectory(path.clone()),
        Predicate::CommandSucceeds { command, args, expected_exit_code: _ } => {
            RepairAction::RunCommand {
                command: command.clone(),
                args: args.clone(),
                rationale: format!("Re-run command to satisfy predicate"),
            }
        }
        Predicate::AlwaysFalse => RepairAction::Unrepaiable {
            reason: "AlwaysFalse predicate can never be satisfied".into(),
        },
        Predicate::AlwaysTrue => {
            // Should never fail, but if it does:
            RepairAction::Unrepaiable {
                reason: "AlwaysTrue predicate should always pass — internal error".into(),
            }
        }
        _ => RepairAction::ManualAction {
            description: format!("Manually satisfy: {:?}", predicate),
            predicate_description: format!("{:?}", predicate),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_manifold::{predicate::Predicate, Intent};
    use crate::split_agent::ArchitectAgent;
    use std::path::PathBuf;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("sentinel_repair_")
            .tempdir()
            .unwrap()
    }

    #[test]
    fn test_repair_oracle_diagnoses_missing_file() {
        let tmp = temp_dir();
        let root = tmp.path();

        let agent = ArchitectAgent::new();
        let intent = Intent::new("test", vec!["Rust"]);
        let predicates = vec![Predicate::FileExists(PathBuf::from("missing.rs"))];
        let plan = agent.plan(&intent, &predicates).unwrap();
        let module = &plan.modules[0];

        let diagnosis = RepairOracle::diagnose(module, root);

        assert_eq!(diagnosis.actions.len(), 1);
        assert!(matches!(diagnosis.actions[0], RepairAction::CreateFile(_)));
        assert!(diagnosis.fully_automatable);
    }

    #[test]
    fn test_repair_oracle_apply_creates_file() {
        let tmp = temp_dir();
        let root = tmp.path();

        let agent = ArchitectAgent::new();
        let intent = Intent::new("test", vec!["Rust"]);
        let predicates = vec![Predicate::FileExists(PathBuf::from("auto_created.rs"))];
        let plan = agent.plan(&intent, &predicates).unwrap();
        let module = &plan.modules[0];

        assert!(!root.join("auto_created.rs").exists());

        let passed = RepairOracle::repair_and_verify(module, root);

        assert!(passed, "repair should create the file and verification should pass");
        assert!(root.join("auto_created.rs").exists());
    }

    #[test]
    fn test_repair_oracle_diagnoses_missing_directory() {
        let tmp = temp_dir();
        let root = tmp.path();

        let agent = ArchitectAgent::new();
        let intent = Intent::new("test", vec!["Rust"]);
        let predicates = vec![Predicate::DirectoryExists(PathBuf::from("src/auth"))];
        let plan = agent.plan(&intent, &predicates).unwrap();
        let module = &plan.modules[0];

        let passed = RepairOracle::repair_and_verify(module, root);

        assert!(passed, "repair should create src/auth/ and verification should pass");
        assert!(root.join("src/auth").is_dir());
    }

    #[test]
    fn test_repair_action_describe() {
        let a = RepairAction::CreateFile(PathBuf::from("src/main.rs"));
        assert!(a.describe().contains("src/main.rs"));
        assert!(a.is_automatable());

        let b = RepairAction::ManualAction {
            description: "Deploy to production".into(),
            predicate_description: "api_endpoint(...)".into(),
        };
        assert!(!b.is_automatable());
    }

    #[test]
    fn test_repair_unrepaiable_always_false() {
        let tmp = temp_dir();
        let root = tmp.path();

        let agent = ArchitectAgent::new();
        let intent = Intent::new("test", Vec::<String>::new());
        let predicates = vec![Predicate::AlwaysFalse];
        let plan = agent.plan(&intent, &predicates).unwrap();
        let module = &plan.modules[0];

        let diagnosis = RepairOracle::diagnose(module, root);

        assert_eq!(diagnosis.actions.len(), 1);
        assert!(matches!(diagnosis.actions[0], RepairAction::Unrepaiable { .. }));
        assert!(!diagnosis.fully_automatable);
    }
}
