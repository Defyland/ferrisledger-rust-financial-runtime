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
| Local store concurrency | JSONL append and verification must coordinate across processes on the same host, not only across clones inside one process. | Store uses OS file locks plus the existing in-process mutex; tests cover concurrent appends from independent store handles and real CLI child processes sharing one store. |
| Contract specificity | OpenAPI must define concrete event payload shapes rather than an unconstrained `object`, and Rust serialization must not drift from those shapes. | `npx @redocly/cli lint openapi.yaml`; event serialization contract tests. |
| Runtime API-key posture | The API must reject weak configured keys, avoid default image secrets, and document the exact local secret contract. | API configuration tests, CLI help, Docker build, Compose config, security docs. |
| Invalid-auth abuse control | Missing or wrong API keys must be throttled separately from authenticated request buckets. | API auth-failure rate-limit test and `--auth-failure-rate-limit-per-minute` CLI option. |
| Endpoint workflow coverage | HTTP tests must exercise operational surfaces and money-flow endpoints, not only happy-path account creation. | `cargo test -p ferrisledger-api --all-targets`. |
| Operator CLI coverage | The CLI must have executable smoke evidence for local verification, account workflow, replay, weak-key rejection, and multi-process shared-store behavior. | `cargo test -p ferrisledger-cli --all-targets`. |
| CI/toolchain fidelity | Local and CI builds must use the declared Rust MSRV and validate the Compose runtime configuration, not only the Docker image build. | `rust-toolchain.toml`, `dtolnay/rust-toolchain@1.95`, and `FERRISLEDGER_API_KEY=dev-secret-local docker compose config`. |
| Local coverage gate | Coverage must run on the local Homebrew Rust toolchain without requiring `rustup` and must fail below the CI floor. | `cargo llvm-cov --workspace --all-targets --lcov --output-path /tmp/ferrisledger-lcov-final.info --fail-under-lines 85`; summary measured 90.86% line coverage. |
| Load resource evidence | At least one k6 load result must include p50/p95/p99 latency, throughput, error rate, server CPU, and server RSS. | `benchmarks/results/2026-05-31-load.md`. |
| Benchmark script fidelity | k6 smoke/load/stress/spike assets must expose full latency percentiles, including p99, so future local runs are comparable. | `k6 inspect benchmarks/k6-smoke.js`, `k6-load.js`, `k6-stress.js`, and `k6-spike.js`. |
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
