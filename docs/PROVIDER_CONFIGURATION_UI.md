# 🔐 Provider Configuration UI - Guida Sicura

## Panoramica

SENTINEL ora include un'interfaccia grafica sicura per configurare le API keys dei provider LLM direttamente nell'estensione VSCode.

## ✅ Sicurezza al 100%

### Come sono protette le tue API keys:

1. **VSCode SecretStorage**
   - Le API keys sono criptate usando il keychain del tuo sistema operativo
   - **macOS**: Keychain
   - **Windows**: Credential Manager
   - **Linux**: Secret Service API / libsecret

2. **Crittografia**
   - I dati sono criptati a riposo
   - Solo VSCode può accedere alle keys
   - Non sono mai sincronizzate nel cloud
   - Non sono mai inviate a server SENTINEL

3. **Isolamento**
   - Ogni workspace ha le proprie keys isolate
   - Le keys non sono visibili in plain text
   - Richiede autenticazione OS per accedere

## 🚀 Come Usare

### 1. Aprire la UI di Configurazione

**Metodo 1 - Command Palette:**
```
Cmd/Ctrl + Shift + P → "Sentinel: Configure LLM Providers"
```

**Metodo 2 - Sidebar:**
- Clicca sull'icona SENTINEL nella sidebar
- Seleziona la tab "Provider Config"

**Metodo 3 - Shortcut:**
```
Cmd/Ctrl + Shift + Alt + P (configurabile)
```

### 2. Aggiungere un Provider

1. **Seleziona il provider** dalla lista (es. OpenRouter, OpenAI, etc.)
2. **Clicca "Get Key →"** per aprire il sito del provider
3. **Crea una nuova API key** sul sito del provider
4. **Copia la key** e torna su VSCode
5. **Incolla la key** nell'input field
6. **Clicca "Save Securely"**

### 3. Provider Supportati

| Provider | Icona | Modello Default | Costo |
|----------|-------|----------------|-------|
| **OpenRouter** | 🌐 | deepseek/deepseek-r1-0528:free | Gratuito |
| **OpenAI** | 🤖 | gpt-4o-mini | $ |
| **Anthropic** | 🧠 | claude-3-5-sonnet | $$ |
| **Google** | 🔍 | gemini-1.5-flash | $ |
| **Groq** | ⚡ | llama-3.1-70b | $ |
| **Ollama** | 📦 | llama3.2 | Gratuito (locale) |

### 4. Funzionalità Avanzate

#### Test Connessione
- Clicca l'icona 🧪 per testare la connessione
- Verifica che la key sia valida senza consumare token

#### Rimozione Sicura
- Clicca l'icona 🗑️ per rimuovere una key
- La key viene eliminata permanentemente dal sistema

#### Export Script
- Clicca "📋 Export Env Script" per generare uno script bash
- Utile per configurare l'ambiente CLI
- Lo script viene copiato negli appunti

#### Auto-Approve
Configura pattern per approvazione automatica:
```typescript
// Esempio: Auto-approva operazioni sicure
hitl.addAutoApprovePattern({
  request_type: ApprovalType.FileModification,
  condition: (req) => req.security_level === SecurityLevel.Low
});
```

## 🔒 Best Practices

### 1. Rotazione delle Keys
- Rota le API keys ogni 90 giorni
- Usa la funzione "Delete" e poi aggiungi la nuova key

### 2. Multiple Provider
- Configura almeno 2 provider per fallback automatico
- Se OpenRouter fallisce, SENTINEL passa a OpenAI automaticamente

### 3. Ollama (Locale)
- Per massima privacy, usa Ollama
- I modelli girano localmente, zero costi, zero dati inviati

### 4. Limiti di Spesa
- Configura limiti di spesa sui siti dei provider
- SENTINEL mostra un avviso quando si supera la soglia

## 🛡️ Sicurezza Aggiuntiva

### Human-in-the-Loop (HITL)
Quando configurato, SENTINEL richiede approvazione per:
- Modifiche a file esistenti
- Decisioni architetturali importanti
- Operazioni di sicurezza
- Superamento soglie di costo

### Audit Trail
Tutte le operazioni sono loggate:
```
[SENTINEL] Provider openrouter configured successfully
[SENTINEL] Provider openai removed
[SENTINEL] Testing anthropic connection...
```

### Nessun Telemetry
- SENTINEL non invia telemetry data
- Le tue keys rimangono solo sul tuo sistema
- Nessun tracking delle richieste

## 🐛 Troubleshooting

### "Invalid API key format"
- Verifica di aver copiato l'intera key
- Controlla che non ci siano spazi extra
- Ollama richiede un URL (es. http://localhost:11434)

### "Connection failed"
- Verifica la connessione internet
- Controlla che la key sia attiva sul provider
- Prova a rigenerare la key

### "Provider not responding"
- Il provider potrebbe essere down
- SENTINEL passerà automaticamente al provider fallback
- Controlla lo stato su https://status.openai.com (o simili)

## 📊 Monitoraggio

### Status Bar
L'icona SENTINEL nella status bar mostra:
- ✅ Verde: Tutti i provider OK
- ⚠️ Giallo: Alcuni provider non configurati
- 🔴 Rosso: Nessun provider configurato

### Provider Health
Ogni 60 secondi SENTINEL verifica:
- Connettività provider
- Validità API keys
- Latenza media
- Fallback attivo

## 📝 Esempi

### Configurazione Base
```bash
# 1. Configura OpenRouter (gratuito)
Provider: OpenRouter
API Key: sk-or-v1-xxxxxxxx

# 2. Configura OpenAI (fallback)
Provider: OpenAI
API Key: sk-xxxxxxxx

# 3. Testa la connessione
Clicca 🧪 su entrambi
```

### Configurazione Avanzata (Multi-provider)
```bash
# Primary: OpenRouter (free tier)
# Fallback 1: OpenAI (gpt-4o-mini)
# Fallback 2: Anthropic (claude-3-haiku)
# Local: Ollama (llama3.2)
```

### Configurazione Enterprise
```bash
# Tutti i provider configurati
# HITL abilitato per operazioni critiche
# Auto-approve per operazioni sicure
# Audit logging attivo
```

## 🎓 Conclusione

La configurazione dei provider in SENTINEL è:
- ✅ **100% sicura** - Crittografia OS-level
- ✅ **Facile** - UI intuitiva
- ✅ **Flessibile** - Supporto multi-provider
- ✅ **Affidabile** - Fallback automatico

**Non devi più preoccuparti di gestire file .env o variabili d'ambiente!**
