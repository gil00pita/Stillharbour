# Stillharbour - M2 - Previewed organization and verified import

## Objective

Enable safe operations only after immutable planning, no-clobber execution and crash recovery work on disposable fixtures.

## Current authorized work — 2026-11-01

The owner authorized continuing M2 and then M3 without stopping at intermediate checkpoints, and pushing those checkpoints to draft PR #9. M2 remains unfinished; M3 has not started. Experimental work includes retained single-root copy planning, approvals/jobs, native staged copying/continuation, mixed whole-bundle import and explicit referenced output indexing. Generated rename/undo has actual instrumented UI review/approval, atomic path commits and schema-7 durable organization attempts/events in its own actual Catalog (ADR-0053–0059); schema-7 full local/Windows native checks pass and its target CI remains pending. ADR-0058 develops strict test-only cold recovery across twelve actual catalog process exits; focused and full local checks pass (twelve actual exits plus malformed-scope admission); checkpoint CI remains pending. Real-library original mutations and duplicate quarantine remain disabled.

Complete Windows/macOS CI passes checkpoints 8415096 (41273367880), 8b7891c (41274810329) and 1f8846b (41275768949), including strict generated VHDX/APFS probes. Checkpoint dfcc529 adds catalog commits and passes complete CI 41276882411. Checkpoint 5869d19 adds actual cold process-exit recovery; complete Windows/macOS CI 41278148486 passes. Local retained fixture versions cannot waive independent recovery.

Remaining gate work includes general application original-organization/undo worker queue/control, watcher reconciliation and restart discovery, multi-root/target-root policies, full case/Unicode/native failures and the other required acceptance scenarios. Test-only marked-scope reconstruction is not production adoption. Record actual results before advancing to M3.

## Historical planning progression — 2026-10-30 onward

The owner requested the next stage and prioritized Organization templates and previews. M2 is in progress; [the first preview slice](../docs/M2_ORGANIZATION_PREVIEWS.md) adds bounded catalog-only template previews through actual library IPC. It is not a persisted executable plan, destination preflight or M2 acceptance. Production copying and original mutation remain unavailable. Continue next with destination-aware persistent planning and confirmed companion inventory before implementing/authorizing the executor. Preserve all P4-06 and preservation gates below.

Next-phase continuation, 2026-10-30: on-demand indexed companion detail review is implemented in source, with bounded UUID-only IPC, explicit missing/excluded/offline observations, truncation and revision warnings. This is a prerequisite slice, not destination-aware persistent planning or bundle confirmation. Frontend checks and actual Rust/native limitations are recorded in [the preview contract](../docs/M2_ORGANIZATION_PREVIEWS.md) and implementation log. The next gate remains persistent destination-aware planning and confirmed companion inventory before executor work.

## Prerequisites and references

Owner continuation, 2026-10-31: continue M2 planning, then finish M2 if unfinished. ADR-0022's bounded destination preflight retains picker-session authority and stale-preview checks. ADR-0023 subsequently verifies scoped Windows retained migration snapshots and introduces schema-3 immutable template versions/drafts with restart review and JSON/CSV text. M2 is still in progress; this does not implement or authorize the executor.

Current checklist:

Owner continuation, 2026-10-31: continue working through M2 and then M3 without stopping after individual planning slices. Preserve prerequisite/native/preservation gates; an unavailable platform or recovery provider is not a passing result. ADR-0024 adds existing-target observations and native source components while retaining draft-only authority.

Conditional M3 request, 2026-10-31: M2 is unfinished, so do not start M3. Continue M2. Selected-path Before/After views now group source labels by library and proposed folders by DSL separators; full native source-folder hierarchy remains open alongside the requirements below. All executor/preservation gates remain unchanged.

- Implemented: pure bounded DSL/relative previews; indexed companion review; native destination identity/overlap; retained migration snapshot and schema-3 transaction; saved template version/CAS and bounded resolver-validated examples; immutable fingerprint-bound drafts/history/staleness and JSON/CSV review text.
- Open planning: equivalence between future native target/case/Unicode names; explicit confirmed-bundle integration into executable plans; full capacity/fidelity evidence; selectable additional template policies; executable-plan approval/preconditions. Existing-target lookup, observed space, backend component-based selected-source hierarchy and no-replace review-file saves (ADR-0025) are implemented; these are not complete inventories or original-file execution acceptance.
- Open execution: central locks/journal state transitions, staging/no-replace verified copy, cancellation/recovery/fault injection, reverse-plan undo, fixture folder import/duplicate skipping/idempotency and target-platform native acceptance.
- Production gate: exact-version independent recovery and native failure gates still required. No real-library mutation enablement.

M0-M1; docs/CODEX_SPEC.md sections 5, 9, 10, 12; acceptance AC-023 through AC-039, AC-081, AC-082, AC-084.

Companion continuation, 2026-10-31: ADR-0026 implements bounded live direct-folder filename inventory and retained per-photo companion decisions. Hidden/unindexed entries and incomplete states remain visible. These records remain separate from saved drafts and execution authority; future plans must explicitly bind/revalidate decisions.

Compiled-scope continuation, 2026-10-31: ADR-0027 reconciles exactly one explicit companion decision per selected photo into an immutable, idempotent copy manifest. Live inventories are revalidated; overlaps combine only identical mappings, conflicts remain blocked. This is the manifest prerequisite for ready-plan/destination/content/approval checks, not execution acceptance.

Owned executor continuation, 2026-10-31: ADR-0028 implements the central source-retaining copy sequence, shared byte pacing and recovery inspection under a constructor-owned temporary fixture capability. Actual process exits and journal write-denial regressions run against that scope. Production catalog/job/approval/manifest integration, full native target/capacity/fidelity, root/path lock registry, import and reverse-plan undo remain open. This is not M2/M3 acceptance or authorization for real-library mutations.

## Work

Implement a pure template parser/resolver with predefined and user-saved templates, token validation, explicit missing metadata/date policy and deterministic collision/sequence assignments. Add shared immutable plan records, revision/digest binding, typed conflicts and backend approval.

Build before/after trees, per-file diffs, counts, bundle warnings, exact target paths and export. Creating or editing a preview must not mutate originals.

Implement the central operation executor: per-root/path locks, source/destination preconditions, native no-replace publish, journaled case-only/cyclic staging, cross-volume verified copy, cancellation and recovery. Create a reverse-plan undo flow and quarantine infrastructure; no permanent deletion. Preserve exact bytes and companions; report unsupported filesystem metadata preservation.

Add folder-based import preview and copy execution, exact-content duplicate skipping, verified destination publication, source preservation, import session state and idempotent retries. Confirmed related-file bundles travel together; ambiguity blocks automatic processing.

## Required failure tests before enablement

Inject failure after every journal/copy/publish/catalog stage. Exercise source changes, symlink swaps, destination collisions, disk-full, permissions and unplugged volumes. Prove that recovery does not blindly replay already completed operations and that undo never overwrites newly created data.

## Gate

Applicable AC-023 through AC-039, AC-081, AC-082 and AC-084 pass. A source-preserving generated import and reviewed same-volume organization work through real UI/IPC on target systems. Any unverified safety primitive keeps original mutation disabled. Record evidence before proceeding to the owner's subsequently authorized M3 continuation; do not skip this prerequisite gate.

## Version 1.1 preservation gate

Read docs/PRESERVATION_CONTRACT.md and docs/PRESERVATION_TESTS.md. Default to source-retaining verified copies. Develop mutation/recovery against disposable fixtures, but leave real-library in-place mutations and duplicate quarantine disabled until exact-version independent recovery evidence and applicable native failure gates are available. M4 may supply that provider later; do not waive it to complete an earlier milestone. No speed optimization may bypass full verification or protected-version retention.

Native-name continuation, 2026-10-31: ADR-0029 exercises exclusive future-name creation, all-participant conflict reporting and retained empty stages in a constructor-owned pending namespace. Bounded journal evidence, interrupted preparation and fresh membership/stage/target-absence checks fail closed. This advances the native-name prerequisite only; complete namespace sealing, ready-plan approval, capacity/fidelity, catalog/job/locks, import, undo and full target-platform acceptance remain open.

Capacity/fidelity continuation, 2026-10-31: ADR-0030 adds on-demand review with fresh explicit-decision/destination validation, observed native allocation/free-space figures, exact per-file rounded data and stated allowances. All unsupported metadata aspects and absent independent backup are visible. This advances read-only review; actual executor capacity checks across output/application-journal volumes, ready-plan approval, catalog/job/locks, import, undo and full native acceptance remain open.

Shared-lock continuation, 2026-10-31: ADR-0031 adds process-wide atomic native read/write scope leases across actual scanning and disposable copy/preparation/recovery. All-or-none admission prevents partial holds and blocks incompatible scans before job creation. Remaining reader/writer adapters, production ready-plan/catalog/job/approval integration, full capacity/fidelity, import/idempotency, undo and native gates remain open; M2/M3 are not accepted.

## Catalog-backed executor continuation — 2026-10-31

ADR-0032 moves current catalog schema to 4 by adding journal scope/unique stream indexes after retained consistent snapshots. Under fixture capability, actual catalog actor streams bind retained draft/companion manifests and exact flat single-root copy plans, validate bounded ordered events and recover through fresh native byte inspection. No imported approval or automatic replay. Multi-root/nested ready plans, session approvals, jobs/controls, complete capacity/fidelity, import/idempotency, undo and the catalog adapter's own process-exit/native gates still precede M2 acceptance/M3.

Catalog exit continuation, 2026-10-31: ADR-0032 now implements the catalog actor's own nine-boundary process-exit matrix against validated generated scope, with parent actor close/reopen, actual file inspection and no replay/journal growth. Local Windows passes; target-platform CI remains pending for this continuation. The remaining production ready-plan/jobs/approval/capacity/fidelity/import/undo gates still precede M2/M3 acceptance.

Nested-location continuation, 2026-10-31: ADR-0033 extends the fixture-only catalog engine from flat to exact single-root nested source/output mappings. Non-lossy source locators and captured parent native IDs are validated around operations; swapped parents fail without adoption. The next gate remains production preparation/ready-plan/approval/job integration, multi-root scope, capacity/fidelity/import/idempotency/undo and full native acceptance before M3.

Prepared-stage continuation, 2026-10-31: ADR-0034 connects the real constructor-owned native name preparation to verified copying, checking exact empty stage identity/revision/link/membership and retaining those native IDs through publication. A separate copy journal prevents preparation intent from becoming replay authority; unbound catalog prepared-stage requests fail. Full production namespace/ready-plan/approval/jobs, multi-root, capacity/fidelity/import/idempotency/undo and native acceptance remain open.

Catalog preparation continuation, 2026-10-31: ADR-0035 journals actual owned native preparation against frozen namespace/manifest evidence in the catalog, reopens thirteen facts and rejects replay, stale writers, failed journal writes and revoked managed grants. Preparation facts remain separate from copy events/approval; production prepared-stage catalog binding, ready-plan/session approval/jobs and the remaining scope/capacity/import/undo/native gates still precede M2 acceptance/M3.

Prepared catalog projection continuation, 2026-10-31: ADR-0036 connects retained namespace facts to exact immutable fixture copy streams, fresh native verification and recovery inspection. Exact reference/source/stage/target/parent matching and one-stream-per-namespace admission block altered plans and new-UUID replay. Production namespace/ready-plan/session approval/jobs, multi-root scope, full capacity/import/idempotency/undo and native gates still precede M2 acceptance/M3.

Native source snapshot continuation, 2026-10-31: ADR-0037 replaces execution-shaped namespace inputs with source-only native snapshots built from actual retained manifests. Future folders remain absent during preview; preparation supplies their real identities and freezes the visible output folder. V2 namespace phases retain this evidence separately. Production full-prefix review, preparation/ready-plan/session approval/jobs, multi-root/capacity/import/resume/undo and native gates remain open before M2 acceptance/M3.

Session approval continuation, 2026-10-31: ADR-0038 adds bounded one-use monotonic session tickets and nonserializable proofs for exact preparation/copy bindings and native selection IDs. Private native wrappers verify their own scope and current managed grants; revocation retains verified stages without publication. Native receipts are fixture-grant evidence only. Production selection/session ownership, review/approval IPC/UI, durable job/audit states, controls, capacity/import/resume/undo and remaining native M2 gates still precede M3.

Application workflow continuation, 2026-10-31: ADR-0039 wires exact preparation/copy reviews and separate approvals/starts into application requests, durable schema-5 operation jobs, cancel/rate/byte-I/O pause and inspection. Source-only retained assessment distinguishes the explicitly prefixed prepared namespace from historical draft targets while keeping fresh source/root/volume/overlap checks. Default builds remain gated; actual native UI/output-audit checks are pending. Multi-root, restart reauthorization/recovery/resume, import/idempotency, cross-volume gates, case/cycle fixtures, undo and full M2 acceptance still precede M3.

Retained inspection continuation, 2026-10-31: ADR-0040 adds existing-job inspection after loss of session authority, with fresh native destination selection, exact catalog/native binding, referenced-root read scopes and structured per-file failures. It does not resume or approve work; incomplete preparation without a ready stream remains blocked. Full/native validation and approved resume/idempotency/import/undo plus remaining M2 acceptance precede M3.

Reviewed continuation, 2026-10-31: ADR-0041 develops same-stream recovery against constructor-owned fixtures only. New exact session grants, full retained-prefix comparison, same-object publication and fresh-verified no-op are distinct from blind replay. Application retry/jobs/namespace recovery inventory, cross-draft idempotency/import and remaining M2 acceptance still precede M3.

Application continuation, 2026-10-31: ADR-0042 integrates newly observed, separately approved same-stream continuation jobs behind the experimental workflow feature. Complete retained inventory, exact digest/prior-stream links, atomic admission and fresh-attempt progress remain required. Full/native checks, cross-draft idempotency/import/undo and remaining M2 acceptance precede M3.

Effect-history continuation, 2026-10-31: ADR-0043 binds the exact native source/template/target mapping to retained output history independently of draft/manifest/session UUIDs. Atomic admission refuses a second namespace; untouched same-draft reviews reuse their existing token. A new equivalent draft exposes prior folders/jobs for fresh complete-byte inspection and separately approved continuation. Legacy candidates are retained and bounded, never adopted or replayed. Schema 6 preserves v5 snapshots/history. Full/native results remain pending; import/duplicate skipping, incomplete preparation, multi-root/cross-volume/case-cycle/undo and remaining M2 gates still precede M3.

Complete-preparation recovery, 2026-10-31: ADR-0044 reopens a stopped preparation only after complete retained proof, unchanged source/native scope and exact original empty-stage inventory. Reconstruct a missing ready-copy binding as catalog planning metadata, without filesystem replay or approval restoration. New copy acknowledgment, approval and start remain required. Actual incomplete creation proofs, changed stages and unknown entries are preserved and blocked. Full/native results remain pending; genuinely incomplete preparation and the remaining import/multi-root/cross-volume/case-cycle/undo gates still precede M3.

Read-only import comparison, 2026-10-31: ADR-0045 adds explicit indexed root scope, bounded size/sample/full-hash candidates and mandatory direct complete-byte matches through actual background IPC jobs. Read-only referenced sources remain separate from managed-copy permission. Native versions/parent membership, shared pacing and maintenance cancellation apply. Offline/inaccessible/incomplete data remains unknown; missing/excluded counts are distinct. The report cannot skip/remove/copy files; bundle-aware import planning, actual skip execution and the remaining M2 gates still precede M3.

Whole-bundle observation continuation, 2026-10-31: ADR-0046 groups transitively shared companions and observes only coherent complete renamed groups with distinct directly compared keepers and indexed-media primary matches. The UI exposes whole-bundle copy/review/attention evidence. Reports still cannot skip/copy files; immutable import decisions, fresh keeper execution and remaining M2 gates precede M3.

Import decision/receipt continuation, 2026-10-31: ADR-0047 retains explicit whole-bundle native keeper decisions and idempotent planning history. All-existing execution develops fresh direct comparisons, separate acknowledgment/approval/start, cancel/restart review and durable skip outcomes without new files. Mixed copy/skip plans remain disabled. Complete checks and Windows native validation now pass; macOS CI and mixed-copy/folder import, multi-root/cross-volume/case-cycle/undo and remaining M2 gates still precede M3.

Mixed-import scope continuation, 2026-11-01: ADR-0048 develops private exact new-copy subsets and captured managed/referenced read-source grants. Actual generated stages/copies must exclude skipped bundles and retain source/target IDs and memberships. The constructor is test-only; fresh keeper/copy/receipt orchestration and mixed import UI remain open before the other M2 gates/M3.

Keeper admission continuation, 2026-11-01: ADR-0049 shares the all-existing complete-byte verifier with private projected work and recovery, holds all source/output/keeper leases atomically, and includes keeper roots in maintenance cancellation. Mixed application orchestration/capacity/receipts/UI and other M2 gates still precede M3.

Mixed import continuation, 2026-11-01: ADR-0050 connects new-only capacity/review, separate approvals/jobs and combined durable verification evidence under the experimental copy capability. Receipt facts alone do not mean successful import; exact committed job success is also required. Full/native validation follows. Folder import/indexing, multi-root/cross-volume/case-cycle/undo and remaining M2 gates still precede M3.

Output catalog continuation, 2026-11-01: ADR-0051 develops explicit discovery of completed import output with exact historical publication-version/native inventory checks and a fresh parent picker. Referenced registration and actual scan progress remain separate; retries reuse known root/scan IDs. Current overlap rules are unchanged, so indexing an output protects its ancestor destination from subsequent drafts. Full/native outcomes follow; target-root policy and remaining M2 gates still precede M3.

Cross-volume continuation, 2026-11-01: ADR-0052 develops nonce-owned Windows VHDX/macOS APFS CI fixtures, actual distinct native identities, application-core preparation/copy and in-process interruption/continuation checks. Local Windows preflight reports administrator unavailable; no mounted-volume pass is claimed. Strict CI evidence and the remaining M2 target-root/multi-root/case-cycle/undo/native gates still precede M3.

Cross-volume CI continuation, 2026-11-01: macOS full/native and the strict APFS probe pass 41265781031, including nine in-process continuations and image cleanup. Windows reaches the virtual-disk guard but requires the raw-CIM BusType correction before any initialization/format; local six-case metadata preflight passes without admin or mounting. Windows mounted evidence and remaining M2 gates still precede M3.

Rename continuation, 2026-11-01: ADR-0053 implements pure case/cycle staging and canonical reverse planning, plus a closed native generated-file experiment. Its local retained versions do not satisfy independent recovery. Native reverse execution, process-exit/journal-denial coverage and actual organization UI integration remain open. Windows CI 41266627076 now passes the strict mounted VHDX copy/continuation gate; macOS APFS evidence is in 41265781031. Neither proves physical-device recovery. M2 remains unfinished before M3.

Native continuation evidence: 2e77a27 passes complete Windows/macOS CI 41268936481, including case/cycle/undo fixture IPC, viewer correction and strict virtual-volume copy/continuation. New journal-denial/process-exit/directory-identity work is under validation. Original-organization/undo catalog and UI integration, multi-root/target-root policy, quarantine infrastructure and remaining native gates still precede M3; real-library original mutations remain disabled.

Relocation continuation, 2026-11-01: ADR-0054 develops same-volume pinned-directory relocation, retained companion quarantine infrastructure and new-scope restoration only on generated files. Keep no-replace publication, complete-byte verification, immutable facts and blocked newer occupants. Native full/IPC/mounted-volume checks and incomplete-state continuation are pending. This is not M3 duplicate authorization or real-library mutation enablement; catalog/UI integration and remaining M2 gates still precede M3.

Explicit relocation continuation, 2026-11-01: ADR-0054 now adds strict v2 owned-scope decoding, fresh complete-byte/identity/phase checks and a distinct one-use continuation approval. Missing completion facts may be reconciled only for the exact original object after verification; completed moves are skipped. Failed/unknown observation writes never grant new moves. Native continuation and remaining failure/catalog/UI checks are under validation; production and duplicate-removal gates remain unchanged before M3.

Stem policy continuation, 2026-11-01: ADR-0055 adds explicit proposed Preserve/Lowercase/Uppercase filename stems, with unchanged folder/extension case and legacy Preserve identity. Nondefault saved metadata is schema 2; no catalog migration or original mutation permission. Full/native checks are pending. Other selectable policies, original-organization/undo catalog/UI and remaining M2 gates still precede M3.


Generated review continuation, 2026-11-01: ADR-0056 connects a typed instrumented-only UI/IPC lifecycle to the closed rename engine, with exact native review observations, acknowledgment, one-use approval, inspection and separately reviewed undo. Validation is running. Private fixture journals do not replace application catalog path updates, jobs or restart integration; M2 remains open before M3.


Generated catalog continuation, 2026-11-01: ADR-0057 connects the rename lab to a constructor-owned actual Catalog actor. Atomic path/receipt commits retain IDs/annotations and separate native completion from catalog completion, including fresh approved retry without repeated moves. Validation is running. General application library organization, durable jobs/restart adoption, multi-root/target-root policies and remaining M2 gates still precede M3.


Generated catalog exit continuation, 2026-11-01: ADR-0058 adds a test-only twelve-case real child-exit matrix across native-to-catalog transaction boundaries, bounded strict cold reconstruction and fresh-approved no-move retry. Validation is running. Production restart discovery/adoption, general application jobs/root organization, multi-root policies and remaining M2 gates remain open before M3.


Organization jobs continuation, 2026-11-01: ADR-0059 develops schema-7 separate immutable organization plans and durable attempts/events. Copy-plan/job semantics remain unchanged. Terminal job verification commits atomically with paths/receipt; interrupted history needs fresh review and never resumes I/O. Typed status, Jobs and the generated dialog display actual states. Migration/full/native validation is running. General original-worker queue/control, watcher reconciliation, library integration and remaining M2 gates still precede M3.
