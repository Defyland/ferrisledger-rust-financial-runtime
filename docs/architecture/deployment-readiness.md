# Deployment Readiness

## Current posture

FerrisLedger now has a runnable HTTP API, CLI, append-only store, worker crate,
metrics, tests, Dockerfile, Docker Compose file, and CI workflow.

## Ready for local operation

- `cargo run -p ferrisledger-cli --bin ferrisledger -- serve --api-key dev-secret`
- `FERRISLEDGER_API_KEY=dev-secret docker compose up --build`
- `/healthz`, `/readyz`, `/metrics`
- `ferrisledger verify --store-path data/events.jsonl`

## Not production-ready yet

- JSONL is local-file based and safe only for same-host writers that honor OS
  file locks.
- API keys are static.
- Metrics/readiness are public in local mode.
- Backups are external to the runtime.

## Deferred platform work

- Kubernetes manifests.
- PostgreSQL-backed event store.
- OpenTelemetry exporter.
- Segment rotation and snapshot compaction.
- Secret-manager integration.
