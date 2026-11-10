> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0039 — Application organization workflow and durable jobs

Date: 2026-10-31. Status: application integration behind non-default copy-workflow build feature; target-platform acceptance remains open.

## Decision

Connect source snapshots, native preparation, exact namespace projection and one-use approvals through typed application requests. The renderer submits retained IDs/digests, native picker selection IDs, fidelity acknowledgment and control values. Native paths/identity/volume observations come from the desktop's private selection state. Never accept renderer plans, source paths, stage names, SQL or execution/recovery proofs.

The backend freezes a new visible output folder and full source-to-output mapping before approval. Preparing names and copying bytes are separate reviews, approvals and starts. The latter requires acknowledgment of byte-only verification and declared metadata limitations. Sources remain in place; neither result is independent backup or permission for original mutation. Limit session reviews/tickets to 64, worker admission to one and native scope requests to the existing atomic read-source/write-destination leases. Work runs outside UI/async threads. Session expiry/discard removes only memory authority, never retained output objects.

Revalidate explicit companion decisions/source metadata and live native source snapshots before approval/start. Keep fresh destination identity/volume/overlap/package/capacity checks. Once the explicit namespace scope is frozen, its own parent/stage proofs replace the draft's historical unprefixed target observation as write preconditions: preparation itself legitimately changes base-directory revisions. Do not weaken original draft assessment. Reopen retained preparation through native held-parent directory lookups and require complete known inventory, unchanged empty single-link stages and absent final targets before copy approval and admission. Unknown objects block and remain untouched.

Open the prepared output relative to the held destination handle for copying and inspection, then compare its native identity with the frozen plan. Do not rebuild an exact-component directory chain from picker display spelling: Windows may return equivalent case or short-name aliases. The generated application regression exercises a different-case destination spelling while retaining exact native source locators and output identity checks.

Recheck output/application-journal storage identity and available space at admission. The output estimate remains the disclosed policy; the journal volume additionally requires 128 MiB at admission (the output estimate's existing 32-MiB journal allowance is credited when both use the same volume). Runtime boundaries retain a 64-MiB output/16-MiB journal reserve and still handle actual I/O/flush/journal failures; these are conservative estimates/checks, not reservations or upper bounds. Metadata/flush/helper limits remain separate from shared paced byte I/O.

Schema 5 adds operation_jobs and its recent-order index. Job scope identity is immutable, history cannot be deleted, state/progress updates use expected-state checks and the same transaction appends a journal fact. Keep source-root IDs as retained audit data without a root FK so removing a catalog root cannot erase operation history. Preserve all existing UUIDs/identifiers/paths. Existing consistent snapshot-before-migration and transactional rollback apply, including independently openable pre-migration-v4 catalogs. No media migration.

Durably record queued work before native effects; persist verified-file progress only after the copy journal records actual complete comparison. Interrupted active jobs become needs_recovery on application restart, without replay. Native worker completion-state write failure remains inspectable; a stopped worker with an unfinished job is marked for recovery on query. Scope leases/worker admission release on unwinding. Library removal requests cancel matching operation work and wait at the existing bounded maintenance gate; unrelated jobs are not canceled.

Expose cancel, aggregate byte-I/O rate/pause/resume and fresh output inspection. Pause is explicitly byte-I/O admission, not interruption of an already admitted syscall or metadata/flush operation. Header Jobs includes organization work alongside scans. Component requests ignore stale/unmounted responses, validate review/job identity and keep approval/start separate. Persisted jobs/facts never recreate session authority.

The feature is enabled for desktop-test builds to validate the real UI/IPC with generated media; normal builds retain a clear unavailable capability reason. Core fixture constructors, supplied-scope-free diagnostic routes and process-exit controls remain separately gated by fixture-probes/test. No original mutation capability is enabled.

## Validation and next gate

Actual application API regression executes preparation and copying with two exact approvals, fresh review/inspection, full generated bytes, preserved companions/neighbor and replay refusal. Component tests cover unavailable builds without a picker, both explicit approval/start phases, fidelity acknowledgment, exact paths, inspection and foreign review refusal. Migration tests cover schema-4 identity/history snapshots, second-index collision rollback and nine actual migration process exits across versions 2/3/4.

The Windows native planning scenario passes through the actual buttons and private native picker, with source manifests and an independent constructor-baseline byte audit of outputs. The audit accepts partial stages only as retained objects and never calls them completed copies. All four applicable Windows desktop cases pass; fifteen macOS cases are skipped locally. Full checks and default/feature release exclusions pass. This does not establish the macOS application workflow or the remaining cross-volume/recovery gates; exact evidence and corrected failures are in IMPLEMENTATION_LOG.md.

Multiple roots, explicit catalog-backed restart reauthorization/recovery UI, approved resume/idempotency, import duplicate skipping, cross-volume native gates, case/cycle mutation fixtures, reverse-plan undo and remaining M2 acceptance still precede M3. Original-file operations/quarantine remain gated on independent exact-version recovery, which this copy workflow does not provide.
