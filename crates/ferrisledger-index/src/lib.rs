//! In-memory projections built from append-only events.

use std::collections::BTreeMap;

use ferrisledger_domain::{AccountId, AccountState, StreamId, TenantId, account_stream_id};
use ferrisledger_events::EventEnvelope;
use ferrisledger_rules::{RuleError, project_account};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Projection index errors.
#[derive(Debug, Error)]
pub enum IndexError {
    /// Domain/rule projection failed.
    #[error(transparent)]
    Rule(#[from] RuleError),
    /// Stream ID could not be constructed.
    #[error("invalid stream id: {0}")]
    InvalidStream(String),
}

/// Materialized account index.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountIndex {
    accounts: BTreeMap<StreamId, AccountState>,
}

impl AccountIndex {
    /// Rebuilds the index from all events.
    pub fn rebuild(events: &[EventEnvelope]) -> Result<Self, IndexError> {
        let mut grouped: BTreeMap<StreamId, Vec<EventEnvelope>> = BTreeMap::new();
        for event in events {
            grouped
                .entry(event.stream_id.clone())
                .or_default()
                .push(event.clone());
        }

        let mut accounts = BTreeMap::new();
        for (stream_id, events) in grouped {
            if let Some(state) = project_account(&events)? {
                accounts.insert(stream_id, state);
            }
        }

        Ok(Self { accounts })
    }

    /// Returns a projected account state.
    pub fn get(
        &self,
        tenant_id: &TenantId,
        account_id: &AccountId,
    ) -> Result<Option<&AccountState>, IndexError> {
        let stream_id = account_stream_id(tenant_id, account_id)
            .map_err(|error| IndexError::InvalidStream(error.to_string()))?;
        Ok(self.accounts.get(&stream_id))
    }

    /// Returns the number of projected accounts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Returns true when no accounts are projected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }
}
