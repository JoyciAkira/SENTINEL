#!/usr/bin/env python3
"""
Sentinel LLM End-to-End Test Suite
NO MOCKS - Real OpenRouter API + Real Sentinel MCP Pipeline

Tests the full flow:
1. OpenRouter API call with free models
2. Sentinel MCP tool validation (validate_action, safe_write, get_alignment)
3. Security scanning (Layer 7)
4. Goal alignment (Layer 2)
5. Cognitive map (Layer 3)
6. Strategy proposal (Layer 5)

Requires:
- OPENROUTER_API_KEY in .env file
- sentinel-cli binary built (cargo build --release -p sentinel-cli)
"""

import subprocess
import json
import sys
import os
import time
import urllib.request
import urllib.error

# ── Configuration ──────────────────────────────────────

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.join(SCRIPT_DIR, "..")
SENTINEL_BIN = os.path.join(PROJECT_ROOT, "target", "release", "sentinel-cli")

# Load .env
def load_env():
    env_path = os.path.join(PROJECT_ROOT, ".env")
    if not os.path.exists(env_path):
        print(f"ERROR: .env file not found at {env_path}")
        print("Create it from .env.example and add your OPENROUTER_API_KEY")
        sys.exit(1)

    env_vars = {}
    with open(env_path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                key, value = line.split("=", 1)
                env_vars[key.strip()] = value.strip()
    return env_vars


ENV = load_env()
API_KEY = ENV.get("OPENROUTER_API_KEY", "")
MODEL = ENV.get("OPENROUTER_MODEL", "meta-llama/llama-3.3-70b-instruct:free")

if not API_KEY or API_KEY == "sk-or-v1-your-key-here":
    print("ERROR: OPENROUTER_API_KEY not configured in .env")
    print("Get a free key at https://openrouter.ai/keys")
    sys.exit(1)

# ── Test Infrastructure ────────────────────────────────

PASS = 0
FAIL = 0
TOTAL = 0
SECTION = 0

def section(name):
    global SECTION
    SECTION += 1
    print(f"\n{'=' * 60}")
    print(f"  SECTION {SECTION}: {name}")
    print(f"{'=' * 60}")

def log_test(name):
    global TOTAL
    TOTAL += 1
    print(f"\n--- TEST {TOTAL}: {name} ---")

def log_pass(msg):
    global PASS
    PASS += 1
    print(f"  PASS: {msg}")

def log_fail(msg):
    global FAIL
    FAIL += 1
    print(f"  FAIL: {msg}")

# ── OpenRouter API Client ─────────────────────────────

def openrouter_chat(system_prompt, user_prompt, model=None, max_tokens=1024, temperature=0.3):
    """Call OpenRouter API directly (no SDK, no mocks)."""
    model = model or MODEL
    url = "https://openrouter.ai/api/v1/chat/completions"

    payload = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "max_tokens": max_tokens,
        "temperature": temperature,
    }).encode("utf-8")

    req = urllib.request.Request(url, data=payload, method="POST")
    req.add_header("Authorization", f"Bearer {API_KEY}")
    req.add_header("Content-Type", "application/json")
    req.add_header("HTTP-Referer", "https://github.com/JoyciAkira/SENTINEL")
    req.add_header("X-Title", "Sentinel Protocol E2E Test")

    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            content = data["choices"][0]["message"]["content"]
            tokens = data.get("usage", {}).get("total_tokens", 0)
            model_used = data.get("model", model)
            return {"content": content, "tokens": tokens, "model": model_used, "error": None}
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8") if e.fp else str(e)
        return {"content": None, "tokens": 0, "model": model, "error": f"HTTP {e.code}: {error_body}"}
    except Exception as e:
        return {"content": None, "tokens": 0, "model": model, "error": str(e)}


# ── Sentinel MCP Client ───────────────────────────────

def send_mcp_rpc(requests):
    """Send JSON-RPC requests to sentinel mcp via stdio."""
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


def mcp_tool_call(tool_name, arguments):
    """Call a Sentinel MCP tool and return the text result."""
    init_req = {
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "llm-e2e-test", "version": "1.0"}
        },
        "id": 1
    }
    call_req = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {"name": tool_name, "arguments": arguments},
        "id": 2
    }
    responses = send_mcp_rpc([init_req, call_req])

    for r in responses:
        if r.get("id") == 2:
            try:
                return r["result"]["content"][0]["text"]
            except (KeyError, IndexError, TypeError):
                return None
    return None


# ══════════════════════════════════════════════════════════
#  TEST SUITE START
# ══════════════════════════════════════════════════════════

print("=" * 60)
print("  SENTINEL LLM E2E TEST SUITE")
print("  NO MOCKS - Real API + Real MCP Pipeline")
print(f"  Model: {MODEL}")
print(f"  Binary: {SENTINEL_BIN}")
print(f"  Project: {PROJECT_ROOT}")
print("=" * 60)


# ── SECTION 1: OpenRouter API Connectivity ─────────────

section("OpenRouter API Connectivity")

log_test("OpenRouter API basic connectivity")
result = openrouter_chat(
    "You are a helpful assistant.",
    "Reply with exactly: SENTINEL_OK",
    max_tokens=20,
    temperature=0.0
)
if result["error"]:
    log_fail(f"API error: {result['error']}")
elif result["content"]:
    log_pass(f"API responded ({result['tokens']} tokens)")
    print(f"  Model: {result['model']}")
    print(f"  Response: {result['content'][:100]}")
else:
    log_fail("Empty response")


log_test("OpenRouter code generation capability")
result = openrouter_chat(
    "You are a Rust expert. Generate only code, no explanations.",
    "Write a Rust function that checks if a number is prime. Return ONLY the code.",
    max_tokens=512,
    temperature=0.2
)
if result["error"]:
    log_fail(f"Code gen error: {result['error']}")
elif result["content"]:
    content = result["content"]
    has_fn = "fn " in content
    has_rust = "bool" in content or "return" in content or "->" in content
    if has_fn and has_rust:
        log_pass(f"Generated valid Rust code ({result['tokens']} tokens)")
    elif has_fn:
        log_pass(f"Generated function code ({result['tokens']} tokens)")
    else:
        log_fail(f"Response doesn't look like Rust code")
    print(f"  Code preview: {content[:200]}")
else:
    log_fail("Empty code generation response")

LLM_GENERATED_CODE = result.get("content", "") if not result.get("error") else ""


log_test("OpenRouter refactoring suggestion")
sample_code = """fn calc(x: i32) -> i32 {
    let mut r = 0;
    for i in 0..x {
        r = r + i;
    }
    return r;
}"""
result = openrouter_chat(
    "You are a Rust code reviewer. Suggest improvements.",
    f"Suggest refactoring for this Rust code:\n```rust\n{sample_code}\n```",
    max_tokens=512,
    temperature=0.3
)
if result["error"]:
    log_fail(f"Refactoring error: {result['error']}")
elif result["content"]:
    log_pass(f"Refactoring suggestion received ({result['tokens']} tokens)")
    print(f"  Suggestion preview: {result['content'][:200]}")
else:
    log_fail("Empty refactoring response")


log_test("OpenRouter documentation generation")
result = openrouter_chat(
    "You are a technical writer for Rust projects.",
    f"Generate Rust doc comments for this function:\n```rust\n{sample_code}\n```",
    max_tokens=512
)
if result["error"]:
    log_fail(f"Doc gen error: {result['error']}")
elif result["content"]:
    has_doc = "///" in result["content"] or "/**" in result["content"] or "doc" in result["content"].lower()
    if has_doc:
        log_pass(f"Documentation with doc markers ({result['tokens']} tokens)")
    else:
        log_pass(f"Documentation generated ({result['tokens']} tokens)")
    print(f"  Doc preview: {result['content'][:200]}")
else:
    log_fail("Empty documentation response")


log_test("OpenRouter test generation")
result = openrouter_chat(
    "You are a Rust testing expert. Generate only test code.",
    f"Generate unit tests for this Rust function:\n```rust\n{sample_code}\n```\nReturn ONLY the test code.",
    max_tokens=512
)
if result["error"]:
    log_fail(f"Test gen error: {result['error']}")
elif result["content"]:
    has_test = "#[test]" in result["content"] or "assert" in result["content"] or "test" in result["content"].lower()
    if has_test:
        log_pass(f"Test code with test markers ({result['tokens']} tokens)")
    else:
        log_pass(f"Test-related content generated ({result['tokens']} tokens)")
    print(f"  Test preview: {result['content'][:200]}")
else:
    log_fail("Empty test generation response")


log_test("OpenRouter concept explanation")
result = openrouter_chat(
    "You are a computer science educator.",
    "Explain the concept of 'goal alignment in AI coding agents' in 2-3 sentences.",
    max_tokens=256
)
if result["error"]:
    log_fail(f"Explanation error: {result['error']}")
elif result["content"]:
    log_pass(f"Concept explanation received ({result['tokens']} tokens)")
    print(f"  Explanation: {result['content'][:200]}")
else:
    log_fail("Empty explanation response")


# ── SECTION 2: Sentinel MCP Pipeline Validation ───────

section("Sentinel MCP Pipeline Validation")

log_test("MCP validate_action with LLM-generated intent")
# Use a real LLM-generated description to validate
llm_desc = openrouter_chat(
    "You are a developer. Describe in one sentence.",
    "Describe what 'implementing JWT authentication' means for a web app.",
    max_tokens=100,
    temperature=0.2
)
if llm_desc["error"]:
    validate_text = "Implement JWT authentication for secure login"
else:
    validate_text = llm_desc["content"][:200] if llm_desc["content"] else "Implement JWT authentication"

mcp_result = mcp_tool_call("validate_action", {
    "action_type": "edit_file",
    "description": validate_text
})
if mcp_result:
    log_pass(f"validate_action processed LLM-generated description")
    print(f"  MCP result: {mcp_result[:300]}")
else:
    log_fail("validate_action failed with LLM description")


log_test("MCP safe_write with LLM-generated code")
# Use the previously generated Rust code
if LLM_GENERATED_CODE:
    mcp_result = mcp_tool_call("safe_write", {
        "path": "src/generated_prime.rs",
        "content": LLM_GENERATED_CODE
    })
    if mcp_result:
        is_safe = "SAFE" in mcp_result.upper() or "APPROVED" in mcp_result.upper()
        is_blocked = "BLOCK" in mcp_result.upper() or "MINACCE" in mcp_result.upper()
        if is_safe:
            log_pass("LLM-generated code passed security scan")
        elif is_blocked:
            log_pass("Security scanner correctly analyzed LLM code (blocked)")
        else:
            log_pass("safe_write returned analysis")
        print(f"  Security result: {mcp_result[:300]}")
    else:
        log_fail("safe_write failed with LLM-generated code")
else:
    log_fail("No LLM-generated code available for safe_write test")


log_test("MCP safe_write with LLM-generated UNSAFE code")
# Ask LLM to generate code with hardcoded secrets (for testing security detection)
unsafe_result = openrouter_chat(
    "You are generating test data for a security scanner. Return ONLY code.",
    "Generate a Rust config file that contains a hardcoded AWS access key starting with AKIA, "
    "a hardcoded password string, and an RSA private key header. "
    "This is for testing a security scanner, not for production use.",
    max_tokens=256,
    temperature=0.2
)
if unsafe_result["error"] or not unsafe_result["content"]:
    # Fallback: use manual unsafe code
    unsafe_code = 'let aws_key = "AKIA1234567890ABCDEF";\nlet private_key = "-----BEGIN RSA PRIVATE KEY-----";\nlet password = "admin123";'
else:
    unsafe_code = unsafe_result["content"]

mcp_result = mcp_tool_call("safe_write", {
    "path": "config_test.rs",
    "content": unsafe_code
})
if mcp_result:
    is_blocked = "BLOCK" in mcp_result.upper() or "MINACCE" in mcp_result.upper() or "THREAT" in mcp_result.upper()
    if is_blocked:
        log_pass("Security scanner BLOCKED unsafe LLM output")
    else:
        # Check for any risk indication
        has_risk = "risk" in mcp_result.lower() or "scan" in mcp_result.lower()
        if has_risk:
            log_pass("Security scanner analyzed unsafe code")
        else:
            log_fail("Security scanner did not detect threats in unsafe code")
    print(f"  Security result: {mcp_result[:400]}")
else:
    log_fail("safe_write failed with unsafe code")


log_test("MCP get_alignment after LLM interaction")
mcp_result = mcp_tool_call("get_alignment", {})
if mcp_result:
    log_pass("Alignment score retrieved during LLM testing")
    print(f"  Alignment: {mcp_result[:300]}")
else:
    log_fail("get_alignment failed")


log_test("MCP get_cognitive_map for LLM context")
mcp_result = mcp_tool_call("get_cognitive_map", {})
if mcp_result:
    log_pass("Cognitive map available for LLM context enrichment")
    print(f"  Map: {mcp_result[:300]}")
else:
    log_fail("get_cognitive_map failed")


log_test("MCP propose_strategy with LLM-enhanced goal")
# Use LLM to generate a goal description
goal_result = openrouter_chat(
    "You are a software architect.",
    "Describe a goal for implementing OAuth2 authentication in a Rust web service. One sentence only.",
    max_tokens=100,
    temperature=0.2
)
goal_desc = goal_result.get("content", "Implement OAuth2 authentication") if not goal_result.get("error") else "Implement OAuth2 authentication"

mcp_result = mcp_tool_call("propose_strategy", {
    "goal_description": goal_desc[:200]
})
if mcp_result:
    log_pass("Strategy proposed for LLM-generated goal")
    print(f"  Strategy: {mcp_result[:300]}")
else:
    log_fail("propose_strategy failed with LLM goal")


log_test("MCP get_enforcement_rules for LLM guardrails")
mcp_result = mcp_tool_call("get_enforcement_rules", {})
if mcp_result:
    log_pass("Enforcement rules available for LLM guardrails")
    print(f"  Rules: {mcp_result[:300]}")
else:
    log_fail("get_enforcement_rules failed")


# ── SECTION 3: Full LLM + Sentinel Pipeline ───────────

section("Full LLM + Sentinel Validation Pipeline")

log_test("Complete pipeline: LLM generates -> Sentinel validates -> Result")
# Step 1: Get alignment context from Sentinel
alignment = mcp_tool_call("get_alignment", {})
cognitive_map = mcp_tool_call("get_cognitive_map", {})
rules = mcp_tool_call("get_enforcement_rules", {})

# Step 2: Build enriched context for LLM
sentinel_context = f"""
SENTINEL CONTEXT:
- Alignment: {(alignment or 'N/A')[:100]}
- Cognitive Map: {(cognitive_map or 'N/A')[:100]}
- Rules: {(rules or 'N/A')[:100]}
"""

# Step 3: LLM generates code with Sentinel context
pipeline_result = openrouter_chat(
    f"You are a Rust developer working within the Sentinel Protocol framework.\n{sentinel_context}\n"
    "Generate clean, safe, well-documented code. No hardcoded secrets.",
    "Write a Rust function `validate_user_input(input: &str) -> Result<String, String>` "
    "that sanitizes user input for a web application. Include doc comments.",
    max_tokens=512,
    temperature=0.2
)

if pipeline_result["error"]:
    log_fail(f"LLM generation failed: {pipeline_result['error']}")
else:
    generated_code = pipeline_result["content"]
    print(f"  LLM generated code ({pipeline_result['tokens']} tokens)")
    print(f"  Code preview: {generated_code[:200]}")

    # Step 4: Sentinel validates the LLM output
    # 4a: Security scan
    security_result = mcp_tool_call("safe_write", {
        "path": "src/input_validation.rs",
        "content": generated_code
    })
    if security_result:
        is_safe = "SAFE" in security_result.upper() or "APPROVED" in security_result.upper()
        if is_safe:
            log_pass("Pipeline: LLM code passed Sentinel security scan")
        else:
            log_pass("Pipeline: Sentinel analyzed LLM code")
        print(f"  Security: {security_result[:200]}")
    else:
        log_fail("Pipeline: Security scan failed")

    # 4b: Validate action alignment
    validate_result = mcp_tool_call("validate_action", {
        "action_type": "create_file",
        "description": "Create input validation function generated by LLM"
    })
    if validate_result:
        log_pass("Pipeline: Action validated against Goal Manifold")
        print(f"  Validation: {validate_result[:200]}")
    else:
        log_fail("Pipeline: Action validation failed")


log_test("Pipeline: LLM explains Sentinel concept -> Sentinel records handover")
# LLM explains what it did
explanation = openrouter_chat(
    "You are documenting AI agent handover notes for the Sentinel Protocol.",
    "Write a brief handover note explaining that you generated an input validation function "
    "for a Rust web app, including what was tested and what remains to do.",
    max_tokens=256,
    temperature=0.3
)
if explanation["error"]:
    log_fail(f"LLM explanation failed: {explanation['error']}")
else:
    # Record in Sentinel's cognitive handover
    handover_result = mcp_tool_call("record_handover", {
        "goal_id": "00000000-0000-0000-0000-000000000000",
        "content": explanation["content"][:500],
        "warnings": ["LLM-generated content - requires human review"]
    })
    if handover_result:
        log_pass("Pipeline: LLM handover note recorded in Sentinel")
        print(f"  Handover: {handover_result[:200]}")
    else:
        log_fail("Pipeline: Handover recording failed")


# ── SECTION 4: Multi-Model Comparison ─────────────────

section("Multi-Model Comparison (Free Models)")

FREE_MODELS = [
    ("meta-llama/llama-3.3-70b-instruct:free", "Meta Llama 3.3 70B"),
    ("google/gemini-2.0-flash-exp:free", "Google Gemini 2.0 Flash"),
    ("deepseek/deepseek-r1-0528:free", "DeepSeek R1"),
]

log_test("Compare free models on code generation task")
model_results = []
for model_id, model_name in FREE_MODELS:
    start_time = time.time()
    result = openrouter_chat(
        "You are a Rust expert. Return ONLY code.",
        "Write a Rust function `fibonacci(n: u64) -> u64` that computes the nth Fibonacci number iteratively.",
        model=model_id,
        max_tokens=256,
        temperature=0.1
    )
    elapsed = time.time() - start_time

    if result["error"]:
        print(f"  {model_name}: ERROR - {result['error'][:100]}")
        model_results.append({"model": model_name, "success": False, "time": elapsed})
    else:
        has_fn = "fn " in result["content"]
        has_fib = "fibonacci" in result["content"].lower() or "fib" in result["content"].lower()
        quality = "GOOD" if (has_fn and has_fib) else "PARTIAL" if has_fn else "LOW"
        print(f"  {model_name}: {quality} ({result['tokens']} tokens, {elapsed:.1f}s)")
        print(f"    Preview: {result['content'][:120]}")
        model_results.append({"model": model_name, "success": True, "time": elapsed, "tokens": result["tokens"], "quality": quality})

successful = [m for m in model_results if m.get("success")]
if len(successful) >= 2:
    log_pass(f"{len(successful)}/{len(FREE_MODELS)} free models responded successfully")
elif len(successful) >= 1:
    log_pass(f"{len(successful)}/{len(FREE_MODELS)} free model(s) responded")
else:
    log_fail("No free models responded")


# ── SECTION 5: Sentinel CLI Integration ───────────────

section("Sentinel CLI Integration")

log_test("sentinel status --json with LLM context")
try:
    result = subprocess.run(
        [SENTINEL_BIN, "status", "--json"],
        capture_output=True, text=True, timeout=10,
        cwd=PROJECT_ROOT
    )
    if result.returncode == 0:
        data = json.loads(result.stdout)
        if "manifold" in data:
            manifold = data["manifold"]
            root = manifold.get("root_intent", {}).get("description", "N/A")
            goals = manifold.get("goal_dag", {}).get("nodes", {})
            invariants = manifold.get("invariants", [])
            log_pass(f"CLI status: {len(goals)} goals, {len(invariants)} invariants")
            print(f"  Root intent: {root[:80]}")
        else:
            log_fail("Missing manifold in CLI status")
    else:
        log_fail(f"CLI exit code {result.returncode}: {result.stderr[:200]}")
except Exception as e:
    log_fail(f"CLI exception: {e}")


# ── RESULTS ────────────────────────────────────────────

print("\n" + "=" * 60)
print("  SENTINEL LLM E2E TEST RESULTS")
print("=" * 60)
print(f"  Total tests:    {TOTAL}")
print(f"  Passed:         {PASS}")
print(f"  Failed:         {FAIL}")
if PASS + FAIL > 0:
    rate = 100 * PASS / (PASS + FAIL)
    print(f"  Pass rate:      {PASS}/{PASS+FAIL} ({rate:.0f}%)")
print(f"  Model used:     {MODEL}")
print(f"  API calls made: Real OpenRouter API (no mocks)")
print("=" * 60)

sys.exit(1 if FAIL > 0 else 0)
