# ADR 0006: Use Local File Locks for the JSONL Store

## Status

Accepted.

## Context

The JSONL event store is intentionally an MVP persistence adapter, but the
previous implementation coordinated appends only with an in-process mutex. That
left a real review gap: independent processes on the same host could write the
same file without a shared lock even though the docs described append and verify
semantics as reliable local behavior.

## Options considered

1. Keep only the in-process mutex and document the limitation
2. Replace JSONL immediately with PostgreSQL
3. Keep JSONL but add operating-system file locks for local same-host writers

## Decision

Keep JSONL for the local MVP and add OS file locks around append/read/verify.
The in-process mutex remains as cheap coordination for cloned store handles in
one process; the file lock is the cross-process guard for one host.

## Consequences

Positive:

- Local append and verification semantics are stronger without adding external
  infrastructure.
- The store remains inspectable and easy to run in CI.
- The production migration path to PostgreSQL stays behind the `EventStore`
  trait.

Negative:

- File locks are not a distributed lock and do not make JSONL multi-host safe.
- Performance remains bounded by full-file reads and `fsync` on append.
- PostgreSQL is still required before multi-replica production writes.
