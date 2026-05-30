# Unsafe Boundaries

FerrisLedger keeps unsafe Rust isolated in `crates/ferrisledger-ffi`.

## Implemented boundary

`ferrisledger_crc32` exposes a C ABI checksum function:

- The public safe wrapper is `checksum_bytes`.
- The raw pointer function has a `# Safety` contract.
- The unsafe operation is limited to `std::slice::from_raw_parts`.
- Tests prove the FFI function matches the safe wrapper and handles the
  null-empty case.

## Policy

- No unsafe code is allowed in domain, rules, store, runtime, API, CLI, or
  worker crates.
- Every unsafe block must have a `SAFETY:` comment.
- FFI must never change the logical event contract.
- Unsafe optimization is accepted only after benchmark evidence shows a real
  bottleneck.
