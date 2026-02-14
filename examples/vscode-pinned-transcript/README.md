# VSCode Extension - Pinned Transcript Example

This example demonstrates the Pinned Transcript feature in Sentinel's VSCode extension.

## Overview

The Pinned Transcript provides a **lightweight, scrollable conversation history** that stays synchronized while you work. It's designed for:

- Long-running AI coding sessions (1000+ turns)
- Context-heavy conversations
- Multi-file refactoring tasks
- Complex architectural decisions

## Features

### 1. Lightweight Transcript (PLT)

Instead of loading full conversation history, Sentinel uses:

```
Pinned Lightweight Transcript (PLT)
├── Turn 1-1000     : Compressed summary
├── Turn 1001-2000   : Compressed summary
├── Turn 2001-3000   : Compressed summary
└── Turn 3001+        : Full recent turns
```

This keeps memory usage at **1.5MB per 10k turns**.

### 2. Smart Anchoring

Important decisions are "pinned" for quick reference:

```typescript
interface AnchorRef {
  id: string;
  type: 'decision' | 'requirement' | 'architecture' | 'bug-fix';
  turn: number;
  excerpt: string;
  timestamp: Date;
}
```

### 3. Frame Navigation

Navigate through conversation by "frames" (groups of 100 turns):

```
Frame 1 (Turns 1-100)
Frame 2 (Turns 101-200)
Frame 3 (Turns 201-300)
...
```

## Prerequisites

- VSCode with Sentinel extension installed
- Sentinel CLI running with MCP server
- Active conversation history

## Usage

### 1. Open Pinned Transcript

1. Open VSCode
2. Click the Sentinel icon in activity bar
3. Select "Pinned Transcript" from sidebar
4. View conversation history

### 2. Navigate by Anchors

Click any anchor to jump to that point in conversation:

```
┌─────────────────────────────────────┐
│ 🔒 Requirement: Implement Auth   │ [jump]
├─────────────────────────────────────┤
│ 🏗️ Architecture: MVC Pattern   │ [jump]
├─────────────────────────────────────┤
│ 🐛 Bug Fix: Race Condition   │ [jump]
└─────────────────────────────────────┘
```

### 3. Search Transcript

Use the search box to find:
- Keywords across all turns
- Code snippets
- Decision rationales
- Error messages

### 4. Export Transcript

Export options:
- **Full**: All turns with metadata
- **Anchors Only**: Just pinned decisions
- **Frame Range**: Specific range of turns

## Example: Pinned Decision

```
┌─────────────────────────────────────────────────────────────┐
│ 🔒 ARCHITECTURE DECISION                        Turn 847 │
├─────────────────────────────────────────────────────────────┤
│                                                      │
│ Using Layer 4 (Memory Manifold) with hierarchical     │
│ memory is critical for this project because:            │
│                                                      │
│ 1. Context window limits (~100k tokens)             │
│ 2. Long-running nature (weeks of development)        │
│ 3. Multiple team members need shared context          │
│                                                      │
│ Selected approach:                                     │
│ - Qdrant for episodic memory (vector search)          │
│ - Neo4j for semantic memory (knowledge graph)          │
│ - SQLite for structured state                           │
│                                                      │
│ Alternatives considered:                                 │
│ - Single-level memory: Rejected (poor scalability)     │
│ - File-based logs: Rejected (no semantic search)       │
│                                                      │
└─────────────────────────────────────────────────────────────┘

                    [View Full Context] [Copy]
```

## Performance Impact

### Memory Usage

| Turns | Full History | PLT | Savings |
|--------|---------------|-------|----------|
| 1,000  | ~50MB         | ~1.5MB | 97% |
| 10,000 | ~500MB        | ~15MB  | 97% |
| 100,000| ~5GB          | ~150MB | 97% |

### Render Performance

- Frame load: <50ms per 100 turns
- Anchor rendering: <10ms each
- Search across 10k turns: <200ms

## Integration

### TypeScript Hook Usage

```typescript
import { usePinnedTranscript } from './hooks/usePinnedTranscript';

function MyComponent() {
  const { transcript, loading, error, refresh } = usePinnedTranscript();

  if (loading) return <div>Loading transcript...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <div>
      {transcript?.anchors.map(anchor => (
        <AnchorCard key={anchor.id} anchor={anchor} />
      ))}
    </div>
  );
}
```

### MCP Message Protocol

```typescript
// Request transcript
{
  "jsonrpc": "2.0",
  "method": "transcript/get",
  "params": {
    "frameStart": 0,
    "frameEnd": 10,
    "includeAnchors": true
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "frames": [...],
    "anchors": [...],
    "metadata": {
      "totalTurns": 1000,
      "compressedFrames": 9
    }
  },
  "id": 1
}
```

## Best Practices

1. **Pin Critical Decisions**: Always pin architecture decisions
2. **Use Descriptive Types**: Choose meaningful anchor types
3. **Regular Compaction**: Let PLT compress older frames
4. **Search First**: Before asking, search pinned transcript

## Troubleshooting

### Transcript not loading

1. Verify MCP connection
2. Check CLI is running
3. Refresh using the button

### Missing recent turns

1. PLT may need compaction
2. Click "Force Compaction"
3. Wait for completion

### Search not working

1. Check indexing is complete
2. Verify query syntax
3. Use broader terms

## Resources

- [Memory Manifold Docs](../../docs/PHASE2_WORLD_MODEL.md)
- [PLT Specification](../../docs/ATOMIC_TRUTH_VISION.md)
- [VSCode Extension Guide](../../integrations/vscode/README.md)
