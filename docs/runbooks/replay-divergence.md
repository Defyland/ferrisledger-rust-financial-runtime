# Runbook: Replay Divergence

## Symptoms

- Snapshot balance differs from a downstream projection.
- Projection worker rebuilds a different account count than expected.

## Checks

1. Run `ferrisledger verify --store-path data/events.jsonl`.
2. Replay the affected account with the CLI.
3. Inspect all events with the same `stream_id`.
4. Check for duplicate external idempotency keys in upstream systems.

## Recovery

If the store verifies, rebuild downstream projections from the append-only log.
If verification fails, follow `event-log-corruption.md`.
