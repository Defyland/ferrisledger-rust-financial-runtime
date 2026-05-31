# FerrisLedger

FerrisLedger is a Rust financial event runtime for append-only account events,
deterministic replay, operational verification, and API/CLI workflows. It is a
portfolio-grade backend and systems project: small enough to run locally, but
modeled like a real financial runtime where auditability, idempotency,
corruption detection, and tenant isolation matter.

## Evidence entrypoints

- Senior case study: [`docs/engineering-case-study.md`](docs/engineering-case-study.md)
- Spec-driven readiness: [`docs/spec-driven/senior-readiness-spec.md`](docs/spec-driven/senior-readiness-spec.md)
- Implementation plan: [`docs/spec-driven/implementation-plan.md`](docs/spec-driven/implementation-plan.md)
- Verification report: [`docs/spec-driven/verification-report.md`](docs/spec-driven/verification-report.md)
- OpenAPI contract: [`openapi.yaml`](openapi.yaml)

## 1. What is this product?

FerrisLedger stores financial account facts as immutable events. Operators and
internal platform clients can open accounts, deposit money, reserve outgoing Pix
transfers, execute settlement, create ledger evidence, replay an account stream,
and verify the event log.

## 2. Problem it solves

Financial platforms need a trusted history of what happened, not just mutable
rows with a current balance. FerrisLedger makes account movement auditable by
persisting typed event envelopes with checksums and rebuilding state from those
events.

## 3. Target users

- Platform engineers building ledger, wallet, Pix, or settlement systems.
- SREs and support engineers investigating event-log corruption or replay drift.
- Technical reviewers evaluating Rust domain modeling, storage, API, and ops
  trade-offs in one cohesive codebase.

## 4. Main features

- Axum HTTP API with API-key authentication.
- Clap CLI for local operations and replay.
- Cargo workspace with domain, events, rules, store, runtime, API, worker,
  telemetry, macros, FFI, and test-support crates.
- Append-only JSONL event store with CRC32 checksums, local OS file locks,
  duplicate idempotency-key rejection, and optimistic stream version checks.
- Deterministic replay into account snapshots and in-memory projections.
- Idempotency keys for repeatable financial commands, with `409` conflict on
  incompatible key reuse.
- In-process API-key rate limiting for local abuse protection.
- Prometheus metrics, structured JSON tracing, health and readiness endpoints.
- Property tests, API tests, corruption tests, auth tests, worker tests, and a
  Criterion replay benchmark.

## 5. Architecture overview

The runtime is a modular monolith. `ferrisledger-runtime` coordinates use cases,
`ferrisledger-rules` protects financial invariants, `ferrisledger-store`
persists append-only events, and API/CLI crates are adapters. This keeps the
domain independent from HTTP and file-system concerns while avoiding premature
microservices for an MVP. Detailed boundaries, data consistency, and
observability are documented in [`docs/architecture`](docs/architecture).

## 6. Tech stack

- Rust stable, edition 2024
- Axum, Tokio, Tower-compatible request tests
- Serde, thiserror, anyhow
- Clap CLI
- Tracing and Prometheus
- Criterion and k6 benchmark assets
- Docker and GitHub Actions CI

## 7. Domain model

Core concepts are tenant, account, money, event envelope, stream, idempotency
key, Pix reservation, settlement, and ledger entry. `AccountState` is rebuilt
from events and enforces available-balance invariants before outgoing Pix
transfers can be reserved or settled.

## 8. API documentation

The canonical contract is [`openapi.yaml`](openapi.yaml). Examples and error
payloads live in [`docs/api`](docs/api).

## 9. Async or event architecture

FerrisLedger is event-first but not event-sourced beyond the account aggregate.
The append-only store is the source of truth. `ferrisledger-worker` demonstrates
an async projection worker that periodically rebuilds materialized account
indexes from persisted events.

## 10. Database design

The MVP uses a single JSONL event log. Each line stores `{ checksum, envelope }`.
The checksum is computed from the canonical serialized envelope. Stream version
checks provide optimistic concurrency, append/read/verify use local OS file
locks for same-host coordination, duplicate idempotency keys are rejected, and
replay detects corrupt or invalid records before returning projections.
Transaction boundaries and migration assumptions are documented in
[`docs/architecture/data-consistency.md`](docs/architecture/data-consistency.md).

## 11. Testing strategy

Run:

```bash
cargo test --workspace --all-targets
```

Coverage includes domain unit tests, property tests for money invariants,
overflow rejection, store corruption and concurrent independent-handle tests,
API authentication and BOLA-style tenant-isolation tests, runtime idempotency
replay/conflict tests, worker projection tests, FFI safety-wrapper tests, and
Criterion replay benchmark compilation.

## 12. Performance benchmarks

Benchmark assets live in [`benchmarks`](benchmarks) and
[`docs/benchmarks`](docs/benchmarks). Criterion measures replay of a stream with
100 deposits. k6 scripts cover smoke, load, stress, and spike HTTP scenarios;
the latest load evidence includes latency, throughput, error rate, server CPU,
and server RSS.

## 13. Observability

The API exposes:

- `/healthz`
- `/readyz`
- `/metrics`

Metrics include HTTP request counts/latency, runtime command counts,
rate-limit rejections, and event store record gauges. JSON tracing records
command acceptance and rejection with event ID, stream ID, generated or
supplied request ID, and correlation ID. HTTP responses return `x-request-id`;
command responses also return `x-correlation-id`.
Dashboard and alert assets live under [`ops/`](ops).

## 14. Security considerations

All `/v1` endpoints require `x-api-key`. Tenant ID is part of the stream ID, so
account IDs are not globally sufficient to read another tenant's events.
Authenticated requests are rate-limited per API key in-process. Sensitive state
is kept out of logs. Secrets are provided by environment variables and
documented in [`docs/security`](docs/security).

## 15. Trade-offs and decisions

- JSONL was chosen over PostgreSQL for a focused storage/runtime MVP.
- Modular monolith was chosen over microservices to keep consistency and replay
  behavior easy to reason about.
- CRC32 is a corruption signal, not a cryptographic integrity guarantee.
- FFI is isolated in one crate and not used by default runtime paths.
- Spec-driven readiness evidence lives in
  [`docs/spec-driven`](docs/spec-driven), including a technical review of what
  was improved and what remains before production.

## 16. How to run locally

```bash
cargo run -p ferrisledger-cli -- serve \
  --bind 127.0.0.1:8080 \
  --store-path data/events.jsonl \
  --api-key dev-secret \
  --rate-limit-per-minute 120
```

Docker:

```bash
FERRISLEDGER_API_KEY=dev-secret docker compose up --build
```

## 17. How to run tests

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo bench -p ferrisledger-runtime --bench replay
k6 run benchmarks/k6-smoke.js
```

## 18. Failure scenarios

- Corrupt event-log line: readiness and replay fail with checksum/JSON errors.
- Duplicate command retry: idempotency returns the original event.
- Idempotency key reused with different command data: command is rejected with
  conflict.
- Money arithmetic overflow or underflow: command is rejected with a domain
  error.
- Duplicate account open: command is rejected with conflict.
- Pix transfer above available balance: command is rejected before append.
- Wrong tenant on read: returns no stream state for that tenant partition.

## 19. Roadmap

- Replace local JSONL with a PostgreSQL-backed event store adapter.
- Add outbox delivery for external consumers.
- Add snapshot compaction and segment rotation.
- Add OpenTelemetry trace export.
- Add JWT/OIDC and scoped API keys.
- Add fuzzing for event-log parser boundaries.
