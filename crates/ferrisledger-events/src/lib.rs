//! Versioned financial event contracts.

use ferrisledger_domain::{
    AccountId, CorrelationId, EventId, IdempotencyKey, LedgerEntryId, Money, SettlementId,
    StreamId, TenantId, account_stream_id,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

/// Event contract errors.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum EventError {
    /// Event ID generation created an invalid domain ID.
    #[error("invalid generated event id: {0}")]
    InvalidGeneratedId(String),
    /// Persisted record uses an unsupported schema version.
    #[error("unsupported schema version {actual}, expected 1")]
    UnsupportedSchemaVersion {
        /// Unsupported version found in storage.
        actual: u16,
    },
    /// Persisted redundant envelope field drifted from the typed payload.
    #[error("persisted {field} mismatch: expected {expected}, got {actual}")]
    PersistedFieldMismatch {
        /// Envelope field that diverged from the payload-derived value.
        field: &'static str,
        /// Canonical value derived from the payload.
        expected: String,
        /// Actual value found in storage.
        actual: String,
    },
    /// Persisted payload could not derive its canonical stream contract.
    #[error("invalid persisted contract: {0}")]
    InvalidPersistedContract(String),
}

/// Canonical event types emitted by the runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Account was opened.
    AccountOpened,
    /// Money was deposited into an account.
    MoneyDeposited,
    /// Pix transfer was requested and funds were reserved.
    PixTransferRequested,
    /// Settlement completed for a reserved outgoing Pix transfer.
    SettlementExecuted,
    /// Ledger entry was recorded for audit/accounting.
    LedgerEntryCreated,
}

impl EventType {
    /// Stable string representation used in logs and API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AccountOpened => "account_opened",
            Self::MoneyDeposited => "money_deposited",
            Self::PixTransferRequested => "pix_transfer_requested",
            Self::SettlementExecuted => "settlement_executed",
            Self::LedgerEntryCreated => "ledger_entry_created",
        }
    }
}

/// Event envelope with CloudEvents-like operational metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    /// Unique event identifier.
    pub event_id: EventId,
    /// Redundant top-level event type for routing and filtering.
    pub event_type: EventType,
    /// Append-only stream identifier.
    pub stream_id: StreamId,
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Correlation ID from the API/CLI caller.
    pub correlation_id: CorrelationId,
    /// Event that caused this event, when available.
    pub causation_id: Option<EventId>,
    /// Event schema version.
    pub schema_version: u16,
    /// Event occurrence timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    /// Producing service.
    pub producer: String,
    /// Typed event payload.
    pub payload: FinancialEvent,
}

impl EventEnvelope {
    /// Creates a new envelope for the given payload.
    pub fn new(payload: FinancialEvent, metadata: EventMetadata) -> Result<Self, EventError> {
        let event_id =
            EventId::new(Uuid::new_v4().to_string()).map_err(EventError::InvalidGeneratedId)?;
        Ok(Self {
            event_id,
            event_type: payload.event_type(),
            stream_id: metadata.stream_id,
            tenant_id: payload.tenant_id().clone(),
            correlation_id: metadata.correlation_id,
            causation_id: metadata.causation_id,
            schema_version: 1,
            occurred_at: metadata.occurred_at,
            producer: metadata.producer,
            payload,
        })
    }

    /// Verifies that persisted redundant fields still match the typed payload.
    pub fn validate_persisted(&self) -> Result<(), EventError> {
        if self.schema_version != 1 {
            return Err(EventError::UnsupportedSchemaVersion {
                actual: self.schema_version,
            });
        }

        let expected_event_type = self.payload.event_type();
        if self.event_type != expected_event_type {
            return Err(EventError::PersistedFieldMismatch {
                field: "event_type",
                expected: expected_event_type.as_str().to_string(),
                actual: self.event_type.as_str().to_string(),
            });
        }

        let expected_tenant_id = self.payload.tenant_id();
        if &self.tenant_id != expected_tenant_id {
            return Err(EventError::PersistedFieldMismatch {
                field: "tenant_id",
                expected: expected_tenant_id.as_str().to_string(),
                actual: self.tenant_id.as_str().to_string(),
            });
        }

        let expected_stream_id =
            account_stream_id(self.payload.tenant_id(), self.payload.account_id())
                .map_err(|error| EventError::InvalidPersistedContract(error.to_string()))?;
        if self.stream_id != expected_stream_id {
            return Err(EventError::PersistedFieldMismatch {
                field: "stream_id",
                expected: expected_stream_id.as_str().to_string(),
                actual: self.stream_id.as_str().to_string(),
            });
        }

        Ok(())
    }
}

/// Metadata supplied by application services before event creation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventMetadata {
    /// Stream to append to.
    pub stream_id: StreamId,
    /// Correlation ID from caller.
    pub correlation_id: CorrelationId,
    /// Optional causal event ID.
    pub causation_id: Option<EventId>,
    /// Timestamp chosen by the application service.
    pub occurred_at: OffsetDateTime,
    /// Producer name.
    pub producer: String,
}

impl EventMetadata {
    /// Creates metadata using the default producer name.
    #[must_use]
    pub fn new(stream_id: StreamId, correlation_id: CorrelationId) -> Self {
        Self {
            stream_id,
            correlation_id,
            causation_id: None,
            occurred_at: OffsetDateTime::now_utc(),
            producer: "ferrisledger".to_string(),
        }
    }
}

/// Financial domain events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FinancialEvent {
    /// Account was opened.
    AccountOpened(AccountOpened),
    /// Money was deposited.
    MoneyDeposited(MoneyDeposited),
    /// Pix transfer was requested.
    PixTransferRequested(PixTransferRequested),
    /// Settlement completed.
    SettlementExecuted(SettlementExecuted),
    /// Ledger entry was created.
    LedgerEntryCreated(LedgerEntryCreated),
}

impl FinancialEvent {
    /// Returns the event type for this payload.
    #[must_use]
    pub const fn event_type(&self) -> EventType {
        match self {
            Self::AccountOpened(_) => EventType::AccountOpened,
            Self::MoneyDeposited(_) => EventType::MoneyDeposited,
            Self::PixTransferRequested(_) => EventType::PixTransferRequested,
            Self::SettlementExecuted(_) => EventType::SettlementExecuted,
            Self::LedgerEntryCreated(_) => EventType::LedgerEntryCreated,
        }
    }

    /// Returns the tenant partition key carried by the payload.
    #[must_use]
    pub const fn tenant_id(&self) -> &TenantId {
        match self {
            Self::AccountOpened(payload) => &payload.tenant_id,
            Self::MoneyDeposited(payload) => &payload.tenant_id,
            Self::PixTransferRequested(payload) => &payload.tenant_id,
            Self::SettlementExecuted(payload) => &payload.tenant_id,
            Self::LedgerEntryCreated(payload) => &payload.tenant_id,
        }
    }

    /// Returns the account impacted by the event.
    #[must_use]
    pub const fn account_id(&self) -> &AccountId {
        match self {
            Self::AccountOpened(payload) => &payload.account_id,
            Self::MoneyDeposited(payload) => &payload.account_id,
            Self::PixTransferRequested(payload) => &payload.account_id,
            Self::SettlementExecuted(payload) => &payload.account_id,
            Self::LedgerEntryCreated(payload) => &payload.account_id,
        }
    }

    /// Returns a client idempotency key when the event was created from a
    /// repeatable external command.
    #[must_use]
    pub const fn idempotency_key(&self) -> Option<&IdempotencyKey> {
        match self {
            Self::AccountOpened(_) => None,
            Self::MoneyDeposited(payload) => Some(&payload.idempotency_key),
            Self::PixTransferRequested(payload) => Some(&payload.idempotency_key),
            Self::SettlementExecuted(payload) => Some(&payload.idempotency_key),
            Self::LedgerEntryCreated(payload) => Some(&payload.idempotency_key),
        }
    }
}

/// Payload for `account_opened`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountOpened {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account ID.
    pub account_id: AccountId,
    /// ISO-4217 currency.
    pub currency: String,
    /// Human-readable account holder.
    pub account_holder_name: String,
}

/// Payload for `money_deposited`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoneyDeposited {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account ID.
    pub account_id: AccountId,
    /// Deposit amount.
    pub amount: Money,
    /// External idempotency key.
    pub idempotency_key: IdempotencyKey,
}

/// Payload for `pix_transfer_requested`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixTransferRequested {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account ID.
    pub account_id: AccountId,
    /// Reserved amount.
    pub amount: Money,
    /// Destination Pix key.
    pub beneficiary_pix_key: String,
    /// External idempotency key.
    pub idempotency_key: IdempotencyKey,
}

/// Payload for `settlement_executed`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementExecuted {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account ID.
    pub account_id: AccountId,
    /// Settled amount.
    pub amount: Money,
    /// Settlement identifier.
    pub settlement_id: SettlementId,
    /// External idempotency key.
    pub idempotency_key: IdempotencyKey,
}

/// Ledger direction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerDirection {
    /// Money increased the account balance.
    Credit,
    /// Money decreased the account balance.
    Debit,
}

/// Payload for `ledger_entry_created`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LedgerEntryCreated {
    /// Tenant partition key.
    pub tenant_id: TenantId,
    /// Account ID.
    pub account_id: AccountId,
    /// Ledger entry ID.
    pub ledger_entry_id: LedgerEntryId,
    /// Direction of the entry.
    pub direction: LedgerDirection,
    /// Entry amount.
    pub amount: Money,
    /// Business reason.
    pub reason: String,
    /// External idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Related domain event when available.
    pub related_event_id: Option<EventId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{
        AccountId, CorrelationId, EventId, IdempotencyKey, LedgerEntryId, Money, SettlementId,
        TenantId, account_stream_id,
    };
    use serde_json::json;

    fn tenant_id() -> TenantId {
        TenantId::new("tenant_001").expect("tenant")
    }

    fn account_id() -> AccountId {
        AccountId::new("account_001").expect("account")
    }

    fn money(cents: i64) -> Money {
        Money::new(cents, "BRL").expect("money")
    }

    fn idempotency_key(value: &str) -> IdempotencyKey {
        IdempotencyKey::new(value).expect("idempotency")
    }

    #[test]
    fn envelope_event_type_matches_payload() {
        let tenant_id = tenant_id();
        let account_id = account_id();
        let payload = FinancialEvent::AccountOpened(AccountOpened {
            tenant_id: tenant_id.clone(),
            account_id: account_id.clone(),
            currency: "BRL".to_string(),
            account_holder_name: "Ada Lovelace".to_string(),
        });
        let metadata = EventMetadata::new(
            account_stream_id(&tenant_id, &account_id).expect("stream"),
            CorrelationId::new("corr_001").expect("correlation"),
        );

        let envelope = EventEnvelope::new(payload, metadata).expect("envelope");

        assert_eq!(envelope.event_type, EventType::AccountOpened);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.producer, "ferrisledger");
    }

    #[test]
    fn rejects_persisted_event_type_drift() {
        let tenant_id = tenant_id();
        let account_id = account_id();
        let payload = FinancialEvent::AccountOpened(AccountOpened {
            tenant_id: tenant_id.clone(),
            account_id: account_id.clone(),
            currency: "BRL".to_string(),
            account_holder_name: "Ada Lovelace".to_string(),
        });
        let metadata = EventMetadata::new(
            account_stream_id(&tenant_id, &account_id).expect("stream"),
            CorrelationId::new("corr_001").expect("correlation"),
        );

        let mut envelope = EventEnvelope::new(payload, metadata).expect("envelope");
        envelope.event_type = EventType::MoneyDeposited;

        assert_eq!(
            envelope
                .validate_persisted()
                .expect_err("persisted event type must match payload"),
            EventError::PersistedFieldMismatch {
                field: "event_type",
                expected: "account_opened".to_string(),
                actual: "money_deposited".to_string(),
            }
        );
    }

    #[test]
    fn serializes_financial_event_payload_contracts() {
        let tenant_id = tenant_id();
        let account_id = account_id();
        let cases = vec![
            (
                FinancialEvent::AccountOpened(AccountOpened {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                }),
                json!({
                    "kind": "account_opened",
                    "data": {
                        "tenant_id": "tenant_001",
                        "account_id": "account_001",
                        "currency": "BRL",
                        "account_holder_name": "Ada Lovelace"
                    }
                }),
            ),
            (
                FinancialEvent::MoneyDeposited(MoneyDeposited {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    amount: money(2_500),
                    idempotency_key: idempotency_key("idem_deposit_001"),
                }),
                json!({
                    "kind": "money_deposited",
                    "data": {
                        "tenant_id": "tenant_001",
                        "account_id": "account_001",
                        "amount": {
                            "cents": 2500,
                            "currency": "BRL"
                        },
                        "idempotency_key": "idem_deposit_001"
                    }
                }),
            ),
            (
                FinancialEvent::PixTransferRequested(PixTransferRequested {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    amount: money(1_200),
                    beneficiary_pix_key: "pix-key@example.test".to_string(),
                    idempotency_key: idempotency_key("idem_pix_001"),
                }),
                json!({
                    "kind": "pix_transfer_requested",
                    "data": {
                        "tenant_id": "tenant_001",
                        "account_id": "account_001",
                        "amount": {
                            "cents": 1200,
                            "currency": "BRL"
                        },
                        "beneficiary_pix_key": "pix-key@example.test",
                        "idempotency_key": "idem_pix_001"
                    }
                }),
            ),
            (
                FinancialEvent::SettlementExecuted(SettlementExecuted {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    amount: money(1_200),
                    settlement_id: SettlementId::new("settlement_001").expect("settlement"),
                    idempotency_key: idempotency_key("idem_settlement_001"),
                }),
                json!({
                    "kind": "settlement_executed",
                    "data": {
                        "tenant_id": "tenant_001",
                        "account_id": "account_001",
                        "amount": {
                            "cents": 1200,
                            "currency": "BRL"
                        },
                        "settlement_id": "settlement_001",
                        "idempotency_key": "idem_settlement_001"
                    }
                }),
            ),
            (
                FinancialEvent::LedgerEntryCreated(LedgerEntryCreated {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    ledger_entry_id: LedgerEntryId::new("ledger_001").expect("ledger"),
                    direction: LedgerDirection::Credit,
                    amount: money(2_500),
                    reason: "deposit booking".to_string(),
                    idempotency_key: idempotency_key("idem_ledger_001"),
                    related_event_id: Some(EventId::new("event_001").expect("event")),
                }),
                json!({
                    "kind": "ledger_entry_created",
                    "data": {
                        "tenant_id": "tenant_001",
                        "account_id": "account_001",
                        "ledger_entry_id": "ledger_001",
                        "direction": "credit",
                        "amount": {
                            "cents": 2500,
                            "currency": "BRL"
                        },
                        "reason": "deposit booking",
                        "idempotency_key": "idem_ledger_001",
                        "related_event_id": "event_001"
                    }
                }),
            ),
        ];

        for (event, expected) in cases {
            assert_eq!(serde_json::to_value(event).expect("json"), expected);
        }
    }

    #[test]
    fn envelope_serializes_with_redundant_event_type_and_typed_payload() {
        let tenant_id = tenant_id();
        let account_id = account_id();
        let payload = FinancialEvent::AccountOpened(AccountOpened {
            tenant_id: tenant_id.clone(),
            account_id: account_id.clone(),
            currency: "BRL".to_string(),
            account_holder_name: "Ada Lovelace".to_string(),
        });
        let metadata = EventMetadata {
            stream_id: account_stream_id(&tenant_id, &account_id).expect("stream"),
            correlation_id: CorrelationId::new("corr_001").expect("correlation"),
            causation_id: None,
            occurred_at: OffsetDateTime::from_unix_timestamp(1_704_067_200).expect("timestamp"),
            producer: "ferrisledger-test".to_string(),
        };

        let envelope = EventEnvelope::new(payload, metadata).expect("envelope");
        let value = serde_json::to_value(envelope).expect("json");

        assert_eq!(value["event_type"], "account_opened");
        assert_eq!(value["stream_id"], "tenant:tenant_001:account:account_001");
        assert_eq!(value["tenant_id"], "tenant_001");
        assert_eq!(value["correlation_id"], "corr_001");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["occurred_at"], "2024-01-01T00:00:00Z");
        assert_eq!(value["producer"], "ferrisledger-test");
        assert_eq!(value["payload"]["kind"], "account_opened");
        assert_eq!(value["payload"]["data"]["account_id"], "account_001");
        assert!(value["event_id"].as_str().is_some_and(|id| !id.is_empty()));
    }
}
