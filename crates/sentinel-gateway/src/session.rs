//! Session management for isolated client sessions

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::channel::ChannelId;
use crate::{GatewayError, Result};

/// Unique session identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(format!("session:{}", Uuid::new_v4()))
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session activation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationMode {
    /// Always active, responds immediately
    AlwaysOn,
    /// Activated on demand
    OnDemand,
    /// Scheduled activation
    Scheduled,
}

/// Session queue mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMode {
    /// First-in, first-out
    Fifo,
    /// Priority-based
    Priority,
    /// Parallel execution
    Parallel,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Activation mode
    pub activation_mode: ActivationMode,

    /// Queue mode
    pub queue_mode: QueueMode,

    /// Maximum concurrent tasks
    pub max_concurrent: usize,

    /// Auto-reply to channel
    pub reply_back: bool,

    /// Session timeout in seconds
    pub timeout_secs: u64,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            activation_mode: ActivationMode::OnDemand,
            queue_mode: QueueMode::Fifo,
            max_concurrent: 1,
            reply_back: true,
            timeout_secs: 3600,
            metadata: HashMap::new(),
        }
    }
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is initializing
    Initializing,
    /// Session is active and ready
    Active,
    /// Session is busy processing
    Busy,
    /// Session is idle (waiting for input)
    Idle,
    /// Session is paused
    Paused,
    /// Session has ended
    Ended,
}

/// Task in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTask {
    /// Task ID
    pub id: Uuid,

    /// Task description
    pub description: String,

    /// Task status
    pub status: TaskStatus,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Started at
    pub started_at: Option<DateTime<Utc>>,

    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,

    /// Result (if completed)
    pub result: Option<serde_json::Value>,

    /// Error (if failed)
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Session - represents an isolated client session
#[derive(Debug, Clone)]
pub struct Session {
    /// Session ID
    pub id: SessionId,

    /// Session configuration
    pub config: SessionConfig,

    /// Current state
    pub state: SessionState,

    /// Associated channel
    pub channel_id: ChannelId,

    /// User ID (if authenticated)
    pub user_id: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Tasks in this session
    tasks: Vec<SessionTask>,

    /// Active tasks count
    active_tasks: usize,

    /// Session context (shared state)
    context: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(channel_id: ChannelId) -> Self {
        Self::with_config(channel_id, SessionConfig::default())
    }

    /// Create a session with custom configuration
    pub fn with_config(channel_id: ChannelId, config: SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            config,
            state: SessionState::Initializing,
            channel_id,
            user_id: None,
            created_at: now,
            last_activity: now,
            tasks: Vec::new(),
            active_tasks: 0,
            context: HashMap::new(),
        }
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Activate the session
    pub fn activate(&mut self) {
        self.state = SessionState::Active;
        self.touch();
    }

    /// Pause the session
    pub fn pause(&mut self) {
        self.state = SessionState::Paused;
        self.touch();
    }

    /// Resume the session
    pub fn resume(&mut self) {
        self.state = SessionState::Active;
        self.touch();
    }

    /// End the session
    pub fn end(&mut self) {
        self.state = SessionState::Ended;
        self.touch();
    }

    /// Update last activity
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        let elapsed = (Utc::now() - self.last_activity).num_seconds() as u64;
        elapsed > self.config.timeout_secs
    }

    /// Check if session can accept more tasks
    pub fn can_accept_task(&self) -> bool {
        matches!(self.state, SessionState::Active | SessionState::Idle | SessionState::Initializing)
            && self.active_tasks < self.config.max_concurrent
    }

    /// Add a task to the session
    pub fn add_task(&mut self, description: impl Into<String>) -> Uuid {
        let task = SessionTask {
            id: Uuid::new_v4(),
            description: description.into(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };
        let id = task.id;
        self.tasks.push(task);
        self.touch();
        id
    }

    /// Start a task
    pub fn start_task(&mut self, task_id: Uuid) -> Result<()> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Running;
            task.started_at = Some(Utc::now());
            self.active_tasks += 1;
            self.state = SessionState::Busy;
            self.touch();
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound(format!(
                "Task not found: {}",
                task_id
            )))
        }
    }

    /// Complete a task
    pub fn complete_task(&mut self, task_id: Uuid, result: serde_json::Value) -> Result<()> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(Utc::now());
            task.result = Some(result);
            self.active_tasks = self.active_tasks.saturating_sub(1);
            if self.active_tasks == 0 {
                self.state = SessionState::Idle;
            }
            self.touch();
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound(format!(
                "Task not found: {}",
                task_id
            )))
        }
    }

    /// Fail a task
    pub fn fail_task(&mut self, task_id: Uuid, error: impl Into<String>) -> Result<()> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Failed;
            task.completed_at = Some(Utc::now());
            task.error = Some(error.into());
            self.active_tasks = self.active_tasks.saturating_sub(1);
            if self.active_tasks == 0 {
                self.state = SessionState::Idle;
            }
            self.touch();
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound(format!(
                "Task not found: {}",
                task_id
            )))
        }
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: Uuid) -> Option<&SessionTask> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    /// Get all tasks
    pub fn tasks(&self) -> &[SessionTask] {
        &self.tasks
    }

    /// Set context value
    pub fn set_context(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.context.insert(key.into(), value);
        self.touch();
    }

    /// Get context value
    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    /// Get session info
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            state: self.state,
            channel_id: self.channel_id.clone(),
            user_id: self.user_id.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            active_tasks: self.active_tasks,
            total_tasks: self.tasks.len(),
        }
    }
}

/// Session summary info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub state: SessionState,
    pub channel_id: ChannelId,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub active_tasks: usize,
    pub total_tasks: usize,
}

/// Session manager - handles all sessions
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,

    /// Session by channel mapping
    channel_sessions: Arc<RwLock<HashMap<ChannelId, SessionId>>>,

    /// Session by user mapping
    user_sessions: Arc<RwLock<HashMap<String, Vec<SessionId>>>>,

    /// Maximum sessions per user
    max_sessions_per_user: usize,

    /// Default session timeout
    default_timeout_secs: u64,
}

impl SessionManager {
    pub fn new(max_sessions_per_user: usize, default_timeout_secs: u64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            channel_sessions: Arc::new(RwLock::new(HashMap::new())),
            user_sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions_per_user,
            default_timeout_secs,
        }
    }

    /// Create a new session
    pub fn create_session(&self, channel_id: ChannelId, config: Option<SessionConfig>) -> Result<SessionId> {
        let mut config = config.unwrap_or_default();
        config.timeout_secs = self.default_timeout_secs;

        let session = Session::with_config(channel_id.clone(), config);
        let session_id = session.id.clone();

        // Check user session limit
        if let Some(ref user_id) = session.user_id {
            let user_sessions = self.user_sessions.read();
            let count = user_sessions.get(user_id).map(|v| v.len()).unwrap_or(0);
            if count >= self.max_sessions_per_user {
                return Err(GatewayError::SessionNotFound(format!(
                    "Max sessions exceeded for user: {}",
                    user_id
                )));
            }
        }

        // Register session
        let mut sessions = self.sessions.write();
        sessions.insert(session_id.clone(), session);

        // Update mappings
        self.channel_sessions.write().insert(channel_id, session_id.clone());
        if let Some(user_id) = sessions.get(&session_id).and_then(|s| s.user_id.clone()) {
            self.user_sessions
                .write()
                .entry(user_id)
                .or_default()
                .push(session_id.clone());
        }

        tracing::info!("Session created: {}", session_id);
        Ok(session_id)
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &SessionId) -> Option<Session> {
        let sessions = self.sessions.read();
        sessions.get(id).cloned()
    }

    /// Get session by channel
    pub fn get_session_by_channel(&self, channel_id: &ChannelId) -> Option<Session> {
        let channel_sessions = self.channel_sessions.read();
        let session_id = channel_sessions.get(channel_id)?;
        self.get_session(session_id)
    }

    /// Update a session
    pub fn update_session<F>(&self, id: &SessionId, f: F) -> Result<()>
    where
        F: FnOnce(&mut Session),
    {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get_mut(id) {
            f(session);
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound(id.to_string()))
        }
    }

    /// End a session
    pub fn end_session(&self, id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.remove(id) {
            // Remove from channel mapping
            self.channel_sessions.write().remove(&session.channel_id);

            // Remove from user mapping
            if let Some(user_id) = &session.user_id {
                let mut user_sessions = self.user_sessions.write();
                if let Some(user_session_list) = user_sessions.get_mut(user_id) {
                    user_session_list.retain(|sid| sid != id);
                }
            }

            tracing::info!("Session ended: {}", id);
            Ok(())
        } else {
            Err(GatewayError::SessionNotFound(id.to_string()))
        }
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.write();
        let expired: Vec<SessionId> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired.len();
        for id in &expired {
            sessions.remove(id);
            tracing::info!("Session expired and removed: {}", id);
        }

        count
    }

    /// Get all active sessions
    pub fn active_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read();
        sessions.values().map(|s| s.info()).collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.read();
        sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let channel_id = ChannelId::new(crate::channel::ChannelType::VsCode);
        let session = Session::new(channel_id.clone());

        assert_eq!(session.state, SessionState::Initializing);
        assert!(session.user_id.is_none());
        assert!(session.can_accept_task());
    }

    #[test]
    fn test_session_lifecycle() {
        let channel_id = ChannelId::new(crate::channel::ChannelType::VsCode);
        let mut session = Session::new(channel_id);

        session.activate();
        assert_eq!(session.state, SessionState::Active);

        session.pause();
        assert_eq!(session.state, SessionState::Paused);

        session.resume();
        assert_eq!(session.state, SessionState::Active);

        session.end();
        assert_eq!(session.state, SessionState::Ended);
    }

    #[test]
    fn test_session_tasks() {
        let channel_id = ChannelId::new(crate::channel::ChannelType::VsCode);
        let mut session = Session::new(channel_id);
        session.activate();

        let task_id = session.add_task("Test task");
        let task = session.get_task(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Pending);

        session.start_task(task_id).unwrap();
        let task = session.get_task(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Running);
        assert_eq!(session.state, SessionState::Busy);

        session
            .complete_task(task_id, serde_json::json!({"result": "success"}))
            .unwrap();
        let task = session.get_task(task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.result.is_some());
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new(5, 3600);

        let channel_id = ChannelId::new(crate::channel::ChannelType::VsCode);
        let session_id = manager.create_session(channel_id, None).unwrap();

        assert!(manager.get_session(&session_id).is_some());
        assert_eq!(manager.session_count(), 1);

        manager.end_session(&session_id).unwrap();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_context() {
        let channel_id = ChannelId::new(crate::channel::ChannelType::VsCode);
        let mut session = Session::new(channel_id);

        session.set_context("key", serde_json::json!("value"));
        let value = session.get_context("key");
        assert_eq!(value, Some(&serde_json::json!("value")));
    }
}