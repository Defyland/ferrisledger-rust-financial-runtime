# Secrets

## Local

`FERRISLEDGER_API_KEY` controls the expected `x-api-key` value.
`FERRISLEDGER_RATE_LIMIT_PER_MINUTE` controls the in-process API-key request
limit for local deployments.
`FERRISLEDGER_AUTH_FAILURE_RATE_LIMIT_PER_MINUTE` controls the separate
in-process limit for missing or invalid API-key attempts.

```bash
FERRISLEDGER_API_KEY=dev-secret-local \
FERRISLEDGER_RATE_LIMIT_PER_MINUTE=120 \
FERRISLEDGER_AUTH_FAILURE_RATE_LIMIT_PER_MINUTE=60 \
cargo run -p ferrisledger-cli -- serve
```

## Docker Compose

`docker-compose.yml` requires `FERRISLEDGER_API_KEY` from the caller's
environment and does not bake a default key into the image or Compose file.
Use a throwaway value such as `dev-secret-local` only for local smoke tests.

## Rules

- API keys shorter than 12 visible ASCII characters, keys longer than 256
  characters, and keys containing whitespace are rejected at startup.
- Never commit real API keys.
- Never log API key values.
- Rotate keys by deploying a new environment value and restarting the process.
- Prefer scoped keys or OIDC before exposing the runtime beyond trusted
  internal networks.
