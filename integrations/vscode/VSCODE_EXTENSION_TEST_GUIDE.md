# VS Code Extension Test Guide
## Sentinel v2.0 - Complete UI Testing

---

## Setup (Already Done)
- ✅ Extension built: `out/` directory ready
- ✅ Extension packaged: `sentinel-vscode-2.0.0.vsix`
- ✅ Extension installed in VS Code
- ✅ Binary symlinked: `~/.local/bin/sentinel`

---

## Launch VS Code for Testing

```bash
# Make sure sentinel is in PATH
export PATH="$HOME/.local/bin:$PATH"

# Open VS Code in the Sentinel project (recommended for testing)
cd "/Users/danielecorrao/Documents/REPOSITORIES_GITHUB/SENTINEL "
code .
```

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
