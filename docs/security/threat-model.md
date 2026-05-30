# Threat Model

## Assets

- append-only event log segments
- event envelopes and schema-versioned payloads
- snapshots and indexes
- CLI and API operator actions
- unsafe and FFI boundaries used for storage or checksum optimization

## Actors

- platform operators running CLI maintenance, replay, and verification commands
- API clients submitting append, read, and replay requests in later phases
- worker processes rebuilding indexes, snapshots, and projections
- local developers iterating on unsafe and FFI-backed performance experiments

## Trust boundaries

- HTTP clients and CLI users submit events and replay commands
- storage readers and writers cross the file-system boundary
- optional unsafe and FFI modules cross the compiler’s normal safety guarantees
- async workers rebuild indexes and snapshots from persisted event streams

## Primary threats

| Threat | Control |
| --- | --- |
| Corrupted event log | checksums, crash recovery, and replay verification |
| Duplicate event append | event ID and stream-level validation |
| Unsafe memory misuse | quarantine unsafe code, require `SAFETY` comments, and test wrappers |
| FFI contract mismatch | safe wrapper over `extern \"C\"` boundary and focused tests |
| Replay divergence | property tests, snapshot verification, and deterministic event ordering |
| Unauthorized mutation | authenticated API or CLI boundaries in later phases and audit logging of operator actions |

## Residual risks

- Unsafe and FFI are intentionally deferred to narrow modules and are not part of the phase 0 runtime.
- Event Store semantics are central to the product, so schema evolution and snapshot compatibility remain ongoing design risks.
- External metadata stores and distributed deployment are deferred behind feature flags.
