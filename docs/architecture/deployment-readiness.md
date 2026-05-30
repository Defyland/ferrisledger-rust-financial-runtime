# Deployment Readiness

FerrisLedger will eventually run as a combination of CLI tooling, an HTTP API, background workers, and a local or containerized storage runtime.

## Current posture

- Phase 0 documents the runtime boundaries and learning goals.
- Axum, Tokio, tracing, benchmarks, and property testing are planned but not implemented yet.
- Workspace-level crate separation is part of the operational design, not only a code-organization preference.

## Deferred platform work

- Kubernetes manifests are deferred until the API, worker, and storage processes exist.
- SQL or external metadata stores are optional later phases and should remain behind feature flags.
- Unsafe and FFI remain tightly scoped until profiling proves they are needed in the runtime path.
