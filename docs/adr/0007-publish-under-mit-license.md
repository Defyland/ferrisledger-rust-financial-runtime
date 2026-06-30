# ADR 0007: Publish the Repository Under the MIT License

## Status

Accepted.

## Context

FerrisLedger is already a public systems-learning repository with explicit
architecture notes, replay benchmarks, security docs, and product guidance.
Without an explicit license, that public teaching surface remains readable but
the legal reuse boundary is still ambiguous for engineers who want to study or
adapt the runtime.

## Options considered

1. Keep the default all-rights-reserved posture
2. Publish under the MIT License
3. Delay licensing until the PostgreSQL event-store path lands

## Decision

Publish the repository under the MIT License and surface that decision in the
README.

## Consequences

Positive:

- Reviewers and learners can fork the runtime, docs, and benchmarks with a
  clear reuse boundary.
- The public portfolio signal now matches the repo's existing documentation
  depth.

Negative:

- Downstream users may reuse the runtime without carrying the same operational
  caveats forward.
- Third-party dependency/license hygiene still has to be maintained separately.
