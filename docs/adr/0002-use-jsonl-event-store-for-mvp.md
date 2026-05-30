# ADR 0002: Use JSONL Event Store for the MVP

## Status

Accepted.

## Context

FerrisLedger needs to demonstrate append-only storage, checksums, replay, and
corruption detection without hiding those behaviors behind a database.

## Options considered

1. PostgreSQL event table
2. SQLite event table
3. JSONL event log with checksum per line

## Decision

Use JSONL for the MVP. Each line stores a checksum and event envelope.

## Consequences

Positive:

- Easy to inspect and corrupt deliberately in tests.
- Small local operational footprint.
- Clear educational value for append-only storage.

Negative:

- Single-writer semantics.
- No native indexing or backup policy.
- Segment rotation and compaction are future work.
