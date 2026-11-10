> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0042 — Application review and approval of retained copy continuation

Date: 2026-10-31. Status: application integration behind copy-workflow; local Windows validation passes, expanded macOS UI scenario pending CI.

## Decision

Create a new bounded live continuation review from an existing stopped job and a fresh private native destination selection. Reopen and validate its existing stream, source snapshot and prepared namespace. Display exact native source-relative/output paths, recorded phases, retained lengths and the fresh-verification requirement. No imported history restores authority. Separate fidelity acknowledgment, exact one-use ResumeFiles approval and explicit start remain mandatory. Default copying/original mutation gates stay unchanged.

Hold a bounded native recovery inventory seal: every recorded directory retains its identity/membership and every original stage object exists at exactly its staged or published name. Reject unknown, missing, linked, foreign or ambiguous objects. Revalidate the seal at scope boundaries. Windows uses a data-rights-free attribute handle compatible with the executor's publication handle; source read sharing remains strict. Do not repeatedly decode the entire retained preparation journal inside the copy loop.

Bind continuation job scope to the new observation/history digest. In one catalog transaction retain its bounded review record, prior-job/stream/binding link, queued job and state fact before native effects. Limit continuation jobs to 64 per stream and live continuation reviews to 64. Retained inspection validates the continuation record defensively against its prepared plan and digest; reading that record never creates an approval. No schema migration or new dependency is needed.

Require fresh source/companion assessments and unchanged source snapshots. Estimate additional allocation from the remaining append ranges, conservatively including their boundary allocation unit. Published or completely staged data needs no new data allowance; metadata, journal and headroom allowances remain explicit estimates. Recheck live output/application-journal identity and available space at admission/runtime. Capacity observation is not a reservation.

Use the same bounded worker, native leases, pacing, cancel and maintenance gate as copying. Recheck roots, inventory and space before continuation; the engine rechecks the exact approved history and observations. Count progress only after a fresh comparison in this attempt, including previously completed files. Retain original completion history; completed streams become freshly verified no-write retries. Worker errors, spawn failure, canceled and stopped-worker recovery remain explicit and retain all files.

Expose continuation from retained Jobs inspection. Ignore stale/unmounted responses, reject foreign scopes/jobs, keep approval/start separate and show recorded state separately from fresh verification. This does not enable incomplete preparation reconciliation, imported/restored catalog root authorization or a background cancellable inspection job.

## Validation and next gate

Generated application regressions pass completed continuation through real review/approval/start methods, zero data allowance, unchanged output objects/revisions, thirteen unchanged original copy events, new digest-linked job inspection and unknown-entry refusal. A fresh controller also resumes an actual application copy stopped by catalog failure, preserving prepared object IDs. Real queued-job denial rolls back the linked review and job atomically; the consumed ticket cannot retry, and new approval succeeds. Native inventory validation initially conflicts with a held Windows writer; the dedicated attribute-only adapter fixes it.

All 262 frontend cases and 173 applicable Windows Rust cases pass, alongside strict/generated IPC, lint/format, denied-warning Clippy and build. Windows native suite passes four applicable cases with sixteen macOS skips, including actual continuation buttons/progress and unchanged independent output/source/neighbor audits. Production-feature Clippy and release exclusions pass. The expanded macOS scenario exercises actual companion/preparation/copy/retained-inspection/continuation controls; it remains unverified until this checkpoint's CI runs.

Cross-draft idempotency, folder import/duplicate skipping, incomplete namespace recovery, multi-root/cross-volume, case/cycle/undo and remaining M2 acceptance still precede M3. Target-platform evidence must be recorded before enablement.
