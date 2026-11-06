# ADR-0094: Utility setup, background reconciliation and report history

Date: 2026-11-05. Status: accepted for the owner-requested utility lifecycle.

## Context

The existing inspections retain immutable catalog findings but mix displayed and running state. First use shows an empty duplicate workspace before setup. Backup comparison retains one live session review and separate bounded destination-digest observations, which cannot reconstruct old reports.

## Decision

Use first-use welcome/setup, separate displayed results from running work, and reconcile read-only on returning to a utility. This checks actual bytes/inventory through existing bounded workers instead of treating unchanged timestamps as proof of freshness. Reconciliation is page-entry behavior, not a new continuous filesystem watcher. Preserve the prior result during work and on failure. Selecting history pins that displayed version independently of polling.

Never replay integrity baseline establishment: refresh through baseline comparison. Keep single-inspection admission, native root identity/containment, shared I/O pacing and explicit first-scan configuration. Offline roots and conflicting active work leave historical results available with an explanation.

Store display-only backup reports as version-1 JSON under scope `utility-report-v1` and keys `report:<run UUID>` in the existing immutable `backup_comparison_history` table. This adds a separate payload namespace without changing schema, triggers, persisted identities or destination-byte manifest format. Destination-observation capacity counting excludes display reports, retaining its existing 32-observation allowance. Display reports have their own 32-record/16-MiB-per-record bound and no expiry. Preserve earlier reports when full; surface unsaved-session status. Quick/incomplete/failure reports cannot become verified destination-byte baselines.

Validate record version, ID, catalog identity, terminal state, byte bound and entry bound on read. Strip plan digest, review plan and execution/fidelity/recovery capabilities both before persistence and after decoding. List at most 32 report summaries and reuse 200-entry filtered paging for findings. Comparison completion is published only after the worker releases its scope lease, so a terminal result admits an immediate safe follow-up. Live preview, cancellation and all transfer/recovery authority remain in the current native review; historical paths are display strings only. History errors remain typed failures, not empty success.

On return, a saved comparison profile can reauthorize its native bindings for a new read-only comparison. An unsaved destination selection cannot be reconstructed from a display path; after restart show its report and require the native picker. Block automatic comparison while existing transfers need attention. Encrypted snapshot creation/restore and credential flows remain explicit separate workflows.

## Consequences and limits

Entering a previously scanned utility performs fresh read I/O, even when the library has not changed; existing pacing/bounds apply. It makes no claim of detecting changes while the user stays on the page. No timer, cleanup, history expiry or production mutation is introduced. Inspection history retains the existing newest-64-job listing. Backup history capacity is finite and explicitly disclosed. A catalog snapshot contains reports, not original bytes or independent recovery evidence. Both-platform release/preservation gates remain unchanged.

## Validation

Component regressions cover first use, retained results during actual start/poll calls, history pinning, offline behavior and compare-only baseline refresh. Rust generated fixtures cover persistent reports, archived paging and absent historical execution authority. The generated native lifecycle case covers each utility and unchanged fixture manifests. Record actual executed outcomes in IMPLEMENTATION_LOG.md; this ADR is not test evidence.
