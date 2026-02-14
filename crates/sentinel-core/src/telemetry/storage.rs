//! Local storage for telemetry events

use super::TelemetryEvent;
use crate::error::{Result, SentinelError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// Local storage for telemetry data
pub struct TelemetryStorage {
    pub(crate) base_path: PathBuf,
}

impl TelemetryStorage {
    /// Create new telemetry storage
    pub async fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| SentinelError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Home directory not found",
            )))?;

        let base_path = std::path::PathBuf::from(home)
            .join(".sentinel")
            .join("telemetry");

        // Create directory if it doesn't exist
        tokio::fs::create_dir_all(&base_path).await?;

        Ok(Self { base_path })
    }

    /// Record a telemetry event
    pub async fn record_event(&self, event: TelemetryEvent) -> Result<()> {
        let events_path = self.base_path.join("events.jsonl");

        // Append event to JSONL file
        let line = serde_json::to_string(&event)?;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .await?;

        use tokio::io::AsyncWriteExt;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;

        Ok(())
    }

    /// Get all recorded events
    pub async fn get_all_events(&self) -> Result<Vec<TelemetryEvent>> {
        let events_path = self.base_path.join("events.jsonl");

        if !events_path.exists() {
            return Ok(Vec::new());
        }

        let content = tokio::fs::read_to_string(&events_path).await?;
        let mut events = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<TelemetryEvent>(line) {
                Ok(event) => events.push(event),
                Err(_) => continue, // Skip malformed lines
            }
        }

        Ok(events)
    }

    /// Get or create user ID
    pub async fn get_or_create_user_id(&self) -> Result<Uuid> {
        let user_id_path = self.base_path.join("user_id.txt");

        if user_id_path.exists() {
            let content = tokio::fs::read_to_string(&user_id_path).await?;
            let uuid_str = content.trim();
            Uuid::parse_str(uuid_str).map_err(|e| {
                SentinelError::InvalidInput(format!("Invalid UUID in user_id.txt: {}", e))
            })
        } else {
            let new_id = Uuid::new_v4();
            tokio::fs::write(&user_id_path, new_id.to_string()).await?;
            Ok(new_id)
        }
    }

    /// Get opt-in status
    pub async fn get_opt_in_status(&self) -> Result<bool> {
        let config_path = self.base_path.join("config.json");

        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path).await?;
            let config: TelemetryConfigFile = serde_json::from_str(&content)?;
            Ok(config.opt_in)
        } else {
            Ok(false) // Default to opt-out
        }
    }

    /// Set opt-in status
    pub async fn set_opt_in_status(&self, opt_in: bool) -> Result<()> {
        let config_path = self.base_path.join("config.json");
        let config = TelemetryConfigFile { opt_in };
        let content = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(&config_path, content).await?;
        Ok(())
    }

    /// Clear all telemetry data
    pub async fn clear_all(&self) -> Result<()> {
        let events_path = self.base_path.join("events.jsonl");

        if events_path.exists() {
            tokio::fs::remove_file(&events_path).await?;
        }

        Ok(())
    }

    /// Get storage path
    pub fn path(&self) -> &PathBuf {
        &self.base_path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TelemetryConfigFile {
    opt_in: bool,
}

/// In-memory telemetry store for testing
#[derive(Debug, Clone, Default)]
pub struct TelemetryStore {
    pub events: Vec<TelemetryEvent>,
    pub opt_in: bool,
    pub user_id: Option<Uuid>,
}

impl TelemetryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_event(&mut self, event: TelemetryEvent) {
        self.events.push(event);
    }

    pub fn get_events(&self) -> &[TelemetryEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn set_opt_in(&mut self, opt_in: bool) {
        self.opt_in = opt_in;
    }

    pub fn is_opted_in(&self) -> bool {
        self.opt_in
    }

    pub fn set_user_id(&mut self, id: Uuid) {
        self.user_id = Some(id);
    }

    pub fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_storage_record_and_retrieve() {
        let storage = TelemetryStorage::new().await.unwrap();

        // Create test event
        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event: crate::telemetry::OnboardingEvent::FirstRun {
                version: "test".to_string(),
            },
        };

        // Record event
        storage.record_event(event.clone()).await.unwrap();

        // Retrieve events
        let events = storage.get_all_events().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_user_id_persistence() {
        let storage = TelemetryStorage::new().await.unwrap();

        // Create new user ID
        let id1 = storage.get_or_create_user_id().await.unwrap();

        // Should return same ID on subsequent calls
        let id2 = storage.get_or_create_user_id().await.unwrap();

        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn test_opt_in_status() {
        let storage = TelemetryStorage::new().await.unwrap();

        // Clean up any existing config first
        let config_path = storage.base_path.join("config.json");
        let _ = tokio::fs::remove_file(&config_path).await;

        // Default is opt-out
        assert!(!storage.get_opt_in_status().await.unwrap());

        // Set opt-in
        storage.set_opt_in_status(true).await.unwrap();
        assert!(storage.get_opt_in_status().await.unwrap());

        // Set opt-out
        storage.set_opt_in_status(false).await.unwrap();
        assert!(!storage.get_opt_in_status().await.unwrap());
    }

    #[tokio::test]
    async fn test_clear_all_data() {
        let storage = TelemetryStorage::new().await.unwrap();

        // Add some events
        let event = TelemetryEvent {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event: crate::telemetry::OnboardingEvent::FirstRun {
                version: "test".to_string(),
            },
        };

        storage.record_event(event).await.unwrap();

        // Clear
        storage.clear_all().await.unwrap();

        // Verify cleared
        let events = storage.get_all_events().await.unwrap();
        assert_eq!(events.len(), 0);
    }
}
