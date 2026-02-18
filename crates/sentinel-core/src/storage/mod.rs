//! Persistent Storage Layer — SQLite WAL
//!
//! Fornisce persistenza concurrency-safe per il Goal Manifold e il Distributed Memory.
//! Usa SQLite in modalità WAL (Write-Ahead Logging) per:
//! - Letture concorrenti senza blocco
//! - Scritture serializzate con lock file-based
//! - Integrità garantita da Blake3 hash su ogni snapshot
//!
//! # Architettura
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  ManifoldStore                      │
//! ├─────────────────────────────────────────────────────┤
//! │  manifold_snapshots  │  agent_messages  │  episodes │
//! │  (versioned JSON)    │  (JSONL ledger)  │  (memory) │
//! └─────────────────────────────────────────────────────┘
//!         ↓ WAL mode: concurrent reads, serialized writes
//! ```
//!
//! # Esempio
//!
//! ```no_run
//! use sentinel_core::storage::ManifoldStore;
//! use sentinel_core::GoalManifold;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let store = ManifoldStore::open("sentinel.db")?;
//! // store.save_manifold(&manifold)?;
//! // let manifold = store.load_latest_manifold()?;
//! # Ok(())
//! # }
//! ```

pub mod manifold_store;

pub use manifold_store::ManifoldStore;
