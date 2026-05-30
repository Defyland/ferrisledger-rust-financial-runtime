# Abuse Cases

| Abuse case | Control | Test evidence |
| --- | --- | --- |
| Missing API key | `/v1` returns `401` | API auth test |
| API key request flood | Rolling-window limiter returns `429` | API rate-limit test |
| Cross-tenant account read | Stream ID includes tenant | BOLA-style API test |
| Duplicate deposit retry | Idempotency key returns original event | Runtime/API tests |
| Transfer above balance | Rule engine rejects before append | Domain/rules tests |
| Corrupt event log | Readiness/replay fail | Store corruption test |
| Raw pointer misuse | FFI isolated and tested | FFI tests |
