# FerrisLedger Engineering Baseline

FerrisLedger now implements the initiative baseline with runnable code and
evidence artifacts.

## Implemented outcomes

- Product-grade README with product and engineering sections.
- Cargo workspace with explicit module boundaries.
- `openapi.yaml` for the HTTP API.
- Typed domain modeling and deterministic replay.
- Append-only JSONL event store with checksums and optimistic stream versions.
- API, CLI, async worker, telemetry, macro, FFI, and test-support crates.
- Unit, property, API, auth, corruption, worker, FFI, and benchmark tests.
- Docker, Docker Compose, GitHub Actions, k6 scripts, and Criterion benchmark.
- Senior-level documentation package under `docs/`.

## Quality gate

The expected local gate is:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo bench -p ferrisledger-runtime --bench replay
k6 run benchmarks/k6-smoke.js
```

## Deferred work

The MVP intentionally defers PostgreSQL, distributed locking, outbox delivery,
JWT/OIDC, segment rotation, snapshot compaction, and Kubernetes manifests.
