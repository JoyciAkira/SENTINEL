//! Sentinel Atomic Sandbox - Isolated Execution Environment
//!
//! Permette di eseguire test e codice generato in un ambiente effimero
//! per garantire che non ci siano effetti collaterali prima dell'unione nel core.

use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::process::Command;

/// Ambiente di esecuzione isolato
pub struct Sandbox {
    /// Directory temporanea che verrà distrutta al drop
    #[allow(dead_code)]
    temp_dir: TempDir,
    /// Percorso radice del sandbox
    pub root_path: PathBuf,
}

/// Risultato di un'esecuzione nel sandbox
#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl Sandbox {
    /// Crea un nuovo sandbox vuoto
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("sentinel-sandbox-")
            .tempdir()?;

        Ok(Self {
            root_path: temp_dir.path().to_path_buf(),
            temp_dir,
        })
    }

    /// Prepara l'ambiente scrivendo file e directory
    pub fn prepare(&self, files: &[(PathBuf, String)]) -> Result<()> {
        for (path, content) in files {
            let full_path = self.root_path.join(path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full_path, content)?;
        }
        Ok(())
    }

    /// Copia un intero progetto nel sandbox filtrando i file non necessari
    pub fn mirror_project(&self, source_root: &PathBuf) -> Result<()> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(source_root).into_iter().filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git" && name != "node_modules"
        }) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let relative_path = path.strip_prefix(source_root)?;
                let dest_path = self.root_path.join(relative_path);

                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(path, dest_path)?;
            }
        }
        Ok(())
    }

    /// Esegue un comando nel sandbox
    pub async fn run(&self, cmd: &str, args: &[String]) -> Result<ExecutionResult> {
        let output = Command::new(cmd)
            .args(args)
            .current_dir(&self.root_path)
            .output()
            .await?;

        Ok(ExecutionResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    /// Verifica una "Atomic Truth" eseguendo i test unitari
    pub async fn verify_atomic_truth(&self) -> Result<bool> {
        // Se è un progetto Rust, esegui cargo test
        if self.root_path.join("Cargo.toml").exists() {
            let res = self
                .run("cargo", &["test".to_string(), "--quiet".to_string()])
                .await?;
            return Ok(res.success);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_lifecycle() -> Result<()> {
        let sandbox = Sandbox::new()?;
        let files = vec![(
            PathBuf::from("hello.sh"),
            "echo 'Hello Sentinel'".to_string(),
        )];
        sandbox.prepare(&files)?;

        let res = sandbox.run("sh", &["hello.sh".to_string()]).await?;
        assert!(res.success);
        assert_eq!(res.stdout.trim(), "Hello Sentinel");

        Ok(())
    }
}
