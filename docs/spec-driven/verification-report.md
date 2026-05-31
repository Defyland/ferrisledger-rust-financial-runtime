# Verification Report

## Summary

This continuation pass rebuilt context from the repository and shared specs,
then verified the existing senior-readiness claims against the actual Rust,
Docker, OpenAPI, benchmark, CI, security, and documentation surfaces.

One real mismatch was found and fixed: the API key was documented as an
environment-managed secret, but the CLI and Docker image still carried
`dev-secret` as a default. `serve` now requires `--api-key` or
`FERRISLEDGER_API_KEY`, Docker Compose requires caller-provided
`FERRISLEDGER_API_KEY`, and ADR 0005 records the decision.

## Commands Run

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | Passed | No formatting diff. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed | Finished dev profile without warnings. |
| `cargo test --workspace --all-targets` | Passed | 19 Rust tests passed across API, domain, events, FFI, rules, runtime, store, and worker crates; benchmark compile smoke also succeeded. |
| `cargo build --release -p ferrisledger-cli` | Passed | Release binary rebuilt after the CLI API-key change. |
| `target/release/ferrisledger serve --help` | Passed | Usage now shows `--api-key <API_KEY>` as required and no default API key. |
| `cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10` | Passed | `replay_100_deposits` measured 239.92 us to 439.14 us, mean 320.18 us. |
| `k6 inspect benchmarks/k6-smoke.js` | Passed | Smoke script parses and exposes p50/p95/p99 trend stats. |
| `k6 inspect benchmarks/k6-load.js` | Passed | Load script parses with p95/p99 thresholds. |
| `k6 inspect benchmarks/k6-stress.js` | Passed | Stress script parses with staged ramp. |
| `k6 inspect benchmarks/k6-spike.js` | Passed | Spike script parses with staged spike. |
| `BASE_URL=http://127.0.0.1:18080 API_KEY=dev-secret k6 run benchmarks/k6-smoke.js` | Passed | 18/18 checks passed, p50 8.49 ms, p95 13.85 ms, p99 15.1 ms, 0.00% errors, 1.956002 req/s. |
| `npx @redocly/cli lint openapi.yaml` | Passed | OpenAPI description valid with no warnings. |
| `cargo audit` | Passed | Cargo.lock scanned with no vulnerabilities reported. |
| `docker build -t ferrisledger:local .` | Passed | Image built successfully after removing `FERRISLEDGER_API_KEY` from Dockerfile; no secret-default warning remained. |
| `FERRISLEDGER_API_KEY=dev-secret docker compose config` | Passed | Compose renders with caller-provided API key and local rate-limit default. |
| `PATH=/Users/allanflavio/.cargo/bin:$PATH cargo llvm-cov --workspace --all-targets --summary-only` | Blocked locally | `cargo-llvm-cov` is installed, but local Homebrew Rust lacks `llvm-tools-preview` and `rustup` is unavailable to add it. CI still installs and runs the coverage gate. |
| `git diff --check` | Passed | No whitespace errors. |

## Passing Criteria

- README and case study present the product, users, workflow, trade-offs,
  failure modes, security posture, operational cost, and roadmap.
- Domain rules are implemented and tested with business-readable names.
- API has valid OpenAPI 3.1, versioned endpoints, API-key auth, 429 behavior,
  idempotency, context headers, and standardized errors.
- API key startup behavior now matches the secret-management docs: no CLI or
  Docker image default key, explicit runtime configuration required.
- Store detects corruption through JSON decoding and checksum verification.
- Observability includes health, readiness, Prometheus metrics, structured
  audit logs, request IDs, correlation IDs, dashboard JSON, and runbooks.
- Benchmarks include current smoke and replay measurements plus load/stress/spike
  assets.
- CI defines format, lint, tests, coverage, audit, OpenAPI, and Docker build
  gates.

## Partial Criteria

- OpenTelemetry trace export is not implemented. Structured tracing exists and
  OTLP export is deferred until there is a collector target.
- Rate limiting is in-process. This proves the abuse-control behavior locally,
  but a multi-replica deployment needs Redis, Envoy, or API-gateway enforcement.
- JSONL storage is production-shaped for append/replay reasoning, but not a
  production-grade shared event database.
- CPU and memory were not sampled for the smoke benchmark. The next load,
  stress, or spike evidence pass should capture them.

## Failed or Blocked Criteria

- Local coverage execution is blocked by the Homebrew Rust toolchain missing
  `llvm-tools-preview` and no local `rustup` binary. The CI workflow installs
  `cargo-llvm-cov` and runs coverage on GitHub-hosted Rust.

## Remaining Risk

- The first production hardening milestone should be PostgreSQL event storage
  with transactional append/outbox semantics.
- Auth should move from one static API key to scoped keys or OIDC.
- Event records should become tamper-evident if used for regulated audit.
- Snapshot compaction is required before very long account streams.
