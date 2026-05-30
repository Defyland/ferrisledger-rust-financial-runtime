//! Application service for append, replay, and idempotency.

use ferrisledger_domain::{AccountId, AccountState, CorrelationId, TenantId, account_stream_id};
use ferrisledger_events::EventEnvelope;
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
        let existing_events = self.store.read_stream(&stream_id)?;

        if let Some(key) = command.idempotency_key()
            && let Some((index, event)) = existing_events
                .iter()
                .enumerate()
                .find(|(_, event)| event.payload.idempotency_key() == Some(key))
        {
            return Ok(CommandOutcome {
                event: event.clone(),
                stream_version: index as u64 + 1,
                global_position: None,
                idempotent_replay: true,
            });
        }

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
        let appended = self.store.append(event, Some(expected_version))?;
        Ok(CommandOutcome::appended(appended))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{AccountId, IdempotencyKey, Money, TenantId};

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
}
