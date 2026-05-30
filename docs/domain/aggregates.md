# Aggregates

## Account Stream

Aggregate root: `tenant_id + account_id`.

Events:

- `account_opened`
- `money_deposited`
- `pix_transfer_requested`
- `settlement_executed`
- `ledger_entry_created`

Consistency boundary: one account stream. Cross-account transfer atomicity is
deferred until a database-backed store or saga/outbox layer exists.
