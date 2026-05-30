# Module Boundaries

| Crate | Responsibility | Must not own |
| --- | --- | --- |
| `ferrisledger-domain` | IDs, money, account state | HTTP, files, async |
| `ferrisledger-events` | Event contracts and envelopes | Business decisions |
| `ferrisledger-rules` | Command validation and replay | Persistence details |
| `ferrisledger-store` | JSONL append/read/verify | Account invariants |
| `ferrisledger-runtime` | Use-case orchestration | HTTP response mapping |
| `ferrisledger-api` | Axum routing, auth, rate limiting, errors | Financial rules |
| `ferrisledger-cli` | Operator commands | Financial rules |
| `ferrisledger-worker` | Async projection rebuild | Store implementation |
| `ferrisledger-telemetry` | Logs and metrics | Domain decisions |
| `ferrisledger-ffi` | Unsafe C ABI checksum boundary | Runtime defaults |
| `ferrisledger-macros` | Local boilerplate macros | Runtime behavior |
