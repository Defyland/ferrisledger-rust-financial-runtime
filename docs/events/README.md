# FerrisLedger Event Contracts

FerrisLedger stores financial events as append-only runtime facts. Event envelopes are versioned because replay, snapshots, rules, and external consumers depend on stable event shape.

## Envelope

Every event must include:

- `event_id`
- `event_type`
- `stream_id`
- `correlation_id`
- `causation_id`
- `schema_version`
- `occurred_at`
- `producer`
- `payload`

## Compatibility policy

- Consumers must deduplicate by `event_id` and tolerate at-least-once delivery.
- Replays must preserve the original `event_id`, `stream_id`, and causal metadata.
- New fields must be optional until all readers can tolerate them.
- Required fields must not change semantics without a new schema version.
- Event payload decoding errors must never silently skip records in a stream.
- Snapshot reconstruction must remain compatible with historical envelopes.

## Producer and consumer expectations

- Producers append events only after domain validation and checksum generation succeed.
- Rebuild, replay, and snapshot jobs must propagate the original `correlation_id` for traceability.
- Consumers must treat events as immutable facts and record replay or projection state separately.
- Unsafe or FFI-backed optimizations must not change the logical event contract seen by readers.

## Versioned schemas

- [account_opened.v1.json](account_opened.v1.json)
- [money_deposited.v1.json](money_deposited.v1.json)
- [pix_transfer_requested.v1.json](pix_transfer_requested.v1.json)
- [ledger_entry_created.v1.json](ledger_entry_created.v1.json)
- [settlement_executed.v1.json](settlement_executed.v1.json)
