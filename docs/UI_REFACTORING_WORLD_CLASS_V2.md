# SENTINEL Extension - World-Class UX Refactoring Plan V2

## Current State Analysis

### Problems Identified

1. **MessageBubble troppo complesso**
   - `simpleMode` mostra "What I changed/approval/next" boxes che sembrano mock
   - Troppe varianti: `showInternals`, `askWhy`, `compact`, `simpleMode`
   - Il parsing del testo per estrarre sezioni è fragile

2. **QuickPrompts generico**
   - "Client Portal", "Booking App", "Spreadsheet to App" non contestualizzati
   - Non riflette lo stato reale del progetto

3. **Layout non ottimizzato per chat**
   - Sidebar icon-only occupa spazio
   - Header con troppi controlli (Start/Pause/Stop)
   - Status bar inutile per l'utente finale

4. **Niente reasoning trace visibile**
   - LLM genera risposte senza mostrare il "perché"
   - Manca la componente educativa per l'utente

---

## Target: Cline-like Experience

### Reference: Cline Extension UX

Cline ha un'UX molto pulita:
- Chat input sempre visibile in basso
- Messaggi con avatar e copy button
- Niente "outcome boxes" artificiali
- Tool calls mostrati come cards pulite
- File approvals integrati nel flow
- Stato mostrato organicamente nel messaggio

### Target UX Principles

1. **Conversational First**: Tutto è una conversazione, non forms
2. **Progressive Disclosure**: Dettagli solo quando richiesti
3. **Contextual Actions**: Azioni basate sullo stato reale
4. **Clean Visual Hierarchy**: Un focus principale, tutto else secondario
5. **Reasoning Visible**: L'utente vede "perché" l'AI ha fatto quella scelta

---

## Implementation Plan

### Phase 1: Simplify MessageBubble (Day 1)

**Goal**: Messaggi puliti come Claude/Cline

**Changes**:

```tsx
// PRIMA: Complex outcome boxes
<div className="sentinel-outcome-card">
  <div><h5>What I changed</h5><ul>...</ul></div>
  <div><h5>What needs your approval</h5><ul>...</ul></div>
  <div><h5>What happens next</h5><ul>...</ul></div>
</div>

// DOPO: Clean markdown content + optional reasoning
<div className="message-content">
  <MarkdownContent>{message.content}</MarkdownContent>
  {message.reasoning && <ReasoningTrace trace={message.reasoning} />}
</div>
```

**Actions**:
- [ ] Rimuovere `simpleMode`, `showInternals`, `askWhy` props
- [ ] Rimuovere `extractStructuredOutcome()` parsing
- [ ] Rimuovere `sentinel-outcome-card` e `sentinel-orchestration-card`
- [ ] Aggiungere `ReasoningTrace` component quando disponibile
- [ ] Semplificare a: Avatar + Content + Actions

### Phase 2: Chat-First Layout (Day 1-2)

**Goal**: Layout focus sulla conversazione

**Changes**:

```tsx
// PRIMA: Sidebar + Header complessi
<Header>...controls...</Header>
<Body>
  <Sidebar>...icons...</Sidebar>
  <Main>...content...</Main>
</Body>

// DOPO: Chat-first con tabs minimi
<ChatView>
  <MessagesList />
  <ChatInput />
</ChatView>
<BottomTabs>Chat | Goals | Network</BottomTabs>
```

**Actions**:
- [ ] Rimuovere sidebar icon-only
- [ ] Spostare feature switching in bottom tabs
- [ ] Header minimale: solo brand + status dot
- [ ] Rimuovere status bar bottom
- [ ] Chat input sempre visibile (sticky bottom)

### Phase 3: Smart QuickPrompts (Day 2)

**Goal**: Azioni contestuali reali

**Changes**:

```tsx
// PRIMA: Mock prompts generici
["Client Portal", "Booking App", "Spreadsheet to App"]

// DOPO: Contestualizzati allo stato
if (!hasGoals) → ["/init <describe your project>", "Help me plan..."]
if (hasPendingApprovals) → ["Approve all", "Review changes..."]
if (goalsCompleted) → ["Run tests", "Deploy..."]
```

**Actions**:
- [ ] Introdurre `SmartPrompts` component
- [ ] Integrare con `useProjectState()` hook
- [ ] Prompt dinamici basati su: goals, approvals, tests, alignment
- [ ] Rimuovere starter prompts hardcoded

### Phase 4: Reasoning Integration (Day 2-3)

**Goal**: Mostrare il "perché" delle decisioni

**Changes**:

```tsx
// Nuovo componente integrato
<MessageWithReasoning message={message}>
  <MessageContent />
  <ReasoningToggle>
    <ReasoningTrace 
      steps={message.reasoningSteps}
      confidence={message.confidence}
    />
  </ReasoningToggle>
  <FileApprovals />
</MessageWithReasoning>
```

**Actions**:
- [ ] Aggiungere `reasoningSteps` al tipo `ChatMessage`
- [ ] Integrare `ReasoningTrace` component
- [ ] Toggle "Show reasoning" per ogni messaggio
- [ ] Backend: popolare reasoning dal Gemini CLI

### Phase 5: File Approvals Redesign (Day 3)

**Goal**: Approvals integrati nel flow conversazionale

**Changes**:

```tsx
// PRIMA: Cards separate
<FileApproval operation={op} />

// DOPO: Inline nel messaggio
<Message>
  <Content />
  <FileChanges diff={operations}>
    <DiffViewer />
    <ApproveButton />
  </FileChanges>
</Message>
```

**Actions**:
- [ ] Creare `FileChanges` component inline
- [ ] Mini diff viewer per ogni file
- [ ] Approve/Reject inline, non cards separate
- [ ] Batch approval quando appropriato

---

## File Changes Summary

### Files to Modify

| File | Changes |
|------|---------|
| `App.tsx` | Simplify layout, remove sidebar, bottom tabs |
| `MessageBubble.tsx` | Remove outcome boxes, simplify props |
| `QuickPrompts.tsx` | Contextual prompts based on state |
| `ChatInput.tsx` | Always visible, sticky bottom |
| `styles.css` | New clean styles for chat-first |

### Files to Create

| File | Purpose |
|------|---------|
| `components/Chat/MessageWithReasoning.tsx` | Message + reasoning toggle |
| `components/Chat/SmartPrompts.tsx` | Contextual action prompts |
| `components/Chat/FileChanges.tsx` | Inline file approvals |
| `hooks/useProjectState.ts` | Derived state for UI |

### Files to Remove

| File | Reason |
|------|--------|
| `components/Chat/ToolCallCard.tsx` | Simplify into message |
| Components referenced but unused | Clean up |

---

## CSS Architecture

### New CSS Variables

```css
:root {
  /* Chat-first colors */
  --sentinel-chat-bg: var(--vscode-editor-background);
  --sentinel-message-user: color-mix(in oklab, var(--sentinel-accent) 8%, transparent);
  --sentinel-message-ai: transparent;
  --sentinel-border-subtle: color-mix(in oklab, var(--sentinel-border) 50%, transparent);
  
  /* Typography */
  --sentinel-text-sm: 13px;
  --sentinel-text-xs: 11px;
  --sentinel-radius: 12px;
}
```

### Layout Classes

```css
/* Chat-first container */
.sentinel-chat-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--sentinel-chat-bg);
}

/* Messages take all space */
.sentinel-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

/* Input sticky bottom */
.sentinel-input-area {
  position: sticky;
  bottom: 0;
  background: inherit;
  border-top: 1px solid var(--sentinel-border-subtle);
  padding: 12px 16px;
}

/* Bottom tabs minimal */
.sentinel-bottom-tabs {
  display: flex;
  gap: 4px;
  padding: 8px 16px;
  border-top: 1px solid var(--sentinel-border-subtle);
}
```

---

## Testing Checklist

- [ ] Chat scrolls smoothly
- [ ] Input always visible
- [ ] Reasoning toggle works
- [ ] File approvals inline
- [ ] Contextual prompts update
- [ ] No layout shift on message
- [ ] Responsive in sidebar mode

---

## Success Metrics

1. **Message rendering**: < 50ms per message
2. **Input responsiveness**: Instant typing feel
3. **Visual clarity**: No "mock" looking elements
4. **Context awareness**: Prompts match project state
5. **Reasoning visibility**: Every AI action explainable

---

## Timeline

| Day | Focus |
|-----|-------|
| 1 | Phase 1 + 2: MessageBubble + Layout |
| 2 | Phase 3 + 4: SmartPrompts + Reasoning |
| 3 | Phase 5: File Approvals + Polish |

---

## Next Step

Implementare **Phase 1** iniziando da `MessageBubble.tsx`:
1. Rimuovere `simpleMode`, `showInternals`, `askWhy`
2. Rimuovere outcome boxes artificiali
3. Semplificare a: Avatar + Markdown + Actions