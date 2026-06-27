//! Append-only event storage with checksum verification.

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ferrisledger_domain::{EventId, IdempotencyKey, StreamId};
use ferrisledger_events::EventEnvelope;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Store-level failures.
#[derive(Debug, Error)]
pub enum StoreError {
    /// File-system operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON encoding/decoding failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A line in the append-only log failed checksum validation.
    #[error("corrupt event log at line {line}: expected checksum {expected}, got {actual}")]
    ChecksumMismatch {
        /// One-based line number.
        line: usize,
        /// Checksum stored with the record.
        expected: u32,
        /// Checksum calculated from the decoded envelope.
        actual: u32,
    },
    /// A decoded event had a duplicate event ID.
    #[error("duplicate event id {0}")]
    DuplicateEventId(EventId),
    /// A decoded event had a duplicate idempotency key.
    #[error("duplicate idempotency key {0}")]
    DuplicateIdempotencyKey(IdempotencyKey),
    /// A decoded log record duplicated an existing event ID.
    #[error(
        "corrupt event log at line {line}: duplicate event id {event_id} first seen at line {first_line}"
    )]
    CorruptDuplicateEventId {
        /// One-based line number where the duplicate was found.
        line: usize,
        /// One-based line number where the original event ID first appeared.
        first_line: usize,
        /// Duplicated event identifier.
        event_id: EventId,
    },
    /// A decoded log record duplicated an existing idempotency key.
    #[error(
        "corrupt event log at line {line}: duplicate idempotency key {idempotency_key} first seen at line {first_line}"
    )]
    CorruptDuplicateIdempotencyKey {
        /// One-based line number where the duplicate was found.
        line: usize,
        /// One-based line number where the original idempotency key first appeared.
        first_line: usize,
        /// Duplicated idempotency key.
        idempotency_key: IdempotencyKey,
    },
    /// A decoded log record violated the persisted envelope contract.
    #[error("corrupt event log at line {line}: {reason}")]
    InvalidEventContract {
        /// One-based line number where the invalid record was found.
        line: usize,
        /// Human-readable contract failure reason.
        reason: String,
    },
    /// Optimistic stream version check failed.
    #[error("stream version conflict: expected {expected}, actual {actual}")]
    VersionConflict {
        /// Expected stream version.
        expected: u64,
        /// Actual stream version.
        actual: u64,
    },
}

/// Durable append-only store contract.
pub trait EventStore: Send + Sync {
    /// Append an event, optionally checking the expected stream version first.
    fn append(
        &self,
        envelope: EventEnvelope,
        expected_stream_version: Option<u64>,
    ) -> Result<AppendedEvent, StoreError>;

    /// Read a single stream in append order.
    fn read_stream(&self, stream_id: &StreamId) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Read all events in append order.
    fn read_all(&self) -> Result<Vec<EventEnvelope>, StoreError>;

    /// Verify that the full log can be decoded and checksummed.
    fn verify(&self) -> Result<StoreVerification, StoreError>;
}

/// Result returned after a successful append.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendedEvent {
    /// Persisted envelope.
    pub envelope: EventEnvelope,
    /// One-based version in the stream after append.
    pub stream_version: u64,
    /// One-based global append position after append.
    pub global_position: u64,
}

/// Verification summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StoreVerification {
    /// Number of records verified.
    pub records: u64,
    /// Number of distinct streams.
    pub streams: u64,
}

/// JSONL file-backed event store.
#[derive(Clone, Debug)]
pub struct FileEventStore {
    path: Arc<PathBuf>,
    lock: Arc<Mutex<()>>,
}

impl FileEventStore {
    /// Creates a store for the given JSONL path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Arc::new(path.into()),
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Returns the backing file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn ensure_parent(&self) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl EventStore for FileEventStore {
    fn append(
        &self,
        envelope: EventEnvelope,
        expected_stream_version: Option<u64>,
    ) -> Result<AppendedEvent, StoreError> {
        let _guard = self.lock.lock().expect("event store lock poisoned");
        self.ensure_parent()?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(self.path.as_ref())?;
        file.lock_exclusive()?;
        let all = read_records_from_file(&mut file)?;

        if all
            .iter()
            .any(|record| record.envelope.event_id == envelope.event_id)
        {
            return Err(StoreError::DuplicateEventId(envelope.event_id));
        }
        if let Some(idempotency_key) = envelope.payload.idempotency_key()
            && all
                .iter()
                .any(|record| record.envelope.payload.idempotency_key() == Some(idempotency_key))
        {
            return Err(StoreError::DuplicateIdempotencyKey(idempotency_key.clone()));
        }

        let stream_version = all
            .iter()
            .filter(|record| record.envelope.stream_id == envelope.stream_id)
            .count() as u64;
        if let Some(expected) = expected_stream_version
            && expected != stream_version
        {
            return Err(StoreError::VersionConflict {
                expected,
                actual: stream_version,
            });
        }

        let record = StoredRecord::from_envelope(envelope.clone())?;
        serde_json::to_writer(&mut file, &record)?;
        file.write_all(b"\n")?;
        file.sync_data()?;

        Ok(AppendedEvent {
            envelope,
            stream_version: stream_version + 1,
            global_position: all.len() as u64 + 1,
        })
    }

    fn read_stream(&self, stream_id: &StreamId) -> Result<Vec<EventEnvelope>, StoreError> {
        read_records_shared(&self.path).map(|records| {
            records
                .into_iter()
                .filter_map(|record| {
                    if &record.envelope.stream_id == stream_id {
                        Some(record.envelope)
                    } else {
                        None
                    }
                })
                .collect()
        })
    }

    fn read_all(&self) -> Result<Vec<EventEnvelope>, StoreError> {
        read_records_shared(&self.path)
            .map(|records| records.into_iter().map(|record| record.envelope).collect())
    }

    fn verify(&self) -> Result<StoreVerification, StoreError> {
        let records = read_records_shared(&self.path)?;
        let mut streams = std::collections::BTreeSet::new();
        for record in &records {
            streams.insert(record.envelope.stream_id.clone());
        }
        Ok(StoreVerification {
            records: records.len() as u64,
            streams: streams.len() as u64,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRecord {
    checksum: u32,
    envelope: EventEnvelope,
}

impl StoredRecord {
    fn from_envelope(envelope: EventEnvelope) -> Result<Self, StoreError> {
        Ok(Self {
            checksum: checksum(&envelope)?,
            envelope,
        })
    }

    fn verify(&self, line: usize) -> Result<(), StoreError> {
        let actual = checksum(&self.envelope)?;
        if actual == self.checksum {
            return Ok(());
        }
        Err(StoreError::ChecksumMismatch {
            line,
            expected: self.checksum,
            actual,
        })
    }
}

fn read_records_shared(path: &Path) -> Result<Vec<StoredRecord>, StoreError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    file.lock_shared()?;
    read_records_from_file(&mut file)
}

fn read_records_from_file(file: &mut File) -> Result<Vec<StoredRecord>, StoreError> {
    file.seek(SeekFrom::Start(0))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut seen_event_ids = HashMap::new();
    let mut seen_idempotency_keys = HashMap::new();
    for (idx, line) in reader.lines().enumerate() {
        let line_number = idx + 1;
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: StoredRecord = serde_json::from_str(&line)?;
        record.verify(line_number)?;
        record
            .envelope
            .validate_persisted()
            .map_err(|error| StoreError::InvalidEventContract {
                line: line_number,
                reason: error.to_string(),
            })?;
        if let Some(first_line) =
            seen_event_ids.insert(record.envelope.event_id.clone(), line_number)
        {
            return Err(StoreError::CorruptDuplicateEventId {
                line: line_number,
                first_line,
                event_id: record.envelope.event_id.clone(),
            });
        }
        if let Some(idempotency_key) = record.envelope.payload.idempotency_key()
            && let Some(first_line) =
                seen_idempotency_keys.insert(idempotency_key.clone(), line_number)
        {
            return Err(StoreError::CorruptDuplicateIdempotencyKey {
                line: line_number,
                first_line,
                idempotency_key: idempotency_key.clone(),
            });
        }
        records.push(record);
    }
    Ok(records)
}

fn checksum(envelope: &EventEnvelope) -> Result<u32, StoreError> {
    let bytes = serde_json::to_vec(envelope)?;
    Ok(crc32fast::hash(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrisledger_domain::{
        AccountId, CorrelationId, IdempotencyKey, Money, TenantId, account_stream_id,
    };
    use ferrisledger_events::{
        AccountOpened, EventEnvelope, EventMetadata, FinancialEvent, MoneyDeposited,
    };
    use proptest::prelude::*;
    use std::{
        sync::{Arc, Barrier},
        thread,
    };

    fn append_raw_record(path: &Path, record: &StoredRecord) {
        let mut file = OpenOptions::new().append(true).open(path).expect("open");
        serde_json::to_writer(&mut file, record).expect("serialize");
        file.write_all(b"\n").expect("newline");
    }

    fn overwrite_raw_bytes(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("write");
    }

    fn assert_corrupt_read<T: std::fmt::Debug>(result: Result<T, StoreError>, context: &str) {
        assert!(
            matches!(
                result,
                Err(StoreError::Io(_))
                    | Err(StoreError::Json(_))
                    | Err(StoreError::ChecksumMismatch { .. })
                    | Err(StoreError::InvalidEventContract { .. })
                    | Err(StoreError::CorruptDuplicateEventId { .. })
                    | Err(StoreError::CorruptDuplicateIdempotencyKey { .. })
            ),
            "expected corruption, got {result:?}; {context}"
        );
    }

    fn opened_event() -> EventEnvelope {
        opened_event_for("account_001")
    }

    fn opened_event_for(account: &str) -> EventEnvelope {
        let tenant_id = TenantId::new("tenant_001").expect("tenant");
        let account_id = AccountId::new(account).expect("account");
        let payload = FinancialEvent::AccountOpened(AccountOpened {
            tenant_id: tenant_id.clone(),
            account_id: account_id.clone(),
            currency: "BRL".to_string(),
            account_holder_name: "Ada Lovelace".to_string(),
        });
        EventEnvelope::new(
            payload,
            EventMetadata::new(
                account_stream_id(&tenant_id, &account_id).expect("stream"),
                CorrelationId::new("corr_001").expect("correlation"),
            ),
        )
        .expect("envelope")
    }

    fn deposit_event(idempotency_key: &str) -> EventEnvelope {
        let tenant_id = TenantId::new("tenant_001").expect("tenant");
        let account_id = AccountId::new("account_001").expect("account");
        let payload = FinancialEvent::MoneyDeposited(MoneyDeposited {
            tenant_id: tenant_id.clone(),
            account_id: account_id.clone(),
            amount: Money::new(1_000, "BRL").expect("money"),
            idempotency_key: IdempotencyKey::new(idempotency_key).expect("idempotency"),
        });
        EventEnvelope::new(
            payload,
            EventMetadata::new(
                account_stream_id(&tenant_id, &account_id).expect("stream"),
                CorrelationId::new("corr_001").expect("correlation"),
            ),
        )
        .expect("envelope")
    }

    #[test]
    fn appends_and_reads_stream_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileEventStore::new(dir.path().join("events.jsonl"));
        let event = opened_event();
        let stream_id = event.stream_id.clone();

        let appended = store.append(event.clone(), Some(0)).expect("append");

        assert_eq!(appended.stream_version, 1);
        assert_eq!(store.read_stream(&stream_id).expect("stream"), vec![event]);
        assert_eq!(
            store.verify().expect("verify"),
            StoreVerification {
                records: 1,
                streams: 1,
            }
        );
    }

    #[test]
    fn rejects_wrong_expected_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileEventStore::new(dir.path().join("events.jsonl"));
        let event = opened_event();

        let error = store.append(event, Some(1)).expect_err("version conflict");

        assert!(matches!(
            error,
            StoreError::VersionConflict {
                expected: 1,
                actual: 0
            }
        ));
    }

    #[test]
    fn rejects_duplicate_idempotency_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FileEventStore::new(dir.path().join("events.jsonl"));

        store
            .append(deposit_event("idem_001"), None)
            .expect("first append");
        let error = store
            .append(deposit_event("idem_001"), None)
            .expect_err("duplicate idempotency");

        assert!(matches!(
            error,
            StoreError::DuplicateIdempotencyKey(idempotency_key)
                if idempotency_key.as_str() == "idem_001"
        ));
    }

    #[test]
    fn detects_corrupt_checksum() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let store = FileEventStore::new(&path);
        store.append(opened_event(), Some(0)).expect("append");

        let contents = std::fs::read_to_string(&path).expect("read");
        let corrupted = contents.replacen("\"checksum\":", "\"checksum\":1", 1);
        std::fs::write(&path, corrupted).expect("write corrupt");

        assert!(matches!(
            store.verify(),
            Err(StoreError::Json(_)) | Err(StoreError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn verify_rejects_unknown_optional_envelope_field_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let store = FileEventStore::new(&path);
        store.append(opened_event(), Some(0)).expect("append");

        let raw = std::fs::read_to_string(&path).expect("read");
        let corrupted = raw.replacen("\"causation_id\":null", "\" ausation_id\":null", 1);
        std::fs::write(&path, corrupted).expect("write");

        assert!(matches!(store.verify(), Err(StoreError::Json(_))));
        assert!(matches!(store.read_all(), Err(StoreError::Json(_))));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn single_byte_mutation_of_persisted_record_fails_closed(
            position in any::<usize>(),
            replacement in any::<u8>(),
        ) {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("events.jsonl");
            let store = FileEventStore::new(&path);
            store.append(opened_event(), Some(0)).expect("append");

            let mut raw = std::fs::read(&path).expect("read");
            let record_len = raw
                .iter()
                .position(|byte| *byte == b'\n')
                .expect("newline");
            let byte_index = position % record_len;
            let original = raw[byte_index];
            raw[byte_index] = if replacement == original {
                replacement.wrapping_add(1)
            } else {
                replacement
            };
            overwrite_raw_bytes(&path, &raw);
            let context = format!(
                "index={byte_index}, original_byte={original}, replacement_byte={}, raw={}",
                raw[byte_index],
                String::from_utf8_lossy(&raw),
            );

            assert_corrupt_read(store.verify(), &context);
            assert_corrupt_read(store.read_all(), &context);
        }
    }

    #[test]
    fn verify_rejects_duplicate_event_id_in_persisted_log() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let store = FileEventStore::new(&path);
        let event = opened_event();
        let stream_id = event.stream_id.clone();
        store.append(event, Some(0)).expect("append");

        let duplicate = serde_json::from_str::<StoredRecord>(
            std::fs::read_to_string(&path)
                .expect("read")
                .lines()
                .last()
                .expect("line"),
        )
        .expect("record");
        append_raw_record(&path, &duplicate);

        assert!(matches!(
            store.verify(),
            Err(StoreError::CorruptDuplicateEventId {
                line: 2,
                first_line: 1,
                ..
            })
        ));
        assert!(matches!(
            store.read_stream(&stream_id),
            Err(StoreError::CorruptDuplicateEventId {
                line: 2,
                first_line: 1,
                ..
            })
        ));
    }

    #[test]
    fn verify_rejects_rechecksummed_stream_id_drift() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let store = FileEventStore::new(&path);
        store.append(opened_event(), Some(0)).expect("append");

        let mut record = serde_json::from_str::<StoredRecord>(
            std::fs::read_to_string(&path)
                .expect("read")
                .lines()
                .last()
                .expect("line"),
        )
        .expect("record");
        let drift_account = AccountId::new("account_999").expect("account");
        record.envelope.stream_id =
            account_stream_id(record.envelope.payload.tenant_id(), &drift_account)
                .expect("drift stream");
        record.checksum = checksum(&record.envelope).expect("checksum");
        let mut bytes = serde_json::to_vec(&record).expect("serialize");
        bytes.push(b'\n');
        overwrite_raw_bytes(&path, &bytes);

        assert!(matches!(
            store.verify(),
            Err(StoreError::InvalidEventContract { line: 1, reason })
                if reason.contains("stream_id")
        ));
        assert!(matches!(
            store.read_all(),
            Err(StoreError::InvalidEventContract { line: 1, reason })
                if reason.contains("stream_id")
        ));
    }

    #[test]
    fn verify_rejects_duplicate_idempotency_key_in_persisted_log() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let store = FileEventStore::new(&path);

        store.append(opened_event(), Some(0)).expect("open");
        store
            .append(deposit_event("idem_001"), Some(1))
            .expect("deposit");

        let mut duplicate = serde_json::from_str::<StoredRecord>(
            std::fs::read_to_string(&path)
                .expect("read")
                .lines()
                .last()
                .expect("line"),
        )
        .expect("record");
        duplicate.envelope.event_id = EventId::new("evt_tampered").expect("event id");
        duplicate.checksum = checksum(&duplicate.envelope).expect("checksum");
        append_raw_record(&path, &duplicate);

        assert!(matches!(
            store.verify(),
            Err(StoreError::CorruptDuplicateIdempotencyKey {
                line: 3,
                first_line: 2,
                ..
            })
        ));
        assert!(matches!(
            store.read_all(),
            Err(StoreError::CorruptDuplicateIdempotencyKey {
                line: 3,
                first_line: 2,
                ..
            })
        ));
    }

    #[test]
    fn independent_store_handles_coordinate_concurrent_appends() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("events.jsonl");
        let thread_count = 20;
        let barrier = Arc::new(Barrier::new(thread_count));
        let mut handles = Vec::new();

        for index in 0..thread_count {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                let store = FileEventStore::new(path);
                let event = opened_event_for(&format!("account_{index:03}"));
                barrier.wait();
                store
                    .append(event, Some(0))
                    .expect("append")
                    .global_position
            }));
        }

        let mut positions = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread"))
            .collect::<Vec<_>>();
        positions.sort_unstable();

        let store = FileEventStore::new(path);
        assert_eq!(positions, (1..=thread_count as u64).collect::<Vec<_>>());
        assert_eq!(store.verify().expect("verify").records, thread_count as u64);
        assert_eq!(store.read_all().expect("events").len(), thread_count);
    }
}
