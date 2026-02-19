#!/usr/bin/env cargo
//! SENTINEL Gateway Binary
//!
//! Standalone WebSocket gateway for multi-channel communication.
//! 
//! # Usage
//! ```bash
//! sentinel-gateway [--port 18789] [--host 127.0.0.1] [--verbose]
//! ```

use clap::Parser;
use sentinel_gateway::{Gateway, GatewayConfig};

/// SENTINEL Gateway - Unified WebSocket Multi-Channel Communication
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to listen on (default: 18789)
    #[arg(short, long, default_value = "18789")]
    port: u16,

    /// Host to bind to (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Maximum concurrent connections
    #[arg(long, default_value = "100")]
    max_connections: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    if args.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_target(true)
            .with_thread_ids(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(false)
            .init();
    }

    let config = GatewayConfig::default()
        .with_host(args.host.clone())
        .with_port(args.port)
        .with_max_connections(args.max_connections);

    print_banner(&args.host, args.port);

    let gateway = Gateway::new(config);
    gateway.start().await?;

    Ok(())
}

fn print_banner(host: &str, port: u16) {
    println!();
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                                                               ║");
    println!("║           🦞  SENTINEL GATEWAY — MULTI-CHANNEL  🦞           ║");
    println!("║                                                               ║");
    println!("║     Unified WebSocket Communication for AI Agents            ║");
    println!("║                                                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("📡 WebSocket Server");
    println!("   └─ ws://{}:{}", host, port);
    println!();
    println!("🔗 HTTP Endpoints");
    println!("   ├─ GET  /         — Landing page");
    println!("   ├─ GET  /health   — Health check");
    println!("   ├─ GET  /status   — Gateway status");
    println!("   └─ WS   /ws       — WebSocket connection");
    println!();
    println!("🎯 Supported Channels");
    println!("   ├─ VSCode Extension");
    println!("   ├─ CLI Client");
    println!("   ├─ Web UI");
    println!("   └─ REST API");
    println!();
    println!("─────────────────────────────────────────────────────────────────");
    println!("Press Ctrl+C to stop the gateway");
    println!();
}