#!/usr/bin/env bash
# SENTINEL — One-line installer
# Usage: curl -fsSL https://raw.githubusercontent.com/JoyciAkira/SENTINEL/master/install.sh | bash
set -euo pipefail

REPO="JoyciAkira/SENTINEL"
BINARY="sentinel-cli"
INSTALL_DIR="${SENTINEL_INSTALL_DIR:-/usr/local/bin}"
VERSION="${SENTINEL_VERSION:-latest}"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

info()    { echo -e "${CYAN}[sentinel]${NC} $*"; }
success() { echo -e "${GREEN}[sentinel]${NC} $*"; }
warn()    { echo -e "${YELLOW}[sentinel]${NC} $*"; }
error()   { echo -e "${RED}[sentinel]${NC} $*" >&2; exit 1; }

# ── Detect OS/Arch ──────────────────────────────────────────────────────────
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64) echo "sentinel-linux-x86_64" ;;
        *) error "Unsupported Linux architecture: $arch. Build from source: cargo install --path crates/sentinel-cli" ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64) echo "sentinel-macos-x86_64" ;;
        arm64)  echo "sentinel-macos-arm64" ;;
        *) error "Unsupported macOS architecture: $arch" ;;
      esac
      ;;
    *)
      error "Unsupported OS: $os. Build from source: cargo install --path crates/sentinel-cli"
      ;;
  esac
}

# ── Resolve version ──────────────────────────────────────────────────────────
resolve_version() {
  if [ "$VERSION" = "latest" ]; then
    if command -v curl &>/dev/null; then
      VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
    elif command -v wget &>/dev/null; then
      VERSION="$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
    else
      error "curl or wget required"
    fi
  fi
  echo "$VERSION"
}

# ── Download ─────────────────────────────────────────────────────────────────
download_binary() {
  local platform="$1"
  local version="$2"
  local url="https://github.com/${REPO}/releases/download/${version}/${platform}"
  local tmp
  tmp="$(mktemp)"

  info "Downloading ${platform} ${version}..."
  if command -v curl &>/dev/null; then
    curl -fsSL "$url" -o "$tmp" || error "Download failed: $url"
  else
    wget -qO "$tmp" "$url" || error "Download failed: $url"
  fi

  chmod +x "$tmp"
  echo "$tmp"
}

# ── Install ───────────────────────────────────────────────────────────────────
install_binary() {
  local tmp="$1"
  local dest="${INSTALL_DIR}/${BINARY}"

  if [ -w "$INSTALL_DIR" ]; then
    mv "$tmp" "$dest"
  else
    info "Requesting sudo to install to ${INSTALL_DIR}..."
    sudo mv "$tmp" "$dest"
  fi

  success "Installed: ${dest}"
}

# ── Verify ────────────────────────────────────────────────────────────────────
verify_install() {
  if command -v sentinel-cli &>/dev/null; then
    local ver
    ver="$(sentinel-cli --version 2>/dev/null || echo 'unknown')"
    success "sentinel-cli is ready: ${ver}"
  else
    warn "sentinel-cli not found in PATH. Add ${INSTALL_DIR} to your PATH:"
    warn "  export PATH=\"\$PATH:${INSTALL_DIR}\""
  fi
}

# ── Build from source fallback ────────────────────────────────────────────────
build_from_source() {
  info "No pre-built binary available. Building from source..."
  if ! command -v cargo &>/dev/null; then
    error "Rust/Cargo not found. Install from https://rustup.rs then retry."
  fi
  if ! command -v git &>/dev/null; then
    error "git not found."
  fi

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  git clone --depth=1 "https://github.com/${REPO}.git" "$tmp_dir"
  cd "$tmp_dir"
  cargo build --release -p sentinel-cli
  install_binary "target/release/sentinel-cli"
  cd - >/dev/null
  rm -rf "$tmp_dir"
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
  info "SENTINEL Installer"
  info "Repository: https://github.com/${REPO}"
  echo ""

  local platform
  platform="$(detect_platform)"
  info "Platform detected: ${platform}"

  local version
  version="$(resolve_version)"
  info "Version: ${version}"

  # Try pre-built binary first, fall back to source build
  local tmp
  if tmp="$(download_binary "$platform" "$version" 2>/dev/null)"; then
    install_binary "$tmp"
  else
    warn "Pre-built binary not available for this platform/version."
    build_from_source
  fi

  verify_install

  echo ""
  success "Installation complete!"
  echo ""
  echo "  Quick start:"
  echo "    sentinel-cli init \"My project description\""
  echo "    sentinel-cli tui"
  echo "    sentinel-cli mcp  # Start MCP server for Cline/Claude Desktop"
  echo ""
  echo "  Docs: https://github.com/${REPO}#readme"
}

main "$@"
