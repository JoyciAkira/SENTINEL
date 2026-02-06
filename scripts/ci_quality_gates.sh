#!/usr/bin/env bash
set -euo pipefail

echo "[quality] running sentinel quality gates"
echo "[quality] rust version: $(rustc --version)"

# Keep gates deterministic and aligned with the current repository baseline.
cargo test -q -p sentinel-core
cargo test -q -p sentinel-agent-native
cargo test -q -p sentinel-cli
cargo test -q -p sentinel-sandbox

echo "[quality] all gates passed"
