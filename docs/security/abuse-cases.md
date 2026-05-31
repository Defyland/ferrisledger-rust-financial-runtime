# Abuse Cases

| Abuse case | Control | Test evidence |
| --- | --- | --- |
| Missing API key | `/v1` returns `401` | API auth test |
| Weak configured API key | Startup rejects short, whitespace, or non-visible API-key values | API configuration test |
| Repeated invalid API keys | Separate rolling-window limiter returns `429` after repeated auth failures | API auth-failure rate-limit test |
| API key request flood | Rolling-window limiter returns `429` | API rate-limit test |
| Cross-tenant account read | Stream ID includes tenant | BOLA-style API test |
| Duplicate deposit retry | Idempotency key returns original event | Runtime/API tests |
| Idempotency key reused with different payload | Runtime compares command semantics and returns `409` | Runtime/API conflict tests |
| Transfer above balance | Rule engine rejects before append | Domain/rules tests |
| Money arithmetic overflow | Checked arithmetic returns domain error | Domain overflow tests |
| Corrupt event log | Readiness/replay fail | Store corruption test |
| Raw pointer misuse | FFI isolated and tested | FFI tests |
