import subprocess
import json
import sys
import os

def test_sentinel_protocol():
    print("🌐 SENTINEL UNIVERSAL PROTOCOL TEST")
    print("-----------------------------------")
    
    # 1. Avvio del Server Sentinel (Subprocess)
    # Simula un client (come Cline o Cursor) che lancia il server MCP
    sentinel_path = "./target/debug/sentinel-cli"
    if not os.path.exists(sentinel_path):
        print(f"❌ Errore: Binario non trovato in {sentinel_path}")
        sys.exit(1)

    process = subprocess.Popen(
        [sentinel_path, "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        text=True,
        bufsize=1
    )

    print("✅ Server Sentinel MCP avviato.")

    # 2. Handshake (Initialize)
    # Lo standard MCP richiede un'inizializzazione
    init_request = {
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "UniversalTestClient", "version": "1.0"}
        },
        "id": 1
    }
    
    print(f"📤 Sending Handshake...")
    process.stdin.write(json.dumps(init_request) + "\n")
    process.stdin.flush()
    
    response = json.loads(process.stdout.readline())
    print(f"📥 Received Handshake: {response['result']['serverInfo']['name']}")

    # 3. Interrogazione Onniscienza (The Protocol Core)
    # Chiediamo: "Qual è il mio scopo?"
    map_request = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_cognitive_map",
            "arguments": {}
        },
        "id": 2
    }

    print(f"📤 Requesting Cognitive Map (The Truth)...")
    process.stdin.write(json.dumps(map_request) + "\n")
    process.stdin.flush()

    response = json.loads(process.stdout.readline())
    content = response['result']['content'][0]['text']

    # 4. Verifica della North Star
    print("\n🔍 ANALYZING RESPONSE FOR PROTOCOL COMPLIANCE:")
    print("-" * 40)
    print(content[:200] + "...") # Stampa l'inizio della mappa
    print("-" * 40)

    if "ULTIMATE GOAL" in content:
        print("\n✅ SUCCESS: Sentinel has injected the Root Intent into the client.")
        print("   The protocol effectively guides the agent towards the North Star.")
    else:
        print("\n❌ FAILURE: Root Intent not found in Cognitive Map.")
        sys.exit(1)

    process.terminate()

if __name__ == "__main__":
    test_sentinel_protocol()
