# State Machines

## Account

```mermaid
stateDiagram-v2
  [*] --> Open: account_opened
  Open --> Open: money_deposited
  Open --> Open: pix_transfer_requested
  Open --> Open: settlement_executed
  Open --> Open: ledger_entry_created
```

Frozen and closed states are modeled in code as future states, but no command
currently emits those transitions. That keeps the current workflow focused
while leaving lifecycle expansion explicit.

## Pix Reservation

```mermaid
stateDiagram-v2
  [*] --> Reserved: pix_transfer_requested
  Reserved --> Settled: settlement_executed
```
