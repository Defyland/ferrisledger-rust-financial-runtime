# ADR 0004: Isolate Unsafe and FFI

## Status

Accepted.

## Context

The project needs to demonstrate Rust unsafe/FFI judgment without allowing raw
pointers to leak into financial rule or storage code.

## Options considered

1. No FFI in the repository
2. Inline unsafe in the store crate
3. Dedicated FFI crate with safe wrapper and tests

## Decision

Keep unsafe code only in `ferrisledger-ffi`. The runtime does not use it by
default.

## Consequences

Positive:

- Unsafe boundary is easy to review.
- Tests cover the safe wrapper and C ABI function.
- Future checksum experiments have a controlled location.

Negative:

- The FFI crate is currently demonstrative rather than performance-critical.
