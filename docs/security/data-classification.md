# Data Classification

| Data | Classification | Handling |
| --- | --- | --- |
| Tenant ID | Internal | Safe in logs |
| Account ID | Confidential | Log only when needed for diagnosis |
| Event ID | Internal | Safe in logs |
| Correlation ID | Internal | Safe in logs |
| Amount/currency | Confidential | Returned only to authenticated callers |
| Pix key | Sensitive | Do not add to structured logs |
| API key | Secret | Environment variable only |
