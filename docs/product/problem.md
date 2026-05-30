# Product Problem

Financial platforms need auditable account movement. A mutable balance column
does not explain which command changed the balance, whether a retry duplicated
work, or whether replay still matches the stored history.

FerrisLedger solves this by storing typed financial events in append order,
validating commands before persistence, and rebuilding account state
deterministically.
