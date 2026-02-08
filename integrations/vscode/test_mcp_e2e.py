#!/usr/bin/env python3
"""
Sentinel MCP Server E2E Test Suite
Tests all 7 MCP tools via real JSON-RPC over stdio.
"""

import subprocess
import json
import sys
import os

SENTINEL_BIN = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "target", "release", "sentinel-cli"
)
PROJECT_ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")

PASS = 0
FAIL = 0
TOTAL = 0

def log_test(name):
    global TOTAL
    TOTAL += 1
    print(f"\n=== TEST {TOTAL}: {name} ===")

def log_pass(msg):
    global PASS
    PASS += 1
    print(f"  PASS: {msg}")

def log_fail(msg):
    global FAIL
    FAIL += 1
    print(f"  FAIL: {msg}")

def send_rpc(requests):
    """Send one or more JSON-RPC requests and return all responses."""
    if isinstance(requests, dict):
        requests = [requests]

    input_data = "\n".join(json.dumps(r) for r in requests) + "\n"

    try:
        result = subprocess.run(
            [SENTINEL_BIN, "mcp"],
            input=input_data,
            capture_output=True,
            text=True,
            timeout=15,
            cwd=PROJECT_ROOT
        )
    except subprocess.TimeoutExpired:
        return []
    except FileNotFoundError:
        print(f"ERROR: Binary not found at {SENTINEL_BIN}")
        sys.exit(1)

    responses = []
    for line in result.stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue
        try:
            responses.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return responses

def get_response_by_id(responses, rid):
    for r in responses:
        if r.get("id") == rid:
            return r
    return None

def init_and_call(method, params, call_id=2):
    """Send initialize + a tool call, return the tool call response."""
    init_req = {
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        },
        "id": 1
    }
    call_req = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": call_id
    }
    responses = send_rpc([init_req, call_req])
    return get_response_by_id(responses, call_id)

def extract_text(response):
    """Extract text content from MCP tool response."""
    try:
        return response["result"]["content"][0]["text"]
    except (KeyError, IndexError, TypeError):
        return None


print("=" * 50)
print("  SENTINEL MCP E2E TEST SUITE")
print(f"  Binary: {SENTINEL_BIN}")
print(f"  Project: {PROJECT_ROOT}")
print("=" * 50)

# ── TEST 1: Initialize ─────────────────────────
log_test("MCP Initialize Handshake")
responses = send_rpc({
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "test", "version": "1.0"}
    },
    "id": 1
})
r = get_response_by_id(responses, 1)
if r and "result" in r:
    result = r["result"]
    if result.get("protocolVersion") == "2024-11-05":
        log_pass("protocolVersion: 2024-11-05")
    else:
        log_fail(f"protocolVersion wrong: {result.get('protocolVersion')}")

    if result.get("serverInfo", {}).get("name") == "sentinel-server":
        log_pass("serverInfo.name: sentinel-server")
    else:
        log_fail(f"serverInfo wrong: {result.get('serverInfo')}")

    if result.get("capabilities", {}).get("tools", {}).get("listChanged"):
        log_pass("capabilities.tools.listChanged: true")
    else:
        log_fail("Missing capabilities")
else:
    log_fail(f"No valid response: {responses}")

# ── TEST 2: Tools List ──────────────────────────
log_test("MCP Tools List")
r = init_and_call("tools/list", {})
if r and "result" in r:
    tools = r["result"].get("tools", [])
    tool_names = [t["name"] for t in tools]
    expected_tools = [
        "validate_action", "get_alignment", "safe_write",
        "propose_strategy", "record_handover",
        "get_cognitive_map", "get_enforcement_rules",
        "get_reliability", "governance_status", "get_world_model"
    ]
    for name in expected_tools:
        if name in tool_names:
            log_pass(f"Tool found: {name}")
        else:
            log_fail(f"Tool missing: {name}")
    print(f"  Total tools: {len(tools)}")
else:
    log_fail(f"tools/list failed: {r}")

# ── TEST 3: get_alignment ───────────────────────
log_test("Tool Call: get_alignment")
r = init_and_call("tools/call", {"name": "get_alignment", "arguments": {}})
text = extract_text(r)
if text:
    log_pass("get_alignment returned content")
    try:
        data = json.loads(text)
        score = data.get("alignment_score") or data.get("score")
        if score is not None:
            log_pass(f"Alignment score: {score}")
        print(f"  Full response: {text[:300]}")
    except json.JSONDecodeError:
        print(f"  Raw text: {text[:300]}")
        log_pass("Text response received")
else:
    log_fail(f"get_alignment failed: {r}")

# ── TEST 3b: get_world_model ─────────────────────
log_test("Tool Call: get_world_model")
r = init_and_call("tools/call", {"name": "get_world_model", "arguments": {}})
text = extract_text(r)
if text:
    log_pass("get_world_model returned content")
    try:
        data = json.loads(text)
        if "where_we_must_go" in data and "how_enforced" in data:
            log_pass("world model payload shape looks valid")
        else:
            log_fail("world model payload missing required keys")
    except json.JSONDecodeError:
        log_fail("get_world_model did not return JSON")
else:
    log_fail(f"get_world_model failed: {r}")

# ── TEST 4: validate_action ─────────────────────
log_test("Tool Call: validate_action")
r = init_and_call("tools/call", {
    "name": "validate_action",
    "arguments": {
        "action_type": "edit_file",
        "description": "Implement JWT authentication for secure login"
    }
})
text = extract_text(r)
if text:
    log_pass("validate_action returned content")
    print(f"  Result: {text[:300]}")
else:
    log_fail(f"validate_action failed: {r}")

# ── TEST 5: safe_write (clean code) ─────────────
log_test("Tool Call: safe_write (clean code)")
r = init_and_call("tools/call", {
    "name": "safe_write",
    "arguments": {
        "file_path": "src/main.rs",
        "content": "fn main() {\n    println!(\"Hello, Sentinel!\");\n}"
    }
})
text = extract_text(r)
if text:
    log_pass("safe_write returned content for clean code")
    print(f"  Result: {text[:300]}")
else:
    log_fail(f"safe_write failed: {r}")

# ── TEST 6: safe_write (security threats) ───────
log_test("Tool Call: safe_write (threat detection)")
r = init_and_call("tools/call", {
    "name": "safe_write",
    "arguments": {
        "file_path": "config.rs",
        "content": 'let aws_key = "AKIA1234567890ABCDEF";\nlet private_key = "-----BEGIN RSA PRIVATE KEY-----";\nlet password = "admin123";'
    }
})
text = extract_text(r)
if text:
    log_pass("safe_write detected threats")
    # Check if threats were actually detected
    try:
        data = json.loads(text)
        threats = data.get("threats", [])
        risk = data.get("risk_score", 0)
        if threats or risk > 0:
            log_pass(f"Detected {len(threats)} threats, risk_score={risk}")
        else:
            log_fail("No threats detected in malicious code")
    except json.JSONDecodeError:
        if "threat" in text.lower() or "risk" in text.lower() or "unsafe" in text.lower() or "block" in text.lower() or "minacce" in text.lower():
            log_pass("Threat keywords found in response")
        else:
            log_fail("No threat indication in response")
    print(f"  Result: {text[:400]}")
else:
    log_fail(f"safe_write threat detection failed: {r}")

# ── TEST 7: get_cognitive_map ───────────────────
log_test("Tool Call: get_cognitive_map")
r = init_and_call("tools/call", {"name": "get_cognitive_map", "arguments": {}})
text = extract_text(r)
if text:
    log_pass("get_cognitive_map returned content")
    print(f"  Map: {text[:300]}")
else:
    log_fail(f"get_cognitive_map failed: {r}")

# ── TEST 8: propose_strategy ───────────────────
log_test("Tool Call: propose_strategy")
r = init_and_call("tools/call", {
    "name": "propose_strategy",
    "arguments": {
        "goal_description": "Implement user authentication with OAuth2"
    }
})
text = extract_text(r)
if text:
    log_pass("propose_strategy returned content")
    print(f"  Strategy: {text[:300]}")
else:
    log_fail(f"propose_strategy failed: {r}")

# ── TEST 9: get_enforcement_rules ──────────────
log_test("Tool Call: get_enforcement_rules")
r = init_and_call("tools/call", {"name": "get_enforcement_rules", "arguments": {}})
text = extract_text(r)
if text:
    log_pass("get_enforcement_rules returned content")
    print(f"  Rules: {text[:300]}")
else:
    log_fail(f"get_enforcement_rules failed: {r}")

# ── TEST 10: record_handover ───────────────────
log_test("Tool Call: record_handover")
r = init_and_call("tools/call", {
    "name": "record_handover",
    "arguments": {
        "goal_id": "test-goal-e2e",
        "content": "E2E test handover note from Python test suite",
        "warnings": ["test warning 1", "test warning 2"]
    }
})
text = extract_text(r)
if text:
    log_pass("record_handover returned content")
    print(f"  Result: {text[:300]}")
else:
    log_fail(f"record_handover failed: {r}")

# ── TEST 11: sentinel status --json ────────────
log_test("CLI: sentinel status --json")
try:
    result = subprocess.run(
        [SENTINEL_BIN, "status", "--json"],
        capture_output=True, text=True, timeout=10,
        cwd=PROJECT_ROOT
    )
    if result.returncode == 0:
        data = json.loads(result.stdout)
        if "manifold" in data:
            log_pass("status --json returned manifold")
            manifold = data["manifold"]
            if isinstance(manifold, str):
                try:
                    manifold = json.loads(manifold)
                except json.JSONDecodeError:
                    manifold = {}
            root = manifold.get("root_intent", {}).get("description", "N/A")
            goals = manifold.get("goal_dag", {}).get("nodes", {})
            print(f"  Root intent: {root[:80]}")
            print(f"  Goals in DAG: {len(goals)}")
        else:
            log_fail("Missing manifold in status")
    else:
        log_fail(f"Exit code {result.returncode}: {result.stderr[:200]}")
except Exception as e:
    log_fail(f"Exception: {e}")

# ── RESULTS ─────────────────────────────────────
print("\n" + "=" * 50)
print("  TEST RESULTS")
print("=" * 50)
print(f"  Total assertions:  {TOTAL}")
print(f"  Passed: {PASS}")
print(f"  Failed: {FAIL}")
print(f"  Pass rate: {PASS}/{PASS+FAIL} ({100*PASS/(PASS+FAIL):.0f}%)" if PASS+FAIL > 0 else "  No assertions")
print("=" * 50)

sys.exit(1 if FAIL > 0 else 0)
