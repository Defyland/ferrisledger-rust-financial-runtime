# Authorization Matrix

| Endpoint | Auth required | Scope checked | Notes |
| --- | --- | --- | --- |
| `GET /healthz` | No | None | Local liveness only |
| `GET /readyz` | No | None | Production ingress should restrict |
| `GET /metrics` | No | None | Production ingress should restrict |
| `POST /v1/accounts` | Yes | API key + rate limit | Opens tenant-scoped account |
| `POST /v1/accounts/{id}/deposits` | Yes | API key + rate limit + tenant stream | Idempotency required |
| `POST /v1/accounts/{id}/pix-transfers` | Yes | API key + rate limit + tenant stream | Enforces available balance |
| `POST /v1/accounts/{id}/settlements` | Yes | API key + rate limit + tenant stream | Enforces pending reservation |
| `POST /v1/accounts/{id}/ledger-entries` | Yes | API key + rate limit + tenant stream | Accounting evidence |
| `GET /v1/accounts/{id}/events` | Yes | API key + rate limit + tenant stream | Tenant ID in query |
| `GET /v1/accounts/{id}/snapshot` | Yes | API key + rate limit + tenant stream | Tenant ID in query |
