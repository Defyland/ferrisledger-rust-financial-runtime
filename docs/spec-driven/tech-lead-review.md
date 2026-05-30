# Tech Lead Review

## What Was Below Senior Bar

Before the hardening pass, the project had a strong shape but still had gaps
that a senior reviewer would notice:

- Spec-driven evidence was missing even though the shared standard requires it.
- Rate limiting was a documented security expectation but not a provable API
  behavior.
- Request/correlation context existed conceptually, but was not propagated
  consistently through HTTP success/error responses.
- Observability leaned on generic HTTP metrics and logs; security/domain
  signals such as rate-limit rejections were not visible enough.
- Commit history did not yet tell an atomic implementation story because the
  workspace changes were still uncommitted.
- Docker build could not be locally proven because the Docker daemon was not
  running.

## Improvements Made

| Improvement | Why it matters in a senior evaluation |
| --- | --- |
| Added `docs/spec-driven/` acceptance criteria, implementation plan, and verification report | Shows deliberate scope, traceability, and evidence instead of ad hoc coding. |
| Added in-process API-key rate limiting with `429` and test coverage | Converts a security claim into executable behavior and verifies abuse resistance. |
| Added `ferrisledger_api_rate_limited_total` metric and dashboard panel | Gives operators a signal for abuse or misconfigured clients. |
| Propagated request/correlation context in logs, headers, and error bodies | Makes incidents diagnosable across clients, API, runtime, and persisted events. |
| Updated OpenAPI to document `429`, context headers, and valid OAS 3.1 semantics | Keeps implementation and public contract aligned. |
| Extended k6 smoke checks to validate context headers and p99 reporting | Measures more than status codes and proves the operational contract under HTTP. |
| Recorded verification commands and residual risks honestly | Avoids fake production-readiness claims and names the next engineering moves. |

## What Is Strong Now

- Domain model and tests express financial language, not generic CRUD.
- Append-only storage has explicit corruption detection and runbooks.
- API has versioning, auth, idempotency, standardized errors, OpenAPI, and
  failure tests.
- Observability includes domain/security metrics, readiness verification, and
  contextual audit logs.
- Documentation explains why the design is intentionally a modular monolith and
  why JSONL is an MVP store.

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
7. Run Docker build and k6 load/stress/spike on a machine with Docker daemon
   enabled, then record those results under `benchmarks/results/`.

## Interview Narrative

The strongest way to present the project is:

> FerrisLedger is intentionally not a banking toy API. It is a financial event
> runtime where the interesting decisions are the consistency boundary,
> append-only durability, replay safety, idempotency, tenant isolation,
> observability, and operational cost. I chose a modular monolith and JSONL to
> make those mechanics inspectable first, then documented the migration path to
> PostgreSQL/outbox/OTLP/OIDC once the MVP proves the domain.
