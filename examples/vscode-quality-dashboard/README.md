# VSCode Extension - Quality Dashboard Example

This example demonstrates the Quality Dashboard features in Sentinel's VSCode extension.

## Features Demonstrated

1. **Multi-dimensional Quality Assessment**
   - Correctness (30% weight)
   - Reliability (20% weight)
   - Outcome Fidelity (20% weight)
   - Cost Efficiency (15% weight)
   - Latency Efficiency (15% weight)

2. **Quality Gates**
   - Automatic pass/fail determination
   - Threshold-based validation
   - Hard gate enforcement

3. **Performance Metrics**
   - PLT memory footprint
   - Frame render time
   - Compaction latency
   - Main-thread blocking

## Prerequisites

- VSCode with Sentinel extension installed
- Sentinel CLI running with MCP server
- Active project with goal manifold

## Usage

### 1. Open Quality Dashboard

1. Open VSCode
2. Click the Sentinel icon in activity bar
3. Select "Quality Dashboard" from sidebar
4. View real-time quality metrics

### 2. Understanding Quality Dimensions

Each dimension is scored 0-100:

```
Correctness    : Tests passing rate
Reliability     : Error-free operation
Outcome Fidelity: Matches acceptance criteria
Cost Efficiency : Token usage optimization
Latency        : Response time performance
```

### 3. Quality Gates

Projects must pass ALL hard gates:

```javascript
{
  "B >= 80": true,           // Overall B score
  "Correctness >= 85": true,   // Minimum correctness
  "OutcomeFidelity >= 85": true // Minimum outcome quality
}
```

### 4. Performance Budgets

Wave 3 performance budgets:

| Metric | Budget | Unit |
|---------|---------|-------|
| PLT Memory | 1.5 | MB per 10k turns |
| Frame Render | 4 | ms (p95) |
| Compaction | 45 | ms (p95) |
| Thread Block | 12 | ms (max) |

### 5. Refresh Quality Report

Click the refresh button to:
- Re-run all validation checks
- Update quality scores
- Re-evaluate gates

### 6. Export Quality Report

Use the export button to download:
- JSON format for automation
- PDF format for documentation
- CSV format for analysis

## Integration with CI/CD

Add quality checks to your pipeline:

```yaml
# .github/workflows/quality-check.yml
name: Quality Gate
on: [pull_request]
jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run Quality Check
        run: |
          sentinel-cli quality check \
            --rubric scripts/benchmark/rubric/fixed-rubric-v1.json \
            --output quality-report.json
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: quality-report
          path: quality-report.json
```

## Example Output

```
╔════════════════════════════════════════╗
║         Sentinel Quality Dashboard               ║
╠════════════════════════════════════════╣
║                                              ║
║  Overall Score: 87/100  ✓ PASS              ║
║                                              ║
║  ┌────────────────────────────────────┐       ║
║  │ Correctness     │ 92/100  │       ║
║  │ Reliability      │ 85/100  │       ║
║  │ Outcome Fidelity│ 88/100  │       ║
║  │ Cost Efficiency  │ 78/100  │       ║
║  │ Latency         │ 90/100  │       ║
║  └────────────────────────────────────┘       ║
║                                              ║
║  Quality Gates:                              ║
║  ✓ B >= 80                                 ║
║  ✓ Correctness >= 85                         ║
║  ✓ OutcomeFidelity >= 85                     ║
║                                              ║
║  [Refresh]  [Export]  [Details]            ║
╚══════════════════════════════════════════╝
```

## Troubleshooting

### Quality score not updating

1. Check Sentinel CLI is running
2. Verify MCP server connection
3. Click "Refresh" button

### Quality gates failing

1. Review failing dimensions
2. Check test coverage
3. Verify acceptance criteria
4. Run performance validation

### Performance budgets exceeded

1. Run benchmark suite
2. Check optimization opportunities
3. Review profiling data
4. Consider architectural changes

## Resources

- [Sentinel Documentation](../../README.md)
- [Benchmark Suite](../../benchmarks/README.md)
- [Quality Rubric](../../scripts/benchmark/rubric/fixed-rubric-v1.json)
