# Operational Cost

## Infrastructure components

Current local deployment:

- One Rust binary
- One JSONL event-log file
- Optional Docker container
- Prometheus scrape target
- In-process rate-limit state

## Non-financial operational cost

The MVP is easy to debug because all state is local and inspectable. The cost
is limited durability, no built-in multi-writer coordination, and rate limiting
that resets on process restart.

## Debugging complexity

Most incidents start with `/readyz`, then `ferrisledger verify`, then replaying
the affected stream. JSONL makes individual records inspectable.

## Deployment complexity

`cargo run` and Docker Compose are both supported. No database migrations are
required in the MVP.

## Backup and retention

The event log should be backed up as an immutable artifact. Future segment
rotation should set retention by tenant and compliance requirement.

## Monitoring requirements

- HTTP request rate and latency
- Runtime command accepted/rejected counts
- API rate-limited request count
- Event store record count
- Readiness failures

## Vendor lock-in risk

The MVP has minimal lock-in. A future PostgreSQL adapter should keep the
`EventStore` trait boundary stable.

## Simpler alternatives rejected

A mutable JSON account file would be simpler, but it would not demonstrate
auditability, replay, idempotency, or corruption handling.
