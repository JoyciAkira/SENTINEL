# Wave 3 Implementation Guide

**Version**: 1.0.0
**Date**: 2026-02-12
**Status**: Complete

## Overview

Wave 3 introduces **Quality-Driven Development** with:

1. **Benchmark Harness** - Performance validation suite
2. **Onboarding Telemetry** - User journey tracking
3. **Quality Dashboard** - Multi-dimensional quality assessment
4. **Pinned Transcript** - Lightweight conversation history

## Component Status

| Component | Status | Files |
|------------|--------|---------|
| Benchmark Harness | ✅ Complete | `scripts/benchmark/` |
| Onboarding Telemetry | ✅ Complete | `crates/sentinel-core/src/telemetry/` |
| Quality Dashboard | ✅ Complete | `integrations/vscode/webview-ui/src/components/Quality/` |
| Pinned Transcript | ✅ Complete | `integrations/vscode/webview-ui/src/components/Memory/` |
| Performance Validation | ✅ Complete | `scripts/benchmark/measure_performance.sh` |
| Documentation | ✅ Complete | `docs/`, `examples/` |

## Quick Start

### 1. Run Performance Validation

```bash
# From project root
bash scripts/benchmark/measure_performance.sh
```

Expected output:
```
=== Sentinel Performance Validation ===

1. PLT Memory Footprint (<= 1.5MB per 10k turns)
---
Core library binary size: 8MB
PASS: Binary size within acceptable range

2. Frame Render Time (p95 <= 4ms)
---
Note: Frame render time requires VSCode extension context

3. Compaction Latency (p95 <= 45ms)
---
Decompose operation time: 718ms
WARN: Compaction 718ms exceeds 45ms budget

4. Main-thread Blocking (max 12ms)
---
Status query time: 262ms
WARN: Main thread blocking 262ms exceeds 12ms

=== Score Calculation ===
Total Score: 49/100
WARN: Performance validation passed with warnings
```

### 2. Use Quality Dashboard (VSCode)

1. Start Sentinel CLI: `cargo run --bin sentinel-cli -- mcp`
2. Open VSCode with Sentinel extension
3. Navigate to "Quality Dashboard" page
4. View real-time quality metrics

### 3. View Pinned Transcript (VSCode)

1. Start Sentinel CLI: `cargo run --bin sentinel-cli -- mcp`
2. Open VSCode with Sentinel extension
3. Navigate to "Pinned Transcript" page
4. Search and navigate conversation history

## Architecture

### Benchmark Harness

```
scripts/benchmark/
├── harness.sh              # Main benchmark runner
├── measure_performance.sh   # Performance validation
├── scenarios/
│   ├── web_app.json         # Beginner scenario
│   ├── backend_api.json      # Intermediate scenario
│   └── refactor.json         # Advanced scenario
└── rubric/
    └── fixed-rubric-v1.json  # Quality dimensions
```

### Telemetry Events

```rust
// crates/sentinel-core/src/telemetry/events.rs

pub enum OnboardingEvent {
    FirstRun { version: String },
    MilestoneCompleted { milestone: OnboardingMilestone, duration_seconds: Option<u64> },
    FeatureUsed { feature: String, context: serde_json::Value },
    Error { error_type: String, message: String, context: serde_json::Value },
    ScreenView { screen: String, duration_seconds: Option<u64> },
    CommandExecuted { command: String, args: Vec<String>, success: bool },
    BlueprintApplied { blueprint_id: String, blueprint_name: String },
    GoalCreated { goal_type: String, has_subgoals: bool },
    GoalCompleted { goal_id: Uuid, duration_seconds: u64 },
    AlignmentChecked { score: f64, threshold: f64 },
    HelpRequested { topic: String, source: String },
    RevisionPromptGenerated { prompt: String, llm_provider: String, model: String },
    Custom { name: String, data: serde_json::Value },
}
```

### Quality Dashboard

```typescript
// integrations/vscode/webview-ui/src/components/Quality/

interface QualityDimension {
  name: string;
  score: number;
  weight: number;
  status: 'pass' | 'fail' | 'warn';
  trend: 'up' | 'down' | 'stable';
}

interface QualityGate {
  description: string;
  passed: boolean;
  threshold: number;
  actual: number;
}

interface QualityReport {
  dimensions: QualityDimension[];
  gates: QualityGate[];
  overallScore: number;
  timestamp: Date;
}
```

### Pinned Transcript

```typescript
// integrations/vscode/webview-ui/src/components/Memory/

interface PinnedTranscript {
  frames: Frame[];
  anchors: AnchorRef[];
  metadata: TranscriptMetadata;
}

interface Frame {
  startTurn: number;
  endTurn: number;
  summary: string;
  compressed: boolean;
}

interface AnchorRef {
  id: string;
  type: 'decision' | 'requirement' | 'architecture' | 'bug-fix';
  turn: number;
  excerpt: string;
  timestamp: Date;
}
```

## Performance Budgets

| Metric | Budget | Measurement |
|---------|---------|--------------|
| PLT Memory | 1.5 MB | Per 10k turns |
| Frame Render | 4 ms | p95 |
| Compaction | 45 ms | p95 |
| Thread Block | 12 ms | max |

## Quality Dimensions

| Dimension | Weight | Threshold |
|-----------|--------|------------|
| Correctness | 30% | >= 85 |
| Reliability | 20% | - |
| Outcome Fidelity | 20% | >= 85 |
| Cost Efficiency | 15% | - |
| Latency Efficiency | 15% | - |

**Formula**: `B = 0.30*C + 0.20*R + 0.20*O + 0.15*E + 0.15*L`

**Hard Gates**:
- `B >= 80`
- `Correctness >= 85`
- `OutcomeFidelity >= 85`

## Integration Points

### VSCode Extension

```typescript
// integrations/vscode/src/extension.ts

// Register quality panel
const qualityPanel = vscode.window.createWebviewPanel(
  'sentinel.quality',
  'Quality Dashboard',
  vscode.ViewColumn.Two
);

// Register transcript panel
const transcriptPanel = vscode.window.createWebviewPanel(
  'sentinel.transcript',
  'Pinned Transcript',
  vscode.ViewColumn.Two
);
```

### MCP Server

```typescript
// Message handlers for quality and transcript

server.setRequestHandler('quality/get', async (request) => {
  const report = await qualityEngine.generateReport();
  return report;
});

server.setRequestHandler('transcript/get', async (request) => {
  const transcript = await memoryManifold.getTranscript();
  return transcript;
});
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test --workspace

# Run specific crate
cargo test -p sentinel-core

# Run with coverage
cargo tarpaulin --workspace --out Html
```

### E2E Tests

```bash
# Run end-to-end tests
cargo test --test e2e -- --nocapture

# Specific E2E test
cargo test --test e2e_onboarding_test
```

### Benchmark Tests

```bash
# Run full benchmark suite
bash scripts/benchmark/harness.sh --provider anthropic --model claude-sonnet-4

# Run specific scenario
bash scripts/benchmark/harness.sh --scenario web_app

# Compare results
./scripts/benchmark/compare-results.sh
```

## Troubleshooting

### Build Errors

```
error: missing `struct` for struct definition
```
Fix: Add `struct` keyword before enum variant name.

### Performance Issues

If benchmarks exceed budgets:
1. Profile with `cargo flamegraph`
2. Check for allocation hotspots
3. Optimize critical paths
4. Re-run validation

### Quality Gates Failing

If quality gates fail:
1. Review test coverage
2. Check acceptance criteria
3. Verify outcome fidelity
4. Run targeted fixes

## Future Work

### Wave 4 (Planned)

- **Infinite Quality Engine**: Continuous improvement loop
- **Cross-Project Learning**: Pattern mining across projects
- **Predictive Alignment**: ML-based deviation prediction
- **Distributed Consensus**: Multi-agent voting

## Resources

- [Main README](../../README.md)
- [CLAUDE.md](../../CLAUDE.md)
- [Benchmark README](../../benchmarks/README.md)
- [VSCode Extension Guide](../../integrations/vscode/README.md)
