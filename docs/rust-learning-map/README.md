# Rust Learning Map

FerrisLedger connects Rust language features to concrete runtime
responsibilities.

| Concept | Where it appears | Why it matters |
| --- | --- | --- |
| Ownership | Event envelopes and command payloads move across runtime boundaries | Prevents accidental shared mutation of financial facts |
| Newtypes | `TenantId`, `AccountId`, `EventId`, `IdempotencyKey` | Avoids mixing domain identifiers |
| Traits | `EventStore` | Lets storage adapters change without API/rules changes |
| Generics | `RuntimeService<S>` and `ProjectionWorker<S>` | Keeps services testable against any store implementation |
| Pattern matching | `RuntimeCommand` and `FinancialEvent` | Makes command/event handling exhaustive |
| Async | Axum handlers and worker loop | Handles API and projection workflows |
| Macros | `validated_string_id!` | Removes repetitive ID boilerplate without procedural macro cost |
| Unsafe | `ferrisledger-ffi` only | Demonstrates a reviewed boundary |
| Property tests | Money/reservation invariants | Checks financial rules across many amounts |
| Benchmarks | Criterion replay benchmark | Measures replay cost instead of claiming performance |
