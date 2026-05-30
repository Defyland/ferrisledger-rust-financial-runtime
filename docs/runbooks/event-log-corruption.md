# Event Log Corruption

Use this runbook when a FerrisLedger segment fails checksum verification or replay cannot continue.

## Triage

- Identify the segment file and offset that failed verification.
- Confirm whether the failure is a partial write, checksum mismatch, or unsupported schema version.
- Check whether earlier events in the same stream still deserialize correctly.
- Verify whether a snapshot can reconstruct state before the corrupt boundary.

## Recovery

- Preserve the raw segment before any repair attempt.
- Truncate only after confirming the corrupt portion is beyond the last valid record boundary.
- Rebuild indexes from the last valid segment state.
- Record the failure in the unsafe or storage notes if the corruption came from low-level I/O assumptions.
