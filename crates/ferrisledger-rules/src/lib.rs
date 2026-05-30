//! Business rules and deterministic replay for FerrisLedger.

use ferrisledger_domain::{
    AccountId, AccountState, CorrelationId, DomainError, EventId, IdempotencyKey, LedgerEntryId,
    Money, SettlementId, TenantId, account_stream_id,
};
use ferrisledger_events::{
    AccountOpened, EventEnvelope, EventError, EventMetadata, FinancialEvent, LedgerDirection,
    LedgerEntryCreated, MoneyDeposited, PixTransferRequested, SettlementExecuted,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Rule-engine failures.
#[derive(Debug, Error)]
pub enum RuleError {
    /// A domain invariant was violated.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Event construction failed.
    #[error(transparent)]
    Event(#[from] EventError),
    /// The account stream does not exist.
    #[error("account does not exist")]
    AccountNotFound,
    /// The account stream already exists.
    #[error("account already exists")]
    AccountAlreadyExists,
    /// Event tenant/account does not match the target stream.
    #[error("event does not belong to stream tenant/account")]
    StreamBoundaryViolation,
    /// A command field failed semantic validation.
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

/// Commands accepted by the financial runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum RuntimeCommand {
    /// Open a new account.
    OpenAccount {
        /// Tenant partition key.
        tenant_id: TenantId,
        /// Account identifier.
        account_id: AccountId,
        /// Account currency.
        currency: String,
        /// Human-readable account holder.
        account_holder_name: String,
    },
    /// Deposit money into an account.
    DepositMoney {
        /// Tenant partition key.
        tenant_id: TenantId,
        /// Account identifier.
        account_id: AccountId,
        /// Amount to deposit.
        amount: Money,
        /// Client-provided idempotency key.
        idempotency_key: IdempotencyKey,
    },
    /// Request an outgoing Pix transfer and reserve funds.
    RequestPixTransfer {
        /// Tenant partition key.
        tenant_id: TenantId,
        /// Account identifier.
        account_id: AccountId,
        /// Amount to reserve.
        amount: Money,
        /// Destination Pix key.
        beneficiary_pix_key: String,
        /// Client-provided idempotency key.
        idempotency_key: IdempotencyKey,
    },
    /// Execute settlement for a reserved Pix transfer.
    ExecuteSettlement {
        /// Tenant partition key.
        tenant_id: TenantId,
        /// Account identifier.
        account_id: AccountId,
        /// Amount to settle.
        amount: Money,
        /// Settlement ID.
        settlement_id: SettlementId,
        /// Client-provided idempotency key.
        idempotency_key: IdempotencyKey,
    },
    /// Record a ledger entry for accounting evidence.
    CreateLedgerEntry {
        /// Tenant partition key.
        tenant_id: TenantId,
        /// Account identifier.
        account_id: AccountId,
        /// Ledger entry ID.
        ledger_entry_id: LedgerEntryId,
        /// Entry direction.
        direction: LedgerDirection,
        /// Entry amount.
        amount: Money,
        /// Business reason.
        reason: String,
        /// Client-provided idempotency key.
        idempotency_key: IdempotencyKey,
        /// Related domain event ID.
        related_event_id: Option<EventId>,
    },
}

impl RuntimeCommand {
    /// Returns tenant ID.
    #[must_use]
    pub const fn tenant_id(&self) -> &TenantId {
        match self {
            Self::OpenAccount { tenant_id, .. }
            | Self::DepositMoney { tenant_id, .. }
            | Self::RequestPixTransfer { tenant_id, .. }
            | Self::ExecuteSettlement { tenant_id, .. }
            | Self::CreateLedgerEntry { tenant_id, .. } => tenant_id,
        }
    }

    /// Returns account ID.
    #[must_use]
    pub const fn account_id(&self) -> &AccountId {
        match self {
            Self::OpenAccount { account_id, .. }
            | Self::DepositMoney { account_id, .. }
            | Self::RequestPixTransfer { account_id, .. }
            | Self::ExecuteSettlement { account_id, .. }
            | Self::CreateLedgerEntry { account_id, .. } => account_id,
        }
    }

    /// Returns an idempotency key when the command can be retried.
    #[must_use]
    pub const fn idempotency_key(&self) -> Option<&IdempotencyKey> {
        match self {
            Self::OpenAccount { .. } => None,
            Self::DepositMoney {
                idempotency_key, ..
            }
            | Self::RequestPixTransfer {
                idempotency_key, ..
            }
            | Self::ExecuteSettlement {
                idempotency_key, ..
            }
            | Self::CreateLedgerEntry {
                idempotency_key, ..
            } => Some(idempotency_key),
        }
    }
}

/// Deterministic rule engine.
#[derive(Clone, Debug, Default)]
pub struct RuleEngine;

impl RuleEngine {
    /// Converts a validated command into a single financial event.
    pub fn decide(
        &self,
        current_state: Option<&AccountState>,
        command: RuntimeCommand,
        correlation_id: CorrelationId,
        causation_id: Option<EventId>,
        occurred_at: OffsetDateTime,
    ) -> Result<EventEnvelope, RuleError> {
        let stream_id = account_stream_id(command.tenant_id(), command.account_id())?;
        let metadata = EventMetadata {
            stream_id,
            correlation_id,
            causation_id,
            occurred_at,
            producer: "ferrisledger".to_string(),
        };

        let event = match command {
            RuntimeCommand::OpenAccount {
                tenant_id,
                account_id,
                currency,
                account_holder_name,
            } => {
                if current_state.is_some() {
                    return Err(RuleError::AccountAlreadyExists);
                }
                if account_holder_name.trim().is_empty() {
                    return Err(RuleError::InvalidCommand(
                        "account_holder_name cannot be empty".to_string(),
                    ));
                }
                Money::zero(currency.clone())?;
                FinancialEvent::AccountOpened(AccountOpened {
                    tenant_id,
                    account_id,
                    currency,
                    account_holder_name,
                })
            }
            RuntimeCommand::DepositMoney {
                tenant_id,
                account_id,
                amount,
                idempotency_key,
            } => {
                ensure_existing_stream(current_state)?;
                FinancialEvent::MoneyDeposited(MoneyDeposited {
                    tenant_id,
                    account_id,
                    amount,
                    idempotency_key,
                })
            }
            RuntimeCommand::RequestPixTransfer {
                tenant_id,
                account_id,
                amount,
                beneficiary_pix_key,
                idempotency_key,
            } => {
                let mut state = ensure_existing_stream(current_state)?.clone();
                if beneficiary_pix_key.trim().is_empty() {
                    return Err(RuleError::InvalidCommand(
                        "beneficiary_pix_key cannot be empty".to_string(),
                    ));
                }
                state.reserve_pix_transfer(&amount)?;
                FinancialEvent::PixTransferRequested(PixTransferRequested {
                    tenant_id,
                    account_id,
                    amount,
                    beneficiary_pix_key,
                    idempotency_key,
                })
            }
            RuntimeCommand::ExecuteSettlement {
                tenant_id,
                account_id,
                amount,
                settlement_id,
                idempotency_key,
            } => {
                let mut state = ensure_existing_stream(current_state)?.clone();
                state.settle_reserved_transfer(&amount)?;
                FinancialEvent::SettlementExecuted(SettlementExecuted {
                    tenant_id,
                    account_id,
                    amount,
                    settlement_id,
                    idempotency_key,
                })
            }
            RuntimeCommand::CreateLedgerEntry {
                tenant_id,
                account_id,
                ledger_entry_id,
                direction,
                amount,
                reason,
                idempotency_key,
                related_event_id,
            } => {
                ensure_existing_stream(current_state)?;
                if reason.trim().is_empty() {
                    return Err(RuleError::InvalidCommand(
                        "ledger entry reason cannot be empty".to_string(),
                    ));
                }
                FinancialEvent::LedgerEntryCreated(LedgerEntryCreated {
                    tenant_id,
                    account_id,
                    ledger_entry_id,
                    direction,
                    amount,
                    reason,
                    idempotency_key,
                    related_event_id,
                })
            }
        };

        EventEnvelope::new(event, metadata).map_err(RuleError::from)
    }
}

fn ensure_existing_stream(
    current_state: Option<&AccountState>,
) -> Result<&AccountState, RuleError> {
    current_state.ok_or(RuleError::AccountNotFound)
}

/// Rebuilds account state from a stream.
pub fn project_account(events: &[EventEnvelope]) -> Result<Option<AccountState>, RuleError> {
    let mut state: Option<AccountState> = None;
    for envelope in events {
        state = apply_event(state, envelope)?;
    }
    Ok(state)
}

/// Applies a single event to an account projection.
pub fn apply_event(
    current_state: Option<AccountState>,
    envelope: &EventEnvelope,
) -> Result<Option<AccountState>, RuleError> {
    let next = match (&envelope.payload, current_state) {
        (FinancialEvent::AccountOpened(payload), None) => Some(AccountState::opened(
            payload.tenant_id.clone(),
            payload.account_id.clone(),
            payload.currency.clone(),
        )?),
        (FinancialEvent::AccountOpened(_), Some(_)) => return Err(RuleError::AccountAlreadyExists),
        (FinancialEvent::MoneyDeposited(payload), Some(mut state)) => {
            ensure_event_matches_state(&state, envelope)?;
            state.deposit(&payload.amount)?;
            Some(state)
        }
        (FinancialEvent::PixTransferRequested(payload), Some(mut state)) => {
            ensure_event_matches_state(&state, envelope)?;
            state.reserve_pix_transfer(&payload.amount)?;
            Some(state)
        }
        (FinancialEvent::SettlementExecuted(payload), Some(mut state)) => {
            ensure_event_matches_state(&state, envelope)?;
            state.settle_reserved_transfer(&payload.amount)?;
            Some(state)
        }
        (FinancialEvent::LedgerEntryCreated(_), Some(mut state)) => {
            ensure_event_matches_state(&state, envelope)?;
            state.version += 1;
            Some(state)
        }
        (_, None) => return Err(RuleError::AccountNotFound),
    };
    Ok(next)
}

fn ensure_event_matches_state(
    state: &AccountState,
    envelope: &EventEnvelope,
) -> Result<(), RuleError> {
    if &state.tenant_id == envelope.payload.tenant_id()
        && &state.account_id == envelope.payload.account_id()
        && state.tenant_id == envelope.tenant_id
    {
        return Ok(());
    }
    Err(RuleError::StreamBoundaryViolation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{AccountId, CorrelationId, IdempotencyKey, TenantId};
    use proptest::prelude::*;

    fn tenant() -> TenantId {
        TenantId::new("tenant_001").expect("tenant")
    }

    fn account() -> AccountId {
        AccountId::new("account_001").expect("account")
    }

    fn corr() -> CorrelationId {
        CorrelationId::new("corr_001").expect("correlation")
    }

    #[test]
    fn rejects_pix_transfer_without_available_funds() {
        let engine = RuleEngine;
        let state = AccountState::opened(tenant(), account(), "BRL").expect("opened");
        let result = engine.decide(
            Some(&state),
            RuntimeCommand::RequestPixTransfer {
                tenant_id: tenant(),
                account_id: account(),
                amount: Money::new(1_000, "BRL").expect("amount"),
                beneficiary_pix_key: "email@example.com".to_string(),
                idempotency_key: IdempotencyKey::new("idem_001").expect("idempotency"),
            },
            corr(),
            None,
            OffsetDateTime::now_utc(),
        );

        assert!(matches!(
            result,
            Err(RuleError::Domain(DomainError::InsufficientFunds { .. }))
        ));
    }

    #[test]
    fn replay_rebuilds_account_snapshot() {
        let engine = RuleEngine;
        let opened = engine
            .decide(
                None,
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
                None,
                OffsetDateTime::now_utc(),
            )
            .expect("opened");
        let state = project_account(std::slice::from_ref(&opened))
            .expect("projection")
            .expect("state");
        let deposited = engine
            .decide(
                Some(&state),
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(5_000, "BRL").expect("amount"),
                    idempotency_key: IdempotencyKey::new("idem_001").expect("idempotency"),
                },
                corr(),
                Some(opened.event_id.clone()),
                OffsetDateTime::now_utc(),
            )
            .expect("deposit");

        let snapshot = project_account(&[opened, deposited])
            .expect("projection")
            .expect("state");

        assert_eq!(snapshot.balance.cents(), 5_000);
        assert_eq!(snapshot.version, 2);
    }

    proptest! {
        #[test]
        fn command_replay_matches_deposit_total(
            first in 1_i64..100_000_i64,
            second in 1_i64..100_000_i64,
        ) {
            let engine = RuleEngine;
            let opened = engine.decide(
                None,
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
                None,
                OffsetDateTime::now_utc(),
            ).expect("opened");
            let mut state = project_account(std::slice::from_ref(&opened)).expect("project").expect("state");
            let first_event = engine.decide(
                Some(&state),
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(first, "BRL").expect("amount"),
                    idempotency_key: IdempotencyKey::new("idem_first").expect("idempotency"),
                },
                corr(),
                Some(opened.event_id.clone()),
                OffsetDateTime::now_utc(),
            ).expect("first");
            state = apply_event(Some(state), &first_event).expect("apply").expect("state");
            let second_event = engine.decide(
                Some(&state),
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(second, "BRL").expect("amount"),
                    idempotency_key: IdempotencyKey::new("idem_second").expect("idempotency"),
                },
                corr(),
                Some(first_event.event_id.clone()),
                OffsetDateTime::now_utc(),
            ).expect("second");

            let snapshot = project_account(&[opened, first_event, second_event]).expect("project").expect("state");
            prop_assert_eq!(snapshot.balance.cents(), first + second);
        }
    }
}
