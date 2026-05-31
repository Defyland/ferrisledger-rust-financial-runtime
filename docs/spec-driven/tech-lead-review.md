# Tech Lead Review

## Current Assessment

FerrisLedger now reads as a cohesive financial runtime rather than a Rust
syntax exercise. The strongest evidence is the alignment between domain nouns,
crate boundaries, event contracts, API behavior, corruption handling,
observability assets, tests, benchmarks, and runbooks.

## Issues Found In This Continuation Pass

| Issue | Action |
| --- | --- |
| Dockerfile embedded `FERRISLEDGER_API_KEY=dev-secret` | Removed the image-level default key and require runtime configuration. |
| `serve` accepted an implicit default API key | Changed the CLI so `--api-key` or `FERRISLEDGER_API_KEY` is required. |
| Compose stored the local key directly | Switched Compose to caller-provided `FERRISLEDGER_API_KEY`. |
| Verification report still marked Docker build blocked | Re-ran Docker locally and refreshed the evidence. |

## What Is Strong Now

- Domain model and tests express financial language, not generic CRUD.
- Append-only storage has explicit corruption detection and runbooks.
- API has versioning, auth, idempotency, standardized errors, OpenAPI, and
  failure tests.
- Observability includes domain/security metrics, readiness verification, and
  contextual audit logs.
- Documentation explains why the design is intentionally a modular monolith and
  why JSONL is an MVP store.
- Secret handling now avoids baked-in API keys for the CLI and Docker image.

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
7. Capture load/stress/spike benchmark result files with CPU and memory notes.

## Interview Narrative

The strongest way to present the project is:

> FerrisLedger is intentionally not a banking toy API. It is a financial event
> runtime where the interesting decisions are the consistency boundary,
> append-only durability, replay safety, idempotency, tenant isolation,
> observability, and operational cost. I chose a modular monolith and JSONL to
> make those mechanics inspectable first, then documented the migration path to
> PostgreSQL/outbox/OTLP/OIDC once the MVP proves the domain.
