# Phase 2: Project World Model

This document describes the deterministic governance layer introduced for Phase 2.

## What is enforced

Sentinel now treats the workspace contract as a first-class world model:

- Dependencies (`required` + `allowed`)
- Frameworks (`required` + `allowed`)
- Endpoints (`allowed`)
- Ports (`allowed`)

Any deterministic drift creates a governance proposal and blocks execution until explicit user approval.

## Deterministic drift behavior

Blocking conditions:

- New dependency/framework/endpoint/port appears outside the contract.
- A required dependency/framework is no longer observed in the workspace.

Proposal payload now includes:

- Additions and removals (not only additions)
- `deterministic_confidence`
- `evidence` lines with machine-observed facts

## MCP tools

- `governance_status`
- `get_world_model`
- `governance_seed`
- `governance_approve`
- `governance_reject`
- `get_quality_status`
- `list_quality_reports`
- `run_quality_harness`

`get_world_model` returns:

- `where_we_are` (observed workspace contract)
- `where_we_must_go` (current governance contract)
- `deterministic_drift`
- `required_missing_now`
- `how_enforced` metadata (pending proposal, history size, manifold hash)

## Persistence

On each manifold save, Sentinel now also writes:

- `.sentinel/world_model.json`

This keeps frontend/runtime explainability synchronized with the same contract source of truth.

## Quality harness

Run:

```bash
./scripts/world_class_harness.sh
```

Outputs:

- `.sentinel/quality/harness-<timestamp>.json`
- `.sentinel/quality/harness-<timestamp>.log`

The harness runs package tests and runtime checks and emits machine-readable KPI summaries.

At runtime (MCP), Sentinel can now:

- trigger the harness (`run_quality_harness`)
- read latest status (`get_quality_status`)
- inspect history (`list_quality_reports`)
