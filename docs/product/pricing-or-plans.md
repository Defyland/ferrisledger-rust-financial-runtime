# Pricing or Plans

FerrisLedger is an internal platform component, not a SaaS product. A realistic
commercial packaging would be:

| Plan | Audience | Runtime constraints |
| --- | --- | --- |
| Local | Developers | JSONL store, single process |
| Team | Internal platform teams | PostgreSQL event store, shared metrics |
| Regulated | Financial operations | Signed records, audit exports, stricter auth |
