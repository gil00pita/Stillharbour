> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0049 — Shared admission and fresh keeper checks for import copies

Date: 2026-11-01. Status: private M2 implementation; complete checks and Windows/macOS native regression pass in CI run 41260412572.

## Decision

Use the same complete-byte keeper verifier for all-existing imports and projected copy workflows. Acquire source read, destination write and all explicit comparison-root read scopes atomically. Retain that lease through preparation/copy and the final keeper recheck. These process-local locks coordinate owned work; native identity/version/membership checks still detect external changes.

Before native work and after successful preparation or copying, recheck each chosen existing member against its original source through paced complete-byte comparison. Revalidate current root availability, indexed keeper identity/revision/presence, primary-media status, native parents and retained comparison digest. Cancellation or failed verification cannot report import completion. Source-volume crossings and source/keeper root aliases require a separate reviewed scope.

Include comparison roots in active-job maintenance cancellation. Check maintenance admission before reserving work and again after publishing the active cancellation handle. Recovery continuation uses the same lease and verification rules; retained journals grant no skip permission.

## Validation and limits

Generated fixtures test atomic admission without partial holds, keeper write exclusion and release, repeated fresh payload reads, changed keeper bytes, inaccessible indexed keepers and cancellation before payload reads or namespace creation. Windows picker paths in the lock test are canonicalized before exact component lookup, matching production admission; native name checks are unchanged.

The projection constructor remains test-only. Application mixed-import review, subset capacity, durable combined receipts and native UI acceptance remain open. No schema, dependency, default feature or original mutation permission changes. M2 remains unfinished and gates M3. Actual check outcomes are recorded in the implementation log.
