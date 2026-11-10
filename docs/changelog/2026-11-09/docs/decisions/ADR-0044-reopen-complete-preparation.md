> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0044 — Reopen a fully recorded prepared namespace for fresh copy approval

Date: 2026-10-31. Status: experimental workflow implementation; full checks and local Windows native validation pass; macOS CI pending.

## Decision

Name creation can finish before its ready-copy binding is committed. Retain the stopped preparation job and every original fact/object. An explicit ID-only reopen request, with fresh private native destination selection, may reconstruct copy-planning metadata after complete retained and native validation. It creates no directory/stage, moves no name, copies no bytes and restores no approval.

Open only the existing bounded namespace anchor and fact chain, linked to the stopped preparation job's exact scope digest, root and file count. Require a complete preparation record, current managed grants, unchanged source snapshot, native destination identity/volume, disjoint read leases, exact empty stage identities/revisions/link counts, complete known inventory and absent target names. Incomplete proofs, changed stages, unknown entries and any existing copy execution remain blocked with all objects retained. Do not adopt an object whose creation completion was lost.

Reuse the original ready stream if present; otherwise bind the ready plan from the retained completed preparation. Existing job/namespace/source identities remain unchanged. A new in-memory review may then use the normal separate fidelity acknowledgment, one-use copy approval and explicit start. Revoke earlier grants for reopened/discarded scopes, even when the same selection and immutable review digest are supplied again. Immutable stopped-job evidence remains accurate; a subsequently approved copy creates its own durable job before filesystem effects.

Expose reopen in the prepared workflow and in Jobs/history for stopped preparation lacking a ready binding. Validate response/job/manifest/native mapping before exposing approval controls. Ignore stale responses and discard a valid late review after its view closes. Keep the default copy feature and all original mutation permissions disabled.

## Validation and limits

Generated application regressions cover normal completion and actual SQLite ready-binding denial after native preparation. A fresh controller reopens the exact empty stage objects without changing their names, revisions or lengths. Reopening revokes old grants; new approval copies and publishes those same objects. Separate actual creation-completion denial, changed-stage and unknown-entry cases refuse reopening with unchanged journals and preserved objects/sources. Component cases cover unapproved recovered review, incomplete-proof errors and late-response discard. Strict types catch the distinction between data responses and the no-data discard route; use the existing library request helper. Denied-warning Clippy catches and removes a test-only clone of a copyable native identity tuple.

Full/native checks follow. Reconciliation of genuinely incomplete preparation, import/duplicate skipping, multi-root/cross-volume, case/cycle/undo and remaining M2 gates still precede M3. This is not native filesystem replay, independent recovery evidence or whole-milestone acceptance.

Complete checks pass 267 frontend and 180 applicable Windows Rust cases, strict/generated IPC, formatting/lint, denied-warning Clippy and build. Separate copy-workflow-only and fixture-only graphs pass; application-only wrappers are explicitly excluded from the latter. Isolated Windows native validation passes four applicable cases with sixteen macOS skips, including actual session discard/reopen-button/private-picker interaction, unchanged empty stages, fresh approval, same-object copying and independent preservation audits. Production assets/release exclusions pass. This checkpoint requires its own macOS CI.
