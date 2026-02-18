#!/usr/bin/env python3
"""
Sentinel Webview UI E2E Test Suite.

Testa l'integrazione tra l'extension VSCode e il backend MCP tramite proxy.
"""

import subprocess
import json
import os
import sys

SENTINEL_BIN = os.environ.get(
    "SENTINEL_BIN",
    os.path.join(os.path.dirname(__file__), "..", "..", "target", "debug", "sentinel-cli"),
)
PROJECT_ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
LLM_ENABLED = bool(os.environ.get("SENTINEL_LLM_BASE_URL"))

PASS = FAIL = TOTAL = 0


def log_test(name):
    global TOTAL
    TOTAL += 1
    print(f"\n=== TEST {TOTAL}: {name} ===")


def log_pass(msg):
    global PASS
    PASS += 1
    print(f"  ✅ PASS: {msg}")


def log_fail(msg):
    global FAIL
    FAIL += 1
    print(f"  ❌ FAIL: {msg}")


def send_rpc(*requests, timeout=30):
    env = {**os.environ}
    inp = "\n".join(json.dumps(r) for r in requests) + "\n"
    try:
        r = subprocess.run(
            [SENTINEL_BIN, "mcp"],
            input=inp, capture_output=True, text=True,
            timeout=timeout, cwd=PROJECT_ROOT, env=env,
        )
    except subprocess.TimeoutExpired:
        return []
    responses = []
    for line in r.stdout.strip().split("\n"):
        if not line.strip():
            continue
        try:
            responses.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return responses


def call_tool(name, args=None, timeout=30):
    rpc = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
           "params": {"name": name, "arguments": args or {}}}
    resps = send_rpc(rpc, timeout=timeout)
    for r in resps:
        if r.get("id") == 1:
            return r.get("result"), r.get("error")
    return None, {"message": "no response"}


def extract_text(result):
    if not result:
        return ""
    c = result.get("content", [])
    if c:
        return c[0].get("text", "")
    return str(result)


print("=" * 60)
print("  SENTINEL WEBVIEW UI E2E TEST SUITE")
print(f"  Binary: {os.path.abspath(SENTINEL_BIN)}")
print(f"  LLM: {'✅ enabled' if LLM_ENABLED else '⚠️  disabled'}")
print("=" * 60)

# TEST 1: Chat message flow
log_test("chat message flow (simula invio da webview)")
result, err = call_tool("chat", {
    "message": "List the main features of SENTINEL in 3 bullet points."
}, timeout=120)
text = extract_text(result)
if result and not result.get("isError") and len(text) > 30:
    log_pass(f"Chat response OK: {text[:150]}...")
else:
    log_fail(f"Chat failed: err={err}, text={text[:100] if text else 'empty'}")

# TEST 2: Memory status
log_test("memory status (webview memory panel)")
result, err = call_tool("chat_memory_status")
text = extract_text(result)
if result and "turn_count" in text:
    log_pass(f"Memory status OK: {text[:100]}")
else:
    log_fail(f"Memory status failed: err={err}")

# TEST 3: Goal graph for Network panel
log_test("goal graph (Network panel visualization)")
result, err = call_tool("get_goal_graph")
text = extract_text(result)
if result and ("nodes" in text or "edges" in text or "goals" in text):
    log_pass(f"Goal graph OK: {text[:100]}...")
else:
    log_fail(f"Goal graph failed: err={err}")

# TEST 4: Agent communication for Swarm panel
log_test("agent communication (Swarm panel)")
result, err = call_tool("agent_communication_status")
text = extract_text(result)
if result and "agent" in text.lower():
    log_pass(f"Agent status OK: {text[:100]}...")
else:
    log_fail(f"Agent status failed: err={err}")

# TEST 5: Quality status for Forge panel
log_test("quality status (Forge panel)")
result, err = call_tool("get_quality_status")
text = extract_text(result)
if result:
    log_pass(f"Quality status OK: {text[:100]}...")
else:
    log_fail(f"Quality status failed: err={err}")

# TEST 6: Alignment for dashboard
log_test("alignment score (dashboard)")
result, err = call_tool("get_alignment")
text = extract_text(result)
if result and "score" in text.lower():
    log_pass(f"Alignment OK: {text[:100]}...")
else:
    log_fail(f"Alignment failed: err={err}")

# RISULTATI
print()
print("=" * 60)
print("  RISULTATI WEBVIEW UI TEST")
print("=" * 60)
print(f"  Totale: {TOTAL}")
print(f"  ✅ PASS: {PASS}")
print(f"  ❌ FAIL: {FAIL}")
rate = int(PASS / max(PASS + FAIL, 1) * 100)
print(f"  Pass rate: {PASS}/{PASS + FAIL} ({rate}%)")
print("=" * 60)

sys.exit(0 if FAIL == 0 else 1)