# Data and Consistency

## Transaction boundary

The current transaction boundary is one account stream. A command reads the
stream, rebuilds account state, validates the business rule, and appends one
event with the expected stream version.

## Constraints

- `event_id` must be unique in the event log.
- `stream_id` is derived from `tenant_id + account_id`.
- Monetary command amounts must be positive.
- Currency must be a 3-letter uppercase code.
- Settlement amount cannot exceed pending Pix reservations.

## Indexes

The JSONL MVP has no physical indexes. Reads scan the file and filter by
stream ID. This is acceptable for the MVP because it keeps append/replay
behavior visible, but a PostgreSQL adapter should add:

- unique index on `event_id`
- index on `(tenant_id, stream_id, stream_version)`
- index on `(tenant_id, account_id)`
- index on idempotency key per stream

## Foreign keys

There are no database foreign keys in JSONL. The equivalent consistency is
enforced by stream construction and replay rules. A PostgreSQL adapter should
store tenants/accounts/events in tables with explicit foreign keys.

## Optimistic locking

`FileEventStore::append` accepts an expected stream version. If the persisted
stream version differs, append fails with `VersionConflict`.

## Isolation assumptions

The JSONL store is safe for a single process and uses an in-process mutex for
append coordination. It is not a multi-process or multi-replica write store.

## Rollback strategy

Persisted events are immutable. Business rollback should append compensating
events in a future command model. Operational rollback for corrupted local data
means restoring the event log from backup and re-running `ferrisledger verify`.

## Migration strategy

The next persistence milestone is a PostgreSQL `EventStore` implementation
behind the existing trait. The API and rules crates should not change for that
migration.
