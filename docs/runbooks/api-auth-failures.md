# Runbook: API Authentication Failures

## Symptoms

- Clients receive `401 unauthorized`.
- Runtime command counters do not increase.

## Checks

1. Confirm the caller sends `x-api-key`.
2. Confirm the process environment has the expected `FERRISLEDGER_API_KEY`.
3. Confirm the request path starts with `/v1`; health/metrics do not require
   the key in local mode.

## Recovery

Rotate the local key by restarting the process with the intended environment
value. For production, rotate through the deployment secret manager.
