# Tech Lead Review

## Current Assessment

FerrisLedger now reads as a cohesive financial runtime rather than a Rust
syntax exercise. The strongest evidence is the alignment between domain nouns,
crate boundaries, event contracts, API behavior, corruption handling,
observability assets, tests, benchmarks, and runbooks.

## Issues Found In This Continuation Pass

| Issue | Action |
| --- | --- |
| Money arithmetic used direct `i64` operators despite `checked_*` names | Added checked arithmetic, explicit overflow error, API mapping, and overflow tests. |
| Idempotency replay ignored divergent command payloads | Added semantic comparison before replay and `409 idempotency_conflict` on mismatch. |
| JSONL coordination relied on in-process mutex only | Added OS file locks for local same-host append/read/verify and a concurrent independent-handle test. |
| OpenAPI modeled event payload as generic `object` | Added concrete `oneOf` schemas for all event payload variants. |
| Local coverage was marked blocked | Ran coverage with Homebrew LLVM via `LLVM_COV` and `LLVM_PROFDATA`. |
| CPU/memory benchmark evidence was missing | Recorded a k6 load baseline with server CPU/RSS samples. |
| Spec evidence lagged the actual review blockers | Added a hardening spec and refreshed the senior-readiness matrix. |

## What Is Strong Now

- Domain model and tests express financial language, not generic CRUD.
- Append-only storage has explicit corruption detection and runbooks.
- API has versioning, auth, idempotency, standardized errors, OpenAPI, and
  failure tests.
- Money arithmetic rejects overflow instead of wrapping.
- Observability includes domain/security metrics, readiness verification, and
  contextual audit logs.
- Documentation explains why the design is intentionally a modular monolith and
  why JSONL is an MVP store.
- JSONL now has local file-lock coordination, while remaining honest about the
  PostgreSQL requirement before multi-replica production writes.
- Benchmarks now include load latency, throughput, error rate, server CPU, and
  server RSS evidence.
- Local coverage is executable without `rustup` by pointing to Homebrew LLVM
  tools.
- Secret handling avoids baked-in API keys for the CLI and Docker image.

## Best Next Moves To Impress

1. Add a PostgreSQL `EventStore` adapter with transactional append, unique
   event IDs, stream-version constraint, and migration docs.
2. Add an outbox table and worker for at-least-once event publication with
   idempotent consumers and dead-letter handling.
3. Add OTLP trace export and a local OpenTelemetry Collector profile.
4. Replace static API keys with scoped keys or OIDC and test role/scope
   authorization.
5. Add snapshot compaction and segment-retention policy for long streams.
6. Add tamper-evident event signatures if positioning the project for regulated
   finance/audit work.
7. Capture stress/spike benchmark result files with CPU and memory notes.

## Interview Narrative

The strongest way to present the project is:

> FerrisLedger is intentionally not a banking toy API. It is a financial event
> runtime where the interesting decisions are the consistency boundary,
> append-only durability, replay safety, idempotency, tenant isolation,
> observability, and operational cost. I chose a modular monolith and JSONL to
> make those mechanics inspectable first, then documented the migration path to
> PostgreSQL/outbox/OTLP/OIDC once the MVP proves the domain.
