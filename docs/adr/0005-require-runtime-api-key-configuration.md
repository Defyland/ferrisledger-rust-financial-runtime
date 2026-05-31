# ADR 0005: Require Runtime API Key Configuration

## Status

Accepted.

## Context

FerrisLedger protects `/v1` endpoints with a static API key while the project is
still a local/internal financial runtime. A default key is useful for examples,
but baking it into the CLI or Docker image weakens the documented secret
management model and causes container scanners to flag the image.

## Options considered

1. Keep `dev-secret` as the CLI and container default
2. Keep a Docker image default but require an explicit CLI key
3. Require `--api-key` or `FERRISLEDGER_API_KEY` at runtime

## Decision

Require the API key to be supplied at runtime through `--api-key` or
`FERRISLEDGER_API_KEY`. Docker Compose requires `FERRISLEDGER_API_KEY` from the
caller environment, and the Docker image does not set a default API key.

## Consequences

Positive:

- The image no longer contains a default secret-like value.
- Local examples can still use a throwaway key explicitly.
- The runtime behavior matches the security documentation.

Negative:

- `ferrisledger serve` now fails fast unless a key is provided.
- Operators must wire a secret source before starting containerized deployments.
