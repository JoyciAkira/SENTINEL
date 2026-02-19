//! Security layer for SENTINEL Gateway
//!
//! Implements DM pairing, allowlist management, and rate limiting
//! inspired by OpenClaw's security model.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{GatewayError, Result};

/// DM Policy for incoming messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DmPolicy {
    /// Require pairing code for new senders
    Pairing,
    /// Allow all messages
    Open,
    /// Reject all messages
    Closed,
}

impl Default for DmPolicy {
    fn default() -> Self {
        Self::Pairing
    }
}

/// User identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Pairing code for DM authorization
#[derive(Debug, Clone)]
pub struct PairingCode {
    /// The code
    pub code: String,

    /// User requesting pairing
    pub user_id: UserId,

    /// Channel type
    pub channel_type: String,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Expires at
    pub expires_at: DateTime<Utc>,
}

impl PairingCode {
    /// Generate a new pairing code
    pub fn generate(user_id: UserId, channel_type: impl Into<String>, ttl_secs: i64) -> Self {
        let now = Utc::now();
        let code = format!(
            "{:04}",
            rand::thread_rng().gen_range(1000..9999)
        );

        Self {
            code,
            user_id,
            channel_type: channel_type.into(),
            created_at: now,
            expires_at: now + chrono::Duration::seconds(ttl_secs),
        }
    }

    /// Check if code is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Security decision for a message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityDecision {
    /// Allow the message
    Allow,
    /// Request pairing
    RequestPairing,
    /// Reject the message
    Reject,
}

/// Rate limit entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

impl RateLimitEntry {
    fn new() -> Self {
        Self {
            count: 1,
            window_start: Instant::now(),
        }
    }

    fn increment(&mut self) -> u32 {
        self.count += 1;
        self.count
    }

    fn is_expired(&self, window: Duration) -> bool {
        self.window_start.elapsed() > window
    }
}

/// Channel security manager
pub struct ChannelSecurity {
    /// DM policy
    dm_policy: DmPolicy,

    /// Allowed users
    allowlist: Arc<RwLock<HashSet<UserId>>>,

    /// Pending pairing codes
    pending_pairings: Arc<RwLock<HashMap<String, PairingCode>>>,

    /// Rate limit entries
    rate_limits: Arc<RwLock<HashMap<UserId, RateLimitEntry>>>,

    /// Rate limit (requests per minute)
    rate_limit_per_minute: u32,

    /// Pairing code TTL (seconds)
    pairing_ttl_secs: i64,
}

impl ChannelSecurity {
    pub fn new(dm_policy: DmPolicy, rate_limit_per_minute: u32) -> Self {
        Self {
            dm_policy,
            allowlist: Arc::new(RwLock::new(HashSet::new())),
            pending_pairings: Arc::new(RwLock::new(HashMap::new())),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            rate_limit_per_minute,
            pairing_ttl_secs: 300, // 5 minutes
        }
    }

    /// Validate a sender
    pub fn validate_sender(&self, user_id: &UserId) -> SecurityDecision {
        match self.dm_policy {
            DmPolicy::Open => {
                if self.check_rate_limit(user_id) {
                    SecurityDecision::Allow
                } else {
                    SecurityDecision::Reject
                }
            }
            DmPolicy::Pairing => {
                if self.is_allowed(user_id) {
                    if self.check_rate_limit(user_id) {
                        SecurityDecision::Allow
                    } else {
                        SecurityDecision::Reject
                    }
                } else {
                    SecurityDecision::RequestPairing
                }
            }
            DmPolicy::Closed => SecurityDecision::Reject,
        }
    }

    /// Check if user is in allowlist
    pub fn is_allowed(&self, user_id: &UserId) -> bool {
        let allowlist = self.allowlist.read();
        allowlist.contains(user_id)
    }

    /// Add user to allowlist
    pub fn allow(&self, user_id: UserId) {
        let mut allowlist = self.allowlist.write();
        allowlist.insert(user_id.clone());
        tracing::info!("User allowed: {}", user_id);
    }

    /// Remove user from allowlist
    pub fn disallow(&self, user_id: &UserId) {
        let mut allowlist = self.allowlist.write();
        allowlist.remove(user_id);
        tracing::info!("User disallowed: {}", user_id);
    }

    /// Get allowlist count
    pub fn allowlist_count(&self) -> usize {
        self.allowlist.read().len()
    }

    /// Create a pairing request
    pub fn create_pairing(&self, user_id: UserId, channel_type: impl Into<String>) -> String {
        let pairing = PairingCode::generate(user_id, channel_type, self.pairing_ttl_secs);
        let code = pairing.code.clone();
        
        let mut pending = self.pending_pairings.write();
        pending.insert(code.clone(), pairing);
        
        tracing::info!("Pairing code created: {}", code);
        code
    }

    /// Approve a pairing code
    pub fn approve_pairing(&self, code: &str) -> Result<UserId> {
        let mut pending = self.pending_pairings.write();
        
        if let Some(pairing) = pending.remove(code) {
            if pairing.is_expired() {
                return Err(GatewayError::SecurityViolation(
                    "Pairing code expired".to_string(),
                ));
            }

            let user_id = pairing.user_id.clone();
            self.allow(user_id.clone());
            
            tracing::info!("Pairing approved for user: {}", user_id);
            Ok(user_id)
        } else {
            Err(GatewayError::SecurityViolation(
                "Invalid pairing code".to_string(),
            ))
        }
    }

    /// Check and update rate limit
    fn check_rate_limit(&self, user_id: &UserId) -> bool {
        let mut rate_limits = self.rate_limits.write();
        let window = Duration::from_secs(60);

        if let Some(entry) = rate_limits.get_mut(user_id) {
            if entry.is_expired(window) {
                // Reset window
                rate_limits.insert(user_id.clone(), RateLimitEntry::new());
                true
            } else {
                let count = entry.increment();
                count <= self.rate_limit_per_minute
            }
        } else {
            rate_limits.insert(user_id.clone(), RateLimitEntry::new());
            true
        }
    }

    /// Get rate limit status for a user
    pub fn rate_limit_status(&self, user_id: &UserId) -> (u32, u32) {
        let rate_limits = self.rate_limits.read();
        if let Some(entry) = rate_limits.get(user_id) {
            (entry.count, self.rate_limit_per_minute)
        } else {
            (0, self.rate_limit_per_minute)
        }
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        // Clean expired pairings
        let mut pending = self.pending_pairings.write();
        pending.retain(|_, p| !p.is_expired());
        
        // Clean expired rate limits
        let window = Duration::from_secs(60);
        let mut rate_limits = self.rate_limits.write();
        rate_limits.retain(|_, e| !e.is_expired(window));
    }

    /// Set DM policy
    pub fn set_policy(&mut self, policy: DmPolicy) {
        self.dm_policy = policy;
    }

    /// Get DM policy
    pub fn policy(&self) -> DmPolicy {
        self.dm_policy
    }
}

/// Security audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub event: AuditEvent,
    pub user_id: Option<UserId>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEvent {
    MessageAllowed,
    MessageRejected,
    PairingCreated,
    PairingApproved,
    PairingExpired,
    RateLimitExceeded,
    UserAllowed,
    UserDisallowed,
}

/// Security auditor
pub struct SecurityAuditor {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    max_entries: usize,
}

impl SecurityAuditor {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent, user_id: Option<UserId>, details: impl Into<String>) {
        let entry = AuditEntry {
            timestamp: Utc::now(),
            event,
            user_id,
            details: details.into(),
        };

        let mut entries = self.entries.write();
        entries.push(entry);

        // Trim if needed
        if entries.len() > self.max_entries {
            let remove_count = entries.len() - self.max_entries;
            entries.drain(0..remove_count);
        }
    }

    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(count).cloned().collect()
    }

    /// Get all entries
    pub fn all(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_code_generation() {
        let user = UserId::new("user-1");
        let code = PairingCode::generate(user.clone(), "vscode", 300);

        assert_eq!(code.code.len(), 4);
        assert!(code.code.chars().all(|c| c.is_numeric()));
        assert!(!code.is_expired());
    }

    #[test]
    fn test_pairing_code_expiry() {
        let user = UserId::new("user-1");
        let mut code = PairingCode::generate(user.clone(), "vscode", 300);
        code.expires_at = Utc::now() - chrono::Duration::seconds(1);
        assert!(code.is_expired());
    }

    #[tokio::test]
    async fn test_channel_security_pairing() {
        let security = ChannelSecurity::new(DmPolicy::Pairing, 60);
        let user = UserId::new("user-1");

        // User not in allowlist - should request pairing
        let decision = security.validate_sender(&user);
        assert_eq!(decision, SecurityDecision::RequestPairing);

        // Create and approve pairing
        let code = security.create_pairing(user.clone(), "vscode");
        let approved = security.approve_pairing(&code).unwrap();
        assert_eq!(approved, user);

        // Now user should be allowed
        let decision = security.validate_sender(&user);
        assert_eq!(decision, SecurityDecision::Allow);
    }

    #[tokio::test]
    async fn test_channel_security_open() {
        let security = ChannelSecurity::new(DmPolicy::Open, 60);
        let user = UserId::new("user-1");

        let decision = security.validate_sender(&user);
        assert_eq!(decision, SecurityDecision::Allow);
    }

    #[tokio::test]
    async fn test_channel_security_closed() {
        let security = ChannelSecurity::new(DmPolicy::Closed, 60);
        let user = UserId::new("user-1");

        let decision = security.validate_sender(&user);
        assert_eq!(decision, SecurityDecision::Reject);
    }

    #[test]
    fn test_rate_limiting() {
        let security = ChannelSecurity::new(DmPolicy::Open, 5);
        let user = UserId::new("user-1");

        // First 5 should be allowed
        for _ in 0..5 {
            assert!(security.check_rate_limit(&user));
        }

        // 6th should be denied
        assert!(!security.check_rate_limit(&user));
    }

    #[test]
    fn test_security_auditor() {
        let auditor = SecurityAuditor::new(100);

        auditor.log(
            AuditEvent::MessageAllowed,
            Some(UserId::new("user-1")),
            "Test message",
        );

        let recent = auditor.recent(10);
        assert_eq!(recent.len(), 1);
    }
}