# Implementation Plan

## Scope

Resolve the May 31 senior/tech-lead hardening findings and the follow-up
review gaps that do not require external infrastructure. The target is a
production-shaped local MVP: critical correctness, security posture, endpoint
coverage, CLI evidence, CI gates, benchmark evidence, and docs must all match
the code that actually runs.

## Files to Create or Update

- `.github/workflows/ci.yml`
- `Dockerfile`
- `docker-compose.yml`
- `Cargo.lock`
- `README.md`
- `crates/ferrisledger-domain/src/lib.rs`
- `crates/ferrisledger-runtime/src/lib.rs`
- `crates/ferrisledger-store/src/lib.rs`
- `crates/ferrisledger-api/src/lib.rs`
- `crates/ferrisledger-cli/Cargo.toml`
- `crates/ferrisledger-cli/src/main.rs`
- `crates/ferrisledger-cli/tests/cli_smoke.rs`
- `benchmarks/k6-smoke.js`
- `benchmarks/k6-load.js`
- `benchmarks/k6-stress.js`
- `benchmarks/k6-spike.js`
- `benchmarks/results/2026-05-30-smoke.md`
- `benchmarks/results/2026-05-31-load.md`
- `openapi.yaml`
- `docs/adr/0005-require-runtime-api-key-configuration.md`
- `docs/adr/0006-use-local-file-locks-for-jsonl-store.md`
- `docs/api/README.md`
- `docs/api/examples.md`
- `docs/architecture/data-consistency.md`
- `docs/architecture/deployment-readiness.md`
- `docs/benchmarks/README.md`
- `docs/domain/invariants.md`
- `docs/engineering-case-study.md`
- `docs/security/abuse-cases.md`
- `docs/security/secrets.md`
- `docs/security/threat-model.md`
- `docs/spec-driven/techlead-hardening-spec.md`
- `docs/spec-driven/senior-readiness-spec.md`
- `docs/spec-driven/implementation-plan.md`
- `docs/spec-driven/verification-report.md`
- `docs/spec-driven/tech-lead-review.md`

## Acceptance Criteria Mapping

| Acceptance criterion | Planned change | Verification |
| --- | --- | --- |
| Money arithmetic cannot wrap | Use checked add/subtract and expose `money_arithmetic_overflow` | Domain/API tests and `cargo test --workspace --all-targets` |
| Idempotency key reuse is semantically safe | Compare reused keys against command type, tenant, account, amount, currency, Pix beneficiary, settlement ID, ledger ID, reason, and related event | Runtime/API conflict tests |
| Store protects local append/verify races | Use advisory OS file locks for append/read/verify and reject duplicate idempotency keys in the store | Store concurrency and duplicate-key tests |
| API keys are not weak by default | Reject weak configured API keys, expose auth-failure throttling, and avoid Docker image secret defaults | API config/auth tests, CLI help, Docker build |
| Authentication abuse is locally bounded | Add a separate in-process limiter for missing/wrong API-key attempts | API auth-failure rate-limit test |
| HTTP endpoint coverage reflects the contract | Add request tests for health, readiness, metrics, event listing, Pix, settlement, and ledger workflows | `cargo test -p ferrisledger-api --all-targets` |
| CLI workflows have executable evidence | Add binary smoke tests for verify, open/deposit/replay, and weak-key serve rejection | `cargo test -p ferrisledger-cli --all-targets` |
| OpenAPI event payloads are concrete | Define event payload `oneOf` schemas and add `409` responses for repeatable command conflicts | `npx @redocly/cli lint openapi.yaml` |
| k6 evidence includes p99 and resource notes | Emit full trend stats from smoke/load/stress/spike scripts and record load CPU/RSS | k6 inspect commands and `benchmarks/results/2026-05-31-load.md` |
| Local coverage is enforced, not just reported | Use Homebrew LLVM binaries and add an 85% line-coverage floor in CI | `cargo llvm-cov --workspace --all-targets --lcov --output-path /tmp/ferrisledger-lcov-final.info --fail-under-lines 85` |
| Senior evidence is current and honest | Update spec matrix, benchmark docs, security docs, case-study references, and verification report | File audit and final report |

## Verification Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --release -p ferrisledger-cli
target/release/ferrisledger serve --help
cargo bench -p ferrisledger-runtime --bench replay -- --sample-size 10
k6 inspect benchmarks/k6-smoke.js
k6 inspect benchmarks/k6-load.js
k6 inspect benchmarks/k6-stress.js
k6 inspect benchmarks/k6-spike.js
BASE_URL=http://127.0.0.1:18082 API_KEY=dev-secret-local k6 run benchmarks/k6-smoke.js
BASE_URL=http://127.0.0.1:18083 API_KEY=dev-secret-local k6 run benchmarks/k6-load.js
npx @redocly/cli lint openapi.yaml
cargo audit
LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov \
  LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata \
  cargo llvm-cov --workspace --all-targets --lcov \
  --output-path /tmp/ferrisledger-lcov-final.info \
  --fail-under-lines 85
LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov \
  LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata \
  cargo llvm-cov report --summary-only
docker build -t ferrisledger:local .
FERRISLEDGER_API_KEY=dev-secret-local docker compose config
git diff --check
```

## Risks

- Advisory file locks coordinate same-host processes that respect locks; they
  are not a distributed lock.
- API keys are validated and auth failures are throttled locally, but keys are
  still static and do not provide scopes, roles, rotation windows, or OIDC.
- In-process rate limiting is intentionally local and resets on restart.
- JSONL remains a local MVP store; PostgreSQL is still the production storage
  evolution.
- Criterion and k6 numbers vary by local machine load.

## Deferred Work

- Redis or gateway-backed distributed rate limiting.
- OpenTelemetry OTLP export to a collector.
- PostgreSQL event-store adapter.
- Snapshot compaction and segment rotation.
- JWT/OIDC and scoped API keys.
- Cryptographic event signing.
