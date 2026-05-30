//! Axum HTTP adapter for FerrisLedger.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use ferrisledger_domain::{
    AccountId, CorrelationId, DomainError, IdempotencyKey, LedgerEntryId, Money, SettlementId,
    TenantId,
};
use ferrisledger_events::LedgerDirection;
use ferrisledger_rules::{RuleError, RuntimeCommand};
use ferrisledger_runtime::{CommandOutcome, RuntimeError, RuntimeService};
use ferrisledger_store::{FileEventStore, StoreError, StoreVerification};
use ferrisledger_telemetry::{Telemetry, TelemetryError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

/// API configuration.
#[derive(Clone, Debug)]
pub struct ApiConfig {
    /// JSONL event store path.
    pub store_path: PathBuf,
    /// Shared API key for the demo runtime.
    pub api_key: String,
    /// Maximum authenticated requests per API key per rolling minute.
    pub rate_limit_per_minute: u32,
}

impl ApiConfig {
    /// Creates a config value.
    #[must_use]
    pub fn new(store_path: impl Into<PathBuf>, api_key: impl Into<String>) -> Self {
        Self {
            store_path: store_path.into(),
            api_key: api_key.into(),
            rate_limit_per_minute: 120,
        }
    }

    /// Overrides the default local rate limit.
    #[must_use]
    pub const fn with_rate_limit_per_minute(mut self, rate_limit_per_minute: u32) -> Self {
        self.rate_limit_per_minute = rate_limit_per_minute;
        self
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    runtime: RuntimeService<FileEventStore>,
    telemetry: Telemetry,
    api_key: String,
    rate_limiter: Arc<Mutex<RateLimiter>>,
}

impl AppState {
    /// Creates application state.
    pub fn new(config: ApiConfig) -> Result<Self, ApiError> {
        Ok(Self {
            runtime: RuntimeService::file(config.store_path),
            telemetry: Telemetry::new()?,
            api_key: config.api_key,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(config.rate_limit_per_minute))),
        })
    }
}

/// Builds the HTTP router.
pub fn router(config: ApiConfig) -> Result<Router, ApiError> {
    let state = AppState::new(config)?;
    Ok(Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/v1/accounts", post(open_account))
        .route("/v1/accounts/{account_id}/events", get(account_events))
        .route("/v1/accounts/{account_id}/snapshot", get(account_snapshot))
        .route("/v1/accounts/{account_id}/deposits", post(deposit_money))
        .route(
            "/v1/accounts/{account_id}/pix-transfers",
            post(request_pix_transfer),
        )
        .route(
            "/v1/accounts/{account_id}/settlements",
            post(execute_settlement),
        )
        .route(
            "/v1/accounts/{account_id}/ledger-entries",
            post(create_ledger_entry),
        )
        .with_state(state))
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn readyz(State(state): State<AppState>) -> Result<Json<StoreVerification>, ApiError> {
    let started = Instant::now();
    let verification = state.runtime.verify_store()?;
    state
        .telemetry
        .set_event_store_records(i64::try_from(verification.records).unwrap_or(i64::MAX));
    state.telemetry.observe_http(
        "GET",
        "/readyz",
        StatusCode::OK.as_u16(),
        started.elapsed().as_secs_f64(),
    );
    Ok(Json(verification))
}

async fn metrics(State(state): State<AppState>) -> Result<String, ApiError> {
    state.telemetry.gather().map_err(ApiError::from)
}

async fn open_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OpenAccountRequest>,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers).with_correlation(&request.correlation_id);
    authorize(&state, &headers, &context)?;
    let command = RuntimeCommand::OpenAccount {
        tenant_id: request.tenant_id,
        account_id: request.account_id,
        currency: request.currency,
        account_holder_name: request.account_holder_name,
    };
    execute(
        &state,
        command,
        request.correlation_id,
        &context,
        "/v1/accounts",
    )
    .map(|outcome| success_response(&context, outcome))
    .map_err(|error| error.with_context(&context))
}

async fn deposit_money(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<MoneyMovementRequest>,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers).with_correlation(&request.correlation_id);
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let command = RuntimeCommand::DepositMoney {
        tenant_id: request.tenant_id,
        account_id,
        amount: Money::new(request.amount_cents, request.currency)
            .map_err(|error| ApiError::from(error).with_context(&context))?,
        idempotency_key: request.idempotency_key,
    };
    execute(
        &state,
        command,
        request.correlation_id,
        &context,
        "/v1/accounts/{account_id}/deposits",
    )
    .map(|outcome| success_response(&context, outcome))
    .map_err(|error| error.with_context(&context))
}

async fn request_pix_transfer(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<PixTransferRequest>,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers).with_correlation(&request.correlation_id);
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let command = RuntimeCommand::RequestPixTransfer {
        tenant_id: request.tenant_id,
        account_id,
        amount: Money::new(request.amount_cents, request.currency)
            .map_err(|error| ApiError::from(error).with_context(&context))?,
        beneficiary_pix_key: request.beneficiary_pix_key,
        idempotency_key: request.idempotency_key,
    };
    execute(
        &state,
        command,
        request.correlation_id,
        &context,
        "/v1/accounts/{account_id}/pix-transfers",
    )
    .map(|outcome| success_response(&context, outcome))
    .map_err(|error| error.with_context(&context))
}

async fn execute_settlement(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SettlementRequest>,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers).with_correlation(&request.correlation_id);
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let command = RuntimeCommand::ExecuteSettlement {
        tenant_id: request.tenant_id,
        account_id,
        amount: Money::new(request.amount_cents, request.currency)
            .map_err(|error| ApiError::from(error).with_context(&context))?,
        settlement_id: request.settlement_id,
        idempotency_key: request.idempotency_key,
    };
    execute(
        &state,
        command,
        request.correlation_id,
        &context,
        "/v1/accounts/{account_id}/settlements",
    )
    .map(|outcome| success_response(&context, outcome))
    .map_err(|error| error.with_context(&context))
}

async fn create_ledger_entry(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<LedgerEntryRequest>,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers).with_correlation(&request.correlation_id);
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let command = RuntimeCommand::CreateLedgerEntry {
        tenant_id: request.tenant_id,
        account_id,
        ledger_entry_id: request.ledger_entry_id,
        direction: request.direction,
        amount: Money::new(request.amount_cents, request.currency)
            .map_err(|error| ApiError::from(error).with_context(&context))?,
        reason: request.reason,
        idempotency_key: request.idempotency_key,
        related_event_id: request.related_event_id,
    };
    execute(
        &state,
        command,
        request.correlation_id,
        &context,
        "/v1/accounts/{account_id}/ledger-entries",
    )
    .map(|outcome| success_response(&context, outcome))
    .map_err(|error| error.with_context(&context))
}

async fn account_events(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    Query(query): Query<TenantQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers);
    let started = Instant::now();
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let events = state
        .runtime
        .read_account_events(&query.tenant_id, &account_id)
        .map_err(ApiError::from)
        .map_err(|error| error.with_context(&context))?;
    state.telemetry.observe_http(
        "GET",
        "/v1/accounts/{account_id}/events",
        StatusCode::OK.as_u16(),
        started.elapsed().as_secs_f64(),
    );
    Ok(success_response(&context, events))
}

async fn account_snapshot(
    State(state): State<AppState>,
    Path(account_id): Path<String>,
    Query(query): Query<TenantQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiErrorResponse> {
    let context = RequestContext::from_headers(&headers);
    let started = Instant::now();
    authorize(&state, &headers, &context)?;
    let account_id = AccountId::new(account_id)
        .map_err(ApiError::bad_request)
        .map_err(|error| error.with_context(&context))?;
    let snapshot = state
        .runtime
        .account_snapshot(&query.tenant_id, &account_id)
        .map_err(ApiError::from)
        .map_err(|error| error.with_context(&context))?;
    state.telemetry.observe_http(
        "GET",
        "/v1/accounts/{account_id}/snapshot",
        StatusCode::OK.as_u16(),
        started.elapsed().as_secs_f64(),
    );
    Ok(success_response(&context, snapshot))
}

fn execute(
    state: &AppState,
    command: RuntimeCommand,
    correlation_id: CorrelationId,
    context: &RequestContext,
    path: &str,
) -> Result<CommandOutcome, ApiError> {
    let started = Instant::now();
    let command_name = command_name(&command);
    let correlation_id_text = correlation_id.to_string();
    let result = state.runtime.execute(command, correlation_id);
    match result {
        Ok(outcome) => {
            info!(
                event_id = %outcome.event.event_id,
                stream_id = %outcome.event.stream_id,
                correlation_id = %correlation_id_text,
                request_id = %context.request_id,
                idempotent_replay = outcome.idempotent_replay,
                "audit: runtime command accepted"
            );
            state.telemetry.observe_command(command_name, "ok");
            state.telemetry.observe_http(
                "POST",
                path,
                StatusCode::OK.as_u16(),
                started.elapsed().as_secs_f64(),
            );
            Ok(outcome)
        }
        Err(error) => {
            warn!(
                error = %error,
                command = command_name,
                correlation_id = %correlation_id_text,
                request_id = %context.request_id,
                "audit: runtime command rejected"
            );
            state.telemetry.observe_command(command_name, "error");
            state.telemetry.observe_http(
                "POST",
                path,
                status_for_runtime_error(&error).as_u16(),
                started.elapsed().as_secs_f64(),
            );
            Err(ApiError::from(error))
        }
    }
}

fn command_name(command: &RuntimeCommand) -> &'static str {
    match command {
        RuntimeCommand::OpenAccount { .. } => "open_account",
        RuntimeCommand::DepositMoney { .. } => "deposit_money",
        RuntimeCommand::RequestPixTransfer { .. } => "request_pix_transfer",
        RuntimeCommand::ExecuteSettlement { .. } => "execute_settlement",
        RuntimeCommand::CreateLedgerEntry { .. } => "create_ledger_entry",
    }
}

fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    context: &RequestContext,
) -> Result<(), ApiErrorResponse> {
    let Some(api_key) = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
    else {
        return Err(ApiError::unauthorized().with_context(context));
    };

    if api_key != state.api_key {
        return Err(ApiError::unauthorized().with_context(context));
    }

    if state
        .rate_limiter
        .lock()
        .expect("rate limiter lock poisoned")
        .allow(api_key)
    {
        return Ok(());
    }

    state.telemetry.observe_rate_limited();
    Err(ApiError::rate_limited().with_context(context))
}

#[derive(Clone, Debug)]
struct RequestContext {
    request_id: String,
    correlation_id: Option<String>,
}

impl RequestContext {
    fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            request_id: header_value(headers, "x-request-id")
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            correlation_id: header_value(headers, "x-correlation-id"),
        }
    }

    fn with_correlation(mut self, correlation_id: &CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
}

fn success_response(context: &RequestContext, value: impl Serialize) -> Response {
    let mut response = Json(value).into_response();
    set_context_headers(response.headers_mut(), context);
    response
}

fn set_context_headers(headers: &mut HeaderMap, context: &RequestContext) {
    if let Ok(request_id) = HeaderValue::from_str(&context.request_id) {
        headers.insert("x-request-id", request_id);
    }
    if let Some(correlation_id) = &context.correlation_id
        && let Ok(correlation_id) = HeaderValue::from_str(correlation_id)
    {
        headers.insert("x-correlation-id", correlation_id);
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Runtime error.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    /// Domain error.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Telemetry error.
    #[error(transparent)]
    Telemetry(#[from] TelemetryError),
    /// Caller is not authenticated.
    #[error("unauthorized")]
    Unauthorized,
    /// Caller exceeded API rate limit.
    #[error("rate limited")]
    RateLimited,
    /// Request validation failed.
    #[error("bad request: {0}")]
    BadRequest(String),
}

impl ApiError {
    fn unauthorized() -> Self {
        Self::Unauthorized
    }

    fn rate_limited() -> Self {
        Self::RateLimited
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::BadRequest(_) | Self::Domain(DomainError::InvalidIdentifier(_)) => {
                StatusCode::BAD_REQUEST
            }
            Self::Domain(DomainError::InvalidMoney(_)) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Domain(DomainError::CurrencyMismatch { .. }) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Domain(DomainError::AccountNotOpen | DomainError::InsufficientFunds { .. }) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::Runtime(error) => status_for_runtime_error(error),
            Self::Telemetry(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::RateLimited => "rate_limited",
            Self::BadRequest(_) => "bad_request",
            Self::Domain(DomainError::InvalidIdentifier(_)) => "invalid_identifier",
            Self::Domain(DomainError::InvalidMoney(_)) => "invalid_money",
            Self::Domain(DomainError::CurrencyMismatch { .. }) => "currency_mismatch",
            Self::Domain(DomainError::AccountNotOpen) => "account_not_open",
            Self::Domain(DomainError::InsufficientFunds { .. }) => "insufficient_funds",
            Self::Runtime(RuntimeError::Rule(RuleError::AccountNotFound)) => "account_not_found",
            Self::Runtime(RuntimeError::Rule(RuleError::AccountAlreadyExists)) => {
                "account_already_exists"
            }
            Self::Runtime(RuntimeError::Rule(RuleError::InvalidCommand(_))) => "invalid_command",
            Self::Runtime(RuntimeError::Store(StoreError::VersionConflict { .. })) => {
                "version_conflict"
            }
            Self::Runtime(RuntimeError::Store(StoreError::ChecksumMismatch { .. })) => {
                "event_log_corrupt"
            }
            Self::Runtime(_) => "runtime_error",
            Self::Telemetry(_) => "telemetry_error",
        }
    }

    fn with_context(self, context: &RequestContext) -> ApiErrorResponse {
        ApiErrorResponse {
            error: self,
            request_id: context.request_id.clone(),
            correlation_id: context.correlation_id.clone(),
        }
    }
}

/// API error plus request context.
#[derive(Debug)]
pub struct ApiErrorResponse {
    error: ApiError,
    request_id: String,
    correlation_id: Option<String>,
}

#[derive(Debug)]
struct RateLimiter {
    max_per_window: u32,
    window: Duration,
    hits_by_key: HashMap<String, VecDeque<Instant>>,
}

impl RateLimiter {
    fn new(max_per_window: u32) -> Self {
        Self {
            max_per_window,
            window: Duration::from_secs(60),
            hits_by_key: HashMap::new(),
        }
    }

    fn allow(&mut self, api_key: &str) -> bool {
        if self.max_per_window == 0 {
            return false;
        }

        let now = Instant::now();
        let hits = self.hits_by_key.entry(api_key.to_string()).or_default();
        while hits
            .front()
            .is_some_and(|hit| now.duration_since(*hit) > self.window)
        {
            hits.pop_front();
        }

        if hits.len() >= self.max_per_window as usize {
            return false;
        }

        hits.push_back(now);
        true
    }
}

fn status_for_runtime_error(error: &RuntimeError) -> StatusCode {
    match error {
        RuntimeError::Rule(RuleError::AccountNotFound) => StatusCode::NOT_FOUND,
        RuntimeError::Rule(RuleError::AccountAlreadyExists)
        | RuntimeError::Store(StoreError::VersionConflict { .. })
        | RuntimeError::Store(StoreError::DuplicateEventId(_)) => StatusCode::CONFLICT,
        RuntimeError::Rule(
            RuleError::Domain(
                DomainError::InvalidMoney(_)
                | DomainError::CurrencyMismatch { .. }
                | DomainError::AccountNotOpen
                | DomainError::InsufficientFunds { .. },
            )
            | RuleError::InvalidCommand(_),
        ) => StatusCode::UNPROCESSABLE_ENTITY,
        RuntimeError::Rule(RuleError::Domain(DomainError::InvalidIdentifier(_)))
        | RuntimeError::InvalidStream(_) => StatusCode::BAD_REQUEST,
        RuntimeError::Store(StoreError::ChecksumMismatch { .. })
        | RuntimeError::Store(StoreError::Json(_))
        | RuntimeError::Store(StoreError::Io(_))
        | RuntimeError::Index(_)
        | RuntimeError::Rule(RuleError::Event(_))
        | RuntimeError::Rule(RuleError::StreamBoundaryViolation) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        let status = self.error.status();
        let body = Json(ErrorEnvelope {
            error: ErrorBody {
                code: self.error.code().to_string(),
                message: self.error.to_string(),
                request_id: Some(self.request_id.clone()),
                correlation_id: self.correlation_id.clone(),
            },
        });
        let context = RequestContext {
            request_id: self.request_id,
            correlation_id: self.correlation_id,
        };
        let mut response = (status, body).into_response();
        set_context_headers(response.headers_mut(), &context);
        response
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let context = RequestContext {
            request_id: Uuid::new_v4().to_string(),
            correlation_id: None,
        };
        self.with_context(&context).into_response()
    }
}

/// Health response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HealthResponse {
    /// Health status.
    pub status: String,
}

/// Standard API error envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErrorEnvelope {
    /// Error payload.
    pub error: ErrorBody,
}

/// Standard API error body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ErrorBody {
    /// Stable machine code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Request ID when propagated by ingress.
    pub request_id: Option<String>,
    /// Correlation ID when available.
    pub correlation_id: Option<String>,
}

/// Tenant query parameter.
#[derive(Clone, Debug, Deserialize)]
pub struct TenantQuery {
    /// Tenant partition key.
    pub tenant_id: TenantId,
}

/// Request to open an account.
#[derive(Clone, Debug, Deserialize)]
pub struct OpenAccountRequest {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account identifier.
    pub account_id: AccountId,
    /// Account currency.
    pub currency: String,
    /// Account holder name.
    pub account_holder_name: String,
    /// Correlation ID.
    pub correlation_id: CorrelationId,
}

/// Request for deposits.
#[derive(Clone, Debug, Deserialize)]
pub struct MoneyMovementRequest {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Amount in minor units.
    pub amount_cents: i64,
    /// ISO-4217 currency.
    pub currency: String,
    /// Idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Correlation ID.
    pub correlation_id: CorrelationId,
}

/// Request for Pix transfer reservation.
#[derive(Clone, Debug, Deserialize)]
pub struct PixTransferRequest {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Amount in minor units.
    pub amount_cents: i64,
    /// ISO-4217 currency.
    pub currency: String,
    /// Destination Pix key.
    pub beneficiary_pix_key: String,
    /// Idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Correlation ID.
    pub correlation_id: CorrelationId,
}

/// Request to execute settlement.
#[derive(Clone, Debug, Deserialize)]
pub struct SettlementRequest {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Amount in minor units.
    pub amount_cents: i64,
    /// ISO-4217 currency.
    pub currency: String,
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Correlation ID.
    pub correlation_id: CorrelationId,
}

/// Request to create a ledger entry.
#[derive(Clone, Debug, Deserialize)]
pub struct LedgerEntryRequest {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Ledger entry ID.
    pub ledger_entry_id: LedgerEntryId,
    /// Direction.
    pub direction: LedgerDirection,
    /// Amount in minor units.
    pub amount_cents: i64,
    /// ISO-4217 currency.
    pub currency: String,
    /// Business reason.
    pub reason: String,
    /// Idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Correlation ID.
    pub correlation_id: CorrelationId,
    /// Related event ID.
    pub related_event_id: Option<ferrisledger_domain::EventId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use http::{Request, StatusCode, header};
    use serde_json::json;
    use tower::ServiceExt;

    fn test_router() -> Router {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.keep().join("events.jsonl");
        router(ApiConfig::new(path, "secret")).expect("router")
    }

    fn rate_limited_router(max_per_minute: u32) -> Router {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.keep().join("events.jsonl");
        router(ApiConfig::new(path, "secret").with_rate_limit_per_minute(max_per_minute))
            .expect("router")
    }

    async fn post_json(
        app: Router,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let (status, _headers, value) = post_json_with_request_id(app, uri, body, None).await;
        (status, value)
    }

    async fn post_json_with_request_id(
        app: Router,
        uri: &str,
        body: serde_json::Value,
        request_id: Option<&str>,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-api-key", "secret");
        if let Some(request_id) = request_id {
            builder = builder.header("x-request-id", request_id);
        }
        let response = app
            .oneshot(builder.body(Body::from(body.to_string())).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let value = serde_json::from_slice(&bytes).expect("json");
        (status, headers, value)
    }

    async fn get_json(app: Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let value = serde_json::from_slice(&bytes).expect("json");
        (status, value)
    }

    #[tokio::test]
    async fn opens_account() {
        let body = json!({
            "tenant_id": "tenant_001",
            "account_id": "account_001",
            "currency": "BRL",
            "account_holder_name": "Ada Lovelace",
            "correlation_id": "corr_001"
        });

        let (status, value) = post_json(test_router(), "/v1/accounts", body).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(value["event"]["event_type"], "account_opened");
    }

    #[tokio::test]
    async fn rejects_missing_api_key() {
        let app = test_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/accounts")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "tenant_id": "tenant_001",
                            "account_id": "account_001",
                            "currency": "BRL",
                            "account_holder_name": "Ada Lovelace",
                            "correlation_id": "corr_001"
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn deposits_are_idempotent_and_snapshot_is_tenant_isolated() {
        let app = test_router();
        let open = json!({
            "tenant_id": "tenant_001",
            "account_id": "account_001",
            "currency": "BRL",
            "account_holder_name": "Ada Lovelace",
            "correlation_id": "corr_001"
        });
        let deposit = json!({
            "tenant_id": "tenant_001",
            "amount_cents": 2500,
            "currency": "BRL",
            "idempotency_key": "deposit_001",
            "correlation_id": "corr_002"
        });

        assert_eq!(
            post_json(app.clone(), "/v1/accounts", open).await.0,
            StatusCode::OK
        );
        let first = post_json(
            app.clone(),
            "/v1/accounts/account_001/deposits",
            deposit.clone(),
        )
        .await;
        let second = post_json(app.clone(), "/v1/accounts/account_001/deposits", deposit).await;
        let snapshot = get_json(
            app.clone(),
            "/v1/accounts/account_001/snapshot?tenant_id=tenant_001",
        )
        .await;
        let other_tenant = get_json(
            app,
            "/v1/accounts/account_001/snapshot?tenant_id=tenant_999",
        )
        .await;

        assert_eq!(first.0, StatusCode::OK);
        assert_eq!(second.0, StatusCode::OK);
        assert_eq!(second.1["idempotent_replay"], true);
        assert_eq!(snapshot.1["balance"]["cents"], 2500);
        assert!(other_tenant.1.is_null());
    }

    #[tokio::test]
    async fn rejects_requests_above_api_key_rate_limit() {
        let app = rate_limited_router(1);
        let first = json!({
            "tenant_id": "tenant_001",
            "account_id": "account_001",
            "currency": "BRL",
            "account_holder_name": "Ada Lovelace",
            "correlation_id": "corr_001"
        });
        let second = json!({
            "tenant_id": "tenant_001",
            "account_id": "account_002",
            "currency": "BRL",
            "account_holder_name": "Grace Hopper",
            "correlation_id": "corr_002"
        });

        assert_eq!(
            post_json(app.clone(), "/v1/accounts", first).await.0,
            StatusCode::OK
        );
        let (status, body) = post_json(app, "/v1/accounts", second).await;

        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body["error"]["code"], "rate_limited");
    }

    #[tokio::test]
    async fn propagates_request_and_correlation_ids_on_success_and_error() {
        let body = json!({
            "tenant_id": "tenant_001",
            "account_id": "account_001",
            "currency": "BRL",
            "account_holder_name": "Ada Lovelace",
            "correlation_id": "corr_001"
        });

        let (status, headers, value) =
            post_json_with_request_id(test_router(), "/v1/accounts", body.clone(), Some("req_001"))
                .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers["x-request-id"], "req_001");
        assert_eq!(headers["x-correlation-id"], "corr_001");
        assert_eq!(value["event"]["correlation_id"], "corr_001");

        let (status, headers, value) = post_json_with_request_id(
            rate_limited_router(0),
            "/v1/accounts",
            body,
            Some("req_rate_limited"),
        )
        .await;

        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(headers["x-request-id"], "req_rate_limited");
        assert_eq!(headers["x-correlation-id"], "corr_001");
        assert_eq!(value["error"]["request_id"], "req_rate_limited");
        assert_eq!(value["error"]["correlation_id"], "corr_001");
    }
}
