//! Shared helpers for integration and benchmark tests.

use std::path::PathBuf;

use ferrisledger_domain::{AccountId, CorrelationId, TenantId};
use ferrisledger_runtime::RuntimeService;
use ferrisledger_store::FileEventStore;

/// Temporary runtime fixture.
pub struct RuntimeFixture {
    _tempdir: tempfile::TempDir,
    /// Event log path.
    pub store_path: PathBuf,
    /// Runtime service.
    pub runtime: RuntimeService<FileEventStore>,
}

impl RuntimeFixture {
    /// Creates a temporary file-backed runtime.
    pub fn new() -> std::io::Result<Self> {
        let tempdir = tempfile::tempdir()?;
        let store_path = tempdir.path().join("events.jsonl");
        let runtime = RuntimeService::file(store_path.clone());
        Ok(Self {
            _tempdir: tempdir,
            store_path,
            runtime,
        })
    }
}

/// Stable tenant fixture.
#[must_use]
pub fn tenant_id() -> TenantId {
    TenantId::new("tenant_001").expect("valid fixture tenant")
}

/// Stable account fixture.
#[must_use]
pub fn account_id() -> AccountId {
    AccountId::new("account_001").expect("valid fixture account")
}

/// Stable correlation fixture.
#[must_use]
pub fn correlation_id() -> CorrelationId {
    CorrelationId::new("corr_001").expect("valid fixture correlation")
}
