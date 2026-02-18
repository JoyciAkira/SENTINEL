#!/usr/bin/env bash
# build_extension.sh — Build completo SENTINEL: Rust binary + VSCode extension + .vsix
#
# Uso:
#   ./scripts/build_extension.sh              # build tutto
#   ./scripts/build_extension.sh --vsix-only  # solo package .vsix (richiede build precedente)
#   ./scripts/build_extension.sh --install    # build + installa in VSCode
#
# Output:
#   target/release/sentinel-cli               (binary Rust)
#   sentinel-extension.vsix                   (pacchetto installabile)
#
# Requisiti: cargo, node >= 18, npm, vsce (npm i -g @vscode/vsce)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VSCODE_DIR="$ROOT/integrations/vscode"
VSIX_OUT="$ROOT/sentinel-extension.vsix"

# ─── Colori ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
info()    { echo -e "${BLUE}[sentinel-build]${NC} $*"; }
success() { echo -e "${GREEN}[sentinel-build]${NC} ✅ $*"; }
warn()    { echo -e "${YELLOW}[sentinel-build]${NC} ⚠️  $*"; }
error()   { echo -e "${RED}[sentinel-build]${NC} ❌ $*"; exit 1; }

# ─── Argomenti ────────────────────────────────────────────────────────────────
VSIX_ONLY=false
INSTALL=false
for arg in "$@"; do
  case "$arg" in
    --vsix-only) VSIX_ONLY=true ;;
    --install)   INSTALL=true ;;
    --help|-h)
      echo "Uso: $0 [--vsix-only] [--install]"
      echo "  --vsix-only  Solo package .vsix (richiede build precedente)"
      echo "  --install    Build + installa in VSCode via 'code --install-extension'"
      exit 0
      ;;
  esac
done

# ─── Prerequisiti ─────────────────────────────────────────────────────────────
check_prereq() {
  command -v "$1" &>/dev/null || error "Prerequisito mancante: '$1'. Installa e riprova."
}

check_prereq cargo
check_prereq node
check_prereq npm

if ! command -v vsce &>/dev/null; then
  warn "vsce non trovato. Installazione automatica..."
  npm install -g @vscode/vsce || error "Impossibile installare vsce"
fi

# ─── Step 1: Build Rust binary ────────────────────────────────────────────────
if [ "$VSIX_ONLY" = false ]; then
  info "Step 1/3: Build sentinel-cli (release)..."
  cd "$ROOT"
  cargo build --release -p sentinel-cli 2>&1 | grep -E "Compiling|Finished|error" || true

  BINARY="$ROOT/target/release/sentinel-cli"
  if [ ! -f "$BINARY" ]; then
    error "Binary non trovato: $BINARY"
  fi
  BINARY_SIZE=$(du -sh "$BINARY" | cut -f1)
  success "sentinel-cli compilato ($BINARY_SIZE)"
else
  info "Step 1/3: Skipped (--vsix-only)"
fi

# ─── Step 2: Build VSCode extension (TypeScript + Webview) ───────────────────
if [ "$VSIX_ONLY" = false ]; then
  info "Step 2/3: Build VSCode extension (TypeScript + Vite webview)..."
  cd "$VSCODE_DIR"

  # Installa dipendenze se node_modules non esiste o è obsoleto
  if [ ! -d "node_modules" ]; then
    info "  Installazione dipendenze npm..."
    npm install --silent || error "npm install fallito"
  fi

  npm run build 2>&1 | grep -E "✓|error|warning|built in" || true

  if [ ! -f "$VSCODE_DIR/out/extension.js" ]; then
    error "Build extension fallita: out/extension.js non trovato"
  fi
  if [ ! -f "$VSCODE_DIR/out/webview/index.html" ]; then
    error "Build webview fallita: out/webview/index.html non trovato"
  fi
  success "Extension compilata (out/extension.js + out/webview/)"
else
  info "Step 2/3: Skipped (--vsix-only)"
  # Verifica che i file esistano
  [ -f "$VSCODE_DIR/out/extension.js" ] || error "out/extension.js non trovato. Esegui senza --vsix-only prima."
  [ -f "$VSCODE_DIR/out/webview/index.html" ] || error "out/webview/index.html non trovato."
fi

# ─── Step 3: Package .vsix ────────────────────────────────────────────────────
info "Step 3/3: Package .vsix..."
cd "$VSCODE_DIR"
vsce package --no-dependencies --out "$VSIX_OUT" 2>&1 | grep -E "DONE|WARNING|error" || true

if [ ! -f "$VSIX_OUT" ]; then
  error ".vsix non prodotto: $VSIX_OUT"
fi

VSIX_SIZE=$(du -sh "$VSIX_OUT" | cut -f1)
success ".vsix prodotto: $VSIX_OUT ($VSIX_SIZE)"

# ─── Opzionale: installa in VSCode ────────────────────────────────────────────
if [ "$INSTALL" = true ]; then
  if command -v code &>/dev/null; then
    info "Installazione in VSCode..."
    code --install-extension "$VSIX_OUT" --force
    success "Extension installata in VSCode"
  else
    warn "'code' non trovato nel PATH. Installa manualmente:"
    echo "  code --install-extension $VSIX_OUT"
  fi
fi

# ─── Riepilogo ────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  SENTINEL Build Completato${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo "  Binary Rust:  target/release/sentinel-cli"
echo "  Extension:    sentinel-extension.vsix"
echo ""
echo "  Per installare in VSCode:"
echo "    code --install-extension sentinel-extension.vsix"
echo ""
echo "  Per usare come MCP server (Cline/Claude Desktop):"
echo "    sentinel mcp"
echo ""
echo "  Configurazione MCP (claude_desktop_config.json):"
echo '    {"mcpServers":{"sentinel":{"command":"'$ROOT'/target/release/sentinel-cli","args":["mcp"]}}}'
echo ""
