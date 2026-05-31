# Domain Invariants

- Account IDs are scoped by tenant.
- Money must be positive for commands.
- Money arithmetic must reject minor-unit overflow or underflow.
- Currency must be a 3-letter uppercase ISO code.
- Deposits require an open account.
- Pix reservations require an open account and cannot exceed available balance.
- Settlements cannot exceed pending Pix reservations.
- Idempotent command retries must return the original event only when the key
  is reused with the same command semantics.
- Idempotency keys must not be reused across different accounts, tenants,
  command types, amounts, currencies, beneficiaries, settlement IDs, ledger
  IDs, reasons, or related events.
- Replay must fail on corrupt or semantically invalid event streams.
