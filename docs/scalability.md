# Scalability

## Hot path

Command execution reads one account stream, replays it, validates a command,
and appends one record with fsync.

## Read-heavy operations

- `GET /v1/accounts/{account_id}/events`
- `GET /v1/accounts/{account_id}/snapshot`
- Projection worker rebuilds

## Write-heavy operations

- Deposits
- Pix transfer reservations
- Settlements
- Ledger entries

## Fastest-growing data

The event log grows fastest. A busy account stream also grows replay cost until
snapshotting or segment compaction exists.

## First bottleneck

Single-file append and full-stream replay are the first limits. This is
acceptable for the MVP because the implementation makes replay and corruption
handling visible.

## Hot partition risk

One tenant or account can become hot because stream IDs are scoped by
`tenant_id + account_id`.

The local rate limiter is keyed by API key. In a multi-replica deployment this
becomes inconsistent unless moved to a shared store such as Redis.

## Horizontal scaling path

Read-only API replicas can serve from a shared database-backed store in a later
phase. JSONL is single-writer and should not be shared by multiple writers.

## Sharding path

Partition by tenant ID first, then by account stream if a tenant is large.

## Asynchronous candidates

Projection rebuilds, outbox publishing, audit exports, and benchmark reporting
can be asynchronous. Balance-affecting command validation must stay
synchronous within an account stream.

## Must never be eventual

Available-balance validation for Pix reservation and settlement must be based
on current stream state before append.
