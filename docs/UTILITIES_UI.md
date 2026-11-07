# Utilities and duplicate review

Owner-requested interface revision, 2026-11-05. The attached Duplicate Studio prototype supplies interaction ideas; its branding, images, sample measurements and deletion behaviour are not application assets or safety evidence.

## Navigation and search

Secondary buttons use flat neutral fills without visible outlines; focus outlines and selected states remain visible. Utilities groups Structure Formater, Duplicates, Integrity, Storage, Backup and Sync. Map remains library navigation. Jobs opens its full main-pane page from the sidebar footer immediately above Settings, in expanded and collapsed navigation. Opening Jobs preserves the current workspace.

Photos, Albums / Tags and Map use one search field for filenames, calendar days and offline city/country suggestions. Typing continues ordinary filename/date searching. Choosing an explicit place clears the lookup text and applies its versioned GPS membership ID; subsequent text can narrow that place. Clear removes the place filter. Suggestions distinguish same-name cities, disclose GPS coverage, handle errors/retries, and support keyboard entry, Down to suggestions and Escape dismissal. The suggestion panel closes when a filename/date has no matching places, keeping the photo results clear. No separate Places button remains. Dates and location filters can still be combined through the filter panel.

## Different tools for different questions

- Duplicates presents a cluster queue and an evidence inspector, with file counts, potential duplicate counts, exact/candidate separation, observed-name search, page-bounded group selection and keeper choices.
- Integrity presents presence, content-check and baseline states. Needs attention includes unavailable entries, decoder warnings, read changes and changed baselines. A changed checksum is not proven damage; no repair or baseline reset is offered.
- Storage retains volume capacity, recursive folder totals, treemap navigation, logical/allocated size accounting and largest-file observations. It no longer repeats health and checksum columns.

After a saved scan exists, inspection setup remains an expandable section. The illustrated matching, file-scope and property-combination cards from main are retained inside it, with advanced algorithm choices and the existing safety labels. The unified search keeps main's search glyph and named Catalog/Saved plans icon controls. Existing start/cancel, retained-job selection, issue paging, coverage and native root checks remain connected. All observations are historical until rechecked. Search uses file names and paths available in the current findings page; groups without loaded names can still be found by their ID or evidence label. Group paging retires the previous page's selection.

## Comparison and keeper review

Choose two indexed media members from the selected group. Side by side, the draggable/keyboard range slider and a difference heatmap render the actual cached previews, up to 512 px with smaller-tier fallback. The heatmap colours a difference blend of unaligned rendered previews: black is unchanged; warm/pale pixels show increasing differences. It includes scaling, colour and framing differences and is not original-pixel equality, damage analysis or cleanup authority. Missing/failed previews show a reason and Retry; overlays require two available previews.

The evidence inspector shows the backend grouping explanation, exact-byte status and available dimensions, capture provenance and catalog annotation differences. It explicitly identifies unmeasured audio fingerprints, framing percentages and export lineage. RAW recognition is a filename-based keeper preference, not proof of camera-original provenance.

Resolution, RAW and largest-file suggestions operate only after complete member loading, retain ties and keep every member when resolution is unknown. Size comparisons retain integer precision. Manual keep-multiple and backend-retained folder protection/preference reviews remain available. Filesystem-date policies in that backend review retain their explicit names; they are never relabelled as capture or edit history.

## Cleanup gates and unavailable signals

The rules drawer closes with its Close action or Escape and restores the triggering control. Its keeper preference is shared with the selected cluster and survives reopening within the page. The rules drawer previews the selected groups (or the current page), a session keeper preference, protection requirements and a visual threshold. Visual scores are not emitted by the current inspection engine: every threshold therefore has zero scored eligible groups. Exact-byte evidence is never presented as a fabricated visual percentage. Newest edit, oldest capture and album-count preferences remain visibly unavailable because this inspection interface does not supply their authoritative ranking data.

Favourites, RAW files, video masters, shared albums and edited derivatives are presented as protection requirements for a future native cleanup plan. These drawer choices are session preview requirements, not saved/enforced removal policy. Unknown lineage/membership cannot be bypassed by unchecking them. Existing native folder protection is retained separately in the keeper review.

Recoverable physical storage remains unknown. Logical duplicate bytes are reported only for complete exact-byte groups whose entries have distinct known objects, one link each and equal recorded sizes. They do not imply reclaimable disk blocks. Same-volume quarantine frees zero bytes.

Can automate explains why no groups qualify in this build. Preview cleanup opens explicit blocked confirmation; Move to Recently Deleted and Undo remain disabled. Recently Deleted explains that no files have been moved by this workflow. There is no simulated success, automatic expiry, purge, renderer mutation or new production permission. Real removal/restore still requires an immutable native plan, fresh full-byte survivor/bundle checks, independent exact-version recovery and the existing native failure/recovery gates. The prototype's 30-day expiry is intentionally not adopted.

## Verification

Run `npm run check`, `npm run desktop:test:build`, then `npm run desktop:test` on generated fixtures only. Component coverage includes cluster search/filter/selection, conservative keeper choices and accounting, blocked cleanup/Undo, preview modes/retry and integrity distinctions. Native coverage updates existing toolbar/footer assertions, actual offline place selection, inspection table checks, rules confirmation, comparison screenshots when an indexed media pair exists, and minimum-window bounds. The native generated media scope may contain no matching photo pair; a separate visibly labelled browser fixture checks the filled queue, comparison pixels and narrow composition without claiming native pair evidence. A browser render does not establish native integration. Record actual outcomes and remaining platform/release limits in IMPLEMENTATION_LOG.md. Restore production frontend output with `npm run build` after instrumented native tests.

The v3 anchored search composition follows [Chakra Popover](https://chakra-ui.com/docs/components/popover); the heatmap is display-only and never attempts to bypass the WebView's image security boundary.

## Scan setup, reconciliation and version history — 2026-11-05

Duplicates, Integrity and Storage open with a welcome and the actual scan controls when no previous run exists. No empty duplicate studio or automatic first scan is shown. Choose registered libraries, scope and the tool's existing mode/properties before explicitly starting. Integrity baseline establishment still requires its own acknowledgement. Safe copy in Sync welcomes users into source/destination and verified/quick comparison setup; versioned backup retains its distinct profile setup and explicit approval flow.

Returning to an inspection restores its saved settings and report. A new read-only reconciliation run checks current files in the background while the previous report stays visible. This intentionally rechecks on page entry rather than trusting timestamps or claiming a watcher detects every byte change. It is not continuous monitoring while the page stays open. A saved baseline-establishment run refreshes as **compare with baseline**, never another establishment. Existing running work is followed rather than duplicated. Another active inspection or unavailable roots leave saved results visible with a reason; missing previously registered roots remain selected until explicitly excluded from the next scan, so scope never narrows silently; no retry loop bypasses admission or offline checks. Canceled/failed refreshes retain the previous displayed report. Scope changes still require explicit Scan settings (Measurement details in Storage) and Start. Storage retains its explorer and clear results canvas: first-use setup is inline, measurement settings/progress/issues/coverage move into the header dialog after a report exists, version/background status sits below the primary title row, and the history icon stays in the header.

Each utility offers a history icon that opens a portalled, keyboard-accessible Scan history popover. It lists dates, scan types and outcomes. Selecting a historical report does not cancel current work or start another scan; the selected version remains displayed when background work completes. Inspection history exposes the backend's newest 64 jobs and their immutable findings, including empty completed reports. Each version retains its actual scope and coverage. History is observations, not an undo or recovery copy of originals.

Backup comparisons now save display-only reports and findings in the existing immutable catalog history store, separately namespaced from destination-byte observations. They survive a catalog reopen and support existing category paging. Verified, quick, incomplete and terminal failure/cancel reports are distinct. At most 32 reports of 16 MiB each are saved, without deleting earlier versions; capacity/save failures are disclosed and the current run remains available for the session. Historical records have no plan digest, review plan or execution capability. Native previews and safe-copy approvals require the current live comparison; historical display paths are never execution inputs. See [ADR-0094](decisions/ADR-0094-utility-scan-history.md).

Returning to a saved comparison profile restores its report and performs a genuine read-only comparison through the existing native identity-bound adapter. No automatic copy, replacement, snapshot backup, restore, credential unlock or baseline creation is added. Active/recovery-needed transfers prevent automatic comparison. An unsaved comparison has no retained native destination authorization after restart: its report is still visible, but a new native destination selection is required before another scan. A relinked source never reuses the old comparison. The versioned-backup profile shows its latest recorded snapshot, with earlier snapshots behind the same history icon; choosing a version does not browse the repository or start a restore. Restic snapshot history and verification distinctions remain separate; refreshing scan observations does not establish current backup protection.

Focused checks: `npm test -- --run src/components/InspectionScreen/InspectionScreen.test.tsx src/components/BackupScreen/BackupScreen.test.tsx src/components/SnapshotWorkflow/SnapshotWorkflow.test.tsx`, `node scripts/cargo.mjs test --locked -p stillharbour-core --lib library::backup_compare`, and after the instrumented build, `npm run desktop:test -- --spec tests/desktop/utilities-scans.e2e.mjs`. Only generated media and disposable destinations are permitted. Actual outcomes belong in IMPLEMENTATION_LOG.md; checked-in tests alone do not establish a native pass.

## Storage content refinement — 2026-11-05

The Storage page keeps the current Stillharbour shell, colour tokens and inspection controls. Its compact overview separates actual per-volume used/available capacity from a coloured logical file-type breakdown. The current native interface does not provide authoritative physical allocation by file type: category sizes must not be subtracted from drive usage. Used space includes all unscanned/other use. Missing available space remains unknown even when capacity is known; no inferred free-space segment is created from folder totals.

`components/StorageExplorer` owns file-type, last-modified age and recorded-camera modes, removable filters, display-only folder breadcrumbs, grouping, proportional nested tiles, a numeric descending list, inspector and session review tray. Colour changes preserve geometry. Known cameras come from current indexed catalog detail through two bounded readers; retained inspection dates remain historical. Missing/unread/failed camera metadata is unknown, non-media is not applicable, and errors offer retry. No filename-based camera inference or new metadata extraction occurs. The inspector uses a labelled file icon rather than a fabricated preview.

File exploration starts with the retained findings page; Load more accumulates at most 2,048 observations. Category totals and filtering explicitly cover loaded observations. Drive totals and the full retained scan size stay fixed. The complete native folder accounting tree remains available through an expandable retained-totals view, with its original ID-based paging/navigation and unknown-allocation handling. Tiny files can be opened in a list and zero-byte files receive no artificial map area. The shared inspection formatter uses decimal B/KB/MB/GB/TB units, integer rounding and unit-boundary carry; raw counts remain in IPC and data attributes only.

The tray makes no deletion or recovery claim. Read-only, base-library and unknown-protection entries cannot be added; simulation excludes selected logical sizes without changing physical drive totals. No production mutation, original access, schema, IPC, credentials or dependency versions change. No demo data is shipped in this flow. Browser screenshots use isolated, visibly labelled generated fixtures under ignored `.context/storage-visual`; native smoke uses only the existing constructor-owned generated fixture.

Focused checks: `npx vitest run src/components/StorageExplorer/StorageExplorer.test.tsx src/inspection/format.test.tsx src/components/InspectionScreen/InspectionScreen.test.tsx`. Native smoke: `npm run desktop:test:build`, then `npm run desktop:test -- --spec tests/desktop/storage-explorer.e2e.mjs`. Existing Foundation assertions read raw data attributes for numerical comparisons rather than stripping punctuation from formatted units. Actual execution outcomes and limitations are recorded in IMPLEMENTATION_LOG.md.

Latest HTML reference (2026-11-05, filters-top revision): place search and filters above the storage overview, then the existing exploration workspace. Order the controls Search, File type, Camera, Age (last modified), Minimum size, Reset. Keep removable chips with the top controls and offer additional 5 GB/10 GB minimum sizes. This supersedes the earlier overview-first hierarchy only; it does not adopt the prototype's branding, embedded assets, browser file picker, download/export paths or duplicate/integrity mock findings. Native measurement, honest capacity handling and preservation limits above remain authoritative.

## Compact Storage workspace — 2026-11-05

The retained Storage explorer starts directly with the file filters, without an introductory paragraph, inline measurement setup, retained-inspection selector, job/status card, issue-review action or inventory coverage text. Concise version/background status sits below the primary title row, before the explorer. Before the first result, the utility welcome and real setup remain inline. The header's Measure storage / Measurement details button opens the native Storage measurement dialog containing the same explicit root/scope controls, start/cancel actions, current job, issues and coverage. The adjacent history icon selects retained versions. Close and Escape return focus to that header button. Escape is handled directly as well as through native dialog cancellation; a child control that consumes Escape keeps its interaction. Active work and incomplete coverage keep concise visible status; initial/error/no-report states remain explicit. Duplicates and Integrity retain their existing inline inspection interfaces. This presentation change preserves all native requests, original-file safeguards and read-only behavior.

## Individual tool workspaces — 2026-11-07

Backup opens the existing versioned snapshot/recovery workflow directly. Sync
opens source/destination comparison and reviewed one-way transfer directly;
there is no mode switch between them. Structure Formater opens directly on the
folder/filename rules, with the entire indexed library as its fixed scope.
Selection is not required. Native batches and result pages are bounded to 240,
but sequence and conflict checks cover the complete library. Unavailable items
are blocked rather than dropped. Saved templates/plans remain available. Actual
restructuring stays disabled pending whole-library execution and recovery gates.
Its Backup & sync check displays native active work, unresolved jobs and enabled
backup schedules, including future schedules. It refreshes automatically and
offers Check again after failures. Previewing remains available while blocked;
the page never stops jobs or disables schedules automatically. Sync has no
recurring scheduler yet. A clear check leaves the execution/recovery gate intact.
The existing selected-photo name-preview/copy-plan workflow remains separate.
The full page-by-page audit is in [UI_WORKSPACE_AUDIT.md](UI_WORKSPACE_AUDIT.md).

Structure Formater's date-based presets use illustrated radio cards; the
missing-date and stem-case options use labelled segmented controls. Event naming
is removed from the form. Custom paths and saved-rule compatibility remain;
choosing a different layout requires a new preview. See M2_ORGANIZATION_PREVIEWS
for the exact patterns and compatibility boundary.

The later expanded-layout request adds Date, Location, Tags, Camera and Custom
category segments with 25 presets in total. Category browsing preserves the
current rule until a preset is chosen. Custom is a searchable folder-chip builder
with explicit literals, remove/reorder actions and a filename pattern. Metadata
and ambiguous tag levels have explicit blocked states. The Event field is shown
only when the rule uses Event; Trip is omitted. See ADR-0096 for exact token,
compatibility, embedded-only location and reader-coverage semantics.
