# Bounded Contexts

## Financial Runtime

Owns account streams, events, replay, and command validation.

## Storage

Owns append-only persistence, checksum verification, optimistic stream version
checks, and corruption signaling.

## Operations

Owns health, readiness, metrics, logs, runbooks, and benchmark evidence.

## Deferred Contexts

Customer onboarding, Pix provider integration, billing, and external reporting
are intentionally outside this MVP.
