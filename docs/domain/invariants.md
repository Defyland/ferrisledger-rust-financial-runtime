# Domain Invariants

- Account IDs are scoped by tenant.
- Money must be positive for commands.
- Currency must be a 3-letter uppercase ISO code.
- Deposits require an open account.
- Pix reservations require an open account and cannot exceed available balance.
- Settlements cannot exceed pending Pix reservations.
- Idempotent command retries must return the original event, not append again.
- Replay must fail on corrupt or semantically invalid event streams.
