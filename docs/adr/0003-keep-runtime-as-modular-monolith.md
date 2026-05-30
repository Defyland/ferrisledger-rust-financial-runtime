# ADR 0003: Keep Runtime as a Modular Monolith

## Status

Accepted.

## Context

The product needs strong boundaries, but the MVP does not need distributed
deployment. Splitting API, replay, storage, and workers into separate services
would increase tracing, consistency, deploy, and debugging cost.

## Options considered

1. Single crate binary
2. Modular monolith Cargo workspace
3. Multiple services

## Decision

Use a Cargo workspace modular monolith.

## Consequences

Positive:

- Clear ownership boundaries with low deployment cost.
- Easier local testing and deterministic replay.
- Future extraction paths remain visible.

Negative:

- Scaling is process-level, not service-level.
- Crate boundaries must be maintained intentionally.
