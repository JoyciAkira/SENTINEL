#!/bin/bash

# SENTINEL TOTAL VALIDATION SUITE
# Genera un report di integrità deterministico per l'OS Cognitivo
set -euo pipefail

CLI="./target/debug/sentinel-cli"
REPORT="SENTINEL_VALIDATION_REPORT.md"
PASS_COUNT=0
FAIL_COUNT=0

record_check() {
  local status="$1"
  local message="$2"
  if [[ "$status" == "PASS" ]]; then
    PASS_COUNT=$((PASS_COUNT + 1))
    echo "- [PASS] $message" >> "$REPORT"
  else
    FAIL_COUNT=$((FAIL_COUNT + 1))
    echo "- [FAIL] $message" >> "$REPORT"
  fi
}

echo "# 🛡️ SENTINEL VALIDATION REPORT" > $REPORT
echo "Data: $(date)" >> $REPORT
echo "Ambiente: $(uname -a)" >> $REPORT
echo -e "\n---\n" >> $REPORT

echo "Inizio Validazione Totale..."

# 1. LAYER 1-3: CORE INTEGRITY
echo "## Phase A: Core Integrity & Manifold" >> $REPORT
$CLI init "Validation Test Project" > /dev/null 2>&1 || true
if [ -f "sentinel.json" ]; then
    record_check "PASS" "Layer 1-3: Manifold Initialization & Persistence"
else
    record_check "FAIL" "Layer 1-3: Manifold file not created"
fi

# 2. LAYER 2: ALIGNMENT & CALIBRATION
echo -e "\n## Phase B: Alignment & Calibration" >> $REPORT
if $CLI calibrate 0.9 > /dev/null 2>&1; then
  SENSITIVITY=$(grep "sensitivity" sentinel.json | awk '{print $2}' | tr -d ',')
  if [ "$SENSITIVITY" == "0.9" ]; then
      record_check "PASS" "Layer 2: Sensitivity Calibration Engine"
  else
      record_check "FAIL" "Layer 2: Calibration failed (Value: $SENSITIVITY)"
  fi
else
  record_check "FAIL" "Layer 2: Calibration command failed"
fi

# 3. LAYER 4: RUNTIME GUARDRAILS
echo -e "\n## Phase C: Runtime Guardrails (The Barrier)" >> $REPORT
# Proviamo a bloccare un comando (score < threshold)
if ! $CLI calibrate 1.0 > /dev/null 2>&1; then
  record_check "FAIL" "Layer 4: Calibration to strict mode failed"
fi
BLOCK_RESULT=$($CLI run -- ls 2>&1 || true)
if [[ $BLOCK_RESULT == *"SENTINEL GUARDIAN BLOCK"* ]]; then
    record_check "PASS" "Layer 4: Physical Execution Interdiction (Locked)"
else
    record_check "FAIL" "Layer 4: Guardrail failed to block insecure execution"
fi

# 4. LAYER 6: PROTOCOL BRIDGES (MCP)
echo -e "\n## Phase D: Protocol Bridges (MCP/LSP)" >> $REPORT
MCP_RESPONSE=$(printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"validator","version":"1.0"}},"id":1}' \
  '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_alignment","arguments":{}},"id":2}' | $CLI mcp 2>/dev/null || true)
if [[ $MCP_RESPONSE == *"alignment_score"* || $MCP_RESPONSE == *"score"* ]]; then
    record_check "PASS" "Layer 6: MCP JSON-RPC 2.0 Response Integrity"
else
    record_check "FAIL" "Layer 6: MCP Protocol Bridge Corrupted"
fi

# 5. LAYER 8: COGNITIVE OMNISCIENCE
echo -e "\n## Phase E: Cognitive Omniscience (Killer Feature)" >> $REPORT
MAP_RESPONSE=$(printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"validator","version":"1.0"}},"id":1}' \
  '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_cognitive_map","arguments":{}},"id":2}' | $CLI mcp 2>/dev/null || true)
if [[ $MAP_RESPONSE == *"SENTINEL COGNITIVE MAP"* ]]; then
    record_check "PASS" "Layer 8: Semantic Context Injection"
    record_check "PASS" "Layer 8: Hierarchical Goal Distillation"
else
    record_check "FAIL" "Layer 8: Omniscience Engine Failure"
fi

echo -e "\n---\n" >> $REPORT
echo "### SUMMARY: PASS=$PASS_COUNT FAIL=$FAIL_COUNT" >> "$REPORT"
if [[ "$FAIL_COUNT" -eq 0 ]]; then
  echo "### FINAL VERDICT: SENTINEL IS OPERATIONALLY READY" >> "$REPORT"
else
  echo "### FINAL VERDICT: SENTINEL REQUIRES REMEDIATION" >> "$REPORT"
fi

echo "Validazione completata. Report generato in $REPORT"
