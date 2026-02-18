//! ManifoldStore — SQLite WAL persistent storage for GoalManifold
//!
//! Implementa persistenza concurrency-safe con:
//! - WAL mode: letture concorrenti non bloccanti
//! - Versioning: ogni save crea un nuovo snapshot immutabile
//! - Integrità: Blake3 hash verificato al load
//! - Agent messages: ledger append-only per comunicazione inter-agente
//! - Episodes: memoria episodica persistente per DistributedMemory

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::GoalManifold;

/// Snapshot del manifold con metadati di versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldSnapshot {
    pub version: i64,
    pub integrity_hash: String,
    pub payload_json: String,
    pub saved_at_ms: i64,
    pub agent_id: Option<String>,
}

/// Messaggio inter-agente persistente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub message_type: String,
    pub payload_json: String,
    pub timestamp_ms: i64,
    pub session_id: Option<String>,
}

/// Episodio di memoria persistente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEpisode {
    pub id: String,
    pub agent_id: Option<String>,
    pub event_type: String,
    pub description: String,
    pub outcome: String,
    pub importance: f64,
    pub payload_json: String,
    pub timestamp_ms: i64,
}

/// Store SQLite WAL per GoalManifold, messaggi agenti ed episodi
pub struct ManifoldStore {
    conn: Connection,
}

impl ManifoldStore {
    /// Apre (o crea) il database SQLite in modalità WAL.
    ///
    /// # Esempio
    ///
    /// ```no_run
    /// use sentinel_core::storage::ManifoldStore;
    /// let store = ManifoldStore::open(".sentinel/sentinel.db").unwrap();
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Creazione directory DB '{}'", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Apertura SQLite '{}'", path.display()))?;

        // Abilita WAL mode: letture concorrenti, scritture serializzate
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
        .context("Configurazione PRAGMA SQLite WAL")?;

        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Esegue le migrazioni DDL idempotenti
    fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            -- Tabella snapshot manifold (versioned, immutable)
            CREATE TABLE IF NOT EXISTS manifold_snapshots (
                version       INTEGER PRIMARY KEY AUTOINCREMENT,
                integrity_hash TEXT NOT NULL,
                payload_json  TEXT NOT NULL,
                saved_at_ms   INTEGER NOT NULL,
                agent_id      TEXT
            );

            -- Indice per recupero rapido dell'ultimo snapshot
            CREATE INDEX IF NOT EXISTS idx_manifold_saved_at
                ON manifold_snapshots(saved_at_ms DESC);

            -- Tabella messaggi inter-agente (append-only ledger)
            CREATE TABLE IF NOT EXISTS agent_messages (
                id            TEXT PRIMARY KEY,
                from_agent    TEXT NOT NULL,
                to_agent      TEXT,
                message_type  TEXT NOT NULL,
                payload_json  TEXT NOT NULL,
                timestamp_ms  INTEGER NOT NULL,
                session_id    TEXT
            );

            -- Indice per query per agente e tipo
            CREATE INDEX IF NOT EXISTS idx_messages_from
                ON agent_messages(from_agent, timestamp_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_messages_to
                ON agent_messages(to_agent, timestamp_ms DESC);

            -- Tabella episodi di memoria (DistributedMemory)
            CREATE TABLE IF NOT EXISTS episodes (
                id            TEXT PRIMARY KEY,
                agent_id      TEXT,
                event_type    TEXT NOT NULL,
                description   TEXT NOT NULL,
                outcome       TEXT NOT NULL,
                importance    REAL NOT NULL DEFAULT 0.5,
                payload_json  TEXT NOT NULL,
                timestamp_ms  INTEGER NOT NULL
            );

            -- Indice per query per importanza e tempo
            CREATE INDEX IF NOT EXISTS idx_episodes_importance
                ON episodes(importance DESC, timestamp_ms DESC);
            ",
            )
            .context("Migrazione schema SQLite")?;
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Manifold snapshots
    // ─────────────────────────────────────────────────────────────────────────

    /// Salva un nuovo snapshot del manifold.
    /// Ogni chiamata crea una nuova versione immutabile (append-only).
    /// Restituisce il numero di versione assegnato.
    pub fn save_manifold(&self, manifold: &GoalManifold, agent_id: Option<&str>) -> Result<i64> {
        let payload_json = serde_json::to_string(manifold)
            .context("Serializzazione manifold per SQLite")?;
        let integrity_hash = manifold.integrity_hash.to_hex().to_string();
        let saved_at_ms = chrono::Utc::now().timestamp_millis();

        self.conn
            .execute(
                "INSERT INTO manifold_snapshots (integrity_hash, payload_json, saved_at_ms, agent_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![integrity_hash, payload_json, saved_at_ms, agent_id],
            )
            .context("INSERT manifold_snapshots")?;

        let version = self.conn.last_insert_rowid();
        Ok(version)
    }

    /// Carica l'ultimo snapshot del manifold.
    /// Verifica l'integrità Blake3 prima di restituire.
    pub fn load_latest_manifold(&self) -> Result<Option<GoalManifold>> {
        let result = self.conn.query_row(
            "SELECT payload_json, integrity_hash FROM manifold_snapshots
             ORDER BY saved_at_ms DESC LIMIT 1",
            [],
            |row| {
                let payload: String = row.get(0)?;
                let hash: String = row.get(1)?;
                Ok((payload, hash))
            },
        );

        match result {
            Ok((payload_json, stored_hash)) => {
                let manifold: GoalManifold = serde_json::from_str(&payload_json)
                    .context("Deserializzazione manifold da SQLite")?;

                // Verifica integrità
                let actual_hash = manifold.integrity_hash.to_hex().to_string();
                if actual_hash != stored_hash {
                    anyhow::bail!(
                        "Integrità manifold violata: hash atteso={}, trovato={}",
                        stored_hash,
                        actual_hash
                    );
                }

                Ok(Some(manifold))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("Query manifold_snapshots"),
        }
    }

    /// Carica uno snapshot specifico per versione
    pub fn load_manifold_version(&self, version: i64) -> Result<Option<ManifoldSnapshot>> {
        let result = self.conn.query_row(
            "SELECT version, integrity_hash, payload_json, saved_at_ms, agent_id
             FROM manifold_snapshots WHERE version = ?1",
            params![version],
            |row| {
                Ok(ManifoldSnapshot {
                    version: row.get(0)?,
                    integrity_hash: row.get(1)?,
                    payload_json: row.get(2)?,
                    saved_at_ms: row.get(3)?,
                    agent_id: row.get(4)?,
                })
            },
        );

        match result {
            Ok(snap) => Ok(Some(snap)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("Query manifold_snapshots by version"),
        }
    }

    /// Lista le ultime N versioni del manifold (metadati, senza payload).
    /// Ordina per `version DESC` (AUTOINCREMENT stabile, non dipende dal clock).
    pub fn list_manifold_versions(&self, limit: usize) -> Result<Vec<ManifoldSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT version, integrity_hash, '' as payload_json, saved_at_ms, agent_id
             FROM manifold_snapshots
             ORDER BY version DESC LIMIT ?1",
        )?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(ManifoldSnapshot {
                    version: row.get(0)?,
                    integrity_hash: row.get(1)?,
                    payload_json: row.get(2)?,
                    saved_at_ms: row.get(3)?,
                    agent_id: row.get(4)?,
                })
            })
            .context("Query list manifold versions")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("Raccolta versioni manifold")
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Agent messages (communication history reale)
    // ─────────────────────────────────────────────────────────────────────────

    /// Persiste un messaggio inter-agente nel ledger append-only
    pub fn append_agent_message(&self, msg: &AgentMessage) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO agent_messages
                 (id, from_agent, to_agent, message_type, payload_json, timestamp_ms, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    msg.id,
                    msg.from_agent,
                    msg.to_agent,
                    msg.message_type,
                    msg.payload_json,
                    msg.timestamp_ms,
                    msg.session_id
                ],
            )
            .context("INSERT agent_messages")?;
        Ok(())
    }

    /// Recupera la history dei messaggi, opzionalmente filtrata per agente
    pub fn get_agent_messages(
        &self,
        agent_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AgentMessage>> {
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(aid) = agent_id {
            (
                "SELECT id, from_agent, to_agent, message_type, payload_json, timestamp_ms, session_id
                 FROM agent_messages
                 WHERE from_agent = ?1 OR to_agent = ?1
                 ORDER BY timestamp_ms DESC LIMIT ?2"
                    .to_string(),
                vec![Box::new(aid.to_string()), Box::new(limit as i64)],
            )
        } else {
            (
                "SELECT id, from_agent, to_agent, message_type, payload_json, timestamp_ms, session_id
                 FROM agent_messages
                 ORDER BY timestamp_ms DESC LIMIT ?1"
                    .to_string(),
                vec![Box::new(limit as i64)],
            )
        };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())), |row| {
                Ok(AgentMessage {
                    id: row.get(0)?,
                    from_agent: row.get(1)?,
                    to_agent: row.get(2)?,
                    message_type: row.get(3)?,
                    payload_json: row.get(4)?,
                    timestamp_ms: row.get(5)?,
                    session_id: row.get(6)?,
                })
            })
            .context("Query agent_messages")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("Raccolta agent messages")
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Episodes (DistributedMemory persistence)
    // ─────────────────────────────────────────────────────────────────────────

    /// Persiste un episodio di memoria
    pub fn append_episode(&self, episode: &PersistedEpisode) -> Result<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO episodes
                 (id, agent_id, event_type, description, outcome, importance, payload_json, timestamp_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    episode.id,
                    episode.agent_id,
                    episode.event_type,
                    episode.description,
                    episode.outcome,
                    episode.importance,
                    episode.payload_json,
                    episode.timestamp_ms
                ],
            )
            .context("INSERT episodes")?;
        Ok(())
    }

    /// Recupera episodi per importanza decrescente
    pub fn get_episodes(
        &self,
        min_importance: f64,
        limit: usize,
    ) -> Result<Vec<PersistedEpisode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_id, event_type, description, outcome, importance, payload_json, timestamp_ms
             FROM episodes
             WHERE importance >= ?1
             ORDER BY importance DESC, timestamp_ms DESC
             LIMIT ?2",
        )?;

        let rows = stmt
            .query_map(params![min_importance, limit as i64], |row| {
                Ok(PersistedEpisode {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    event_type: row.get(2)?,
                    description: row.get(3)?,
                    outcome: row.get(4)?,
                    importance: row.get(5)?,
                    payload_json: row.get(6)?,
                    timestamp_ms: row.get(7)?,
                })
            })
            .context("Query episodes")?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("Raccolta episodes")
    }

    /// Statistiche del database
    pub fn stats(&self) -> Result<serde_json::Value> {
        let manifold_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM manifold_snapshots",
            [],
            |row| row.get(0),
        )?;
        let message_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM agent_messages",
            [],
            |row| row.get(0),
        )?;
        let episode_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM episodes",
            [],
            |row| row.get(0),
        )?;

        Ok(serde_json::json!({
            "manifold_snapshots": manifold_count,
            "agent_messages": message_count,
            "episodes": episode_count,
            "wal_mode": true
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_manifold::Intent;

    fn temp_store() -> ManifoldStore {
        ManifoldStore::open(":memory:").expect("in-memory SQLite should open")
    }

    #[test]
    fn test_manifold_store_save_and_load() {
        let store = temp_store();
        let intent = Intent::new("Test project", vec!["no external deps"]);
        let manifold = GoalManifold::new(intent);

        let version = store
            .save_manifold(&manifold, Some("agent-001"))
            .expect("save should succeed");
        assert!(version >= 1);

        let loaded = store
            .load_latest_manifold()
            .expect("load should succeed")
            .expect("manifold should be present");

        assert_eq!(
            loaded.root_intent.description,
            manifold.root_intent.description
        );
        assert_eq!(
            loaded.integrity_hash.to_hex(),
            manifold.integrity_hash.to_hex()
        );
    }

    #[test]
    fn test_manifold_store_versions() {
        let store = temp_store();
        let intent = Intent::new("Versioned project", Vec::<String>::new());
        let manifold = GoalManifold::new(intent);

        let v1 = store.save_manifold(&manifold, None).unwrap();
        let v2 = store.save_manifold(&manifold, None).unwrap();
        let v3 = store.save_manifold(&manifold, None).unwrap();

        // Le versioni devono essere strettamente crescenti (AUTOINCREMENT)
        assert!(v1 < v2 && v2 < v3, "versions must be strictly increasing: {} < {} < {}", v1, v2, v3);

        let versions = store.list_manifold_versions(10).unwrap();
        assert_eq!(versions.len(), 3);
        // Ordine decrescente per version (AUTOINCREMENT garantisce unicità)
        let vs: Vec<i64> = versions.iter().map(|s| s.version).collect();
        assert!(vs[0] > vs[1] && vs[1] > vs[2], "list should be descending: {:?}", vs);
    }

    #[test]
    fn test_agent_messages_ledger() {
        let store = temp_store();

        let msg = AgentMessage {
            id: "msg-001".to_string(),
            from_agent: "architect-001".to_string(),
            to_agent: Some("worker-001".to_string()),
            message_type: "handoff".to_string(),
            payload_json: r#"{"context":"auth module complete"}"#.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            session_id: Some("session-abc".to_string()),
        };

        store.append_agent_message(&msg).expect("append should succeed");

        let history = store
            .get_agent_messages(Some("architect-001"), 10)
            .expect("query should succeed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].message_type, "handoff");

        // Idempotenza: inserimento duplicato ignorato
        store.append_agent_message(&msg).expect("duplicate should be ignored");
        let history2 = store.get_agent_messages(None, 10).unwrap();
        assert_eq!(history2.len(), 1);
    }

    #[test]
    fn test_episodes_persistence() {
        let store = temp_store();

        let ep = PersistedEpisode {
            id: "ep-001".to_string(),
            agent_id: Some("worker-001".to_string()),
            event_type: "goal_completed".to_string(),
            description: "Implemented auth middleware".to_string(),
            outcome: "success".to_string(),
            importance: 0.9,
            payload_json: r#"{"duration_secs":42}"#.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };

        store.append_episode(&ep).expect("append episode should succeed");

        let episodes = store.get_episodes(0.5, 10).expect("query should succeed");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].event_type, "goal_completed");
        assert!((episodes[0].importance - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_stats() {
        let store = temp_store();
        let stats = store.stats().expect("stats should succeed");
        assert_eq!(stats["manifold_snapshots"], 0);
        assert_eq!(stats["wal_mode"], true);
    }
}
