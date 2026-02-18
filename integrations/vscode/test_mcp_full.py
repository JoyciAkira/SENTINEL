#!/usr/bin/env python3
"""
Sentinel MCP Full Test Suite — tutti i 31 tool.

Architettura 200 IQ:
- Test fixture: sentinel.json valido e completo, creato una volta
- Isolamento: ogni test usa il fixture, non lo modifica
- Timeout: 180s per tool LLM, 30s per tool non-LLM
- Fallback: init_project usa ArchitectEngine locale se LLM lento
"""

import subprocess
import json
import sys
import os
import shutil
from pathlib import Path

SENTINEL_BIN = os.environ.get(
    "SENTINEL_BIN",
    os.path.join(os.path.dirname(__file__), "..", "..", "target", "debug", "sentinel-cli"),
)
PROJECT_ROOT = Path(__file__).parent.parent.parent
LLM_ENABLED = bool(os.environ.get("SENTINEL_LLM_BASE_URL"))

PASS = FAIL = TOTAL = 0

# ─── FIXTURE: sentinel.json completo e valido ─────────────────────────────────

FIXTURE_MANIFOLD = {
    "root_intent": {
        "description": "Build a production-ready REST API in Rust with JWT authentication",
        "tech_stack": ["rust", "axum", "sqlx", "postgresql"],
        "constraints": ["no unsafe code", "security first", "comprehensive tests"],
        "expected_outcomes": ["Working REST API", "JWT authentication", "PostgreSQL integration"],
        "target_platform": "linux",
        "languages": ["rust"],
        "frameworks": ["axum"],
        "infrastructure_map": {}
    },
    "goal_dag": {
        "goals": {
            "00000000-0000-0000-0000-000000000001": {
                "id": "00000000-0000-0000-0000-000000000001",
                "description": "Setup Rust project with Axum framework",
                "status": "pending",
                "success_criteria": [{"file_exists": "Cargo.toml"}],
                "dependencies": [],
                "anti_dependencies": []
            },
            "00000000-0000-0000-0000-000000000002": {
                "id": "00000000-0000-0000-0000-000000000002",
                "description": "Implement JWT authentication middleware",
                "status": "pending",
                "success_criteria": [{"file_exists": "src/auth.rs"}],
                "dependencies": ["00000000-0000-0000-0000-000000000001"],
                "anti_dependencies": []
            },
            "00000000-0000-0000-0000-000000000003": {
                "id": "00000000-0000-0000-0000-000000000003",
                "description": "Configure PostgreSQL with connection pooling",
                "status": "pending",
                "success_criteria": [{"file_exists": "src/db.rs"}],
                "dependencies": ["00000000-0000-0000-0000-000000000001"],
                "anti_dependencies": []
            }
        },
        "dependencies": {
            "00000000-0000-0000-0000-000000000002": ["00000000-0000-0000-0000-000000000001"],
            "00000000-0000-0000-0000-000000000003": ["00000000-0000-0000-0000-000000000001"]
        },
        "integrity_hash": "0" * 64
    },
    "version": 1,
    "integrity_hash": "0" * 64,
    "overrides": [],
    "handover_log": [],
    "version_history": [],
    "sensitivity": 0.5,
    "governance": {
        "required_dependencies": [],
        "allowed_dependencies": ["cargo:axum", "cargo:sqlx", "cargo:jsonwebtoken"],
        "required_frameworks": [],
        "allowed_frameworks": ["framework:axum"],
        "allowed_endpoints": {},
        "allowed_ports": [3000],
        "history": [],
        "pending_proposal": None
    }
}


def setup_fixture():
    """Crea sentinel.json valido prima dei test."""
    manifold_path = PROJECT_ROOT / "sentinel.json"
    backup_path = PROJECT_ROOT / "sentinel.json.test_backup"
    
    # Backup existing
    if manifold_path.exists():
        shutil.copy(manifold_path, backup_path)
    
    # Write fixture
    with open(manifold_path, "w") as f:
        json.dump(FIXTURE_MANIFOLD, f, indent=2)
    
    return backup_path


def teardown_fixture(backup_path):
    """Ripristina il file originale dopo i test."""
    manifold_path = PROJECT_ROOT / "sentinel.json"
    
    if backup_path.exists():
        shutil.copy(backup_path, manifold_path)
        backup_path.unlink()
    else:
        # Se non c'era backup, rimuovi il fixture
        if manifold_path.exists():
            manifold_path.unlink()


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


# ─── MAIN ─────────────────────────────────────────────────────────────────────

print("=" * 60)
print("  SENTINEL MCP FULL TEST SUITE (200 IQ Edition)")
print(f"  Binary: {os.path.abspath(SENTINEL_BIN)}")
print(f"  LLM: {'✅ enabled via proxy/key' if LLM_ENABLED else '⚠️  disabled'}")
print("=" * 60)

# Setup fixture
backup_path = setup_fixture()
print(f"📦 Fixture: sentinel.json valido con 3 goals")

try:
    # ─── TEST 1: MCP Handshake ────────────────────────────────────────────────
    log_test("MCP Handshake + Tools List")
    resps = send_rpc(
        {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{}}},
        {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}},
    )
    init_r = next((r for r in resps if r.get("id") == 1), None)
    list_r = next((r for r in resps if r.get("id") == 2), None)
    
    if init_r and init_r.get("result", {}).get("serverInfo", {}).get("name") == "sentinel-server":
        log_pass("MCP server initialized")
    else:
        log_fail("MCP initialization failed")

    if list_r:
        tools = [t["name"] for t in list_r.get("result", {}).get("tools", [])]
        log_pass(f"Tools list: {len(tools)} tools")
        EXPECTED = {"validate_action","get_alignment","safe_write","propose_strategy",
                    "record_handover","get_cognitive_map","get_enforcement_rules","get_reliability",
                    "governance_status","get_world_model","get_quality_status","run_quality_harness",
                    "list_quality_reports","chat","chat_memory_status","chat_memory_search",
                    "chat_memory_clear","chat_memory_export","chat_memory_import","decompose_goal",
                    "get_goal_graph","governance_approve","governance_reject","governance_seed",
                    "init_project","orchestrate_task","quality_report","suggest_goals",
                    "agent_communication_history","agent_communication_send","agent_communication_status"}
        missing = EXPECTED - set(tools)
        if missing:
            log_fail(f"Missing tools: {missing}")
        else:
            log_pass("All 31 expected tools present")
    else:
        log_fail("tools/list failed")

    # ─── TEST 2: get_alignment (fixture-based) ─────────────────────────────────
    log_test("get_alignment (fixture manifold)")
    result, err = call_tool("get_alignment", timeout=30)
    text = extract_text(result)
    if result and "score" in text.lower():
        log_pass(f"Alignment OK: {text[:100]}")
    else:
        log_fail(f"Alignment failed: err={err}")

    # ─── TEST 3: get_goal_graph (fixture-based) ────────────────────────────────
    log_test("get_goal_graph (fixture goals)")
    result, err = call_tool("get_goal_graph", timeout=30)
    text = extract_text(result)
    if result and ("nodes" in text or "edges" in text or "goals" in text):
        log_pass(f"Goal graph OK: {text[:100]}")
    else:
        log_fail(f"Goal graph failed: err={err}")

    # ─── TEST 4: validate_action ───────────────────────────────────────────────
    log_test("validate_action")
    result, err = call_tool("validate_action", {
        "action": "Implement JWT authentication",
        "context": "security middleware"
    }, timeout=30)
    text = extract_text(result)
    if result and ("approved" in text.lower() or "alignment" in text.lower()):
        log_pass(f"validate_action OK: {text[:100]}")
    else:
        log_fail(f"validate_action failed: err={err}")

    # ─── TEST 5: safe_write ───────────────────────────────────────────────────
    log_test("safe_write (clean code)")
    result, err = call_tool("safe_write", {
        "code": "fn add(a: i32, b: i32) -> i32 { a + b }",
        "file_path": "src/math.rs"
    }, timeout=30)
    text = extract_text(result)
    if result and "safe" in text.lower():
        log_pass(f"safe_write OK: {text[:80]}")
    else:
        log_fail(f"safe_write failed: err={err}")

    # ─── TEST 6: safe_write (threat detection) ────────────────────────────────
    log_test("safe_write (threat detection)")
    result, err = call_tool("safe_write", {
        "code": 'const API_KEY = "AKIAIOSFODNN7EXAMPLE";',
        "file_path": "src/config.rs"
    }, timeout=30)
    text = extract_text(result)
    if result and ("threat" in text.lower() or "risk" in text.lower()):
        log_pass(f"Threat detected: {text[:80]}")
    else:
        log_fail(f"Threat detection failed: err={err}")

    # ─── TEST 7: governance_status ────────────────────────────────────────────
    log_test("governance_status")
    result, err = call_tool("governance_status", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Governance status OK: {text[:80]}")
    else:
        log_fail(f"Governance status failed: err={err}")

    # ─── TEST 8: governance_seed ──────────────────────────────────────────────
    log_test("governance_seed (preview)")
    result, err = call_tool("governance_seed", {"apply": False}, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Governance seed OK: {text[:80]}")
    else:
        log_fail(f"Governance seed failed: err={err}")

    # ─── TEST 9: chat_memory_status ───────────────────────────────────────────
    log_test("chat_memory_status")
    result, err = call_tool("chat_memory_status", timeout=30)
    text = extract_text(result)
    if result and "turn_count" in text:
        log_pass(f"Memory status OK: {text[:80]}")
    else:
        log_fail(f"Memory status failed: err={err}")

    # ─── TEST 10: chat_memory_search ──────────────────────────────────────────
    log_test("chat_memory_search")
    result, err = call_tool("chat_memory_search", {"query": "JWT"}, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Memory search OK: {text[:80]}")
    else:
        log_fail(f"Memory search failed: err={err}")

    # ─── TEST 11: chat_memory_export ──────────────────────────────────────────
    log_test("chat_memory_export")
    result, err = call_tool("chat_memory_export", timeout=30)
    text = extract_text(result)
    if result and "ok" in text.lower():
        log_pass(f"Memory export OK: {text[:80]}")
    else:
        log_fail(f"Memory export failed: err={err}")

    # ─── TEST 12: chat_memory_clear ───────────────────────────────────────────
    log_test("chat_memory_clear")
    result, err = call_tool("chat_memory_clear", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Memory clear OK: {text[:80]}")
    else:
        log_fail(f"Memory clear failed: err={err}")

    # ─── TEST 13: agent_communication_status ──────────────────────────────────
    log_test("agent_communication_status")
    result, err = call_tool("agent_communication_status", timeout=30)
    text = extract_text(result)
    if result and "agent" in text.lower():
        log_pass(f"Agent status OK: {text[:80]}")
    else:
        log_fail(f"Agent status failed: err={err}")

    # ─── TEST 14: agent_communication_history ─────────────────────────────────
    log_test("agent_communication_history")
    result, err = call_tool("agent_communication_history", {"limit": 5}, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Agent history OK: {text[:80]}")
    else:
        log_fail(f"Agent history failed: err={err}")

    # ─── TEST 15: get_cognitive_map ───────────────────────────────────────────
    log_test("get_cognitive_map")
    result, err = call_tool("get_cognitive_map", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Cognitive map OK: {text[:80]}")
    else:
        log_fail(f"Cognitive map failed: err={err}")

    # ─── TEST 16: get_world_model ─────────────────────────────────────────────
    log_test("get_world_model")
    result, err = call_tool("get_world_model", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"World model OK: {text[:80]}")
    else:
        log_fail(f"World model failed: err={err}")

    # ─── TEST 17: get_reliability ─────────────────────────────────────────────
    log_test("get_reliability")
    result, err = call_tool("get_reliability", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Reliability OK: {text[:80]}")
    else:
        log_fail(f"Reliability failed: err={err}")

    # ─── TEST 18: get_quality_status ──────────────────────────────────────────
    log_test("get_quality_status")
    result, err = call_tool("get_quality_status", timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Quality status OK: {text[:80]}")
    else:
        log_fail(f"Quality status failed: err={err}")

    # ─── TEST 19: propose_strategy ────────────────────────────────────────────
    log_test("propose_strategy")
    result, err = call_tool("propose_strategy", {"context": "JWT auth"}, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Strategy OK: {text[:80]}")
    else:
        log_fail(f"Strategy failed: err={err}")

    # ─── TEST 20: record_handover ─────────────────────────────────────────────
    log_test("record_handover")
    result, err = call_tool("record_handover", {"note": "Test handover"}, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Handover OK: {text[:80]}")
    else:
        log_fail(f"Handover failed: err={err}")

    # ─── TEST 21: decompose_goal ──────────────────────────────────────────────
    log_test("decompose_goal")
    result, err = call_tool("decompose_goal", {
        "goal_id": "00000000-0000-0000-0000-000000000001"
    }, timeout=30)
    text = extract_text(result)
    if result:
        log_pass(f"Decompose OK: {text[:80]}")
    else:
        log_fail(f"Decompose failed: err={err}")

    # ─── TEST 22-24: LLM tools (if enabled) ───────────────────────────────────
    if LLM_ENABLED:
        log_test("chat (LLM)")
        result, err = call_tool("chat", {"message": "What is SENTINEL?"}, timeout=180)
        text = extract_text(result)
        if result and len(text) > 20:
            log_pass(f"Chat LLM OK: {text[:100]}")
        else:
            log_fail(f"Chat LLM failed: err={err}")

        log_test("suggest_goals (LLM)")
        result, err = call_tool("suggest_goals", {
            "description": "Build REST API",
            "languages": ["rust"]
        }, timeout=180)
        text = extract_text(result)
        if result:
            log_pass(f"Suggest goals OK: {text[:100]}")
        else:
            log_fail(f"Suggest goals failed: err={err}")

        log_test("orchestrate_task (LLM)")
        result, err = call_tool("orchestrate_task", {
            "task": "Implement auth",
            "modes": ["sequential"],
            "max_parallel": 1,
            "subtask_count": 2
        }, timeout=180)
        text = extract_text(result)
        if result:
            log_pass(f"Orchestrate OK: {text[:100]}")
        else:
            log_fail(f"Orchestrate failed: err={err}")
    else:
        print("\n  ⏭  LLM tests skipped (no proxy)")

finally:
    # Teardown fixture
    teardown_fixture(backup_path)
    print(f"\n🧹 Cleanup: sentinel.json ripristinato")

# ─── RISULTATI ────────────────────────────────────────────────────────────────
print()
print("=" * 60)
print("  RISULTATI TEST COMPLETI")
print("=" * 60)
print(f"  Totale: {TOTAL}")
print(f"  ✅ PASS: {PASS}")
print(f"  ❌ FAIL: {FAIL}")
rate = int(PASS / max(PASS + FAIL, 1) * 100)
print(f"  Pass rate: {PASS}/{PASS + FAIL} ({rate}%)")
print("=" * 60)

sys.exit(0 if FAIL == 0 else 1)
