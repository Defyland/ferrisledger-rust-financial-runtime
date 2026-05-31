# API Documentation

The canonical OpenAPI contract is [`../../openapi.yaml`](../../openapi.yaml).

## Authentication

All `/v1` endpoints require:

```http
x-api-key: <local-api-key>
```

Health, readiness, and metrics are public in local mode. Production should
protect metrics and readiness at ingress.

Callers may send `x-request-id`; when it is absent the API generates one.
Command audit logs and HTTP responses include request and correlation context.

## Standard Error Format

```json
{
  "error": {
    "code": "insufficient_funds",
    "message": "insufficient funds: available 1000, requested 1001",
    "request_id": "req_001",
    "correlation_id": "corr_002"
  }
}
```

Success and error responses return `x-request-id`. Command responses also
return `x-correlation-id` from the command payload.

Stable error codes include `unauthorized`, `bad_request`,
`account_not_found`, `account_already_exists`, `invalid_money`,
`money_arithmetic_overflow`, `currency_mismatch`, `insufficient_funds`,
`idempotency_conflict`, `version_conflict`, and `event_log_corrupt`.

`rate_limited` is returned with HTTP `429` when an authenticated API key exceeds
the configured rolling-window request limit.

Repeatable command endpoints replay the original event only when the reused
idempotency key matches the same command semantics. Reusing a key with different
amount, currency, command type, account, tenant, settlement/ledger ID,
beneficiary, reason, or related event returns HTTP `409` with
`idempotency_conflict`.
