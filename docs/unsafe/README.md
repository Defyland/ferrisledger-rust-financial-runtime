# Unsafe Boundaries

FerrisLedger will keep unsafe Rust isolated and documented.

## Phase 0 policy

- Unsafe code is not implemented yet.
- Any future unsafe block must live in a narrow module such as storage I/O or FFI.
- Every unsafe block must have a `SAFETY:` comment explaining pointer validity, alignment, ownership, and lifetime assumptions.
- Every unsafe abstraction must be wrapped by a safe public API and backed by focused tests.
