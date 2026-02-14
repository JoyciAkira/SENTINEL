# Augment Context Engine MCP Integration (Commercial-Safe)

Last updated: 2026-02-07
Owner: SENTINEL Core + VSCode Integration
Status: Proposed (ready for implementation)

## 1. Objective

Integrate Augment Context Engine MCP into SENTINEL as an optional context provider to improve retrieval quality, while preserving:
- legal/commercial safety,
- deterministic governance,
- full fallback to open-source retrieval when unavailable.

## 2. What Augment MCP Adds

Augment Context Engine MCP provides a `codebase-retrieval` capability via MCP with:
- local mode (Auggie CLI, local indexing + auto-workspace),
- remote mode (hosted endpoint for multi-repo context),
- connectors for code + docs/runbooks.

Practical impact: better context recall and fewer irrelevant tool calls in long coding sessions.

## 3. Commercial/Licensing Constraints (Critical)

Based on Augment legal pages as of 2026-02-07:

- Access is licensed for "internal business purposes".
- Restrictions include:
  - no reverse engineering,
  - no service-bureau redistribution,
  - no use to build/enhance competing services.
- Professional terms state customer code/output is not used for training.
- Community terms are materially different (include training-related language).

### Commercial-safe interpretation for SENTINEL

1. Allowed now (Green):
- Internal use by our own engineering organization.
- Customer BYO-Augment account, where each customer authenticates directly with Augment from their own environment.

2. Allowed with contract alignment (Yellow):
- Customer-facing SENTINEL features that invoke Augment, only if legal confirms plan + terms + any required addendum for that distribution model.

3. Not allowed by default (Red):
- Reselling Augment capability as our own hosted backend proxy without explicit contractual rights.
- Sharing one Augment tenant across third-party customers.

Note: this document is implementation guidance, not legal advice. Final approval must come from legal counsel.

## 4. Product Policy in SENTINEL

### 4.1 Provider modes

- `augment_disabled` (default)
- `augment_internal_only`
- `augment_byo_customer`

### 4.2 Hard gates

SENTINEL runtime must refuse Augment retrieval when:
- deployment mode is multi-tenant hosted and `augment_byo_customer` is not active,
- credentials are platform-owned for third-party tenants,
- workspace ownership cannot be proven.

### 4.3 Fallback policy

On any gate failure, timeout, or policy violation:
- automatically fallback to OSS retrieval providers,
- emit explainability event `context_provider_fallback`.

## 5. Target Architecture

```text
User Prompt
  -> SENTINEL Planner
    -> Context Orchestrator
      -> Provider Router
         -> Augment MCP Provider (optional, policy-gated)
         -> Local Vector Provider (OSS)
         -> Code Graph Provider (OSS)
      -> Context Fusion + Ranking
    -> Tool Execution / Codegen
```

### 5.1 New abstraction

Introduce `ContextProvider` trait/interface:
- `name()`
- `health()`
- `retrieve(query, workspace, constraints)`
- `compliance_mode()`

Implementations:
- `AugmentMcpProvider`
- `LocalEmbeddingProvider`
- `CodeGraphProvider`

### 5.2 Contract-aware routing

Provider router evaluates:
- policy mode,
- tenant type (internal vs external),
- credential origin (user-provided vs platform-managed),
- data residency constraints (if configured).

Only then selects Augment.

## 6. Config Changes

## 6.1 `sentinel.json` (new section)

```json
{
  "context": {
    "providers": {
      "augment": {
        "enabled": false,
        "mode": "internal_only",
        "transport": "stdio",
        "command": "auggie",
        "args": ["--mcp", "--mcp-auto-workspace"],
        "allow_in_multitenant": false,
        "require_customer_credentials": true,
        "timeout_ms": 3500
      },
      "oss_vector": {
        "enabled": true
      },
      "code_graph": {
        "enabled": true
      }
    }
  }
}
```

## 6.2 Environment variables

- `SENTINEL_CONTEXT_PROVIDER_PRIORITY=augment,oss_vector,code_graph`
- `SENTINEL_AUGMENT_MODE=disabled|internal_only|byo_customer`
- `SENTINEL_AUGMENT_ENFORCE_BYO=true|false`

## 7. Implementation Plan (3 phases)

## Phase A: Safe foundation
- Add provider interface + router.
- Add policy gates and structured denial reasons.
- Add telemetry events for provider choice/fallback.

Exit criteria:
- zero behavior change when augment disabled.

## Phase B: Augment provider integration
- Implement MCP client adapter for Augment (`codebase-retrieval`).
- Add workspace binding and request shaping.
- Add resilience (timeouts, retries, circuit breaker).

Exit criteria:
- Augment retrieval works in local/internal mode.

## Phase C: Commercial-safe rollout
- Add BYO credential flow in VSCode UI.
- Add compliance banner + mode indicator in webview.
- Enforce multi-tenant deny by default.

Exit criteria:
- cannot activate unsafe mode without explicit policy override.

## 8. UX/Explainability Requirements

For every turn, store and show:
- selected context providers,
- whether Augment was used or denied,
- fallback reason,
- retrieval confidence and token impact.

UI labels:
- `Context: Augment (BYO)`
- `Context: OSS fallback`
- `Policy: Commercial-safe enforced`

## 9. Test Matrix

Required tests:

1. Unit:
- policy gate evaluation,
- provider selection precedence,
- fallback on Augment timeout/error.

2. Integration:
- successful Augment retrieval with BYO credentials,
- denied retrieval in multi-tenant mode,
- deterministic fallback to OSS provider.

3. E2E (VSCode):
- user toggles provider mode,
- chat turn explainability reflects actual provider path,
- no hidden provider calls when denied.

## 10. Go/No-Go Checklist

Go only if all are true:
- legal review accepted for intended commercial distribution model,
- BYO credential path implemented,
- multi-tenant platform-owned Augment access disabled by default,
- audit logs include provider + policy decisions per request.

## 11. Official Sources Reviewed

- Augment Context Engine MCP overview:
  - https://docs.augmentcode.com/context-services/mcp/overview
- Augment Codex quickstart:
  - https://docs.augmentcode.com/context-services/mcp/quickstart-codex
- Augment GA announcement (MCP):
  - https://www.augmentcode.com/changelog/context-engine-mcp-in-ga
- Augment Professional Terms:
  - https://www.augmentcode.com/legal/professional-terms-of-service
- Augment Community Terms:
  - https://www.augmentcode.com/legal/community-terms-of-service
- Augment Pricing (commercial/training FAQ):
  - https://www.augmentcode.com/pricing

