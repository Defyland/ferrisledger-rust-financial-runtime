# Engineering Case Study

## 1. Product Context

FerrisLedger addresses a financial-platform problem: teams need a verifiable
history of account movements and operational evidence that replayed balances
match persisted facts. The product is intentionally scoped to internal
platform clients and operators rather than end-user banking screens.

## 2. Domain Model

The model centers on tenant, account, money, stream, event envelope,
idempotency key, Pix reservation, settlement, and ledger entry. Account state is
not edited directly; it is rebuilt by applying immutable events in append order.

## 3. Architecture

The system is a modular monolith built as a Cargo workspace. Domain and rule
crates are independent of API and storage. The runtime crate coordinates use
cases and adapters. This gives clear boundaries without the operational cost of
early microservices.

## 4. Key Trade-offs

JSONL storage keeps the MVP inspectable and makes corruption tests simple, but
it is not a long-term multi-writer database. The local store now uses advisory
OS file locks to coordinate same-host processes, which improves the MVP without
claiming distributed locking. CRC32 detects accidental corruption but does not
prove malicious tamper resistance. API-key auth is enough for local/internal
demonstration, while production would need scoped keys or OIDC.

## 5. Data Model

Every stored record is one JSON line with a checksum and an event envelope. The
envelope includes event ID, type, stream ID, tenant ID, correlation ID,
causation ID, schema version, timestamp, producer, and typed payload.

## 6. Consistency Model

Commands read the current stream, rebuild state, validate rules, then append
with an expected stream version. This optimistic check prevents blind writes
against a stale stream. Money movement is strongly consistent within one
account stream. Repeatable commands use globally unique idempotency keys:
identical retries replay the original event, while incompatible reuse returns a
conflict before appending financial facts.

## 7. Failure Scenarios

- Corrupt JSON or checksum mismatch blocks readiness and replay.
- Duplicate event IDs are rejected before append.
- Repeated idempotent commands return the original event only when command
  semantics match; incompatible key reuse returns conflict.
- Monetary overflow and underflow return domain errors instead of wrapping.
- Insufficient funds are rejected before persistence.
- Tenant/account mismatches fail projection.

## 8. Performance Strategy

The hot path is read stream, replay account, validate command, append one line,
and fsync. Criterion measures replay cost for 100 deposit events. k6 scripts
exercise smoke, load, stress, and spike API profiles. The latest load baseline
records latency, throughput, error rate, server CPU, and server RSS.

## 9. Scalability Strategy

The first limit is a single growing JSONL file. The next scale step is segment
rotation or a PostgreSQL event store. Account streams are independent, so future
sharding can partition by tenant ID or account stream.

## 10. Security Model

The MVP protects `/v1` endpoints with `x-api-key`, isolates stream IDs by
tenant, validates configured API keys at startup, compares keys without
data-dependent early exit, rate-limits authenticated callers per API key,
throttles repeated invalid-auth attempts through a separate local bucket,
validates identifiers, avoids logging sensitive payloads, and documents secrets
through environment variables. The API key must be provided at runtime; it is
not baked into the CLI default or Docker image. Tenant-isolation, rate-limit,
auth-failure, and weak-key behavior are covered by API/CLI tests, and
idempotency-confusion abuse is covered by runtime and API conflict tests.

## 11. Observability

Health, readiness, Prometheus metrics, alert rules, a Grafana dashboard, and
structured JSON audit logs are first-class runtime surfaces. Readiness verifies
the event store, so corruption becomes visible before replay produces
misleading output.

## 12. Operational Cost

The local MVP needs only one process and one mounted data directory. That keeps
debugging and deployment cheap. The accepted cost is limited concurrency and no
external backup story beyond file snapshots.

## 13. Maintainability

Crate boundaries map to ownership: domain IDs and money, event contracts,
business rules, store, runtime service, API adapter, CLI adapter, worker,
telemetry, FFI, and test support. New financial rules should start in
`ferrisledger-rules`, not in HTTP handlers.

## 14. Product Decisions

The product intentionally prioritizes auditability, idempotency, and replay
over broad banking features. It does not model customers, KYC, card rails,
interest, tax, or external Pix provider integrations.

## 15. What I Would Do Next

1. Add a PostgreSQL event-store adapter with transactional outbox.
2. Add snapshot compaction and segment rotation.
3. Replace static API keys with scoped keys or OIDC.
4. Add OpenTelemetry trace export.
5. Add fuzzing for malformed event-log records.
