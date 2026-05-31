# Implementation Plan

## Scope

Resolve the May 31 senior/tech-lead hardening findings without relying on
external services. The scope is a production-shaped local MVP: close local
correctness, consistency, contract, benchmark, and verification gaps while
documenting infrastructure that remains intentionally outside this repository.

## Files to Create or Update

- `crates/ferrisledger-domain/src/lib.rs`
- `crates/ferrisledger-runtime/src/lib.rs`
- `crates/ferrisledger-store/src/lib.rs`
- `crates/ferrisledger-api/src/lib.rs`
- `benchmarks/k6-smoke.js`
- `benchmarks/results/2026-05-30-smoke.md`
- `benchmarks/results/2026-05-31-load.md`
- `openapi.yaml`
- `README.md`
- `docs/adr/0006-use-local-file-locks-for-jsonl-store.md`
- `docs/api/README.md`
- `docs/architecture/data-consistency.md`
- `docs/benchmarks/README.md`
- `docs/domain/invariants.md`
- `docs/engineering-case-study.md`
- `docs/security/abuse-cases.md`
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
| Idempotency key reuse is semantically safe | Compare the reused key against command type, tenant, account, amount, currency, Pix beneficiary, settlement ID, ledger ID, reason, and related event | Runtime/API conflict tests |
| Store protects local append/verify races | Use advisory OS file locks for append/read/verify and reject duplicate idempotency keys in the store | Store concurrency and duplicate-key tests |
| OpenAPI event payloads are concrete | Define event payload `oneOf` schemas and add `409` responses for repeatable command conflicts | `npx @redocly/cli lint openapi.yaml` |
| k6 load evidence includes resource notes | Make generated account IDs VU-safe and record a load run with server CPU/RSS sampling | `benchmarks/results/2026-05-31-load.md` |
| Local coverage is no longer blocked | Use Homebrew LLVM binaries through `LLVM_COV` and `LLVM_PROFDATA` | `cargo llvm-cov report --summary-only` |
| Senior evidence is current and honest | Update spec matrix, case study, docs, and verification report | File audit and final report |

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
BASE_URL=http://127.0.0.1:18082 API_KEY=dev-secret k6 run benchmarks/k6-smoke.js
BASE_URL=http://127.0.0.1:18083 API_KEY=dev-secret k6 run benchmarks/k6-load.js
npx @redocly/cli lint openapi.yaml
cargo audit
LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov \
  LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata \
  cargo llvm-cov --workspace --all-targets --summary-only --text
LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov \
  LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata \
  cargo llvm-cov report --summary-only
docker build -t ferrisledger:local .
FERRISLEDGER_API_KEY=dev-secret docker compose config
git diff --check
```

## Risks

- Advisory file locks coordinate same-host processes that respect locks; they
  are not a distributed lock.
- API keys are still static; this pass removes unsafe defaults but does not add
  scoped keys, rotation endpoints, or OIDC.
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
