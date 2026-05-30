# Architecture Overview

FerrisLedger is a modular Rust monolith. The API and CLI are adapters, the
runtime crate owns use-case orchestration, the rules crate owns financial
decisions, and the store crate owns durability.

```mermaid
flowchart LR
  CLI[CLI] --> Runtime[Runtime Service]
  API[Axum API] --> Runtime
  API --> Limiter[API-key Rate Limiter]
  Runtime --> Rules[Rules]
  Runtime --> Store[Append-only Store]
  Runtime --> Index[Index Rebuild]
  Worker[Projection Worker] --> Runtime
  Store --> File[(JSONL Event Log)]
  API --> Telemetry[Telemetry]
```

The main architectural constraint is that domain/rules must not depend on HTTP,
CLI, rate-limit state, or file-system APIs.
