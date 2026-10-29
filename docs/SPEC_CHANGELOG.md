# Stillharbour - Specification change log

## Selected-folder action refinement - 2026-10-27

Applied the owner's selected-folder annotation: remove the native identity message and place “Save & Start scanning” at the lower right of the card. CODEX_SPEC.md section 16 and AC-001 record the action and its current M0 state: disabled with a visible accessible reason, no persistence/indexing or fabricated success. The separate M1 gate remains unchanged. Implementation evidence belongs in IMPLEMENTATION_LOG.md.

## Library empty-state refinement - 2026-10-27

Applied the owner's annotated screen review to the initial library view. Section 16 of CODEX_SPEC.md and AC-001 now specify the direct folder-selection heading/action, omit promotional copy and developer status/footer notices, and retain native readiness, actionable errors and an explicit unscanned selection state. This is a presentation refinement within M0; it does not authorise M1 indexing or original-file operations. Actual implementation and validation results are recorded in IMPLEMENTATION_LOG.md.

## 1.2.1 - 2026-10-26

Naming-only revision following the owner's selection of **Stillharbour**. Updated the package title, main product specification, repository instructions, all three Codex prompts, document titles, decision status and catalog filename example (`Family.stillharbour`). Added PRODUCT_IDENTITY.md and the exact owner-supplied SVG at assets/branding/stillharbour-logo.svg.

The v1.2 feature requirements, preservation contract and all 187 specified acceptance/failure scenarios are retained. No feature has been implemented or newly approved. No original-file operation, user-catalog rename, runtime migration or public trademark/domain clearance was performed. Historical research dates remain unchanged; external evidence has not been re-researched for this naming-only update.

Regenerated the package content manifest and checked the documentation, examples, SVG structure and final archive. See SPEC_VALIDATION.md for the actual package-level checks. The source v1.2 archive remains unchanged.

## 1.2 - 2026-10-26

Added the owner's six library features: integrity reports; Recently added; crop/rotate/mirror recipe editing including a mobile follow-on; large Google Photos/iCloud import paths; save/open catalog files; photo/video tags. Introduced detailed contracts, current official-source register, example data, staged N1-N5 tasks and a prerequisite-gated Codex prompt.

Provider research distinguishes Google Takeout (including repeated/incremental exports) from limited Picker downloads, and Apple original exports/local copies from a later native PhotoKit adapter. Added snapshot-safe and defensively validated catalog portability, source-preserving metadata/editing, historical integrity baselines, archive safety and explicit import-fidelity/completeness states. Require a patched SQLite runtime, including the documented WAL-reset fix.

Original preservation contract, source documents, reference images, templates and implementation evidence remain retained; original archives are unchanged. This is a specification revision only. See SPEC_VALIDATION.md for document checks, not application test claims.

## 1.1 - 2026-10-23

Owner steer: preventing loss/damage of any file is more important than speed, organization convenience or storage savings. A working name was proposed at this stage; the later owner-selected identity is recorded in version 1.2.1. Public brand clearance remains outstanding.

Added the overriding preservation contract, fixed-base-library Safe copy workflow, exact reviewed comparison, retained-version destination replacement, meaningful context-specific errors, global transfer/verification pacing, honest helper limits and 40 acceptance/failure scenarios.

Strengthened defaults: organize by verified copy while retaining source; original-file mutation requires exact-version independent recovery evidence. Full-library protection scope is separate from gallery visibility. No automatic deletion propagation, permanent purge, two-way source writes or history expiry.

Updated docs/CODEX_SPEC.md, repository guidance, decisions, M2/M3 safety gates, M4 staged delivery, M6 release gate and starter prompts. Kept original research, images, example organization templates and all 85 original acceptance scenarios.

This is a document/package revision only. Application implementation has not been performed here. Structural checks of specification files are not application safety or cross-platform tests. Record later real implementation results in IMPLEMENTATION_LOG.md.

## 2026-10-27 — Authorised M1 read-only library

The owner approved the next milestone. Save & Start scanning is now backed by native root registration and an actual scanner, replacing the historical M0 unavailable state. Gallery/grid/text browsing, provenance, filters, jobs, cached previews and catalog-only annotations are implemented within M1. No original-file mutation gate changed. See [M1 validation](M1_VALIDATION.md) and [ADR-0002](decisions/ADR-0002-read-only-library.md) for evidence and unsupported native environments/formats.

## 2026-10-27 — Scan setup switches, tags and Managed default

Owner-requested changes: rename Enter hidden folders to Include hidden folders; replace scan checkboxes with switches; use removable tags for the four list fields; remove scan From/Through dates and clear old scan bounds when saving; default new folders to Managed and offer a Read only switch. Existing folder modes, browsing date filters, catalog identities and all preservation/mutation gates remain in place.

## 2026-10-27 — Advanced accordion and photo extension preset

Owner changes: version-only sidebar footer, no welcome after folder selection, natural form height, full-width Advanced accordion, removal of Include patterns, and File Extensions with the exact 33-photo-suffix default. Matching remains case-insensitive; dots are display/input syntax, stored suffixes are canonical. Unsupported suffixes are discovery candidates without new decoder claims. Existing saved extension scopes and preservation gates remain intact.

## 2026-10-27 — Jobs scan progress bar

Owner requested a Jobs progress bar. Running and pending-cancellation scans show indeterminate activity because streaming discovery has no fixed total; native counts remain visible. Only completed scans fill the track; other terminal outcomes stop activity without claiming success. Accessible labels, reduced motion and minimum-window containment are required. Existing polling, cancellation and preservation gates remain unchanged.

## 2026-10-27 — Scan size units

Owner requested unit dropdowns for minimum/maximum scan sizes. Advanced now pairs decimal amount inputs with independent KB/MB/GB selectors and retains exact byte-string storage. Blank/zero remain distinct, existing limits survive without rounding, and invalid drafts block Save/Preview. No scanner, schema, browsing-filter or preservation gate change.

## 2026-10-27 — Image-only masonry Gallery and timeline

Owner requested a Google Photos-like masonry Gallery with no gaps, labels or selection checkboxes, plus the supplied right-hand scrolling timeline reference. Gallery now uses flush justified rows and image-only tiles; selection controls remain in Grid/Text list. A real visible-row date drives the month/year indicator beside existing indexed year navigation. Native minimum-window access, bounded pages, error states and preservation gates remain required; no backend/schema change or full-library infinite-scroll claim.

## 2026-10-27 — Chronological canvas and cached-preview viewer

Owner's expanded gallery brief replaces the previous gapless/no-checkbox display with aspect-preserving justified rows, 4 px gutters, date headings, hover/range selection, fixed filename search, collapsible navigation, filters and thumbnail density controls. Real filtered year/month buckets and keyset seeks support capture/Recently added chronology. A modal viewer adds zoom/pan, filtered previous/next, optional filmstrip/Info and exact scroll/focus return. Compare shows explicit 2–4 selections without duplicate claims; batch tags merge existing catalog tags atomically. Text list remains image-free. Preview resolution stays capped at 512 px. Similarity groups remain visibly unavailable pending M3; original mutations, edit/export and albums keep their separate gates.

## 2026-10-28 — Settings and indexed photo-folder locations

Owner requested Settings for editable defaults and clarified Library location means photo folders being indexed. Add persistent browsing/new-folder defaults and library location/relink/add controls, reusing native identity validation and existing setup/scan workflows. Existing folders retain settings/IDs; changing defaults never starts scans or changes originals. Catalog/cache relocation remains outside scope. Versioned native app-owned preferences require validation and explicit error states; catalog schema/preservation gates do not change.

## 2026-10-28 — Condensed gallery filters and scroll loading

Apply the owner's annotated screenshot: remove gallery Add folder, stacked controls/counts/availability and pagination footer. Keep fixed search/Filter; consolidate query, sorting, display, facets, availability and applied-query selection in a modal side panel with Apply/Cancel. Load native 240-row chunks on scroll, retain at most 720 summaries, restore anchors across bidirectional window changes and preserve explicit retry/unavailable states. Folder management stays in Settings; similarity and all preservation gates remain unchanged.

## 2026-10-28 — Inline Scan settings group

Owner requested moving Scan settings into the Settings form. Existing-root controls now appear as an inline group with a library selector and separate Save & Rescan / Discard scan changes actions. Gallery links target this group; per-location launch buttons are removed. Reuse saved root settings and existing read-only native commands, preserve app defaults independently and retain explicit failure/busy states. No new schema, dependency, permission or preservation scope.

## 2026-10-28 — Remove library and clear indexed previews

Owner requested a folder-library removal option and cache clearing. Add per-root removal with explicit catalog-note/history loss review, plus all-library indexed preview-cache cleanup. Both preserve original photos/sidecars. Bound native approvals and file batches, exclude scans/preview publication, retain roots on partial failure, atomically remove catalog dependencies and preserve surviving IDs. Unknown cache files remain retained; no schema, new dependency or original-mutation authorization.

## 2026-10-28 — Folders as a file explorer

Owner requested folder-first browsing instead of a flat gallery in Folders. Add registered-library roots, immediate indexed child folders/files, breadcrumbs/Back/Up, Icons/Details, exact-folder photo viewing and read-only non-media details. Preserve native path bytes, bounded cursor pages, explicit offline/error states and original-file safety. The chronological Photos canvas and its scroll feed stay intact.

## 2026-10-28 — Sidebar Jobs/Settings footer

Move Jobs and Settings below the primary library menu into a fixed sidebar footer above the version. Preserve onboarding availability, collapsed names, active/running states and independent content/menu scrolling. Presentation only; no backend/data change.

## 2026-10-28 — Text list column controls

Use actual Modified Date as the default date column without repeated date-source subtitles. Add a session-local column chooser/reset and existing native sort shortcuts, with supported sortable headings and horizontally reachable columns. Optional effective capture date/provenance remain available. No original-file access, persisted identity, schema or milestone gate changes.

## 2026-10-28 — P0 performance baseline

Add opt-in bounded diagnostic infrastructure and executable generated-fixture protocols, reconcile missing addendum checklist/source references, and report actual release-backend/native-debug observations in PERFORMANCE_BASELINE. Keep product UI, original-file behavior, dependencies, schema and preservation gates unchanged. No later performance phase is authorised or represented as completed.
