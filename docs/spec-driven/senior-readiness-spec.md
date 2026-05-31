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
| Critical money rules are tested | `crates/ferrisledger-domain/src/lib.rs`, `crates/ferrisledger-rules/src/lib.rs` | Done | Unit/property tests cover deposits, reservations, settlements, replay, and overflow/underflow rejection. |
| Architecture boundaries are documented | `docs/architecture/module-boundaries.md`, `Cargo.toml`, `crates/*` | Done | Crates map to domain/events/rules/store/runtime/adapters. |
| Rejected alternatives are recorded | `docs/adr/0002-use-jsonl-event-store-for-mvp.md`, `docs/adr/0003-keep-runtime-as-modular-monolith.md`, `docs/adr/0004-isolate-unsafe-and-ffi.md`, `docs/adr/0005-require-runtime-api-key-configuration.md`, `docs/adr/0006-use-local-file-locks-for-jsonl-store.md` | Done | JSONL, modular monolith, FFI boundaries, runtime secret configuration, and local file locking are justified. |
| API contract is valid | `openapi.yaml`, command `npx @redocly/cli lint openapi.yaml` | Done | Versioned endpoints, auth, errors, 409, 429, examples, and concrete event payload schemas. |
| Data consistency and corruption behavior are real | `crates/ferrisledger-store/src/lib.rs`, `docs/architecture/data-consistency.md`, `docs/runbooks/event-log-corruption.md` | Done | Expected stream version, local file locks, duplicate idempotency-key rejection, isolation assumptions, rollback, migration path, and CRC32 verification exist. |
| API auth is tested | `crates/ferrisledger-api/src/lib.rs` | Done | Missing API key returns `401`. |
| API key is supplied at runtime, not baked into the image | `crates/ferrisledger-cli/src/main.rs`, `Dockerfile`, `docker-compose.yml`, `docs/security/secrets.md`, `docs/adr/0005-require-runtime-api-key-configuration.md` | Done | `serve` requires `--api-key` or `FERRISLEDGER_API_KEY`; Docker Compose requires caller-provided `FERRISLEDGER_API_KEY`. |
| API key configuration is locally hardened | `crates/ferrisledger-api/src/lib.rs`, `crates/ferrisledger-cli/src/main.rs`, `docs/security/secrets.md`, `docs/security/threat-model.md` | Done | Runtime rejects short, whitespace, non-visible, or overlong configured keys; comparisons avoid data-dependent early exit. |
| Invalid-auth abuse is throttled separately | `crates/ferrisledger-api/src/lib.rs`, `docs/security/abuse-cases.md`, `docs/api/README.md` | Done | Missing or wrong API-key attempts use a separate in-process rolling-window limiter and return `429` after the configured threshold. |
| Tenant isolation/BOLA is tested | `crates/ferrisledger-api/src/lib.rs` | Done | Cross-tenant snapshot returns no state. |
| Idempotency conflict semantics are tested | `crates/ferrisledger-runtime/src/lib.rs`, `crates/ferrisledger-api/src/lib.rs`, `docs/security/abuse-cases.md` | Done | Same key and same command replays; incompatible reuse returns `409` across command type, account, Pix, settlement, ledger, amount, and payload changes. |
| Rate limiting is implemented and tested | `crates/ferrisledger-api/src/lib.rs`, `crates/ferrisledger-telemetry/src/lib.rs` | Done | Per-API-key rolling window returns `429` and increments metric. |
| Operational HTTP endpoints are tested | `crates/ferrisledger-api/src/lib.rs`, `docs/spec-driven/verification-report.md` | Done | Tests cover health, readiness, metrics, event listing, Pix reservation, settlement, and ledger-entry flows. |
| CLI operator workflows are tested | `crates/ferrisledger-cli/tests/cli_smoke.rs`, `docs/spec-driven/verification-report.md` | Done | Binary smoke tests cover verify, open/deposit/replay, and weak-key serve rejection. |
| Request/correlation IDs are propagated through HTTP | `crates/ferrisledger-api/src/lib.rs`, `openapi.yaml` | Done | Success and error responses include context headers and error body context. |
| Audit logging includes command context | `crates/ferrisledger-api/src/lib.rs` | Done | Structured logs include event/stream/correlation/request IDs. |
| Unsafe boundary is isolated and tested | `crates/ferrisledger-ffi/src/lib.rs`, `docs/unsafe/README.md` | Done | Runtime does not use FFI by default. |
| Observability surfaces exist | `crates/ferrisledger-telemetry/src/lib.rs`, `docs/architecture/observability.md`, `ops/grafana/ferrisledger-dashboard.json`, `docs/runbooks/*.md` | Done | Health, readiness, Prometheus metrics, JSON logs, dashboard, runbooks. |
| Trace context exists locally | `crates/ferrisledger-api/src/lib.rs`, `docs/architecture/observability.md` | Done | Structured JSON traces/logs include request ID, correlation ID, event ID, stream ID, and command result. |
| OTLP collector export is scoped | `docs/spec-driven/verification-report.md`, `docs/spec-driven/techlead-hardening-spec.md` | Planned | External collector export is deferred; local evaluation does not require an external collector. |
| Benchmarks have scripts and measured baseline | `benchmarks/*.js`, `crates/ferrisledger-runtime/benches/replay.rs`, `benchmarks/results/2026-05-30-smoke.md`, `benchmarks/results/2026-05-31-load.md` | Done | Smoke, load, and Criterion results are recorded; load/stress/spike assets expose p99 trend output. |
| CPU and memory notes are measured under load | `benchmarks/results/2026-05-31-load.md`, `docs/benchmarks/README.md` | Done | Load baseline records p50 13 ms, p95 39.39 ms, p99 68.21 ms, 19.240515 req/s, 0.00% errors, server CPU/RSS, and k6 memory. |
| Scalability limits are explicit | `docs/scalability.md`, `docs/operational-cost.md` | Done | Names JSONL local-file and rate-limit multi-replica limits. |
| Alerts are documented | `ops/prometheus/ferrisledger-alerts.yml`, `docs/architecture/observability.md` | Done | Error-rate, rate-limit spike, and empty-store alerts exist. |
| CI covers required gates | `.github/workflows/ci.yml` | Done | Format, lint, tests, 85% line-coverage floor, audit, OpenAPI, Docker build. |
| Local coverage command runs | `docs/spec-driven/verification-report.md` | Done | Homebrew LLVM binaries are supplied through `LLVM_COV` and `LLVM_PROFDATA`; local report measured 90.32% line coverage. |
| Docker build validates locally | `Dockerfile`, `docker-compose.yml`, `docs/spec-driven/verification-report.md` | Done | Docker build passed locally after removing the image-level default API key. |
| Tech-lead hardening spec is explicit | `docs/spec-driven/techlead-hardening-spec.md` | Done | May 31 hardening criteria are listed with verification evidence. |

## Out of Scope

- Real Pix provider integration.
- PostgreSQL event store and distributed locking.
- Redis-backed distributed rate limiting.
- JWT/OIDC authorization.
- OpenTelemetry OTLP exporter.
- Kubernetes manifests.
- Cryptographic event signing.
