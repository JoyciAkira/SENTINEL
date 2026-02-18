#!/usr/bin/env python3
"""
gemini_cli_proxy.py — Proxy HTTP OpenAI-compatible → Gemini CLI (OAuth).

Avvia un server locale su http://localhost:9191 che espone:
  POST /v1/chat/completions  (OpenAI-compatible)

e invia le richieste al Gemini CLI autenticato tramite OAuth Google.

Uso:
  python3 scripts/gemini_cli_proxy.py

Poi configura l'extension (o il CLI):
  export SENTINEL_LLM_BASE_URL=http://localhost:9191/v1
  export SENTINEL_LLM_MODEL=gemini-3-flash-preview
  # Nessuna API key necessaria

Compatibilità: Python 3.8+, nessuna dipendenza esterna.
"""

import http.server
import json
import subprocess
import threading
import time
import sys
import os
import logging
from urllib.parse import urlparse

PORT = int(os.environ.get("GEMINI_PROXY_PORT", 9191))
GEMINI_BIN = os.environ.get("GEMINI_BIN", "gemini")
LOG_LEVEL = os.environ.get("GEMINI_PROXY_LOG", "INFO")

logging.basicConfig(
    level=getattr(logging, LOG_LEVEL, logging.INFO),
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("gemini-proxy")


def call_gemini_cli(prompt: str, timeout: int = 60) -> tuple[str, int]:
    """
    Chiama `gemini -p <prompt> -o json` e ritorna (response_text, token_count).
    """
    try:
        result = subprocess.run(
            [GEMINI_BIN, "-p", prompt, "-o", "json"],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        if result.returncode != 0:
            err = result.stderr.strip()
            log.error("gemini CLI error (exit %d): %s", result.returncode, err[:300])
            return f"[Gemini CLI error: {err[:200]}]", 0

        raw = result.stdout.strip()
        if not raw:
            return "[Gemini CLI returned empty response]", 0

        try:
            parsed = json.loads(raw)
            response = parsed.get("response", raw)
            # Estrai token count dagli stats
            tokens = 0
            models = parsed.get("stats", {}).get("models", {})
            for model_stats in models.values():
                tokens += model_stats.get("tokens", {}).get("total", 0)
            return response, tokens
        except json.JSONDecodeError:
            return raw, 0

    except subprocess.TimeoutExpired:
        log.error("gemini CLI timeout after %ds", timeout)
        return "[Gemini CLI timeout]", 0
    except FileNotFoundError:
        log.error("gemini binary not found at: %s", GEMINI_BIN)
        return "[gemini CLI not found in PATH]", 0


def build_prompt_from_messages(messages: list) -> str:
    """
    Converte i messaggi OpenAI (system/user/assistant) in un singolo prompt per Gemini CLI.
    """
    parts = []
    for msg in messages:
        role = msg.get("role", "user")
        content = msg.get("content", "")
        if not content:
            continue
        if role == "system":
            parts.append(f"[SYSTEM]\n{content}")
        elif role == "assistant":
            parts.append(f"[ASSISTANT]\n{content}")
        else:
            parts.append(f"[USER]\n{content}")
    return "\n\n".join(parts)


def make_openai_response(content: str, model: str, token_count: int) -> dict:
    """
    Crea una risposta JSON compatibile con OpenAI /v1/chat/completions.
    """
    import time as t
    return {
        "id": f"chatcmpl-gemini-{int(t.time())}",
        "object": "chat.completion",
        "created": int(t.time()),
        "model": model,
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content,
                },
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": max(0, token_count - len(content.split())),
            "completion_tokens": len(content.split()),
            "total_tokens": token_count if token_count > 0 else len(content.split()),
        },
    }


class GeminiProxyHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        # Sostituisce il logging default con il nostro
        log.debug("HTTP %s", format % args)

    def send_json(self, status: int, data: dict):
        body = json.dumps(data).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        self.end_headers()

    def do_GET(self):
        if self.path in ("/health", "/v1/health"):
            self.send_json(200, {"status": "ok", "backend": "gemini-cli", "port": PORT})
        elif self.path in ("/v1/models",):
            self.send_json(200, {
                "object": "list",
                "data": [
                    {"id": "gemini-3-flash-preview", "object": "model"},
                    {"id": "gemini-2.5-pro-preview", "object": "model"},
                    {"id": "gemini-2.0-flash", "object": "model"},
                ]
            })
        else:
            self.send_json(404, {"error": "not found"})

    def do_POST(self):
        if self.path not in ("/v1/chat/completions", "/chat/completions"):
            self.send_json(404, {"error": {"message": f"Unknown endpoint: {self.path}"}})
            return

        # Leggi body
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length > 0 else b"{}"

        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            self.send_json(400, {"error": {"message": "Invalid JSON body"}})
            return

        messages = payload.get("messages", [])
        model = payload.get("model", "gemini-3-flash-preview")

        if not messages:
            self.send_json(400, {"error": {"message": "messages array is required"}})
            return

        # Build prompt e chiama Gemini CLI
        prompt = build_prompt_from_messages(messages)
        log.info("→ Gemini CLI [%s] prompt=%d chars", model, len(prompt))

        start = time.time()
        response_text, token_count = call_gemini_cli(prompt)
        elapsed = time.time() - start

        log.info(
            "← Gemini CLI response: %d chars, ~%d tokens, %.1fs",
            len(response_text), token_count, elapsed,
        )

        result = make_openai_response(response_text, model, token_count)
        self.send_json(200, result)


def check_gemini_available() -> bool:
    try:
        r = subprocess.run(
            [GEMINI_BIN, "--version"],
            capture_output=True, text=True, timeout=5
        )
        return r.returncode == 0
    except Exception:
        return False


def main():
    if not check_gemini_available():
        log.error("❌ Gemini CLI non trovato o non eseguibile: %s", GEMINI_BIN)
        log.error("   Installa con: npm install -g @google/gemini-cli")
        log.error("   Poi autenticati con: gemini (primo avvio)")
        sys.exit(1)

    server = http.server.HTTPServer(("127.0.0.1", PORT), GeminiProxyHandler)

    log.info("🚀 Gemini CLI Proxy avviato su http://127.0.0.1:%d", PORT)
    log.info("   Backend: %s (OAuth Google AI Pro)", GEMINI_BIN)
    log.info("")
    log.info("   Configura SENTINEL:")
    log.info("   export SENTINEL_LLM_BASE_URL=http://localhost:%d/v1", PORT)
    log.info("   export SENTINEL_LLM_MODEL=gemini-3-flash-preview")
    log.info("")
    log.info("   Health check: curl http://localhost:%d/health", PORT)
    log.info("   Premi Ctrl+C per fermare")

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Proxy fermato.")
        server.server_close()


if __name__ == "__main__":
    main()
