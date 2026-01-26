# Layer 8: Multi-Agent Synchronization (The Social Manifold)

L'obiettivo di questo Layer è trasformare Sentinel nel "Semaforo Cognitivo" per team di agenti AI.

## Week 1: Identity & Ownership
- [ ] Definizione della struct `Agent` e del sistema di `AgentID`.
- [ ] Implementazione del meccanismo di `Goal Lock` nel Manifold.
- [ ] Logica di prevenzione conflitti: un file protetto da un'invariante di un goal occupato non può essere scritto da altri agenti.

## Week 2: Cognitive Handover
- [ ] Implementazione del `HandoverLog`: memoria persistente per il passaggio di consegne tra agenti.
- [ ] Tool MCP `handover_context`: permette a un agente di spiegare "perché" ha fatto certe scelte al prossimo che arriverà.

## Week 3: Social Visualization
- [ ] Tab "Multi-Agent" nella TUI: mappa real-time delle attività degli agenti.
- [ ] Integrazione LSP: Code Lenses che mostrano "Agent X is working here" direttamente nell'IDE.

**Status**: 🚀 PLANNED
