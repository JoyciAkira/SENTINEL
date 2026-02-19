//! SENTINEL Gateway - Unified Multi-Channel Communication
//!
//! This crate provides a unified WebSocket gateway for SENTINEL,
//! enabling communication from multiple channels (VSCode, CLI, Web UI, API).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    SENTINEL Gateway                      │
//! ├─────────────────────────────────────────────────────────┤
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────┐   │
//! │  │ VSCode  │ │   CLI   │ │  Web UI │ │  REST API   │   │
//! │  │ Client  │ │ Client  │ │ Client  │ │   Client    │   │
//! │  └────┬────┘ └────┬────┘ └────┬────┘ └──────┬──────┘   │
//! │       │           │           │             │          │
//! │       └───────────┴───────────┴─────────────┘          │
//! │                         │                               │
//! │              ┌──────────▼──────────┐                   │
//! │              │   Channel Router    │                   │
//! │              └──────────┬──────────┘                   │
//! │                         │                               │
//! │              ┌──────────▼──────────┐                   │
//! │              │  Session Manager    │                   │
//! │              └──────────┬──────────┘                   │
//! │                         │                               │
//! │       ┌─────────────────┼─────────────────┐            │
//! │       │                 │                 │            │
//! │  ┌────▼────┐      ┌────▼────┐      ┌────▼────┐        │
//! │  │ Agent 1 │      │ Agent 2 │      │ Agent N │        │
//! │  │ (Swarm) │      │ (Swarm) │      │ (Swarm) │        │
//! │  └─────────┘      └─────────┘      └─────────┘        │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Multi-channel support**: VSCode, CLI, Web UI, REST API
//! - **Session isolation**: Each client gets isolated sessions
//! - **Security**: DM pairing, allowlist, rate limiting
//! - **OAuth providers**: Claude, ChatGPT, Gemini with rotation
//! - **Skills system**: Configurable templates and guardrails

pub mod channel;
pub mod config;
pub mod error;
pub mod gateway;
pub mod oauth;
pub mod security;
pub mod session;
pub mod skills;

pub use channel::{Channel, ChannelRouter, ChannelType};
pub use config::GatewayConfig;
pub use error::{GatewayError, Result};
pub use gateway::Gateway;
pub use oauth::{OAuthManager, OAuthProvider, OAuthToken};
pub use security::{ChannelSecurity, DmPolicy, SecurityDecision};
pub use session::{Session, SessionConfig, SessionId, SessionManager};
pub use skills::{Skill, SkillRegistry, SkillTemplate};

/// Gateway version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default WebSocket port
pub const DEFAULT_PORT: u16 = 18789;

/// Default host
pub const DEFAULT_HOST: &str = "127.0.0.1";