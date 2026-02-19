# SENTINEL — Context Summary per Nuova Sessione

**Data:** 2026-02-18  
**Commit HEAD:** `250f904`  
**Repo:** https://github.com/JoyciAkira/SENTINEL.git

---

## Cosa abbiamo fatto oggi

### 1. Gap-closure Sprint (5 commit)

| Commit | Cosa |
|--------|------|
| `49cdfef` | CI/CD completo (6 job), MCP auth token-based, feedback loop learning, install.sh |
| `03e2002` | SQLite WAL ManifoldStore (3 tabelle: manifold_snapshots, agent_messages, episodes) |
| `4c1f2c8` | Fix test ordering (version DESC) |
| `80a3afe` | Docs aggiornate con gap chiusi e residui |
| `250f904` | Build extension completa + .vsix installabile + CI release-vsix |

### 2. Extension VSCode

- **Build completa:** `npm run build` → `out/extension.js` + `out/webview/` (2031 moduli, 421KB)
- **`.vsix` prodotto:** `sentinel-extension.vsix` (431KB, 31 file, zero warning)
- **Installata in VSCode:** `code --install-extension sentinel-extension.vsix`
- **activationEvents fixati:** Da `['*']` a 18 eventi specifici

### 3. Backend MCP funzionante

- **30 tool esposti** via `sentinel mcp`
- **Testati:** `init_project`, `get_alignment`, `get_goal_graph`, `chat`
- **Chat memory:** Persistente in `.sentinel/chat_memory.json`
- **Outcomes ledger:** Persistente in `.sentinel/outcomes.jsonl`
- **SQLite WAL:** `.sentinel/sentinel.db` (manifold_snapshots, agent_messages, episodes)

---

## Stato attuale

### ✅ Funziona

- CLI: `sentinel init`, `sentinel status`, `sentinel ui`, `sentinel mcp`
- MCP Server: risponde a 30 tool
- Extension: installata, webview compilata
- SQLite WAL: persistenza manifold, messaggi, episodi
- CI/CD: 7 job (Rust, Webview, TS SDK, Python, Security, Release, Release-vsix)

### ⚠️ Gap ancora aperti

| Gap | Priorità | Descrizione |
|-----|----------|-------------|
| **Predicati → Sandbox** | Alta | `TestsPassing` e `CommandSucceeds` implementati ma `PredicateState` non popolato con risultati test reali |
| **Embeddings semantici** | Media | `candle-core` in dipendenze ma nessun modello caricato. Memoria usa overlap lessicale |
| **agent_communication_send → SQLite** | Bassa | Il tool risponde "success" ma non persiste nel DB |
| **Federation P2P** | Rimandato | libp2p presente ma non connesso |

---

## Come riprendere

### Per testare l'extension

```bash
# In VSCode/Cursor:
# 1. Cmd+Shift+P → "Extensions: Install from VSIX"
# 2. Seleziona: sentinel-extension.vsix
# 3. Riavvia VSCode/Cursor
# 4. Cerca "Sentinel" nella sidebar
```

### Per testare il MCP

```bash
cd /Users/danielecorrao/intent/workspaces/necessary-harrier/sentinel
./target/release/sentinel-cli mcp
# In altro terminale:
echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | nc localhost STDIN
```

### Per rebuild completo

```bash
./scripts/build_extension.sh --install
```

---

## Chat history

La chat history di **Sentinel** (non di Cline) è in:
- `.sentinel/chat_memory.json` — memoria conversazionale persistente
- `.sentinel/outcomes.jsonl` — ledger degli outcome registrati
- `.sentinel/sentinel.db` — SQLite WAL con manifold_snapshots, agent_messages, episodes

La chat history di **Cline** è gestita da Cursor/VSCode e non è accessibile direttamente da filesystem. Ogni nuova sessione Cline inizia senza contesto precedente.

---

## Cosa fare dopo

1. **Popolare PredicateState** — Quando si esegue un goal, eseguire i test e popolare `state.test_results`
2. **Embeddings** — Scaricare modello `all-MiniLM-L6-v2` e implementare inferenza con candle-core
3. **agent_communication_send** — Aggiungere `ManifoldStore::append_agent_message` nel handler MCP
4. **UI** — Verificare che la webview si connetta al MCP backend

---

## File chiave

```
sentinel/
├── sentinel.json                    # Goal Manifold corrente
├── sentinel-extension.vsix          # Extension installabile
├── .sentinel/
│   ├── chat_memory.json             # Memoria chat
│   ├── outcomes.jsonl               # Ledger outcome
│   └── sentinel.db                  # SQLite WAL
├── docs/IMPLEMENTATION_STATUS_2026-02-18.md
├── scripts/build_extension.sh       # Build unificato
└── target/release/sentinel-cli      # Binary Rust