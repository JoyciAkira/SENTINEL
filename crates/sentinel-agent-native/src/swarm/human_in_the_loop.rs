//! Human-in-the-Loop System
//!
//! Allows human intervention at critical decision points:
//! - File modifications approval
//! - Architecture decisions
//! - Security-sensitive operations
//! - Cost threshold exceeded

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};

use crate::swarm::{AgentId};

/// Human approval system
pub struct HumanInTheLoop {
    /// Pending approvals
    pending: Arc<RwLock<HashMap<String, PendingApproval>>>,
    /// Channel for sending requests to UI
    request_tx: mpsc::UnboundedSender<ApprovalRequest>,
    /// Default timeout
    default_timeout: Duration,
    /// Auto-approve patterns (for trusted operations)
    auto_approve_patterns: Arc<RwLock<Vec<AutoApprovePattern>>>,
}

/// Pending approval with response channel
struct PendingApproval {
    request: ApprovalRequest,
    response_tx: oneshot::Sender<ApprovalResponse>,
}

/// Request for human approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub request_type: ApprovalType,
    pub title: String,
    pub description: String,
    pub requested_by: AgentId,
    pub timestamp: u64,
    pub timeout_secs: u64,
    pub metadata: ApprovalMetadata,
}

/// Type of approval needed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalType {
    FileModification,
    ArchitectureDecision,
    SecurityOperation,
    CostThreshold,
    ExternalApiCall,
    ToolExecution,
}

/// Metadata for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalMetadata {
    pub files_affected: Vec<String>,
    pub estimated_cost: Option<f64>,
    pub security_level: SecurityLevel,
    pub preview: Option<String>,
}

/// Security level for operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Human response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub request_id: String,
    pub approved: bool,
    pub comment: Option<String>,
    pub approved_by: Option<String>,
    pub timestamp: u64,
}

/// Pattern for auto-approval
pub struct AutoApprovePattern {
    pub request_type: ApprovalType,
    pub condition: Box<dyn Fn(&ApprovalRequest) -> bool + Send + Sync>,
}

impl HumanInTheLoop {
    /// Create new HITL system
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ApprovalRequest>) {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        
        let hitl = Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            request_tx,
            default_timeout: Duration::from_secs(300), // 5 minutes
            auto_approve_patterns: Arc::new(RwLock::new(Vec::new())),
        };
        
        (hitl, request_rx)
    }
    
    /// Request approval for an action
    pub async fn request_approval(&self, request: ApprovalRequest) -> Result<ApprovalResponse> {
        // Check auto-approve patterns
        if self.should_auto_approve(&request).await {
            return Ok(ApprovalResponse {
                request_id: request.id.clone(),
                approved: true,
                comment: Some("Auto-approved by pattern".to_string()),
                approved_by: Some("system".to_string()),
                timestamp: now(),
            });
        }
        
        // Get timeout before moving request
        let timeout_secs = request.timeout_secs;
        let request_id = request.id.clone();
        
        // Create oneshot channel for response
        let (response_tx, response_rx) = oneshot::channel();
        
        // Add to pending
        {
            let mut pending = self.pending.write().await;
            pending.insert(request_id.clone(), PendingApproval {
                request: request.clone(),
                response_tx,
            });
        }
        
        // Send to UI
        self.request_tx.send(request)
            .map_err(|_| anyhow!("Failed to send approval request"))?;
        
        // Wait for response with timeout
        let timeout = Duration::from_secs(timeout_secs);
        let response = tokio::time::timeout(timeout, response_rx).await
            .map_err(|_| anyhow!("Approval request timed out"))?
            .map_err(|_| anyhow!("Response channel closed"))?;
        
        // Remove from pending
        {
            let mut pending = self.pending.write().await;
            pending.remove(&request_id);
        }
        
        Ok(response)
    }
    
    /// Check if should auto-approve
    async fn should_auto_approve(&self, request: &ApprovalRequest) -> bool {
        let patterns = self.auto_approve_patterns.read().await;
        
        for pattern in patterns.iter() {
            if pattern.request_type == request.request_type && (pattern.condition)(request) {
                return true;
            }
        }
        
        false
    }
    
    /// Add auto-approve pattern
    pub async fn add_auto_approve_pattern(&self, pattern: AutoApprovePattern) {
        let mut patterns = self.auto_approve_patterns.write().await;
        patterns.push(pattern);
    }
    
    /// Get pending approvals
    pub async fn get_pending(&self) -> Vec<ApprovalRequest> {
        let pending = self.pending.read().await;
        pending.values().map(|p| p.request.clone()).collect()
    }
    
    /// Submit approval response
    pub async fn submit_response(&self, response: ApprovalResponse) -> Result<()> {
        let mut pending = self.pending.write().await;
        if let Some(approval) = pending.remove(&response.request_id) {
            approval.response_tx.send(response)
                .map_err(|_| anyhow!("Failed to send response - requester dropped"))?;
        }
        Ok(())
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_hitl_creation() {
        let (hitl, _rx) = HumanInTheLoop::new();
        let pending = hitl.get_pending().await;
        assert!(pending.is_empty());
    }
}
