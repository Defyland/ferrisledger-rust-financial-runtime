# Verification Report

## Summary

This hardening pass took the May 31 senior/tech-lead review as the acceptance
bar and converted each concrete blocker into code, contract, tests, benchmark
evidence, and docs.

Fixed blockers: money arithmetic now rejects overflow, idempotency keys cannot
silently replay divergent command data, JSONL append/read/verify use local OS
file locks, OpenAPI defines concrete event payload schemas, local coverage runs
with Homebrew LLVM tools, and k6 load evidence records CPU/RSS resource usage.

## Commands Run

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | No formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed | Finished dev profile without warnings. |
| `cargo test --workspace --all-targets` | Passed | 31 Rust tests passed across API, domain, events, FFI, rules, runtime, store, and worker crates; benchmark compile smoke also succeeded. |
| `cargo build --release -p ferrisledger-cli` | Passed | Release binary rebuilt after the CLI API-key change. |
| `target/release/ferrisledger serve --help` | Passed | Usage shows `--api-key <API_KEY>` as required and no default API key. |
| `cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10` | Passed | `replay_100_deposits` measured 365.43 us to 719.54 us, point estimate 537.04 us; Criterion flagged a local regression after adding file-lock/idempotency checks. |
| `k6 inspect benchmarks/k6-smoke.js` | Passed | Smoke script parses and exposes p50/p95/p99 trend stats. |
| `k6 inspect benchmarks/k6-load.js` | Passed | Load script parses with p95/p99 thresholds. |
| `k6 inspect benchmarks/k6-stress.js` | Passed | Stress script parses with staged ramp. |
| `k6 inspect benchmarks/k6-spike.js` | Passed | Spike script parses with staged spike. |
| `BASE_URL=http://127.0.0.1:18082 API_KEY=dev-secret k6 run benchmarks/k6-smoke.js` | Passed | 18/18 checks passed, p50 8.65 ms, p95 14.73 ms, p99 15.49 ms, 0.00% errors, 1.955845 req/s. |
| `BASE_URL=http://127.0.0.1:18083 API_KEY=dev-secret k6 run benchmarks/k6-load.js` | Passed | 3426/3426 checks passed, p50 15.16 ms, p95 52.47 ms, p99 109.59 ms, max 924.72 ms, 0.00% errors, 18.959445 req/s; server max CPU 10.90%, server max RSS 23,184 KiB, k6 max RSS 43,408 KiB. |
| `npx @redocly/cli lint openapi.yaml` | Passed | OpenAPI description valid with concrete event payload schemas and no warnings. |
| `cargo audit` | Passed | Cargo.lock scanned with no vulnerabilities reported. |
| `LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata cargo llvm-cov --workspace --all-targets --summary-only --text` | Passed | Coverage run completed all workspace tests locally with Homebrew LLVM tools. |
| `LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata cargo llvm-cov report --summary-only` | Passed | Total coverage: 76.13% regions, 70.64% functions, 75.83% lines. |
| `docker build -t ferrisledger:local .` | Passed | Image built successfully with explicit runtime API-key configuration. |
| `FERRISLEDGER_API_KEY=dev-secret docker compose config` | Passed | Compose renders with caller-provided API key and local rate-limit default. |
| `git diff --check` | Passed | No whitespace errors after report updates. |

## Passing Criteria

- README and case study present the product, users, workflow, trade-offs,
  failure modes, security posture, operational cost, and roadmap.
- Domain rules are implemented and tested with business-readable names.
- Money arithmetic rejects overflow and underflow at the domain boundary.
- API has valid OpenAPI 3.1, versioned endpoints, API-key auth, 409 behavior,
  429 behavior, idempotency replay/conflict behavior, context headers,
  concrete event payload schemas, and standardized errors.
- API key startup behavior matches the secret-management docs: no CLI or
  Docker image default key, explicit runtime configuration required.
- Store detects corruption through JSON decoding and checksum verification,
  coordinates local same-host access with OS file locks, and rejects duplicate
  idempotency keys.
- Observability includes health, readiness, Prometheus metrics, structured
  audit logs, request IDs, correlation IDs, dashboard JSON, and runbooks.
- Benchmarks include current smoke, load, and replay measurements plus
  load/stress/spike assets.
- Coverage now runs locally without `rustup` by pointing `cargo-llvm-cov` at
  Homebrew `llvm-cov` and `llvm-profdata`.
- CI defines format, lint, tests, coverage, audit, OpenAPI, and Docker build
  gates.

## Partial Criteria

- OpenTelemetry trace export is not implemented. Structured tracing exists and
  OTLP export is deferred until there is a collector target.
- Rate limiting is in-process. This proves the abuse-control behavior locally,
  but a multi-replica deployment needs Redis, Envoy, or API-gateway enforcement.
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
