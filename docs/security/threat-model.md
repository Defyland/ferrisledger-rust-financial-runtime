# Threat Model

## Assets

- Append-only event log segments.
- Event envelopes and schema-versioned payloads.
- Replayed account snapshots and in-memory projections.
- API keys and operator CLI actions.
- Unsafe and FFI checksum boundary.
- Prometheus metrics and structured logs.

## Actors

- Internal platform clients calling the HTTP API.
- Platform operators running CLI maintenance, replay, and verification.
- Worker processes rebuilding indexes from persisted event streams.
- Developers experimenting with storage and FFI optimizations.
- Malicious or buggy callers retrying, replaying, or crossing tenant boundaries.

## Trust boundaries

- HTTP clients cross into the runtime through `/v1` endpoints.
- CLI users cross from local shell into the event store.
- The store crosses the file-system durability boundary.
- FFI crosses Rust memory-safety guarantees.
- Metrics and readiness cross into operational monitoring.

## Abuse cases and controls

| Threat | Control |
| --- | --- |
| Missing or wrong API key | Reject `/v1` requests with `401` |
| API key abuse or accidental loops | In-process rolling-window rate limit returns `429` |
| Cross-tenant read/BOLA | Stream ID includes tenant ID and API tests cover tenant mismatch |
| Duplicate command retry | Idempotency key lookup returns original event |
| Corrupted event log | CRC32 checksums, JSON decoding, readiness verification |
| Replay divergence | Deterministic rules and projection tests |
| Unsafe memory misuse | FFI isolated to one crate with `SAFETY` comments and tests |
| Sensitive Pix key logging | Structured logs exclude request payload values |
| Stale stream append | Expected stream version check |

## Residual risks

- CRC32 is not cryptographic tamper proofing.
- Static API keys are not sufficient outside trusted internal networks.
- In-process rate limiting does not protect multiple replicas without a shared
  limiter such as Redis.
- JSONL is single-writer and not a production-grade shared database.
- Metrics/readiness are public in local mode and should be restricted at ingress
  in production.
