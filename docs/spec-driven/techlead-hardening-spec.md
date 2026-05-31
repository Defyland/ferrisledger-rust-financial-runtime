# Tech Lead Hardening Spec

## Goal

Close the review blockers found in the May 31 senior/tech-lead assessment
without depending on external services. The repository should remain honest
about being a local MVP while proving the critical financial/runtime semantics
that a reviewer expects before production migration.

## Non-Negotiable Acceptance Criteria

| Area | Required behavior | Verification |
| --- | --- | --- |
| Money arithmetic | Balance and reservation arithmetic must reject `i64` overflow and underflow instead of wrapping or panicking. | Unit/property tests in `ferrisledger-domain`; `cargo test --workspace --all-targets`. |
| Idempotency | Reusing an idempotency key with the same command must replay the prior event; reusing it with different command type, amount, currency, account, tenant, settlement/ledger IDs, beneficiary, reason, or related event must return conflict. | Runtime and API tests assert replay and conflict paths. |
| Local store concurrency | JSONL append and verification must coordinate across processes on the same host, not only across clones inside one process. | Store uses OS file locks plus the existing in-process mutex; tests cover concurrent appends from independent store handles. |
| Contract specificity | OpenAPI must define concrete event payload shapes rather than an unconstrained `object`. | `npx @redocly/cli lint openapi.yaml`; manual schema audit. |
| Local coverage | Coverage must run on the local Homebrew Rust toolchain without requiring `rustup`. | `LLVM_COV=/opt/homebrew/opt/llvm/bin/llvm-cov LLVM_PROFDATA=/opt/homebrew/opt/llvm/bin/llvm-profdata cargo llvm-cov report --summary-only`. |
| Load resource evidence | At least one k6 load result must include latency percentiles, throughput, error rate, server CPU, and server RSS. | `benchmarks/results/2026-05-31-load.md`. |
| Review evidence | Senior-readiness docs must record the new evidence and avoid claiming production readiness where only local guarantees exist. | Spec matrix and verification report updated with commands and residual risks. |

## Out of Scope

- Replacing JSONL with PostgreSQL.
- Distributed locking across hosts or replicas.
- OIDC/JWT/scoped key infrastructure.
- Real payment-provider integration.
- Cryptographic event signatures.

## Review Standard

The target is not "production deployed"; it is "production-shaped local MVP".
For each remaining production gap, the code must either implement the local
equivalent or document exactly why the next step requires infrastructure outside
this repository.
