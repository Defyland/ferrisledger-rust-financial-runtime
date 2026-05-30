# ADR 0001: Use a Cargo Workspace for the Financial Runtime

## Status

Accepted.

## Context

FerrisLedger needs to demonstrate multiple layers of Rust: domain modeling, event contracts, storage, indexes, rules, API, workers, CLI, macros, FFI, telemetry, and test support. Keeping all of that in one crate would hide the architectural boundaries and make ownership of concepts less explicit.

## Decision

FerrisLedger will be structured as a Cargo workspace with focused crates for domain, events, store, index, rules, API, worker, CLI, macros, FFI, telemetry, and test support.

## Consequences

- Workspace-level configuration and lockfile stay shared.
- Crate boundaries make domain independence and unsafe isolation visible.
- The repo becomes a realistic Rust monorepo rather than a single binary with internal folders.
