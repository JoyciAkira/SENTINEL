use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Compliance modes for external context providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextComplianceMode {
    Disabled,
    InternalOnly,
    ByoCustomer,
}

impl Default for ContextComplianceMode {
    fn default() -> Self {
        Self::Disabled
    }
}

/// Tenant classification used for policy enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantMode {
    Internal,
    SingleTenantCustomer,
    MultiTenantHosted,
}

impl Default for TenantMode {
    fn default() -> Self {
        Self::Internal
    }
}

/// Credential provenance used for compliance decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialOrigin {
    UserProvided,
    PlatformManaged,
    None,
}

impl Default for CredentialOrigin {
    fn default() -> Self {
        Self::None
    }
}

/// Logical context providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextProviderKind {
    NativeMemory,
    OssVector,
    CodeGraph,
    AugmentMcp,
}

impl ContextProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NativeMemory => "native_memory",
            Self::OssVector => "oss_vector",
            Self::CodeGraph => "code_graph",
            Self::AugmentMcp => "augment_mcp",
        }
    }
}

/// Provider health state used for deterministic routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextProviderHealth {
    Healthy,
    Degraded,
    Unavailable,
}

/// Generic context provider interface.
pub trait ContextProvider: Send + Sync {
    fn kind(&self) -> ContextProviderKind;
    fn health(&self) -> ContextProviderHealth;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProviderPolicy {
    pub augment_mode: ContextComplianceMode,
    pub tenant_mode: TenantMode,
    pub credential_origin: CredentialOrigin,
    pub allow_augment_in_multitenant: bool,
    pub require_customer_credentials: bool,
}

impl Default for ContextProviderPolicy {
    fn default() -> Self {
        Self {
            augment_mode: ContextComplianceMode::Disabled,
            tenant_mode: TenantMode::Internal,
            credential_origin: CredentialOrigin::None,
            allow_augment_in_multitenant: false,
            require_customer_credentials: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRoutingEvent {
    pub timestamp: DateTime<Utc>,
    pub selected_provider: String,
    pub fallback_provider: Option<String>,
    pub denied_provider: Option<String>,
    pub reason: Option<String>,
    pub policy_mode: ContextComplianceMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextRoutingStats {
    pub total_routes: u64,
    pub fallback_routes: u64,
    pub policy_denials: u64,
}

#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub selected: ContextProviderKind,
    pub fallback_from: Option<ContextProviderKind>,
    pub denied_provider: Option<ContextProviderKind>,
    pub reason: Option<String>,
}

pub struct ContextProviderRouter {
    policy: ContextProviderPolicy,
    priority: Vec<ContextProviderKind>,
    registry: HashMap<ContextProviderKind, Box<dyn ContextProvider>>,
    stats: ContextRoutingStats,
    events: Vec<ContextRoutingEvent>,
}

impl ContextProviderRouter {
    pub fn new() -> Self {
        let default_priority = vec![
            ContextProviderKind::OssVector,
            ContextProviderKind::CodeGraph,
            ContextProviderKind::NativeMemory,
            ContextProviderKind::AugmentMcp,
        ];
        let priority = priority_from_env().unwrap_or(default_priority);
        Self {
            policy: ContextProviderPolicy::default(),
            priority,
            registry: HashMap::new(),
            stats: ContextRoutingStats::default(),
            events: Vec::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Box<dyn ContextProvider>) {
        self.registry.insert(provider.kind(), provider);
    }

    pub fn set_policy(&mut self, policy: ContextProviderPolicy) {
        self.policy = policy;
    }

    pub fn policy(&self) -> &ContextProviderPolicy {
        &self.policy
    }

    pub fn set_priority(&mut self, priority: Vec<ContextProviderKind>) {
        if !priority.is_empty() {
            self.priority = priority;
        }
    }

    pub fn stats(&self) -> ContextRoutingStats {
        self.stats.clone()
    }

    pub fn events(&self) -> &[ContextRoutingEvent] {
        &self.events
    }

    pub fn route(&mut self) -> RouteDecision {
        self.stats.total_routes += 1;

        let mut denied: Option<(ContextProviderKind, String)> = None;
        for candidate in self.priority.clone() {
            if let Some(deny_reason) = self.deny_reason_for(candidate) {
                denied = Some((candidate, deny_reason));
                continue;
            }

            if let Some(provider) = self.registry.get(&candidate) {
                if provider.health() == ContextProviderHealth::Healthy {
                    let event = ContextRoutingEvent {
                        timestamp: Utc::now(),
                        selected_provider: candidate.as_str().to_string(),
                        fallback_provider: None,
                        denied_provider: denied.as_ref().map(|(k, _)| k.as_str().to_string()),
                        reason: denied.as_ref().map(|(_, r)| r.clone()),
                        policy_mode: self.policy.augment_mode,
                    };
                    if denied.is_some() {
                        self.stats.policy_denials += 1;
                    }
                    self.events.push(event);
                    self.trim_events();

                    return RouteDecision {
                        selected: candidate,
                        fallback_from: None,
                        denied_provider: denied.as_ref().map(|(k, _)| *k),
                        reason: denied.as_ref().map(|(_, r)| r.clone()),
                    };
                }
            }
        }

        // Deterministic fallback to native memory.
        self.stats.fallback_routes += 1;
        if denied.is_some() {
            self.stats.policy_denials += 1;
        }

        let decision = RouteDecision {
            selected: ContextProviderKind::NativeMemory,
            fallback_from: Some(ContextProviderKind::AugmentMcp),
            denied_provider: denied.as_ref().map(|(k, _)| *k),
            reason: denied.as_ref().map(|(_, r)| r.clone()),
        };

        self.events.push(ContextRoutingEvent {
            timestamp: Utc::now(),
            selected_provider: decision.selected.as_str().to_string(),
            fallback_provider: decision.fallback_from.map(|k| k.as_str().to_string()),
            denied_provider: decision.denied_provider.map(|k| k.as_str().to_string()),
            reason: decision.reason.clone(),
            policy_mode: self.policy.augment_mode,
        });
        self.trim_events();

        decision
    }

    fn deny_reason_for(&self, provider: ContextProviderKind) -> Option<String> {
        if provider != ContextProviderKind::AugmentMcp {
            return None;
        }

        if self.policy.augment_mode == ContextComplianceMode::Disabled {
            return Some("augment_disabled".to_string());
        }

        if self.policy.tenant_mode == TenantMode::MultiTenantHosted
            && !self.policy.allow_augment_in_multitenant
        {
            return Some("augment_blocked_multi_tenant".to_string());
        }

        if self.policy.require_customer_credentials
            && self.policy.credential_origin != CredentialOrigin::UserProvided
        {
            return Some("augment_requires_byo_credentials".to_string());
        }

        if self.policy.augment_mode == ContextComplianceMode::InternalOnly
            && self.policy.tenant_mode != TenantMode::Internal
        {
            return Some("augment_internal_only".to_string());
        }

        None
    }

    fn trim_events(&mut self) {
        const MAX_EVENTS: usize = 256;
        if self.events.len() > MAX_EVENTS {
            let drain = self.events.len().saturating_sub(MAX_EVENTS);
            self.events.drain(0..drain);
        }
    }
}

impl Default for ContextProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

fn priority_from_env() -> Option<Vec<ContextProviderKind>> {
    let raw = std::env::var("SENTINEL_CONTEXT_PROVIDER_PRIORITY").ok()?;
    let mut ordered = Vec::<ContextProviderKind>::new();
    for token in raw.split(',') {
        let key = token.trim().to_ascii_lowercase();
        let mapped = match key.as_str() {
            "qdrant_mcp" | "qdrant" | "oss_vector" => Some(ContextProviderKind::OssVector),
            "filesystem_mcp" | "git_mcp" | "code_graph" => Some(ContextProviderKind::CodeGraph),
            "memory_mcp" | "native_memory" | "memory" => Some(ContextProviderKind::NativeMemory),
            "augment_mcp" | "augment" => Some(ContextProviderKind::AugmentMcp),
            _ => None,
        };
        if let Some(kind) = mapped {
            if !ordered.contains(&kind) {
                ordered.push(kind);
            }
        }
    }
    if ordered.is_empty() {
        None
    } else {
        Some(ordered)
    }
}

impl std::fmt::Debug for ContextProviderRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextProviderRouter")
            .field("policy", &self.policy)
            .field("priority", &self.priority)
            .field("stats", &self.stats)
            .field("events_len", &self.events.len())
            .finish()
    }
}

/// Native memory provider placeholder (always available).
pub struct NativeMemoryProvider;

impl ContextProvider for NativeMemoryProvider {
    fn kind(&self) -> ContextProviderKind {
        ContextProviderKind::NativeMemory
    }

    fn health(&self) -> ContextProviderHealth {
        ContextProviderHealth::Healthy
    }
}

/// Future OSS vector provider placeholder.
pub struct OssVectorProvider;

impl ContextProvider for OssVectorProvider {
    fn kind(&self) -> ContextProviderKind {
        ContextProviderKind::OssVector
    }

    fn health(&self) -> ContextProviderHealth {
        ContextProviderHealth::Unavailable
    }
}

/// Future code graph provider placeholder.
pub struct CodeGraphProvider;

impl ContextProvider for CodeGraphProvider {
    fn kind(&self) -> ContextProviderKind {
        ContextProviderKind::CodeGraph
    }

    fn health(&self) -> ContextProviderHealth {
        ContextProviderHealth::Unavailable
    }
}

/// Future Augment MCP provider.
pub struct AugmentMcpProvider {
    health: ContextProviderHealth,
}

impl AugmentMcpProvider {
    pub fn unavailable() -> Self {
        Self {
            health: ContextProviderHealth::Unavailable,
        }
    }

    pub fn healthy() -> Self {
        Self {
            health: ContextProviderHealth::Healthy,
        }
    }
}

impl ContextProvider for AugmentMcpProvider {
    fn kind(&self) -> ContextProviderKind {
        ContextProviderKind::AugmentMcp
    }

    fn health(&self) -> ContextProviderHealth {
        self.health
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AugmentMcpConfig {
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
}

impl Default for AugmentMcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: "auggie".to_string(),
            args: vec!["--mcp".to_string(), "--mcp-auto-workspace".to_string()],
            timeout_ms: 3500,
        }
    }
}

impl AugmentMcpConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();

        if let Ok(v) = std::env::var("SENTINEL_AUGMENT_ENABLED") {
            cfg.enabled = matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("SENTINEL_AUGMENT_COMMAND") {
            if !v.trim().is_empty() {
                cfg.command = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("SENTINEL_AUGMENT_ARGS") {
            let args = v
                .split_whitespace()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            if !args.is_empty() {
                cfg.args = args;
            }
        }
        if let Ok(v) = std::env::var("SENTINEL_AUGMENT_TIMEOUT_MS") {
            if let Ok(ms) = v.parse::<u64>() {
                cfg.timeout_ms = ms.max(500);
            }
        }

        cfg
    }
}

#[derive(Debug, Clone)]
pub struct ExternalContextChunk {
    pub text: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct AugmentMcpClient {
    config: AugmentMcpConfig,
}

impl AugmentMcpClient {
    pub fn new(config: AugmentMcpConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &AugmentMcpConfig {
        &self.config
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn is_available(&self) -> bool {
        if !self.config.enabled {
            return false;
        }
        command_exists(&self.config.command)
    }

    pub async fn retrieve(
        &self,
        query: &str,
        workspace: &Path,
        limit: usize,
    ) -> anyhow::Result<Vec<ExternalContextChunk>> {
        if !self.is_enabled() {
            anyhow::bail!("augment_mcp_disabled");
        }

        let mut child = Command::new(&self.config.command)
            .args(&self.config.args)
            .current_dir(workspace)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("augment_mcp_no_stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("augment_mcp_no_stdout"))?;
        let mut reader = BufReader::new(stdout);

        let init = serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{
                "protocolVersion":"2024-11-05",
                "capabilities":{},
                "clientInfo":{"name":"sentinel-agent-native","version":"0.1.0"}
            }
        });
        stdin
            .write_all((init.to_string() + "\n").as_bytes())
            .await?;
        stdin.flush().await?;
        let _ = read_json_line(&mut reader, self.config.timeout_ms).await?;

        let initialized = serde_json::json!({
            "jsonrpc":"2.0",
            "method":"notifications/initialized",
            "params":{}
        });
        stdin
            .write_all((initialized.to_string() + "\n").as_bytes())
            .await?;
        stdin.flush().await?;

        let tool_call = serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"tools/call",
            "params":{
                "name":"codebase-retrieval",
                "arguments":{
                    "query": query,
                    "directory_path": workspace.to_string_lossy(),
                    "top_k": limit.max(1)
                }
            }
        });
        stdin
            .write_all((tool_call.to_string() + "\n").as_bytes())
            .await?;
        stdin.flush().await?;

        let response = read_json_line(&mut reader, self.config.timeout_ms).await?;
        let _ = child.kill().await;

        parse_retrieval_chunks(&response)
    }
}

async fn read_json_line(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    timeout_ms: u64,
) -> anyhow::Result<Value> {
    let mut line = String::new();
    timeout(
        Duration::from_millis(timeout_ms),
        reader.read_line(&mut line),
    )
    .await??;
    if line.trim().is_empty() {
        anyhow::bail!("augment_mcp_empty_response");
    }
    let value: Value = serde_json::from_str(line.trim())?;
    Ok(value)
}

fn parse_retrieval_chunks(response: &Value) -> anyhow::Result<Vec<ExternalContextChunk>> {
    let result = response
        .get("result")
        .ok_or_else(|| anyhow::anyhow!("augment_mcp_missing_result"))?;
    let content = result
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("augment_mcp_missing_content"))?;

    let mut out = Vec::new();
    for entry in content {
        if let Some(text) = entry.get("text").and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(ExternalContextChunk {
                    text: trimmed.to_string(),
                    score: 0.65,
                });
            }
        }
    }

    if out.is_empty() {
        anyhow::bail!("augment_mcp_no_chunks")
    }

    Ok(out)
}

fn command_exists(command: &str) -> bool {
    if command.trim().is_empty() {
        return false;
    }
    std::process::Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {}", command))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_defaults_to_native_when_augment_disabled() {
        let mut router = ContextProviderRouter::new();
        router.register_provider(Box::new(AugmentMcpProvider::unavailable()));
        router.register_provider(Box::new(NativeMemoryProvider));
        router.set_priority(vec![
            ContextProviderKind::AugmentMcp,
            ContextProviderKind::NativeMemory,
        ]);

        let decision = router.route();
        assert_eq!(decision.selected, ContextProviderKind::NativeMemory);
        assert_eq!(decision.reason.as_deref(), Some("augment_disabled"));
        assert_eq!(router.stats().policy_denials, 1);
    }

    #[test]
    fn route_blocks_multi_tenant_without_override() {
        let mut router = ContextProviderRouter::new();
        router.register_provider(Box::new(AugmentMcpProvider::unavailable()));
        router.register_provider(Box::new(NativeMemoryProvider));
        router.set_priority(vec![
            ContextProviderKind::AugmentMcp,
            ContextProviderKind::NativeMemory,
        ]);
        router.set_policy(ContextProviderPolicy {
            augment_mode: ContextComplianceMode::ByoCustomer,
            tenant_mode: TenantMode::MultiTenantHosted,
            credential_origin: CredentialOrigin::UserProvided,
            allow_augment_in_multitenant: false,
            require_customer_credentials: true,
        });

        let decision = router.route();
        assert_eq!(decision.selected, ContextProviderKind::NativeMemory);
        assert_eq!(
            decision.reason.as_deref(),
            Some("augment_blocked_multi_tenant")
        );
    }

    #[test]
    fn route_requires_byo_credentials() {
        let mut router = ContextProviderRouter::new();
        router.register_provider(Box::new(AugmentMcpProvider::unavailable()));
        router.register_provider(Box::new(NativeMemoryProvider));
        router.set_priority(vec![
            ContextProviderKind::AugmentMcp,
            ContextProviderKind::NativeMemory,
        ]);
        router.set_policy(ContextProviderPolicy {
            augment_mode: ContextComplianceMode::ByoCustomer,
            tenant_mode: TenantMode::SingleTenantCustomer,
            credential_origin: CredentialOrigin::PlatformManaged,
            allow_augment_in_multitenant: false,
            require_customer_credentials: true,
        });

        let decision = router.route();
        assert_eq!(decision.selected, ContextProviderKind::NativeMemory);
        assert_eq!(
            decision.reason.as_deref(),
            Some("augment_requires_byo_credentials")
        );
    }

    #[test]
    fn parse_retrieval_chunks_extracts_text() {
        let payload = serde_json::json!({
            "jsonrpc":"2.0",
            "id":2,
            "result":{
                "content":[
                    {"type":"text","text":"chunk-a"},
                    {"type":"text","text":"chunk-b"}
                ]
            }
        });

        let chunks = parse_retrieval_chunks(&payload).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "chunk-a");
    }
}
