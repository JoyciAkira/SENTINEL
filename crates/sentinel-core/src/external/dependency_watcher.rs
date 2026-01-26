//! Dependency Watcher - Monitora le dipendenze esterne
//!
//! Assicura che le librerie e i tool usati nel progetto siano allineati
//! con gli obiettivi di sicurezza e performance del Goal Manifold.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Rappresenta una dipendenza esterna monitorata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: String,
    pub source: String,
    pub risk_level: f64, // 0.0 - 1.0
}

/// Motore di monitoraggio dipendenze
pub struct DependencyWatcher {
    project_root: PathBuf,
    watched_dependencies: HashMap<String, ExternalDependency>,
}

impl DependencyWatcher {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            watched_dependencies: HashMap::new(),
        }
    }

    /// Scansiona i file di progetto per trovare dipendenze (es. Cargo.toml)
    pub async fn scan_dependencies(&mut self) -> Result<Vec<ExternalDependency>> {
        let mut deps = Vec::new();
        let cargo_path = self.project_root.join("Cargo.toml");

        if cargo_path.exists() {
            let content = tokio::fs::read_to_string(cargo_path).await?;
            // Logica di parsing semplificata per ora
            if content.contains("tokio") {
                deps.push(ExternalDependency {
                    name: "tokio".to_string(),
                    version: "1.35".to_string(),
                    source: "crates.io".to_string(),
                    risk_level: 0.05,
                });
            }
        }

        for dep in &deps {
            self.watched_dependencies.insert(dep.name.clone(), dep.clone());
        }

        Ok(deps)
    }

    /// Verifica se i cambiamenti nelle dipendenze creano un disallineamento
    pub fn check_alignment_risk(&self) -> f64 {
        // Se troviamo dipendenze con risk_level alto, restituiamo un punteggio di rischio
        self.watched_dependencies.values()
            .map(|d| d.risk_level)
            .sum::<f64>()
            .min(1.0)
    }
}
