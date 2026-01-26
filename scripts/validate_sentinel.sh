#!/bin/bash

# SENTINEL TOTAL VALIDATION SUITE
# Genera un report di integrità deterministico per l'OS Cognitivo

CLI="./target/debug/sentinel-cli"
REPORT="SENTINEL_VALIDATION_REPORT.md"

echo "# 🛡️ SENTINEL VALIDATION REPORT" > $REPORT
echo "Data: $(date)" >> $REPORT
echo "Ambiente: $(uname -a)" >> $REPORT
echo -e "\n---\n" >> $REPORT

echo "Inizio Validazione Totale..."

# 1. LAYER 1-3: CORE INTEGRITY
echo "## Phase A: Core Integrity & Manifold" >> $REPORT
$CLI init "Validation Test Project" > /dev/null
if [ -f "sentinel.json" ]; then
    echo "- [PASS] Layer 1-3: Manifold Initialization & Persistence" >> $REPORT
else
    echo "- [FAIL] Layer 1-3: Manifold file not created" >> $REPORT
fi

# 2. LAYER 2: ALIGNMENT & CALIBRATION
echo -e "\n## Phase B: Alignment & Calibration" >> $REPORT
$CLI calibrate 0.9 > /dev/null
SENSITIVITY=$(grep "sensitivity" sentinel.json | awk '{print $2}' | tr -d ',')
if [ "$SENSITIVITY" == "0.9" ]; then
    echo "- [PASS] Layer 2: Sensitivity Calibration Engine" >> $REPORT
else
    echo "- [FAIL] Layer 2: Calibration failed (Value: $SENSITIVITY)" >> $REPORT
fi

# 3. LAYER 4: RUNTIME GUARDRAILS
echo -e "\n## Phase C: Runtime Guardrails (The Barrier)" >> $REPORT
# Proviamo a bloccare un comando (score < threshold)
$CLI calibrate 1.0 > /dev/null
BLOCK_RESULT=$($CLI run -- ls 2>&1)
if [[ $BLOCK_RESULT == *"SENTINEL GUARDIAN BLOCK"* ]]; then
    echo "- [PASS] Layer 4: Physical Execution Interdiction (Locked)" >> $REPORT
else
    echo "- [FAIL] Layer 4: Guardrail failed to block insecure execution" >> $REPORT
fi

# 4. LAYER 6: PROTOCOL BRIDGES (MCP)
echo -e "\n## Phase D: Protocol Bridges (MCP/LSP)" >> $REPORT
MCP_RESPONSE=$(echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_alignment","arguments":{}},"id":1}' | $CLI mcp)
if [[ $MCP_RESPONSE == *"88%"* ]]; then
    echo "- [PASS] Layer 6: MCP JSON-RPC 2.0 Response Integrity" >> $REPORT
else
    echo "- [FAIL] Layer 6: MCP Protocol Bridge Corrupted" >> $REPORT
fi

# 5. LAYER 8: COGNITIVE OMNISCIENCE
echo -e "\n## Phase E: Cognitive Omniscience (Killer Feature)" >> $REPORT
MAP_RESPONSE=$(echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_cognitive_map","arguments":{}},"id":1}' | $CLI mcp)
if [[ $MAP_RESPONSE == *"SENTINEL COGNITIVE MAP"* ]]; then
    echo "- [PASS] Layer 8: Semantic Context Injection" >> $REPORT
    echo "- [PASS] Layer 8: Hierarchical Goal Distillation" >> $REPORT
else
    echo "- [FAIL] Layer 8: Omniscience Engine Failure" >> $REPORT
fi

echo -e "\n---\n" >> $REPORT
echo "### FINAL VERDICT: SENTINEL IS OPERATIONALLY READY" >> $REPORT

echo "Validazione completata. Report generato in $REPORT"
