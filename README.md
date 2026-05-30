# FerrisLedger

Financial event runtime built in Rust to explore production-grade Rust concepts through a realistic backend and systems project: typed financial domain modeling, append-only storage, replay, API, CLI, async workers, macros, and safe wrappers over unsafe and FFI.

## Status

Phase 0 bootstrap only. This repository currently establishes naming, scope, documentation structure, and engineering expectations. It does not yet contain the Cargo workspace crates, Axum API, Tokio workers, append-only store implementation, macros, or FFI bindings.

## Product intent

FerrisLedger is planned as a Rust financial runtime that can append, validate, store, read, replay, and snapshot financial events such as account opening, deposits, Pix transfer requests, settlement execution, and ledger entry creation. It is designed both as a realistic product shape and as a deliberate path through the hard parts of Rust.

## Planned stack

- Rust stable
- Cargo Workspace
- Tokio
- Axum
- Tower
- Serde
- Clap
- Tracing
- Prometheus
- Criterion
- Proptest
- thiserror
- anyhow
- bytes
- memmap2 as an optional later-phase storage optimization
- libc or `cc` as an optional FFI support layer
- Docker

## Engineering focus

This project is meant to demonstrate:

- strong domain modeling with enums, newtypes, traits, generics, and type-state patterns
- append-only storage, checksums, replay, and corruption detection in a realistic event runtime
- async APIs, workers, channels, and backpressure with Tokio and Axum
- procedural macros, declarative macros, and feature flags in a multi-crate Rust workspace
- explicit unsafe and FFI boundaries behind safe abstractions
- property-based tests, benchmarks, and learning-oriented documentation for core Rust concepts

## Bootstrap contents

- repository initialized and synchronized with GitHub
- mandatory documentation folders created, including `docs/rust-learning-map/` and `docs/unsafe/`
- baseline engineering spec captured in `docs/engineering-baseline.md`
- financial runtime event contracts documented in `docs/events/README.md`
- threat model documented in `docs/security/threat-model.md`
- deployment readiness documented in `docs/architecture/deployment-readiness.md`

## Next phase

The first implementation slice should prioritize the Cargo workspace layout, typed domain and event crates, rules validation, append-only storage format, and the first learning-map documents before adding API, worker, CLI, macros, and FFI layers.
