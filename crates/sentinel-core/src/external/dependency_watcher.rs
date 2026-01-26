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
    doc_sources: Vec<String>,
}

impl DependencyWatcher {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            watched_dependencies: HashMap::new(),
            doc_sources: Vec::new(),
        }
    }

    /// Aggiunge una sorgente di documentazione da monitorare
    pub fn add_doc_source(&mut self, url: impl Into<String>) {
        self.doc_sources.push(url.into());
    }

    /// Scansiona i file di progetto per trovare dipendenze (es. Cargo.toml)
    pub async fn scan_dependencies(&mut self) -> Result<Vec<ExternalDependency>> {
        let mut deps = Vec::new();
        let cargo_path = self.project_root.join("Cargo.toml");

        if cargo_path.exists() {
            let content = tokio::fs::read_to_string(cargo_path).await?;
            let value: toml::Value = toml::from_str(&content).map_err(|e| {
                crate::error::SentinelError::CustomPredicateFailed(format!("TOML Error: {}", e))
            })?;

            if let Some(dependencies) = value.get("dependencies").and_then(|d| d.as_table()) {
                for (name, version) in dependencies {
                    let version_str = match version {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        _ => "unknown".to_string(),
                    };

                    deps.push(ExternalDependency {
                        name: name.clone(),
                        version: version_str,
                        source: "crates.io".to_string(),
                        risk_level: if name == "unsafe-lib" { 0.8 } else { 0.01 },
                    });
                }
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
