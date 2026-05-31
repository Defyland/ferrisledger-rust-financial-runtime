# Observability

## Logs

The API emits structured JSON logs through `tracing`. Command audit logs include:

- event ID
- stream ID
- correlation ID
- request ID when the caller supplies `x-request-id`
- idempotent replay flag
- accepted/rejected outcome

Request payload values such as Pix keys are not logged.

## Metrics

Prometheus metrics:

- `ferrisledger_http_requests_total`
- `ferrisledger_http_request_duration_seconds`
- `ferrisledger_runtime_commands_total`
- `ferrisledger_api_rate_limited_total`
- `ferrisledger_event_store_records`

## Health and readiness

- `/healthz` proves the process is alive.
- `/readyz` verifies the append-only event store and updates the event-record
  gauge.

## Dashboard and alerts

Grafana dashboard: `ops/grafana/ferrisledger-dashboard.json`.

Prometheus alert rules: `ops/prometheus/ferrisledger-alerts.yml`.

## Trace status

The runtime has structured JSON trace/log context with request ID, correlation
ID, event ID, stream ID, command result, and idempotent replay status. OTLP
collector export is intentionally deferred because this local MVP does not run
an external collector; the next production-shaped observability step is to add
`tracing-opentelemetry` and an OpenTelemetry Collector profile.
