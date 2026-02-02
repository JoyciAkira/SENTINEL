import sys
import os
import shutil
import tempfile
import json

# Aggiungi il path dell'SDK per l'import senza installazione
sdk_path = os.path.abspath("sdks/python")
sys.path.append(sdk_path)

from sentinel_sdk import SentinelClient, GoalManifold, Intent

def run_test():
    # 1. Configurazione Binario
    binary_path = os.path.abspath("target/release/sentinel-cli") # Cargo usa il nome del package
    # Check if the binary exists with the package name, otherwise try just 'sentinel'
    if not os.path.exists(binary_path):
        binary_path = os.path.abspath("target/release/sentinel")
    
    if not os.path.exists(binary_path):
        print(f"❌ ERRORE CRITICO: Binario non trovato in {binary_path}")
        print("Hai eseguito 'cargo build --release -p sentinel-cli'?")
        sys.exit(1)

    print(f"✅ Binario trovato: {binary_path}")

    # 2. Setup Ambiente di Test
    test_dir = tempfile.mkdtemp(prefix="sentinel_sdk_test_")
    print(f"📂 Ambiente di test creato: {test_dir}")
    
    try:
        # 3. Istanziazione Client
        client = SentinelClient(executable=binary_path, working_dir=test_dir)
        
        # 4. Test INIT
        print("\n🔹 Esecuzione 'init'...")
        description = "Costruire un'API Python sicura per il trading finanziario"
        client.init(description)
        
        # Verifica esistenza file
        json_path = os.path.join(test_dir, "sentinel.json")
        if os.path.exists(json_path):
            print("✅ sentinel.json creato correttamente.")
        else:
            print("❌ sentinel.json NON trovato.")
            sys.exit(1)

        # 5. Test STATUS
        print("\n🔹 Esecuzione 'status'...")
        manifold = client.status()
        
        # 6. Validazione Output Tipizzato
        print(f"✅ Risposta parsata come GoalManifold (Pydantic Model)")
        print(f"   Descrizione Intento: '{manifold.root_intent.description}'")
        print(f"   Numero Goal: {len(manifold.goals)}")
        print(f"   Versione: {len(manifold.version_history)}") # Dovrebbe essere >= 1 col nostro fix
        
        if manifold.root_intent.description == description:
            print("\n🎉 TEST INTEGRAZIONE SUPERATO: L'SDK Python comunica correttamente col Core Rust.")
        else:
            print("\n❌ TEST FALLITO: Discrepanza nei dati.")

    except Exception as e:
        print(f"\n❌ ECCEZIONE DURANTE IL TEST:\n{e}")
        import traceback
        traceback.print_exc()
    finally:
        # Cleanup
        shutil.rmtree(test_dir)
        print(f"\n🧹 Ambiente di test pulito.")

if __name__ == "__main__":
    run_test()
