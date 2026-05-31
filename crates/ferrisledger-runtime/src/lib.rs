//! Application service for append, replay, and idempotency.

use ferrisledger_domain::{AccountId, AccountState, CorrelationId, TenantId, account_stream_id};
use ferrisledger_events::{EventEnvelope, FinancialEvent};
use ferrisledger_index::{AccountIndex, IndexError};
use ferrisledger_rules::{RuleEngine, RuleError, RuntimeCommand, project_account};
use ferrisledger_store::{
    AppendedEvent, EventStore, FileEventStore, StoreError, StoreVerification,
};
use serde::Serialize;
use thiserror::Error;
use time::OffsetDateTime;

/// Runtime failures mapped by API/CLI adapters.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Store operation failed.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Domain rule failed.
    #[error(transparent)]
    Rule(#[from] RuleError),
    /// Projection failed.
    #[error(transparent)]
    Index(#[from] IndexError),
    /// An idempotency key was reused with different command semantics.
    #[error("idempotency key {idempotency_key} conflicts with existing event {existing_event_id}")]
    IdempotencyConflict {
        /// Reused idempotency key.
        idempotency_key: String,
        /// Existing event that owns the key.
        existing_event_id: ferrisledger_domain::EventId,
    },
    /// Stream ID could not be built.
    #[error("invalid stream id: {0}")]
    InvalidStream(String),
}

/// Outcome of command execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommandOutcome {
    /// Event that represents the command result.
    pub event: EventEnvelope,
    /// One-based stream version.
    pub stream_version: u64,
    /// One-based global position when freshly appended.
    pub global_position: Option<u64>,
    /// True when the event came from a previous idempotent request.
    pub idempotent_replay: bool,
}

impl CommandOutcome {
    fn appended(appended: AppendedEvent) -> Self {
        Self {
            event: appended.envelope,
            stream_version: appended.stream_version,
            global_position: Some(appended.global_position),
            idempotent_replay: false,
        }
    }
}

/// Runtime service parameterized over a store implementation.
#[derive(Clone, Debug)]
pub struct RuntimeService<S> {
    store: S,
    rules: RuleEngine,
}

impl RuntimeService<FileEventStore> {
    /// Creates a file-backed runtime.
    #[must_use]
    pub fn file(path: impl Into<std::path::PathBuf>) -> Self {
        Self::new(FileEventStore::new(path))
    }
}

impl<S> RuntimeService<S>
where
    S: EventStore,
{
    /// Creates a runtime from a store implementation.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            rules: RuleEngine,
        }
    }

    /// Returns the underlying store.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    /// Executes a command with deterministic replay/idempotency checks.
    pub fn execute(
        &self,
        command: RuntimeCommand,
        correlation_id: CorrelationId,
    ) -> Result<CommandOutcome, RuntimeError> {
        let stream_id = account_stream_id(command.tenant_id(), command.account_id())
            .map_err(|error| RuntimeError::InvalidStream(error.to_string()))?;
        let all_events = self.store.read_all()?;
        let command_for_idempotency_race = command.clone();

        if let Some(outcome) = idempotent_replay_outcome(&command, &stream_id, &all_events) {
            return outcome;
        }

        let existing_events = all_events
            .into_iter()
            .filter(|event| event.stream_id == stream_id)
            .collect::<Vec<_>>();
        let current_state = project_account(&existing_events)?;
        let causation_id = existing_events.last().map(|event| event.event_id.clone());
        let event = self.rules.decide(
            current_state.as_ref(),
            command,
            correlation_id,
            causation_id,
            OffsetDateTime::now_utc(),
        )?;
        let expected_version = existing_events.len() as u64;
        match self.store.append(event, Some(expected_version)) {
            Ok(appended) => Ok(CommandOutcome::appended(appended)),
            Err(StoreError::DuplicateIdempotencyKey(idempotency_key)) => {
                let latest_events = self.store.read_all()?;
                if let Some(outcome) = idempotent_replay_outcome(
                    &command_for_idempotency_race,
                    &stream_id,
                    &latest_events,
                ) {
                    return outcome;
                }
                Err(RuntimeError::Store(StoreError::DuplicateIdempotencyKey(
                    idempotency_key,
                )))
            }
            Err(error) => Err(RuntimeError::Store(error)),
        }
    }

    /// Reads all events for an account stream.
    pub fn read_account_events(
        &self,
        tenant_id: &TenantId,
        account_id: &AccountId,
    ) -> Result<Vec<EventEnvelope>, RuntimeError> {
        let stream_id = account_stream_id(tenant_id, account_id)
            .map_err(|error| RuntimeError::InvalidStream(error.to_string()))?;
        self.store
            .read_stream(&stream_id)
            .map_err(RuntimeError::from)
    }

    /// Rebuilds account state from persisted events.
    pub fn account_snapshot(
        &self,
        tenant_id: &TenantId,
        account_id: &AccountId,
    ) -> Result<Option<AccountState>, RuntimeError> {
        let events = self.read_account_events(tenant_id, account_id)?;
        project_account(&events).map_err(RuntimeError::from)
    }

    /// Rebuilds the full in-memory account index.
    pub fn rebuild_index(&self) -> Result<AccountIndex, RuntimeError> {
        let events = self.store.read_all()?;
        AccountIndex::rebuild(&events).map_err(RuntimeError::from)
    }

    /// Verifies store integrity.
    pub fn verify_store(&self) -> Result<StoreVerification, RuntimeError> {
        self.store.verify().map_err(RuntimeError::from)
    }
}

fn idempotent_replay_outcome(
    command: &RuntimeCommand,
    stream_id: &ferrisledger_domain::StreamId,
    events: &[EventEnvelope],
) -> Option<Result<CommandOutcome, RuntimeError>> {
    let key = command.idempotency_key()?;
    let event = events
        .iter()
        .find(|event| event.payload.idempotency_key() == Some(key))?;

    if event.stream_id != *stream_id || !command_matches_idempotent_event(command, event) {
        return Some(Err(RuntimeError::IdempotencyConflict {
            idempotency_key: key.to_string(),
            existing_event_id: event.event_id.clone(),
        }));
    }

    Some(Ok(CommandOutcome {
        event: event.clone(),
        stream_version: stream_version_for_event(events, event),
        global_position: None,
        idempotent_replay: true,
    }))
}

fn command_matches_idempotent_event(command: &RuntimeCommand, event: &EventEnvelope) -> bool {
    match (command, &event.payload) {
        (
            RuntimeCommand::DepositMoney {
                tenant_id,
                account_id,
                amount,
                idempotency_key,
            },
            FinancialEvent::MoneyDeposited(payload),
        ) => {
            tenant_id == &payload.tenant_id
                && account_id == &payload.account_id
                && amount == &payload.amount
                && idempotency_key == &payload.idempotency_key
        }
        (
            RuntimeCommand::RequestPixTransfer {
                tenant_id,
                account_id,
                amount,
                beneficiary_pix_key,
                idempotency_key,
            },
            FinancialEvent::PixTransferRequested(payload),
        ) => {
            tenant_id == &payload.tenant_id
                && account_id == &payload.account_id
                && amount == &payload.amount
                && beneficiary_pix_key == &payload.beneficiary_pix_key
                && idempotency_key == &payload.idempotency_key
        }
        (
            RuntimeCommand::ExecuteSettlement {
                tenant_id,
                account_id,
                amount,
                settlement_id,
                idempotency_key,
            },
            FinancialEvent::SettlementExecuted(payload),
        ) => {
            tenant_id == &payload.tenant_id
                && account_id == &payload.account_id
                && amount == &payload.amount
                && settlement_id == &payload.settlement_id
                && idempotency_key == &payload.idempotency_key
        }
        (
            RuntimeCommand::CreateLedgerEntry {
                tenant_id,
                account_id,
                ledger_entry_id,
                direction,
                amount,
                reason,
                idempotency_key,
                related_event_id,
            },
            FinancialEvent::LedgerEntryCreated(payload),
        ) => {
            tenant_id == &payload.tenant_id
                && account_id == &payload.account_id
                && ledger_entry_id == &payload.ledger_entry_id
                && direction == &payload.direction
                && amount == &payload.amount
                && reason == &payload.reason
                && idempotency_key == &payload.idempotency_key
                && related_event_id == &payload.related_event_id
        }
        (RuntimeCommand::OpenAccount { .. }, _) => false,
        (_, _) => false,
    }
}

fn stream_version_for_event(events: &[EventEnvelope], target: &EventEnvelope) -> u64 {
    let mut stream_version = 0;
    for event in events {
        if event.stream_id == target.stream_id {
            stream_version += 1;
        }
        if event.event_id == target.event_id {
            return stream_version;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{AccountId, IdempotencyKey, Money, TenantId};
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    fn runtime() -> RuntimeService<FileEventStore> {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.keep().join("events.jsonl");
        RuntimeService::file(path)
    }

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
    fn executes_open_and_deposit_with_idempotency() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");

        let command = RuntimeCommand::DepositMoney {
            tenant_id: tenant(),
            account_id: account(),
            amount: Money::new(10_000, "BRL").expect("money"),
            idempotency_key: IdempotencyKey::new("deposit_001").expect("idempotency"),
        };

        let first = runtime.execute(command.clone(), corr()).expect("first");
        let second = runtime.execute(command, corr()).expect("second");

        assert!(!first.idempotent_replay);
        assert!(second.idempotent_replay);
        assert_eq!(first.event.event_id, second.event.event_id);
        assert_eq!(
            runtime
                .account_snapshot(&tenant(), &account())
                .expect("snapshot")
                .expect("account")
                .balance
                .cents(),
            10_000
        );
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_payload() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("deposit_001").expect("idempotency"),
                },
                corr(),
            )
            .expect("first deposit");

        let error = runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(20_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("deposit_001").expect("idempotency"),
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_command_type() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("idem_same").expect("idempotency"),
                },
                corr(),
            )
            .expect("deposit");

        let error = runtime
            .execute(
                RuntimeCommand::RequestPixTransfer {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    beneficiary_pix_key: "email@example.com".to_string(),
                    idempotency_key: IdempotencyKey::new("idem_same").expect("idempotency"),
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_account() {
        let runtime = runtime();
        let other_account = AccountId::new("account_002").expect("account");
        for account_id in [account(), other_account.clone()] {
            runtime
                .execute(
                    RuntimeCommand::OpenAccount {
                        tenant_id: tenant(),
                        account_id,
                        currency: "BRL".to_string(),
                        account_holder_name: "Ada Lovelace".to_string(),
                    },
                    corr(),
                )
                .expect("open");
        }
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("idem_global").expect("idempotency"),
                },
                corr(),
            )
            .expect("deposit");

        let error = runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: other_account,
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("idem_global").expect("idempotency"),
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_pix_beneficiary() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("deposit_for_pix").expect("idempotency"),
                },
                corr(),
            )
            .expect("deposit");
        runtime
            .execute(
                RuntimeCommand::RequestPixTransfer {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    beneficiary_pix_key: "first@example.com".to_string(),
                    idempotency_key: IdempotencyKey::new("pix_same").expect("idempotency"),
                },
                corr(),
            )
            .expect("pix");

        let error = runtime
            .execute(
                RuntimeCommand::RequestPixTransfer {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    beneficiary_pix_key: "second@example.com".to_string(),
                    idempotency_key: IdempotencyKey::new("pix_same").expect("idempotency"),
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_settlement_id() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(10_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new("deposit_for_settlement")
                        .expect("idempotency"),
                },
                corr(),
            )
            .expect("deposit");
        runtime
            .execute(
                RuntimeCommand::RequestPixTransfer {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    beneficiary_pix_key: "beneficiary@example.com".to_string(),
                    idempotency_key: IdempotencyKey::new("pix_for_settlement")
                        .expect("idempotency"),
                },
                corr(),
            )
            .expect("pix");
        runtime
            .execute(
                RuntimeCommand::ExecuteSettlement {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    settlement_id: ferrisledger_domain::SettlementId::new("settlement_001")
                        .expect("settlement"),
                    idempotency_key: IdempotencyKey::new("settle_same").expect("idempotency"),
                },
                corr(),
            )
            .expect("settlement");

        let error = runtime
            .execute(
                RuntimeCommand::ExecuteSettlement {
                    tenant_id: tenant(),
                    account_id: account(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    settlement_id: ferrisledger_domain::SettlementId::new("settlement_002")
                        .expect("settlement"),
                    idempotency_key: IdempotencyKey::new("settle_same").expect("idempotency"),
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn rejects_idempotency_key_reuse_with_different_ledger_reason() {
        let runtime = runtime();
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");
        runtime
            .execute(
                RuntimeCommand::CreateLedgerEntry {
                    tenant_id: tenant(),
                    account_id: account(),
                    ledger_entry_id: ferrisledger_domain::LedgerEntryId::new("ledger_001")
                        .expect("ledger"),
                    direction: ferrisledger_events::LedgerDirection::Credit,
                    amount: Money::new(1_000, "BRL").expect("money"),
                    reason: "manual adjustment".to_string(),
                    idempotency_key: IdempotencyKey::new("ledger_same").expect("idempotency"),
                    related_event_id: None,
                },
                corr(),
            )
            .expect("ledger");

        let error = runtime
            .execute(
                RuntimeCommand::CreateLedgerEntry {
                    tenant_id: tenant(),
                    account_id: account(),
                    ledger_entry_id: ferrisledger_domain::LedgerEntryId::new("ledger_001")
                        .expect("ledger"),
                    direction: ferrisledger_events::LedgerDirection::Credit,
                    amount: Money::new(1_000, "BRL").expect("money"),
                    reason: "different reason".to_string(),
                    idempotency_key: IdempotencyKey::new("ledger_same").expect("idempotency"),
                    related_event_id: None,
                },
                corr(),
            )
            .expect_err("idempotency conflict");

        assert!(matches!(error, RuntimeError::IdempotencyConflict { .. }));
    }

    #[test]
    fn concurrent_same_idempotent_command_replays_after_store_race() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        RuntimeService::file(path.clone())
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(),
                    account_id: account(),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                corr(),
            )
            .expect("open");

        let barrier = Arc::new(Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let runtime = RuntimeService::file(path);
                    let command = RuntimeCommand::DepositMoney {
                        tenant_id: tenant(),
                        account_id: account(),
                        amount: Money::new(10_000, "BRL").expect("money"),
                        idempotency_key: IdempotencyKey::new("deposit_concurrent")
                            .expect("idempotency"),
                    };
                    barrier.wait();
                    runtime.execute(command, corr()).expect("deposit")
                })
            })
            .collect::<Vec<_>>();

        let mut outcomes = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread"))
            .collect::<Vec<_>>();
        outcomes.sort_by_key(|outcome| outcome.idempotent_replay);

        assert_eq!(outcomes[0].event.event_id, outcomes[1].event.event_id);
        assert!(!outcomes[0].idempotent_replay);
        assert!(outcomes[1].idempotent_replay);
    }
}
