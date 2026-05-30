# Verification Report

## Summary

FerrisLedger now satisfies the senior-readiness spec for a local, portfolio
MVP: product narrative, domain model, modular architecture, API contract,
append-only store, security controls, observability, benchmarks, CI, and
spec-driven evidence are present and verified.

The remaining production gaps are intentionally documented: distributed rate
limiting, PostgreSQL event storage, OTLP trace export, OIDC/scoped keys,
snapshot compaction, and Docker build verification on a machine with the Docker
daemon running.

## Commands Run

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | No formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed | Finished dev profile without warnings. |
| `cargo test --workspace --all-targets` | Passed | Workspace tests passed, including API auth, rate limit, tenant isolation, corruption, FFI, worker, and property tests. |
| `cargo build --release -p ferrisledger-cli` | Passed | Release binary built successfully. |
| `cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10` | Passed | `replay_100_deposits` measured 257.07 us to 328.93 us, mean 292.95 us. |
| `k6 inspect benchmarks/k6-smoke.js` | Passed | Smoke script parses and exposes p50/p95/p99 trend stats. |
| `k6 inspect benchmarks/k6-load.js` | Passed | Load script parses. |
| `k6 inspect benchmarks/k6-stress.js` | Passed | Stress script parses. |
| `k6 inspect benchmarks/k6-spike.js` | Passed | Spike script parses. |
| `k6 run benchmarks/k6-smoke.js` | Passed | 18/18 checks passed, p50 10.54 ms, p95 28.76 ms, p99 30.51 ms, 0.00% errors. |
| `npx @redocly/cli lint openapi.yaml` | Passed | OpenAPI description valid with no warnings. |
| `cargo audit` | Passed | Cargo.lock scanned with no vulnerabilities reported. |
| `docker build -t ferrisledger:local .` | Blocked locally | Docker CLI exists, but daemon is not running at `/Users/allanflavio/.docker/run/docker.sock`. CI keeps Docker build as a required gate. |

## Passing Criteria

- README and case study present the product, users, workflow, trade-offs,
  failure modes, security posture, operational cost, and roadmap.
- Domain rules are implemented and tested with business-readable names.
- API has valid OpenAPI 3.1, versioned endpoints, API-key auth, 429 behavior,
  idempotency, context headers, and standardized errors.
- Store detects corruption through JSON decoding and checksum verification.
- Observability includes health, readiness, Prometheus metrics, structured
  audit logs, request IDs, correlation IDs, dashboard JSON, and runbooks.
- Benchmarks include real smoke and replay measurements plus load/stress/spike
  assets.
- CI defines format, lint, tests, coverage, audit, OpenAPI, and Docker build
  gates.
- Commit history can now be created atomically from the current logical change
  groups.

## Partial Criteria

- OpenTelemetry trace export is not implemented. Structured tracing exists and
  OTLP export is deferred until there is a collector target.
- Rate limiting is in-process. This proves the abuse-control behavior locally,
  but a multi-replica deployment needs Redis, Envoy, or API-gateway enforcement.
- JSONL storage is production-shaped for append/replay reasoning, but not a
  production-grade shared event database.

## Failed or Blocked Criteria

- Local Docker build was blocked because the Docker daemon is unavailable.

## Remaining Risk

- The first production hardening milestone should be PostgreSQL event storage
  with transactional append/outbox semantics.
- Auth should move from static API keys to scoped keys or OIDC.
- Event records should become tamper-evident if used for regulated audit.
- Snapshot compaction is required before very long account streams.
