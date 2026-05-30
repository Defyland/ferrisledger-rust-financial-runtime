//! Structured logs and Prometheus metrics.

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};
use thiserror::Error;
use tracing_subscriber::{EnvFilter, fmt};

/// Telemetry setup failures.
#[derive(Debug, Error)]
pub enum TelemetryError {
    /// Prometheus metric registration failed.
    #[error("prometheus registration failed: {0}")]
    Prometheus(#[from] prometheus::Error),
    /// Prometheus text encoding failed.
    #[error("prometheus encoding failed: {0}")]
    Encoding(#[from] std::string::FromUtf8Error),
}

/// Runtime metrics exported to Prometheus.
#[derive(Clone)]
pub struct Telemetry {
    registry: Registry,
    http_requests: IntCounterVec,
    http_latency: HistogramVec,
    runtime_commands: IntCounterVec,
    rate_limited_requests: IntCounter,
    event_store_records: IntGauge,
}

impl Telemetry {
    /// Creates a registry and registers FerrisLedger metrics.
    pub fn new() -> Result<Self, TelemetryError> {
        let registry = Registry::new();
        let http_requests = IntCounterVec::new(
            Opts::new(
                "ferrisledger_http_requests_total",
                "Total HTTP requests handled by path, method, and status.",
            ),
            &["method", "path", "status"],
        )?;
        let http_latency = HistogramVec::new(
            HistogramOpts::new(
                "ferrisledger_http_request_duration_seconds",
                "HTTP request latency by method and path.",
            ),
            &["method", "path"],
        )?;
        let runtime_commands = IntCounterVec::new(
            Opts::new(
                "ferrisledger_runtime_commands_total",
                "Financial runtime commands by command type and result.",
            ),
            &["command", "result"],
        )?;
        let rate_limited_requests = IntCounter::new(
            "ferrisledger_api_rate_limited_total",
            "Requests rejected by the in-process API key rate limiter.",
        )?;
        let event_store_records = IntGauge::new(
            "ferrisledger_event_store_records",
            "Verified append-only event records.",
        )?;

        registry.register(Box::new(http_requests.clone()))?;
        registry.register(Box::new(http_latency.clone()))?;
        registry.register(Box::new(runtime_commands.clone()))?;
        registry.register(Box::new(rate_limited_requests.clone()))?;
        registry.register(Box::new(event_store_records.clone()))?;

        Ok(Self {
            registry,
            http_requests,
            http_latency,
            runtime_commands,
            rate_limited_requests,
            event_store_records,
        })
    }

    /// Records an HTTP request.
    pub fn observe_http(&self, method: &str, path: &str, status: u16, elapsed_seconds: f64) {
        let status = status.to_string();
        self.http_requests
            .with_label_values(&[method, path, status.as_str()])
            .inc();
        self.http_latency
            .with_label_values(&[method, path])
            .observe(elapsed_seconds);
    }

    /// Records a runtime command.
    pub fn observe_command(&self, command: &str, result: &str) {
        self.runtime_commands
            .with_label_values(&[command, result])
            .inc();
    }

    /// Records a request rejected by rate limiting.
    pub fn observe_rate_limited(&self) {
        self.rate_limited_requests.inc();
    }

    /// Updates the verified event record gauge.
    pub fn set_event_store_records(&self, records: i64) {
        self.event_store_records.set(records);
    }

    /// Encodes all gathered metrics as Prometheus text.
    pub fn gather(&self) -> Result<String, TelemetryError> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

/// Initializes JSON tracing with `RUST_LOG` support.
///
/// It is safe to call this more than once in tests; subsequent calls are
/// ignored by the subscriber global registry.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt().json().with_env_filter(filter).finish();
    let _ignored = tracing::subscriber::set_global_default(subscriber);
}
