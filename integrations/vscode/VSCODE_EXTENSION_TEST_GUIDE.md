# VS Code / Cursor Extension Test Guide
## Sentinel v2.0 - Complete UI Testing

---

## Installare l’estensione in Cursor (o VS Code)

1. **Build (se non già fatto):**
   ```bash
   # Dalla root del repo Sentinel
   cd sdks/typescript && npm install && npm run build && cd ../..
   cd integrations/vscode && npm run build
   ```

2. **Creare il VSIX:**
   ```bash
   cd integrations/vscode && npx --yes @vscode/vsce package --no-dependencies
   ```
   File generato: `integrations/vscode/sentinel-vscode-2.0.1.vsix`

3. **Installare in Cursor:**
   - Apri Cursor.
   - `Cmd+Shift+P` (Mac) / `Ctrl+Shift+P` (Windows/Linux) → **“Extensions: Install from VSIX...”**.
   - Scegli il file `sentinel-vscode-2.0.1.vsix` (path assoluto: `.../sentinel/integrations/vscode/sentinel-vscode-2.0.1.vsix`).
   - Ricarica la finestra se richiesto.

4. **Binario `sentinel` in PATH:** l’estensione avvia `sentinel` (es. `sentinel mcp`, `sentinel lsp`). Assicurati che il binario sia in PATH (es. `~/.local/bin/sentinel` o `cargo install --path crates/sentinel-cli` dalla root del repo).

---

## Setup (Already Done)
- ✅ Extension built: `out/` directory ready
- ✅ Extension packaged: `sentinel-vscode-2.0.1.vsix`
- ✅ Extension installed in VS Code / Cursor (vedi sopra)
- ✅ Binary in PATH: `sentinel` (required for MCP + LSP)

---

## Launch Cursor / VS Code for Testing

```bash
# Make sure sentinel is in PATH
export PATH="$HOME/.local/bin:$PATH"

# Apri Cursor nella cartella del progetto Sentinel (consigliato)
cursor .   # oppure: code .
```

---

## Cosa funziona al momento / Requisiti per la chat LLM

**Funziona senza configurazione aggiuntiva (se `sentinel` è in PATH):**
- Sidebar Sentinel con pannello Chat (webview unica con tab: Command, Chat, Forge, Network, Audit, Settings).
- Connessione MCP (estensione avvia `sentinel` e parla in JSON-RPC su stdio).
- Status bar con alignment (se MCP risponde a `get_alignment`).
- CodeLens su file Rust/TS/JS/Python (se LSP è attivo).
- Comandi: Open Chat, Refresh Goals, Validate Action, Show Alignment, Blueprint List/Show/Apply/Quickstart.
- Slash command in chat: `/init`, `/execute-first-pending`, `/help`, `/memory-status`, `/memory-search`, `/memory-export`, `/memory-import`.
- Workflow Assistant (card che suggerisce la prossima azione).
- Timeline, temi, approval safe-write (flusso UI).

**Inferenza LLM in chat:** la chat invia il messaggio al tool MCP `chat`; il backend (`sentinel-cli`) chiama `chat_with_llm()` che usa `ProviderRouter::from_env()`. Per avere risposte dall’LLM serve **almeno uno** di questi configurato:
- `OPENROUTER_API_KEY` (e opzionale `OPENROUTER_MODEL`)
- `OPENAI_API_KEY` (e opzionale `OPENAI_MODEL`)
- `ANTHROPIC_API_KEY` (e opzionale `ANTHROPIC_MODEL`)
- `GEMINI_API_KEY` (e opzionale `GEMINI_MODEL`)
- Oppure: `SENTINEL_LLM_BASE_URL` + `SENTINEL_LLM_MODEL` (e opzionale `SENTINEL_LLM_API_KEY`)

Se nessun provider è configurato, il tool `chat` risponde con messaggio di errore tipo “Errore durante l'inferenza dell'agente. Verifica le API key.” (vedi `mcp.rs`).

**Nota:** in questo workspace `cargo build -p sentinel-cli` può fallire (errori di compilazione). Se hai già un binario `sentinel` funzionante in PATH (es. installato in precedenza), l’estensione lo userà; altrimenti va sistemato il build di `sentinel-cli` prima di avere MCP e chat operativi.

---

## Test Checklist

### 1. Connection Status
When VS Code opens, look for:
- [ ] **Sentinel Chat Panel** appears in sidebar (should show "Connected: Sentinel MCP")
- [ ] **Status Bar** shows alignment score in bottom-right: `$(shield) 100%`
- [ ] **Output Channel** (View > Output > Sentinel) shows MCP connection log

---

### 2. Sidebar Views (5 TreeViews)

Open the **Sentinel** sidebar (icon should appear in activity bar):

#### Alignment View
- [ ] Shows alignment score (0-100)
- [ ] Shows confidence percentage
- [ ] Shows any violations
- [ ] Color-coded icons (green=good, yellow=warning, red=critical)

#### Goals View
- [ ] Shows Goal DAG tree
- [ ] Root intent displayed
- [ ] Goal status icons (✓, ⟳, 🧪, etc.)
- [ ] Goal count summary at top

#### Agents View
- [ ] Shows active agents
- [ ] Shows file locks
- [ ] Shows handover notes

#### Security View (Layer 7)
- [ ] Shows current risk level
- [ ] Shows security alerts
- [ ] Shows dependency status

#### Network View (Layers 9-10)
- [ ] Shows P2P peer count
- [ ] Shows consensus status
- [ ] Shows connected nodes

---

### 3. CodeLens (Top of File)

Open a Rust file (e.g., `crates/sentinel-core/src/lib.rs`):
- [ ] CodeLens at top shows: `Alignment: 100% $(shield)`
- [ ] Clicking the CodeLens shows alignment details

---

### 4. Chat Panel (Main Feature)

In the Sentinel Chat sidebar:

#### Initial State
- [ ] Shows "Connected to Sentinel MCP" banner
- [ ] Shows alignment gauge at top
- [ ] Shows Goal manifold tree (collapsible)
- [ ] Input area is enabled

#### Send a Message
Type: `Hello Sentinel, what is my current alignment?`

Expected behavior:
- [ ] Message appears in chat (right-aligned, blue bubble)
- [ ] Sentinel responds (left-aligned, markdown rendered)
- [ ] Tool call cards appear if MCP tools were called
- [ ] Alignment updates if changed

#### Test Tool Calls
Type: `Generate a Rust function to check if a number is prime`

Expected:
- [ ] LLM generates code (via OpenRouter)
- [ ] Tool call card shows: `validate_action`
- [ ] Tool call card shows: `safe_write` (security scan)
- [ ] File approval card appears (Approve/Reject buttons)
- [ ] Code is rendered with syntax highlighting

#### Test File Approval
1. Click "Approve" on a file operation
- [ ] File is created/modified
- [ ] Toast notification appears
- [ ] Chat shows confirmation

Or click "Reject"
- [ ] Toast: "Operation rejected"
- [ ] Chat shows rejection reason

---

### 5. Commands (Command Palette)

Open Command Palette (`Cmd+Shift+P` on Mac, `Ctrl+Shift+P` on Windows/Linux):

#### Sentinel: Open Chat
- [ ] Opens/focuses the Sentinel Chat panel

#### Sentinel: Refresh Goals
- [ ] Refreshes the Goals tree view
- [ ] Shows toast: "Goals refreshed"

#### Sentinel: Validate Action
- [ ] Prompts for action description
- [ ] Shows validation result

#### Sentinel: Show Alignment
- [ ] Shows current alignment report
- [ ] Displays score, confidence, violations

---

### 6. MCP Tools (via Chat)

Type these commands in the chat:

```
/get_alignment
```
- [ ] Returns alignment score and details

```
/validate Create a new Rust module for authentication
```
- [ ] Validates action against goals
- [ ] Shows success probability

```
/safe_write
path: test.txt
content: Hello, this is a test file.
```
- [ ] Runs security scan
- [ ] Returns SAFE or BLOCK with threats

```
/strategy
Implement OAuth2 authentication
```
- [ ] Returns Layer 5 strategy suggestions

```
/cognitive_map
```
- [ ] Returns full cognitive state map

```
/handover
```
- [ ] Returns current handover notes

---

### 7. Real-Time Updates

While testing:
- [ ] Polling updates alignment every 60 seconds
- [ ] Status bar updates when alignment changes
- [ ] Connection status shows if MCP reconnects

---

### 8. Error Handling

Test edge cases:
- [ ] Stop MCP server: extension shows "Disconnected", tries to reconnect
- [ ] Invalid command: graceful error message
- [ ] Security threat detected: BLOCK message with details
- [ ] Low alignment: Warning in status bar

---

## Troubleshooting

### Extension not loading
```bash
# Check VS Code logs
code --verbose

# Reload window
Cmd+Shift+P > "Developer: Reload Window"
```

### MCP connection failed
```bash
# Verify sentinel is in PATH
which sentinel
sentinel --version

# Test MCP manually
echo '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' | sentinel mcp
```

### Chat panel empty
- Open Output channel: View > Output > Sentinel
- Check for errors in the log

### Webview not rendering
- Check `out/webview/` directory exists
- Check `index.html` and `assets/index.js` are present

---

## Expected Test Results

**Total Items:** ~50 checkboxes
**Expected Pass Rate:** 95%+ (some features may have graceful fallbacks)

---

## After Testing

Report results:
```bash
# Pass: ___ / 50
# Fail: ___ / 50
# Notes: ___________________
```
