# Senior Readiness Spec

## Product Bar

FerrisLedger must read as a believable internal financial runtime, not a Rust
syntax exercise. The README, product docs, and case study must name the user,
problem, core workflow, business value, non-goals, and roadmap.

## Domain Bar

The domain must use explicit financial language: tenant, account, stream,
event envelope, money, idempotency key, Pix reservation, settlement, and ledger
entry. Critical invariants must be documented and backed by tests.

## Architecture Bar

The architecture must justify a modular monolith Cargo workspace, explain why
JSONL is the current store, document module boundaries, include sequence flows,
and name deferred complexity without pretending it is implemented.

## API Bar

The API must have a valid OpenAPI 3.1 contract, versioned `/v1` endpoints,
API-key authentication, rate-limit behavior, standardized errors, examples,
idempotency rules, and failure responses.

## Data and Consistency Bar

Docs must explain stream-level consistency, optimistic version checks,
checksum verification, transaction boundaries, isolation assumptions, rollback
limits, and the PostgreSQL migration path.

## Security Bar

Docs and tests must cover API key auth, in-process rate limiting, tenant
isolation/BOLA, idempotency abuse, input validation, secret management, audit
logging, unsafe/FFI isolation, and residual risks.

## Observability Bar

The runtime must expose health, readiness, Prometheus metrics, structured JSON
logs, generated or supplied request ID, correlation ID, audit command logs, a
Grafana dashboard, and runbooks. Full OpenTelemetry trace export may be marked
planned if not implemented.

## Performance Bar

Benchmarks must include Criterion replay and k6 smoke/load/stress/spike
assets. Measured results must include latency percentiles, throughput, error
rate, dataset, bottleneck, and next optimization.

## Scalability Bar

Docs must identify hot paths, read/write-heavy paths, event-log growth, hot
partitions, queue/projection buildup, async candidates, and operations that
must not become eventual.

## Operational Cost Bar

Docs must name infrastructure components, debugging cost, deploy cost,
backup/retention cost, monitoring burden, vendor-lock risk, and simpler
alternatives rejected.

## Maintainability Bar

Module boundaries, extension points, error codes, test strategy, seed/test
fixtures, scripts, ADRs, and expected ownership must be documented.

## Readability Bar

Code, tests, and docs must use domain nouns and avoid generic claims such as
"production-ready" without evidence.

## Test and CI Bar

The repository must include format, lint, unit, property, API, auth, rate
limit, tenant isolation, corruption, worker, FFI, benchmark, security audit,
OpenAPI, Docker build, and coverage gates. If a gate cannot be run locally,
the reason must be recorded.

## Evidence Matrix

| Criterion | Evidence | Status | Notes |
| --- | --- | --- | --- |
| Product problem, users, workflow, business value are explicit | `README.md`, `docs/product/problem.md`, `docs/product/personas.md`, `docs/engineering-case-study.md` | Done | Internal platform/SRE workflow is named. |
| Non-goals and roadmap are explicit | `docs/product/non-goals.md`, `docs/product/roadmap.md`, `README.md` | Done | Defers KYC, real Pix provider, multi-region storage, and OIDC. |
| Domain language and aggregate boundary are documented | `docs/domain/*.md`, `crates/ferrisledger-domain/src/lib.rs` | Done | Account stream is the consistency boundary. |
| Critical money rules are tested | `crates/ferrisledger-domain/src/lib.rs`, `crates/ferrisledger-rules/src/lib.rs` | Done | Unit/property tests cover deposits, reservations, settlements, and replay. |
| Architecture boundaries are documented | `docs/architecture/module-boundaries.md`, `Cargo.toml`, `crates/*` | Done | Crates map to domain/events/rules/store/runtime/adapters. |
| Rejected alternatives are recorded | `docs/adr/0002-use-jsonl-event-store-for-mvp.md`, `docs/adr/0003-keep-runtime-as-modular-monolith.md`, `docs/adr/0004-isolate-unsafe-and-ffi.md` | Done | JSONL, modular monolith, and FFI boundaries are justified. |
| API contract is valid | `openapi.yaml`, command `npx @redocly/cli lint openapi.yaml` | Done | Versioned endpoints, auth, errors, 429, examples. |
| Data consistency and corruption behavior are real | `crates/ferrisledger-store/src/lib.rs`, `docs/architecture/data-consistency.md`, `docs/runbooks/event-log-corruption.md` | Done | Expected stream version, isolation assumptions, rollback, migration path, and CRC32 verification exist. |
| API auth is tested | `crates/ferrisledger-api/src/lib.rs` | Done | Missing API key returns `401`. |
| Tenant isolation/BOLA is tested | `crates/ferrisledger-api/src/lib.rs` | Done | Cross-tenant snapshot returns no state. |
| Rate limiting is implemented and tested | `crates/ferrisledger-api/src/lib.rs`, `crates/ferrisledger-telemetry/src/lib.rs` | Done | Per-API-key rolling window returns `429` and increments metric. |
| Request/correlation IDs are propagated through HTTP | `crates/ferrisledger-api/src/lib.rs`, `openapi.yaml` | Done | Success and error responses include context headers and error body context. |
| Audit logging includes command context | `crates/ferrisledger-api/src/lib.rs` | Done | Structured logs include event/stream/correlation/request IDs. |
| Unsafe boundary is isolated and tested | `crates/ferrisledger-ffi/src/lib.rs`, `docs/unsafe/README.md` | Done | Runtime does not use FFI by default. |
| Observability surfaces exist | `crates/ferrisledger-telemetry/src/lib.rs`, `docs/architecture/observability.md`, `ops/grafana/ferrisledger-dashboard.json`, `docs/runbooks/*.md` | Done | Health, readiness, Prometheus metrics, JSON logs, dashboard, runbooks. |
| OpenTelemetry exporter exists | `docs/spec-driven/verification-report.md` | Partial | Structured tracing exists; OTLP export is deferred. |
| Benchmarks have scripts and measured baseline | `benchmarks/*.js`, `crates/ferrisledger-runtime/benches/replay.rs`, `benchmarks/results/2026-05-30-smoke.md` | Done | Smoke and Criterion results are recorded; load/stress/spike scripts are inspectable. |
| CPU and memory notes are measured under load | `benchmarks/results/2026-05-30-smoke.md` | Partial | Smoke records that CPU/memory were not separately sampled; next load/stress evidence pass must capture them. |
| Scalability limits are explicit | `docs/scalability.md`, `docs/operational-cost.md` | Done | Names JSONL single-writer and rate-limit multi-replica limits. |
| Alerts are documented | `ops/prometheus/ferrisledger-alerts.yml`, `docs/architecture/observability.md` | Done | Error-rate, rate-limit spike, and empty-store alerts exist. |
| CI covers required gates | `.github/workflows/ci.yml` | Done | Format, lint, tests, coverage, audit, OpenAPI, Docker build. |
| Docker build validates locally | `Dockerfile`, `docker-compose.yml`, `docs/spec-driven/verification-report.md` | Blocked | Docker daemon was unavailable locally; CI has the gate. |

## Out of Scope

- Real Pix provider integration.
- PostgreSQL event store and distributed locking.
- Redis-backed distributed rate limiting.
- JWT/OIDC authorization.
- OpenTelemetry OTLP exporter.
- Kubernetes manifests.
- Cryptographic event signing.
