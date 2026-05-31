# Secrets

## Local

`FERRISLEDGER_API_KEY` controls the expected `x-api-key` value.
`FERRISLEDGER_RATE_LIMIT_PER_MINUTE` controls the in-process API-key request
limit for local deployments.

```bash
FERRISLEDGER_API_KEY=dev-secret \
FERRISLEDGER_RATE_LIMIT_PER_MINUTE=120 \
cargo run -p ferrisledger-cli -- serve
```

## Docker Compose

`docker-compose.yml` requires `FERRISLEDGER_API_KEY` from the caller's
environment and does not bake a default key into the image or Compose file.
Use a throwaway value such as `dev-secret` only for local smoke tests.

## Rules

- Never commit real API keys.
- Never log API key values.
- Rotate keys by deploying a new environment value and restarting the process.
- Prefer scoped keys or OIDC before exposing the runtime beyond trusted
  internal networks.
