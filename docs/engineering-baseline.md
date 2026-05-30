# FerrisLedger Engineering Baseline

This repository follows the initiative-wide standards below.

## Mandatory outcomes

- product-grade `README.md` with product and engineering sections
- `openapi.yaml` once the HTTP surface exists
- `docs/adr/`, `docs/architecture/`, `docs/events/`, `docs/benchmarks/`, `docs/api/`, `docs/diagrams/`, `docs/runbooks/`, and `docs/security/`
- atomic Conventional Commit history
- observability with structured logs, metrics, traces, request IDs, and readiness endpoints
- documented benchmark baselines

## FerrisLedger-specific emphasis

- Cargo workspace boundaries that separate domain, events, store, index, rules, API, worker, CLI, macros, FFI, telemetry, and test support
- typed financial domain modeling with traits, generics, and type-state patterns
- append-only event storage with checksums, replay, and corruption handling
- explicit isolation of unsafe and FFI behind documented safe abstractions
- property-based testing, Criterion benchmarks, and learning-map documents that connect Rust concepts to the implementation
- feature flags that make optional runtime surfaces explicit instead of leaking complexity into the default build

## Phase 0 boundary

This repository intentionally stops before scaffolding the Rust workspace, implementing storage code, or writing unsafe and FFI modules. The goal of this phase is only to lock scope and standards.
