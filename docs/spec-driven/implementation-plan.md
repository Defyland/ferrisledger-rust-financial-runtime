# Implementation Plan

## Scope

Apply the new spec-driven senior quality workflow to FerrisLedger without
rewriting unrelated implementation. The scope is limited to
`ferrisledger-rust-financial-runtime/` after reading shared specs from the
repository root.

## Files to Create or Update

- `docs/spec-driven/senior-readiness-spec.md`
- `docs/spec-driven/implementation-plan.md`
- `docs/spec-driven/verification-report.md`
- `docs/spec-driven/tech-lead-review.md`
- `README.md`
- `openapi.yaml`
- `crates/ferrisledger-api/src/lib.rs`
- `crates/ferrisledger-cli/src/main.rs`
- `crates/ferrisledger-telemetry/src/lib.rs`
- `Dockerfile`
- `docker-compose.yml`
- `benchmarks/k6-smoke.js`
- `docs/api/README.md`
- `docs/architecture/data-consistency.md`
- `docs/architecture/observability.md`
- `docs/architecture/overview.md`
- `docs/architecture/module-boundaries.md`
- `docs/operational-cost.md`
- `docs/scalability.md`
- `docs/security/*.md`
- `ops/grafana/ferrisledger-dashboard.json`
- `ops/prometheus/ferrisledger-alerts.yml`

## Acceptance Criteria Mapping

| Acceptance criterion | Planned change | Verification |
| --- | --- | --- |
| Spec-driven files exist and define the quality bar | Create three files under `docs/spec-driven/` | File audit and final report |
| README points to evidence docs | Add evidence entrypoints | Manual read and file audit |
| API documents and implements rate limiting | Add per-key in-memory limiter, 429 error, OpenAPI 429 responses | API unit test, OpenAPI lint |
| Security docs match actual behavior | Update threat model, authorization matrix, abuse cases, secrets | File audit |
| Observability includes domain/security metric | Add `ferrisledger_api_rate_limited_total` and dashboard panel | Clippy/tests and dashboard JSON validation |
| Request/correlation IDs appear in command audit logs and HTTP responses | Generate or propagate `x-request-id`, return context headers, include error body context | API unit test and OpenAPI lint |
| Benchmark output can report p99 | Add k6 summary trend stats | `k6 run benchmarks/k6-smoke.js` |
| Existing quality gates remain green | Run format, lint, tests, OpenAPI, audit, build, benchmark, k6 | Verification report |

## Verification Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release -p ferrisledger-cli
cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10
k6 run benchmarks/k6-smoke.js
k6 inspect benchmarks/k6-load.js
k6 inspect benchmarks/k6-stress.js
k6 inspect benchmarks/k6-spike.js
npx @redocly/cli lint openapi.yaml
cargo audit
docker build -t ferrisledger:local .
```

## Risks

- In-process rate limiting is intentionally local and resets on restart.
- JSONL remains a single-writer MVP store.
- Docker build depends on the local Docker daemon.
- Criterion results vary across local machines.

## Deferred Work

- Redis or gateway-backed distributed rate limiting.
- OpenTelemetry OTLP trace exporter.
- PostgreSQL event-store adapter.
- Snapshot compaction and segment rotation.
- JWT/OIDC and scoped API keys.
