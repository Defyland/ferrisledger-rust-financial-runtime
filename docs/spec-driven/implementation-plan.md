# Implementation Plan

## Scope

Continue FerrisLedger from the current repository state using the shared senior
quality specs as the acceptance bar. The pass is intentionally scoped to this
project directory. It audits existing claims, fixes only real code/doc
divergence, refreshes spec-driven evidence, and reruns local verification.

## Files to Create or Update

- `crates/ferrisledger-cli/src/main.rs`
- `Dockerfile`
- `docker-compose.yml`
- `README.md`
- `docs/adr/0005-require-runtime-api-key-configuration.md`
- `docs/api/README.md`
- `docs/architecture/deployment-readiness.md`
- `docs/architecture/deployment-view.md`
- `docs/engineering-case-study.md`
- `docs/security/secrets.md`
- `docs/spec-driven/senior-readiness-spec.md`
- `docs/spec-driven/implementation-plan.md`
- `docs/spec-driven/verification-report.md`
- `docs/spec-driven/tech-lead-review.md`

## Acceptance Criteria Mapping

| Acceptance criterion | Planned change | Verification |
| --- | --- | --- |
| Shared senior specs drive the work | Re-read global specs and project spec-driven docs before edits | File audit recorded in final summary |
| Documentation reflects the real system | Audit README, docs, OpenAPI, CI, Docker, and Rust crates against implementation | `rg`, file reads, and validation commands |
| API key handling matches secret-management claims | Remove default API key from CLI/container startup and require runtime configuration | `cargo clippy`, `cargo test`, `docker build`, `docker compose config` |
| Security decision is explainable | Add ADR for runtime API-key configuration | ADR file audit |
| Docker build is locally proven when daemon is available | Re-run Docker build after secret default removal | `docker build -t ferrisledger:local .` |
| Existing quality gates remain green | Re-run format, lint, tests, build, benchmark, OpenAPI, audit, k6, Docker, and Compose config | Verification report |

## Verification Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release -p ferrisledger-cli
cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10
k6 inspect benchmarks/k6-smoke.js
k6 inspect benchmarks/k6-load.js
k6 inspect benchmarks/k6-stress.js
k6 inspect benchmarks/k6-spike.js
BASE_URL=http://127.0.0.1:18080 API_KEY=dev-secret k6 run benchmarks/k6-smoke.js
npx @redocly/cli lint openapi.yaml
cargo audit
PATH=/Users/allanflavio/.cargo/bin:$PATH cargo llvm-cov --workspace --all-targets --summary-only
docker build -t ferrisledger:local .
FERRISLEDGER_API_KEY=dev-secret docker compose config
```

## Risks

- API keys are still static; this pass removes unsafe defaults but does not add
  scoped keys, rotation endpoints, or OIDC.
- In-process rate limiting is intentionally local and resets on restart.
- JSONL remains a single-writer MVP store.
- Criterion and k6 numbers vary by local machine load.
- Local coverage requires a Rust toolchain with `llvm-tools-preview`.

## Deferred Work

- Redis or gateway-backed distributed rate limiting.
- OpenTelemetry OTLP trace exporter.
- PostgreSQL event-store adapter.
- Snapshot compaction and segment rotation.
- JWT/OIDC and scoped API keys.
- Tamper-evident event signatures.
