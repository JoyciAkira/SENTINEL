# SENTINEL - Sistema di Controllo per Agenti AI

## Overview

SENTINEL è un sistema avanzato di controllo e allineamento per agenti AI coding, progettato per garantire che le azioni degli agenti siano sempre allineate con gli obiettivi del progetto.

## Architettura

Il sistema è organizzato in 5 layer principali:

1. **Goal Manifold** - Rappresentazione immutabile e crittograficamente verificata degli obiettivi
2. **Alignment Field** - Validazione continua dell'allineamento con gli obiettivi
3. **Cognitive State** - Esecuzione auto-consapevole con meta-cognizione
4. **Memory Manifold** - Sistema di memoria gerarchica a contesto infinito
5. **Meta-Learning** - Apprendimento cross-progetto e miglioramento continuo

## Struttura del Progetto

```
crates/
├── sentinel-core/        # Core engine (layer 1-5)
├── sentinel-cli/         # Interfaccia CLI e TUI
├── sentinel-agent-native/# Agente nativo con LLM integration
└── sentinel-sandbox/     # Sandbox per esecuzione sicura
```

## Requisiti

- Rust 1.75+
- Tokio runtime
- Dipendenze native per candle (ML)

## Installazione

```bash
git clone https://github.com/JoyciAkira/SENTINEL
cd SENTINEL
cargo build --release
```

## Utilizzo

```bash
# Avvia CLI
sentinel-cli

# Avvia in modalità agente
sentinel-agent-native --project /path/to/project
```

## Licenza

MIT License - vedi LICENSE per i dettagli
