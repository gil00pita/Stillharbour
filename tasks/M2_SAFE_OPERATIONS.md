# Stillharbour - M2 - Previewed organization and verified import

## Objective

Enable safe operations only after immutable planning, no-clobber execution and crash recovery work on disposable fixtures.

## Current authorized work — 2026-10-30

The owner requested the next stage and prioritized Organization templates and previews. M2 is in progress; [the first preview slice](../docs/M2_ORGANIZATION_PREVIEWS.md) adds bounded catalog-only template previews through actual library IPC. It is not a persisted executable plan, destination preflight or M2 acceptance. Production copying and original mutation remain unavailable. Continue next with destination-aware persistent planning and confirmed companion inventory before implementing/authorizing the executor. Preserve all P4-06 and preservation gates below.

Next-phase continuation, 2026-10-30: on-demand indexed companion detail review is implemented in source, with bounded UUID-only IPC, explicit missing/excluded/offline observations, truncation and revision warnings. This is a prerequisite slice, not destination-aware persistent planning or bundle confirmation. Frontend checks and actual Rust/native limitations are recorded in [the preview contract](../docs/M2_ORGANIZATION_PREVIEWS.md) and implementation log. The next gate remains persistent destination-aware planning and confirmed companion inventory before executor work.

## Prerequisites and references

M0-M1; docs/CODEX_SPEC.md sections 5, 9, 10, 12; acceptance AC-023 through AC-039, AC-081, AC-082, AC-084.

## Work

Implement a pure template parser/resolver with predefined and user-saved templates, token validation, explicit missing metadata/date policy and deterministic collision/sequence assignments. Add shared immutable plan records, revision/digest binding, typed conflicts and backend approval.

Build before/after trees, per-file diffs, counts, bundle warnings, exact target paths and export. Creating or editing a preview must not mutate originals.

Implement the central operation executor: per-root/path locks, source/destination preconditions, native no-replace publish, journaled case-only/cyclic staging, cross-volume verified copy, cancellation and recovery. Create a reverse-plan undo flow and quarantine infrastructure; no permanent deletion. Preserve exact bytes and companions; report unsupported filesystem metadata preservation.

Add folder-based import preview and copy execution, exact-content duplicate skipping, verified destination publication, source preservation, import session state and idempotent retries. Confirmed related-file bundles travel together; ambiguity blocks automatic processing.

## Required failure tests before enablement

Inject failure after every journal/copy/publish/catalog stage. Exercise source changes, symlink swaps, destination collisions, disk-full, permissions and unplugged volumes. Prove that recovery does not blindly replay already completed operations and that undo never overwrites newly created data.

## Gate

Applicable AC-023 through AC-039, AC-081, AC-082 and AC-084 pass. A source-preserving generated import and reviewed same-volume organization work through real UI/IPC on target systems. Any unverified safety primitive keeps original mutation disabled. Record evidence and stop before M3.

## Version 1.1 preservation gate

Read docs/PRESERVATION_CONTRACT.md and docs/PRESERVATION_TESTS.md. Default to source-retaining verified copies. Develop mutation/recovery against disposable fixtures, but leave real-library in-place mutations and duplicate quarantine disabled until exact-version independent recovery evidence and applicable native failure gates are available. M4 may supply that provider later; do not waive it to complete an earlier milestone. No speed optimization may bypass full verification or protected-version retention.
