# Sentinel for VS Code

The official Visual Studio Code extension for **Sentinel**, the Cognitive Operating System for AI Agents.

## Features

- **Atomic Forge**: Visual goal management with real-time dependency graph.
- **Sentinel Chat**: AI-powered assistant for architectural reasoning.
- **Alignment Gauge**: Constant visibility into your project's integrity score.
- **Auto-Fix**: One-click remediation for security invariant violations.

## Requirements

- Sentinel CLI installed (`cargo install sentinel-cli` or pre-built binary).
- Rust 1.75+ (if building from source).

## Usage

1. Open the Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`).
2. Run `Sentinel: Open Chat` to start the agent.
3. Use `Sentinel: Refresh Goals` to sync with the local manifold.

## Architecture

This extension uses the **Sentinel TypeScript SDK** to communicate with the core Rust engine via JSON-RPC over Stdio.
