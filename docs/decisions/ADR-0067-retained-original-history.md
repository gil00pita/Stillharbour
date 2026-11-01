# ADR-0067: Read-only retained original-operation review

Date: 2026-11-01
Status: experimental implementation; full/native validation pending; M2 remains open

The pure graph validator compiles in normal builds for defensive catalog reads. Reverse planning, generated name probes and all native execution remain under their existing gates; no second graph validator is introduced.

Expose a bounded catalog-only organization-history query by validated job UUID. The backend verifies the retained payload hash, catalog/root/job identity, schema policy, source object number encoding, reverse relationship and exact canonical rename graph before returning saved Before/After members. Imported records carry no authority. Unknown fields, mismatched identities, altered graphs or capabilities and oversized records fail closed. History remains available when a registered root is unavailable because it does not inspect or adopt the filesystem.

Return explicit history-only, current-locations-unchecked and execution-unavailable flags. The Jobs review shows the recorded attempt, planned paths, retained digest and reverse relationship in bounded pages. It has no approval, resume, rollback or file-opening control. Closing, changing job identity or unmounting retires pending responses. Paths render as text. Errors require an explicit retry; a failed read never becomes a successful empty table.

The generated rename lab uses the same review component and backend validator over its constructor-owned private catalog, through an instrumented command excluded from production. This keeps native UI evidence separate from the ordinary application's catalog history. No new original-file mutation path, persistent approval, schema or dependency is introduced.

This advances recovery discovery and historical review. It does not establish current native state, general runtime mutation adoption, independent recovery, physical power-loss durability or M2 completion. The existing cold generated process-exit tests and actual native continuation approvals remain separate evidence. Keep production mutation and duplicate quarantine disabled.

Run focused defensive-history/core and frontend state regressions, full checks, instrumented native review and production exclusions. Record actual results and failures in the implementation log before the next authorized checkpoint.
