# Verification Report

## Summary

This hardening pass took the May 31 senior/tech-lead review and the follow-up
"no bullshit" review as the acceptance bar. The work closed local gaps without
pretending external production infrastructure exists.

Fixed or raised: money arithmetic rejects overflow, idempotency keys cannot
silently replay divergent command data, JSONL append/read/verify use local OS
file locks, persisted duplicate event IDs and duplicate idempotency keys fail
closed, unknown event-envelope fields no longer deserialize silently, a store
property test mutates persisted record bytes to prove fail-closed parser
behavior, recomputed-checksum envelope drift between payload and persisted
`stream_id` now fails closed, OpenAPI defines concrete event payload schemas and
Rust-side JSON serialization contracts, API keys are validated at startup, invalid-auth
attempts are throttled separately, API and CLI workflows have broader
executable coverage, CLI smoke covers real multi-process store access, CI pins
Rust 1.95 and enforces an 85% line coverage floor, and k6 load evidence records
full latency percentiles plus CPU/RSS resource usage.

## Commands Run

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | No formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed | Finished dev profile without warnings. |
| `cargo test --workspace --all-targets` | Passed | 46 Rust tests passed: API 11, CLI smoke 4, domain 5, events 3, FFI 2, rules 3, runtime 8, store 9, worker 1; benchmark compile smoke also succeeded. |
| `cargo test -p ferrisledger-store --all-targets` | Passed | Store tests now cover checksum corruption, persisted duplicate event IDs, persisted duplicate idempotency keys, unknown optional envelope-field names, recomputed-checksum `stream_id` drift rejection, concurrent same-host append coordination, and property-based single-byte mutation rejection. |
| `cargo test -p ferrisledger-cli --test cli_smoke` | Passed | CLI smoke covers verify, open/deposit/replay, weak-key serve rejection, and eight concurrent OS child processes appending to one locked JSONL store. |
| `cargo test -p ferrisledger-events --all-targets` | Passed | Event tests lock all serialized payload `kind`/`data` shapes and the envelope's redundant `event_type` plus typed payload shape. |
| `cargo build --release -p ferrisledger-cli` | Passed | Release binary rebuilt after API-key and CLI changes. |
| `target/release/ferrisledger serve --help` | Passed | Usage shows required `--api-key <API_KEY>`, no default API key, and `--auth-failure-rate-limit-per-minute`. |
| `cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10` | Passed | `replay_100_deposits` measured 256.12 us to 281.14 us, point estimate 268.82 us. |
| `k6 inspect benchmarks/k6-smoke.js` | Passed | Smoke script parses and exposes p50/p90/p95/p99 trend stats. |
| `k6 inspect benchmarks/k6-load.js` | Passed | Load script parses with p95/p99 thresholds and full trend stats. |
| `k6 inspect benchmarks/k6-stress.js` | Passed | Stress script parses with staged ramp and full trend stats. |
| `k6 inspect benchmarks/k6-spike.js` | Passed | Spike script parses with staged spike and full trend stats. |
| `BASE_URL=http://127.0.0.1:18082 API_KEY=dev-secret-local k6 run benchmarks/k6-smoke.js` | Passed | 18/18 checks passed, p50 12.24 ms, p95 19.6 ms, p99 20.8 ms, 0.00% errors, 1.942228 req/s. |
| `BASE_URL=http://127.0.0.1:18083 API_KEY=dev-secret-local k6 run benchmarks/k6-load.js` | Passed | 3522/3522 checks passed, p50 13 ms, p95 39.39 ms, p99 68.21 ms, max 133.19 ms, 0.00% errors, 19.240515 req/s; server max CPU 25.40%, server max RSS 25,120 KiB, k6 max RSS 42,608 KiB. |
| `npx @redocly/cli lint openapi.yaml` | Passed | OpenAPI description valid with concrete event payload schemas and no warnings. |
| `cargo audit` | Passed | Cargo.lock scanned with no vulnerabilities reported. |
| `LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata cargo llvm-cov --workspace --all-targets --lcov --output-path /tmp/ferrisledger-lcov-final.info --fail-under-lines 85` | Passed | CI-equivalent local coverage gate passed. |
| `LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata cargo llvm-cov report --summary-only` | Passed | Total coverage: 87.95% regions, 84.79% functions, 90.86% lines. |
| `docker build -t ferrisledger:local .` | Passed | Image built successfully after removing the false-positive auth-failure ENV secret warning. |
| `FERRISLEDGER_API_KEY=dev-secret-local docker compose config` | Passed | Compose renders with caller-provided API key, authenticated rate limit, auth-failure rate limit, and local store path. |
| `git diff --check` | Passed | No whitespace errors after report updates. |

## Passing Criteria

- README and case study present the product, users, workflow, trade-offs,
  failure modes, security posture, operational cost, and roadmap.
- Domain rules are implemented and tested with business-readable names.
- Money arithmetic rejects overflow and underflow at the domain boundary.
- API has valid OpenAPI 3.1, versioned endpoints, API-key auth, 409 behavior,
  429 behavior, idempotency replay/conflict behavior, context headers,
  concrete event payload schemas, Rust serialization tests for all event
  variants, and standardized errors.
- API key startup behavior matches the secret-management docs: no CLI or
  Docker image default key, explicit runtime configuration required, weak keys
  rejected, and auth-failure attempts throttled separately.
- HTTP tests now cover health, readiness, metrics, account creation, deposits,
  Pix transfer reservation, settlement, ledger evidence, event listing,
  snapshots, tenant isolation, auth, rate limiting, idempotency conflict, and
  request/correlation headers.
- CLI smoke tests cover local verify, open/deposit/replay, weak-key serve
  rejection, and shared-store multi-process append behavior through the built
  binary.
- Store detects corruption through JSON decoding, checksum verification,
  deny-unknown-field event parsing, property-based single-byte mutation tests,
  persisted envelope-contract validation, coordinates local same-host access
  with OS file locks, and rejects duplicate event IDs and duplicate
  idempotency keys.
- Observability includes health, readiness, Prometheus metrics, structured
  audit logs, request IDs, correlation IDs, dashboard JSON, and runbooks.
- Benchmarks include current smoke, load, and replay measurements plus
  load/stress/spike assets with explicit p99 output.
- Coverage now runs locally without `rustup`; CI pins Rust 1.95 and enforces an
  85% line coverage floor.
- CI defines format, lint, tests, coverage threshold, audit, OpenAPI, and Docker
  Compose config plus Docker build gates.

## Partial Criteria

- OpenTelemetry trace export is not implemented. Structured tracing exists and
  OTLP export is deferred until there is a collector target.
- Rate limiting is in-process. This proves authenticated and invalid-auth
  abuse-control behavior locally, but a multi-replica deployment needs Redis,
  Envoy, or API-gateway enforcement.
- JSONL storage is production-shaped for local append/replay reasoning and
  same-host coordination, but not a production-grade shared event database.

## Failed or Blocked Criteria

None for the agreed local, no-external-services scope.

## Remaining Risk

- The first production hardening milestone should be PostgreSQL event storage
  with transactional append/outbox semantics.
- Auth should move from one static API key to scoped keys or OIDC.
- Event records should become tamper-evident if used for regulated audit.
- Snapshot compaction is required before very long account streams.
