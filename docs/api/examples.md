# API Examples

## Open Account

```bash
curl -s http://localhost:8080/v1/accounts \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret-local' \
  -d '{
    "tenant_id": "tenant_001",
    "account_id": "account_001",
    "currency": "BRL",
    "account_holder_name": "Ada Lovelace",
    "correlation_id": "corr_001"
  }'
```

## Deposit

```bash
curl -s http://localhost:8080/v1/accounts/account_001/deposits \
  -H 'content-type: application/json' \
  -H 'x-api-key: dev-secret-local' \
  -d '{
    "tenant_id": "tenant_001",
    "amount_cents": 2500,
    "currency": "BRL",
    "idempotency_key": "deposit_001",
    "correlation_id": "corr_002"
  }'
```

## Replay Snapshot

```bash
curl -s 'http://localhost:8080/v1/accounts/account_001/snapshot?tenant_id=tenant_001' \
  -H 'x-api-key: dev-secret-local'
```
