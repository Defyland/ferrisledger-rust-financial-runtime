# Deployment View

## Local

```text
cargo run -p ferrisledger-cli -- serve --store-path data/events.jsonl
```

One process exposes HTTP and writes a local event log.
Rate limiting is in-process and resets when the process restarts.

## Docker Compose

`docker-compose.yml` builds the Rust binary and mounts `./data` into the
container. Prometheus can scrape `/metrics`; a Grafana dashboard definition is
included under `ops/grafana`, and alert rules are under `ops/prometheus`.

## Production Evolution

The first production evolution should replace JSONL with PostgreSQL event
storage and move API keys into a secret manager. Kubernetes, service mesh, and
multi-region replication are intentionally deferred.
