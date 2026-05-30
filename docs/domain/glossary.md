# Domain Glossary

| Term | Meaning |
| --- | --- |
| Tenant | Partition that owns accounts and event streams. |
| Account | Financial balance holder inside one tenant. |
| Stream | Ordered append-only event sequence for one tenant/account. |
| Event envelope | Operational metadata plus typed financial payload. |
| Idempotency key | Client key that makes command retries return the same event. |
| Available balance | Booked balance minus pending Pix reservations. |
| Pix reservation | Outgoing transfer request that reserves funds before settlement. |
| Settlement | Finalization that releases reservation and debits balance. |
| Ledger entry | Accounting evidence tied to a domain event or business reason. |
