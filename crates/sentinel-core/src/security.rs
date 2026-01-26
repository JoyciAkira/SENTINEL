//! Real Security Scanner - Layer 7 Enforcement
//!
//! Scansiona il contenuto dei file alla ricerca di pattern pericolosi
//! (API Keys, Secrets, TODOs critici) prima della scrittura.

use regex::Regex;
use lazy_static::lazy_static;

pub struct SecurityReport {
    pub is_safe: bool,
    pub threats: Vec<String>,
    pub risk_score: f64, // 0.0 (Sicuro) - 1.0 (Critico)
}

lazy_static! {
    static ref PATTERNS: Vec<(&'static str, Regex, f64)> = vec![
        ("AWS Key", Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(), 1.0),
        ("Generic Private Key", Regex::new(r"(?i)private_key\s*=\s*['\u0022][^'\u0022]+['\u0022]").unwrap(), 1.0),
        ("Hardcoded Password", Regex::new(r"(?i)password\s*=\s*['\u0022][^'\u0022]+['\u0022]").unwrap(), 0.9),
        ("TODO Critical", Regex::new(r"(?i)TODO.*(security|fix|hack)").unwrap(), 0.3),
        ("IP Address Hardcoded", Regex::new(r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}").unwrap(), 0.2),
    ];
}

pub struct SecurityScanner;

impl SecurityScanner {
    /// Analizza il contenuto e restituisce un verdetto di sicurezza reale
    pub fn scan(content: &str) -> SecurityReport {
        let mut threats = Vec::new();
        let mut total_risk = 0.0;

        for (name, regex, severity) in PATTERNS.iter() {
            if regex.is_match(content) {
                threats.push(format!("Rilevato {}: Pattern insicuro trovato nel codice.", name));
                total_risk += severity;
            }
        }

        // Normalizzazione del rischio (max 1.0)
        if total_risk > 1.0 { total_risk = 1.0; }

        SecurityReport {
            is_safe: threats.is_empty(),
            threats,
            risk_score: total_risk,
        }
    }
}