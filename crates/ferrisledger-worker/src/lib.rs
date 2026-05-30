//! Async worker helpers for rebuilding projections from the append-only store.

use std::time::Duration;

use ferrisledger_index::AccountIndex;
use ferrisledger_runtime::{RuntimeError, RuntimeService};
use ferrisledger_store::EventStore;
use thiserror::Error;
use tokio::sync::watch;
use tracing::info;

/// Worker failures.
#[derive(Debug, Error)]
pub enum WorkerError {
    /// Runtime operation failed.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

/// Rebuilds materialized account projections.
#[derive(Clone, Debug)]
pub struct ProjectionWorker<S> {
    runtime: RuntimeService<S>,
}

impl<S> ProjectionWorker<S>
where
    S: EventStore,
{
    /// Creates a worker from the runtime service.
    #[must_use]
    pub const fn new(runtime: RuntimeService<S>) -> Self {
        Self { runtime }
    }

    /// Runs one rebuild pass.
    pub fn rebuild_once(&self) -> Result<AccountIndex, WorkerError> {
        let index = self.runtime.rebuild_index()?;
        info!(accounts = index.len(), "projection index rebuilt");
        Ok(index)
    }

    /// Rebuilds periodically until shutdown is signaled.
    pub async fn run_until_shutdown(
        &self,
        interval: Duration,
        mut shutdown: watch::Receiver<bool>,
    ) -> Result<(), WorkerError> {
        let mut ticker = tokio::time::interval(interval);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.rebuild_once()?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        info!("projection worker shutdown");
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{AccountId, CorrelationId, TenantId};
    use ferrisledger_rules::RuntimeCommand;
    use ferrisledger_runtime::RuntimeService;

    #[test]
    fn rebuild_once_materializes_accounts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let runtime = RuntimeService::file(dir.path().join("events.jsonl"));
        runtime
            .execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: TenantId::new("tenant_001").expect("tenant"),
                    account_id: AccountId::new("account_001").expect("account"),
                    currency: "BRL".to_string(),
                    account_holder_name: "Ada Lovelace".to_string(),
                },
                CorrelationId::new("corr_001").expect("correlation"),
            )
            .expect("open account");

        let worker = ProjectionWorker::new(runtime);
        let index = worker.rebuild_once().expect("rebuild");

        assert_eq!(index.len(), 1);
    }
}
