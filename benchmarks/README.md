# Sentinel Benchmark Suite

Performance validation suite for Sentinel core components (Wave 3.7).

## Overview

This benchmark harness tests the performance of critical Sentinel operations:
- **Alignment Computation**: Time to compute alignment scores
- **Memory Compaction**: PLT generation for large conversation histories
- **Quality Evaluation**: Multi-dimensional quality assessment

## Usage

### Run All Benchmarks

```bash
./benchmarks/run-benchmark.sh
```

### Run Specific Scenario

```bash
SENTINEL_CLI=./target/release/sentinel ITERATIONS=10 ./benchmarks/run-benchmark.sh
```

### Compare Results

```bash
./benchmarks/compare-results.sh
```

## Scenarios

| Scenario | Description | Threshold |
|----------|-------------|-----------|
| `small-project` | 10 goals, simple deps | 500ms |
| `medium-project` | 50 goals, moderate deps | 1000ms |
| `large-project` | 100+ goals, complex deps | 2000ms |
| `compaction-benchmark` | 10k turns PLT generation | 1500ms |
| `quality-evaluation` | 5-dimensional quality check | 800ms |

## Results

Results are stored in `benchmarks/results/` as JSON:

```json
{
  "scenario": "small-project",
  "iterations": 5,
  "total_time_ms": 2100,
  "average_time_ms": 420,
  "timestamp": "2026-01-27T10:30:00Z"
}
```

## Regression Detection

The comparison tool detects performance regressions:

```bash
./benchmarks/compare-results.sh
```

Regression threshold defaults to **10%** (configurable via `REGRESSION_THRESHOLD`).

## CI/CD Integration

Add to your CI pipeline:

```yaml
# .github/workflows/benchmarks.yml
- name: Run Benchmarks
  run: |
    cargo build --release
    ./benchmarks/run-benchmark.sh

- name: Check Regressions
  run: ./benchmarks/compare-results.sh
```
