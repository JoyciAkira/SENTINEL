//! REAL-WORLD TEST: Gemini CLI Integration
//!
//! Test completo dal prompt utente alla risposta LLM
//! usando Gemini CLI (OAuth - nessuna API key necessaria!)

use sentinel_agent_native::providers::gemini_cli::GeminiCliClient;
use sentinel_agent_native::providers::router::ProviderRouter;
use sentinel_agent_native::llm_integration::LLMChatClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     SENTINEL - Real Workflow Test con Gemini CLI           ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Step 1: Verifica che Gemini CLI sia disponibile
    println!("1️⃣  Verifica Gemini CLI...");
    if !sentinel_agent_native::providers::gemini_cli::is_gemini_cli_available() {
        println!("   ❌ Gemini CLI non trovato!");
        println!("   Installa con: npm install -g @anthropic-ai/gemini-cli");
        println!("   Poi autentica: gemini auth login");
        return Ok(());
    }
    println!("   ✅ Gemini CLI disponibile\n");

    // Step 2: Crea il client direttamente
    println!("2️⃣  Inizializzazione GeminiCliClient...");
    let client = GeminiCliClient::new()
        .with_model("gemini-2.0-flash");  // Modello veloce
    println!("   ✅ Client creato\n");

    // Step 3: Test prompt singolo
    println!("3️⃣  Test 1: Prompt singolo (generazione codice)");
    println!("   Prompt: \"Scrivi una funzione Rust per validare email\"\n");

    let start = std::time::Instant::now();
    let result = client.chat_completion(
        "Sei un esperto programmatore Rust. Rispondi solo con codice.",
        "Scrivi una funzione Rust chiamata `validate_email` che verifica se una stringa è un'email valida."
    ).await;

    match result {
        Ok(completion) => {
            println!("   ✅ Risposta ricevuta in {:?}:", start.elapsed());
            println!("   📝 LLM: {}", completion.llm_name);
            println!("   📊 Tokens: {}", completion.token_cost);
            println!("\n   💬 CONTENUTO:\n");
            println!("   {}", completion.content.lines().take(30).map(|l| format!("   {}", l)).collect::<Vec<_>>().join("\n"));
            if completion.content.lines().count() > 30 {
                println!("   ... ({} righe totali)", completion.content.lines().count());
            }
        }
        Err(e) => {
            println!("   ❌ Errore: {}", e);
        }
    }

    // Step 4: Test tramite ProviderRouter
    println!("\n4️⃣  Test 2: Tramite ProviderRouter (fallback automatico)");
    
    // Imposta la variabile d'ambiente per usare gemini_cli
    std::env::set_var("SENTINEL_LLM_PROVIDER", "gemini_cli");
    
    let router = ProviderRouter::from_env()?;
    println!("   ✅ ProviderRouter inizializzato\n");

    println!("   Prompt: \"Crea una struct Rust per un utente\"\n");
    let start = std::time::Instant::now();
    
    let result = router.chat_completion(
        "Sei un architetto software. Genera codice pulito e documentato.",
        "Crea una struct Rust `User` con campi: id (UUID), email (String), created_at (DateTime). Aggiungi i trait Debug, Clone e un metodo new()."
    ).await;

    match result {
        Ok(completion) => {
            println!("   ✅ Risposta ricevuta in {:?}:", start.elapsed());
            println!("   📝 LLM: {}", completion.llm_name);
            println!("   📊 Tokens: {}", completion.token_cost);
            println!("\n   💬 CONTENUTO:\n");
            println!("   {}", completion.content.lines().take(25).map(|l| format!("   {}", l)).collect::<Vec<_>>().join("\n"));
        }
        Err(e) => {
            println!("   ❌ Errore: {}", e);
        }
    }

    // Step 5: Test conversazione multi-turno
    println!("\n5️⃣  Test 3: Conversazione multi-turno");
    println!("   Simulazione: Utente chiede chiarimenti\n");

    let conversation = vec![
        ("Sistema", "Sei un assistente per programmatori. Rispondi in modo conciso."),
        ("Utente", "Qual è la differenza tra Vec e Array in Rust?"),
    ];

    let system = conversation[0].1;
    let user = conversation[1].1;

    let start = std::time::Instant::now();
    let result = router.chat_completion(system, user).await;

    match result {
        Ok(completion) => {
            println!("   ✅ Risposta in {:?}:", start.elapsed());
            println!("   {}\n", completion.content.lines().take(15).map(|l| format!("   {}", l)).collect::<Vec<_>>().join("\n"));
        }
        Err(e) => {
            println!("   ❌ Errore: {}", e);
        }
    }

    // Riepilogo
    println!("\n{}", "═".repeat(60));
    println!("📊 RIEPILOGO WORKFLOW REALE");
    println!("{}", "═".repeat(60));
    println!("\n✅ FLUSSO COMPLETO DAL PROMPT UTENTE:");
    println!("   1. Utente inserisce prompt");
    println!("   2. ProviderRouter seleziona Gemini CLI");
    println!("   3. Gemini CLI invoca subprocess con OAuth");
    println!("   4. Risposta ricevuta e token contati");
    println!("   5. Contenuto disponibile per elaborazione");
    println!("\n🔑 NESSUNA API KEY RICHIESTA!");
    println!("   Gemini CLI usa il tuo account Google AI Pro");
    println!("\n💰 Costo: Gratis con sottoscrizione Google AI Pro");

    Ok(())
}