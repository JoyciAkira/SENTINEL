# Layer 6: Integration & Tooling Plan

## Overview
Il Layer 6 trasforma Sentinel da una libreria core a un set di strumenti interattivi. L'obiettivo è fornire interfacce visive e programmabili affinché lo sviluppatore umano e gli agenti AI (Cline, Cursor) possano collaborare restando allineati al Goal Manifold.

## Architecture: The "Ponte" System

```
┌──────────────────┐      ┌─────────────────────────┐      ┌──────────────────┐
│   AI TOOLS       │      │    SENTINEL BRIDGE      │      │   SENTINEL CORE  │
│ (Cline, Cursor)  │ <──> │ (CLI / LSP / MCP / TUI) │ <──> │   (Layer 1-5)    │
└──────────────────┘      └─────────────────────────┘      └──────────────────┘
```

## Componenti Principali

### 1. Interactive TUI (Terminal User Interface)
**Tecnologia**: `ratatui` (Rust)
Interfaccia visiva nel terminale per il monitoraggio in tempo reale.
- **Dashboard**: Visualizzazione KPI (Alignment Score, Velocity, Drift).
- **Goal Tree**: Navigazione interattiva nel DAG del Goal Manifold.
- **Action Monitor**: Log in tempo reale delle decisioni prese da Sentinel.

### 2. MCP Server (Model Context Protocol)
**Tecnologia**: Standard Anthropic / Rust
Permette a tool come Cline e Claude Desktop di "interrogare" Sentinel come se fosse una risorsa esterna.
- **Tools**: `get_alignment_status`, `propose_strategy`, `validate_action`.
- **Resources**: Accesso alla Knowledge Base dei pattern appresi (Layer 5).

### 3. LSP Server (Language Server Protocol)
**Tecnologia**: `tower-lsp` (Rust)
Integrazione profonda con VS Code e Cursor.
- **Diagnostics**: Mostra errori di "disallineamento architettonico" direttamente nel codice come linee rosse/gialle.
- **Code Lenses**: Suggerimenti contestuali sopra le funzioni (es: "Sentinel suggerisce: implementa i test prima per questo pattern").

### 4. CLI Bridge
**Tecnologia**: `clap` (Rust)
Interfaccia a riga di comando per integrazione manuale e script CI/CD.
- `sentinel check`: Verifica l'integrità del manifold.
- `sentinel learn`: Trigger manuale per l'apprendimento da un progetto.

## Roadmap di Implementazione

### Week 1: CLI & TUI Foundation
- [x] Setup del crate `sentinel-cli`.
- [x] Implementazione dei comandi base per interagire con il GoalManifold.
- [x] Prima bozza della TUI con visualizzazione della "Alignment Bar".
- [x] Integrazione end-to-end del loop di apprendimento nel CognitiveState.

**Status**: ✅ COMPLETED (2026-01-25)

### Week 2: MCP Integration (Cline Support)
- [x] Implementazione del server MCP (Base JSON-RPC over stdin/stdout).
- [x] Esposizione dei tool di validazione azioni (`validate_action`, `get_alignment`).
- [ ] Test di integrazione end-to-end con Cline.

**Status**: 🚧 IN PROGRESS (Foundation ready)

### Week 3: LSP & Editor Support
- [x] Sviluppo del server LSP per diagnostica di allineamento (Base: `did_open`, `did_change`).
- [ ] Creazione estensione VS Code "Sentinel-Vision" (In pianificazione).
- [ ] Visualizzazione del manifold come grafico interattivo (LSP Code Lenses).

**Status**: 🚧 IN PROGRESS (LSP Backend ready)

### Week 4: Dashboard & Analytics
- [ ] Dashboard web per analisi storica dei progetti.
- [ ] Visualizzazione delle relazioni tra pattern appresi.

## KPI di Successo per il Layer 6
- **Latenza**: Ogni check di validazione deve avvenire in <100ms per non rallentare l'IDE.
- **Usabilità**: Lo sviluppatore deve poter capire lo stato del progetto in <3 secondi guardando la TUI.
- **Interoperabilità**: Supporto garantito per Cline e Cursor fin dal day one.

## Prossimi Passi Immediati
1. Inizializzare il crate `sentinel-cli` nel workspace.
2. Definire il protocollo di comunicazione JSON tra Core e CLI.
