# FerrisLedger Event Contracts

FerrisLedger stores financial facts as immutable event envelopes. Replay,
readiness verification, projections, and future consumers all depend on stable
event shape.

## Envelope

Every event includes:

- `event_id`
- `event_type`
- `stream_id`
- `tenant_id`
- `correlation_id`
- `causation_id`
- `schema_version`
- `occurred_at`
- `producer`
- `payload`

`stream_id` is derived from `tenant_id + account_id`, making tenant partitioning
explicit in the event contract.

## Compatibility policy

- Consumers deduplicate by `event_id`.
- Producers use idempotency keys for repeatable external commands.
- Replays preserve original event IDs and causal metadata.
- New fields must be optional until all readers tolerate them.
- Required field semantics must not change without a new version.
- Event decoding or checksum errors must fail replay, not silently skip records.

## Producer responsibilities

- Validate domain rules before append.
- Compute and persist checksum with the envelope.
- Propagate `correlation_id`.
- Use expected stream version for optimistic concurrency.

## Consumer responsibilities

- Treat events as immutable facts.
- Track projection offsets separately.
- Expect duplicate delivery in future broker/outbox integrations.
- Send failed messages to a dead-letter process when asynchronous delivery is
  added.

## Versioned schemas

- [account_opened.v1.json](account_opened.v1.json)
- [money_deposited.v1.json](money_deposited.v1.json)
- [pix_transfer_requested.v1.json](pix_transfer_requested.v1.json)
- [ledger_entry_created.v1.json](ledger_entry_created.v1.json)
- [settlement_executed.v1.json](settlement_executed.v1.json)
