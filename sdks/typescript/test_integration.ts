import { SentinelClient } from './src/client';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';

async function runTest() {
    console.log("🚀 SENTINEL TS SDK INTEGRATION TEST");

    // 1. Configurazione Binario
    let binaryPath = path.resolve('../../target/release/sentinel-cli');
    if (!fs.existsSync(binaryPath)) {
        binaryPath = path.resolve('../../target/release/sentinel');
    }

    if (!fs.existsSync(binaryPath)) {
        console.error(`❌ Binario non trovato in: ${binaryPath}`);
        console.error("Esegui 'cargo build --release' nella root del progetto.");
        process.exit(1);
    }
    console.log(`✅ Binario trovato: ${binaryPath}`);

    // 2. Setup Ambiente
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'sentinel-ts-test-'));
    console.log(`📂 Temp Dir: ${tempDir}`);

    try {
        // 3. Istanziazione Client
        const client = new SentinelClient(binaryPath, tempDir);

        // 4. Test INIT
        console.log("\n🔹 Esecuzione 'init'...");
        const description = "Sviluppare un front-end React sicuro per Sentinel";
        await client.init(description);

        const jsonPath = path.join(tempDir, "sentinel.json");
        if (fs.existsSync(jsonPath)) {
            console.log("✅ sentinel.json creato.");
        } else {
            throw new Error("sentinel.json non trovato dopo init.");
        }

        // 5. Test STATUS (Tipizzato)
        console.log("\n🔹 Esecuzione 'status'...");
        const manifold = await client.status();

        console.log(`✅ Risposta parsata come GoalManifold (Interface)`)
        console.log(`   Descrizione: '${manifold.root_intent.description}'`);
        console.log(`   Goal Totali: ${manifold.goals.length}`);
        
        // Verifica valori
        if (manifold.root_intent.description !== description) {
            throw new Error(`Mismatch descrizione. Atteso: '${description}', Ottenuto: '${manifold.root_intent.description}'`);
        }

        if (manifold.goals.length === 0) {
             console.warn("⚠️  Nessun goal trovato. L'Architect Engine ha generato goal?");
        } else {
            console.log(`   Primo Goal: ${manifold.goals[0].description}`);
            console.log(`   Status: ${manifold.goals[0].status}`);
        }

        console.log("\n🎉 TEST PASSATO: TypeScript SDK operativo.");

    } catch (error) {
        console.error("\n❌ TEST FALLITO:", error);
        process.exit(1);
    } finally {
        // Cleanup
        try {
            fs.rmSync(tempDir, { recursive: true, force: true });
            console.log("\n🧹 Pulizia completata.");
        } catch (e) {
            console.error("Errore pulizia:", e);
        }
    }
}

runTest();
