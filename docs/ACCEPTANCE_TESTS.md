# Stillharbour - Acceptance and failure-injection test backlog

These are required test specifications, not claims that tests currently exist or pass. Assign each ID to a real test and record execution evidence during implementation. Use generated/disposable fixtures. For native behavior, run on both Windows and macOS; do not substitute a browser mock or a Linux container.

## 1. User-requirement traceability

| Requirement | Specification section | Milestone | Key acceptance IDs |
| --- | --- | --- | --- |
| Windows and macOS | 3, 20 | M0, M6 | AC-001, AC-077, AC-078 |
| Choose a base library and extra roots | 6 | M1 | AC-002 through AC-007 |
| Rename files/folders with presets and custom templates | 9, 10 | M2 | AC-023 through AC-032 |
| Preview new folder organization | 9.3 | M2 | AC-023, AC-024, AC-031 |
| Compare filename, extension, size, content, dates, attributes | 11 | M3 | AC-040 through AC-045 |
| Byte comparison, SHA-1, MD5 and modern hashes | 11.2, 11.3 | M3 | AC-040, AC-041, AC-046 |
| Ignore characters and descriptive metadata | 11.1, 11.2 | M3 | AC-044, AC-047, AC-048 |
| ID3 option | 11.2 | M3 scope explanation | AC-049; explicitly not applicable to photo-only modes |
| Ignore/handle hard links | 11.2, 14.2 | M3 | AC-050, AC-064 |
| Duplicate preview and preferred keeper folders | 11.4 | M3 | AC-051 through AC-055 |
| Remove newest/oldest by filesystem dates | 11.4 | M3 | AC-052, AC-053 |
| Backup to SMB, FTP, SFTP | 13 | M4 | AC-066 through AC-073 |
| Import selected source folders | 12 | M2 | AC-033 through AC-036 |
| Include/exclude folders and hidden entries | 6.3 | M1 | AC-008 through AC-010 |
| Corrupt-image checks | 14.1 | M3 | AC-059 through AC-062 |
| Largest files | 14.2 | M3 | AC-063 |
| Newest-to-oldest timeline and text-only list | 8 | M1 | AC-016 through AC-021 |
| Folder sizes and SpaceSniffer-style treemap | 14.2 | M3 | AC-063 through AC-065 |
| GPS map | 15 | M5 | AC-074 through AC-076 |
| Screenshot-inspired appearance | 16 | M1, M5, M6 | AC-018, AC-075, AC-079 |

## 2. Foundation and roots

| ID | Scenario and expected result |
| --- | --- |
| AC-001 | Build and launch a real Tauri window on each target OS. Native picker and a typed Rust round trip work. A browser-only shell does not pass. With no registered library, the initial screen shows the original folder icon, "Start by selecting your Photo library folder." and "Choose a folder"; the empty-library illustration is absent. In Photos, show the decorative, theme-aware illustration and "Your library is empty" only after a successful zero-result query when all registered roots are online, previously scanned and contain zero indexed files, with no active jobs or preview preparation. Do not illustrate loading, failed queries, offline/unscanned roots or filtered results in populated libraries as an empty library. Selection waits for native readiness; failures remain actionable. In M0, selecting a folder reports "Not scanned" without indexing or modifying it. Its card shows "Save & Start scanning" disabled, with the visible accessible explanation "Saving and scanning aren’t available yet." and no native identity message. No save/scan success is fabricated. In authorised M1, the button becomes functional only with a native selection and saves/starts the actual scan; failures retain the selection and show an error. |
| AC-002 | Choose an existing base folder containing nested JPEG/PNG files. Scan indexes expected entries without changing original bytes, names, or sidecars. Document unavoidable OS access-time behavior separately. |
| AC-003 | Add two nonoverlapping external roots. Queries filter by root or combine them without conflating duplicate content into one file location. |
| AC-004 | Add an already registered or nested root. The app rejects overlap with an explanatory message and does not double-count files. |
| AC-005 | Disconnect an external drive. Cached browsing remains available, root becomes offline, and original-dependent actions are disabled without deleting catalog records. |
| AC-006 | Reuse a drive letter/mount path for a different volume. The app does not trust it as the original library automatically. Explicit relink validates identity/evidence. |
| AC-007 | New root setup defaults to Managed with Read only switched off. Switch Read only on to register a root as referenced/read-only; reopening an existing root retains its saved mode. Both M1 modes only read originals. Register a root as referenced/read-only. Direct IPC calls attempting import/move/quarantine are rejected even if the frontend button is bypassed. |
| AC-008 | Test include glob matching a deep descendant of a nonmatching parent. The descendant is discovered; an explicitly excluded ancestor still prunes traversal. |
| AC-009 | Scan filters label the independent switches “Include hidden files” and “Include hidden folders”. Include hidden folders means traverse their contents, with hidden-file inclusion evaluated separately. Test dot-hidden files and OS hidden attributes with all combinations of file/folder inclusion settings. Results match the displayed rule explanation. |
| AC-010 | Add links, junctions/reparse points and a loop. Default scan skips them; optional permitted traversal cannot escape the root or recurse forever. |
| AC-011 | A nested directory returns access denied. Report that directory as incomplete, retain prior inventory, and continue unrelated directories. |
| AC-012 | A cloud placeholder is offline. Scan does not silently download it and does not call it corrupt; an authorized hydration step is explicit. |
| AC-013 | Read an image named with leading hyphens, quotes, percent characters, newlines, emoji and unusual Unicode. Metadata tools treat the name as data, not flags or protocol instructions. |
| AC-014 | An ExifTool config file in the working directory contains unwanted behavior. Bundled metadata extraction does not load it. |
| AC-015 | Kill a metadata/decoder worker on a malformed input. One typed error is recorded, the worker is replaced, and scanning continues. |

Windows picker regression, 2026-10-30 (selection-only AC-001 evidence): validate a generated ordinary directory and the containing drive root using read-attributes directory handles. Both return directory metadata without child enumeration; the handle cannot write. A regular file is rejected as a non-folder; a missing path stays an error and is not created. Fixture bytes remain unchanged. Final-component reparse points are refused. The owner-approved NTFS continuation now supplies verified identity for supported local volumes; unverified selections keep scanning unavailable. The regression does not authorize a drive scan or establish native dialog automation, network-drive, junction-ancestor or packaged support.

Identity alert continuation, 2026-10-30: a selection with identityAvailable=false renders a highlighted role=alert explaining why scanning is unavailable for that platform/volume and listing the Windows local NTFS limitation. Save & Start scanning remains disabled and aria-describedby resolves to the alert. Programmatic form submission must not call Save. A later verified selection removes the alert/description and enables the normal action; existing registered-root settings retain their behavior. The Windows desktop UI fixture exercises this alert at desktop/minimum sizes and verifies zero native roots/jobs; it is a mocked picker response, not a Windows scan or native directory-dialog test.

## 3. Browsing, dates and performance

Windows scan lookup regression, 2026-10-30 (AC-002/010/022): compare fresh handle-bound exact-name lookup IDs against literal enumeration across multiple directory buffers. Test successive different names, exact UTF-16 including an unpaired surrogate, case mismatch, wildcard-shaped names, missing/replaced entries, no-delete sharing and malformed response bounds/IDs. Retain junction, root/volume, filter and source-preservation tests. The opt-in generated 256/2,048-entry comparison reports warm old/new lookup times without a flaky timing threshold; it does not pass end-to-end large-library/P4 throughput acceptance.

| ID | Scenario and expected result |
| --- | --- |
| AC-016 | Index photos whose EXIF capture date differs from filesystem dates. Default timeline uses the selected capture-date provenance chain and labels fallback fields. |
| AC-017 | Use known UTC offsets, missing offsets, pre-1970 dates, subseconds and daylight-saving boundaries. Raw values remain intact; no unknown offset becomes UTC implicitly. |
| AC-018 | Toggle gallery/grid/text-only list with an active query and selection. Sort/filter/selection/anchor are preserved. Text-only mode initiates no thumbnail jobs. |
| AC-019 | Jump to a distant year in a 100,000-row catalog. Use an indexed cursor/date-bucket query; do not fetch all preceding rows. |
| AC-020 | Select all results in a paginated query. Backend scope includes unloaded rows exactly once and remains bound to a selection/query revision. |
| AC-021 | Change the computer timezone. Capture-local default ordering and a saved organization preview do not silently change. Explicit UTC sorting labels unknown-offset items. |
| AC-022 | Scroll during a scan and run large synthetic-catalog queries. Record p50/p95 latency, RSS, thumbnail-cache use and UI long tasks against declared hardware; report failures, not invented performance numbers. |

## 4. Planning, organization, import and recovery

M2 template-example continuation (2026-10-31, partial AC-024–026/031 coverage): save up to four independent synthetic examples using portable relative source names, optional capture-local dates, explicit sequences and expected targets. Real Rust IPC rejects mismatches, invalid dates, unsafe names and oversized payloads without appending a template version. Correcting and retrying retains examples with the immutable version; loading restores them. Missing dates obey Block or whole-pattern Undated fallback, and extension case remains exact. Empty lists preserve older templates/callers. Examples perform no filesystem I/O and do not establish native destination, collision, bundle or execution acceptance. Native fixtures use generated libraries and independent unchanged-source/destination receipts.

| ID | Scenario and expected result |
| --- | --- |
| AC-023 | Apply a preset to a fixture tree. Preview shows exact old/new paths, operation type, conflicts, companions and space estimate. Source tree is unchanged before approval. |
| AC-024 | Change a template or source metadata after creating a plan. The old approval cannot execute new targets; plan is invalidated or a new revision requires approval. |
| AC-025 | Render missing event/date/camera tokens. Documented fallback or blocking error appears; no fabricated metadata and no empty unsafe filename. |
| AC-026 | Render tokens containing parent traversal, separators, Windows reserved names, trailing dots/spaces and illegal control characters. Result is sanitized/rejected according to the visible policy. |
| AC-027 | Create a destination between preflight and publication. Native no-replace behavior preserves the existing file and returns a conflict. |
| AC-028 | Run a case-only rename and a cycle A->B, B->A on each target filesystem. Journaled staging preserves both contents and reaches correct final names. |
| AC-029 | Present canonically equivalent Unicode names or case conflicts on the destination filesystem. Collision handling uses actual destination semantics without lowercasing all original paths. |
| AC-030 | Simulate disk-full, permission loss, source unplug and destination unplug at each copy/publish phase. Source survives; staged files are recoverable; no false completed state. |
| AC-031 | Apply the same template twice to unchanged data. Second plan is no-op unless an explicit new policy was chosen. Preview and execution never allocate different sequence suffixes. |
| AC-032 | Organize media in a directory containing excluded or unknown files. Those files remain untouched; literal whole-folder rename inventories and previews all descendants separately. |
| AC-033 | Import two source folders containing renamed exact copies and new files. Exact-duplicate skipping is content-based, source files remain, and new targets are verified. |
| AC-034 | Import RAW/JPEG, Live Photo and sidecar bundles. Confirmed companions retain association and destination stems; ambiguous relationships require review. |
| AC-035 | Interrupt an import and restart it. Verified entries are not duplicated; mismatching existing targets produce conflicts. |
| AC-036 | Attempt import with destination inside a forbidden staging/backup/root-overlap location. Backend rejects recursion/authorization hazards. |
| AC-037 | Kill the process after journal intent, after temp creation, during copy, after destination publication and before/after catalog update. Recovery identifies actual state and never blindly repeats a removal. |
| AC-038 | Undo a completed rename after another file occupies the old path. Reverse-plan preview reports the conflict; the newer file is not overwritten. |
| AC-039 | Modify a selected source or replace a path with a link after approval. Revalidation detects identity/containment changes and stops dependent operations. Document any OS limitations honestly. |

M2 first-slice evidence, 2026-10-30: [organization previews](M2_ORGANIZATION_PREVIEWS.md) cover catalog-only relative names and lexical/missing-value checks for up to240 explicit IDs. Pure/catalog/component/native regressions are mapped there and in the implementation log. AC-023/024/025/026/029/031/032 have only partial preview coverage: no destination authorization, execution approval, native Unicode collision proof, execution idempotency, complete bundle inventory or space estimate is claimed. AC-027/028/030/033–039 and production enablement remain pending under the unchanged full scenarios above.

M2 companion-review continuation, 2026-10-30 (partial AC-023/032/034 evidence only): requesting details for a selected photo uses its catalog UUID, returns at most 64 same-directory candidates with explicit truncation, and includes missing/excluded entries and root availability separately. Same-stem and full-filename XMP/AAE matches are candidates only. Test no-match uncertainty, large byte counts, retry/error, stale preview/current revisions and late-response disposal. Source/candidate native identity changes alter the review fingerprint. A 65th candidate must not silently disappear behind a complete-inventory claim. Neither candidate review nor an empty result confirms a bundle, allocates companion destinations or enables copying. The generated native scenario retains source manifests; the generated-library native scenario remains macOS-gated, with actual Rust/frontend outcomes recorded in the implementation log.

M2 destination continuation, 2026-10-31 (partial AC-023/024/029/036/039 only): use picker-session authority and exact preview fingerprints to review a generated separate destination. Reject renderer paths/approval/rows, stale previews, wrong volume and equal/ancestor/descendant/app-data overlap. Independently open resolved native directory components, reject junction/symlink traversal on that chain and retain exact identity binding after picker spelling resolution. Missing or replaced indexed roots never prove nonoverlap. Test no automatic check on opening, picker cancellation retaining prior observations, unsupported identity, retry/error, stale in-flight results and late-response disposal. Source manifests and a destination-only fixture file remain unchanged. No probe file is created and no target name is reserved. Persistent plans, actual target collisions/case/Unicode equivalence, complete companions, capacity/fidelity, copying and executor recovery remain unpassed.

Retained M2 planning continuation, 2026-10-31 (partial AC-023/024 evidence): save template versions using latest-version compare-and-swap; previous versions cannot be rewritten/deleted. Save a destination-bound draft only through picker-session authority and an exact fresh preview, retaining its resolved rows/template reference/native evidence and content fingerprint. Reopen after restart and retain identical digest/targets; changed catalog inputs show staleness without rebasing. Reject a mismatched catalog or damaged digest; unknown history cursors are errors. Review JSON/CSV text omits private native locators and CSV formula cells are protected. Migration tests independently open retained schema-2 snapshots, preserve catalog UUID/history, roll back schema/table collisions and exit an owned child after snapshot/before commit/after commit. These tests do not establish original-file executor recovery or physical power-loss durability. All saved plans remain drafts with explicit blockers and execution disabled. Complete companion inventory, native target semantics, space/fidelity, trees, native file-save export, approvals, executor/import/undo and applicable full AC-023–039 scenarios remain open; M2 is not accepted.

M2 selected-path tree review, 2026-10-31 (partial AC-023 only): expandable Before groups literal source display paths by stable root ID, preserving duplicate names across roots and nonportable separators as text. After groups proposed relative folders, retaining every collision participant, file/folder collisions, exact large byte counts and unresolved rows. Hostile names such as __proto__ and markup are data. Replacing a preview replaces its tree scope; search filters the diff table without concealing selected rows in the trees. Native generated-library review compares tree counts with actual retained IPC rows and verifies source/destination preservation. Full source-folder hierarchy, native target validation and M2/M3 completion are not claimed.

## 5. Duplicate matching and removal policy

Native review-file save coverage, 2026-10-31 (partial AC-023/036): save JSON/CSV from retained plan review into a picker-authorized generated folder outside all photo roots/app-data. Reject renderer paths/text/names/overwrite flags and native overlap/identity failures. Verify exact generated bytes and unchanged destination-only neighbors. A repeated filename must preserve the existing file; an interrupted/conflicting stage remains retained with journal intent, without automatic replacement/deletion/replay. Cancellation/unsupported picker/native failures produce no success receipt. This is new metadata-document output, not original-file executor/undo/recovery or backup acceptance.

Continuous M2/M3 prerequisite coverage, 2026-10-31: existing target review must distinguish occupied files, file-shaped parents, unsafe symlink/junction/package ancestors, preview blocks and native absence. Test existing case/Unicode aliases against actual filesystem lookups, unchanged destination-only bytes, fingerprint changes after target insertion and explicit unreserved space figures. Test backend-supplied source components, Unix literal backslashes, hostile names and omitted legacy fields preserving immutable row/plan digests. This is partial AC-023/024/029/036 coverage; future-name equivalence, full capacity/fidelity and original-file execution/recovery remain open.

| ID | Scenario and expected result |
| --- | --- |
| AC-040 | Two files have identical bytes but different names/extensions/paths. Exact-content mode finds them; names do not restrict the preset. |
| AC-041 | Two files have identical size and sampled regions but differ elsewhere. Full hashing/comparison rejects exact equality. |
| AC-042 | Same filename/size/dates but different image content matches a property rule. It is labeled a candidate, not a verified duplicate, and cannot auto-quarantine. |
| AC-043 | AND/OR advanced criteria yield the documented groups. Missing birthtime or unsupported attributes are unknown and never silently ignored. |
| AC-044 | Name normalization removes selected characters/case/whitespace. UI displays comparison keys; original filenames are unchanged and empty keys do not cause universal matches. |
| AC-045 | Modified-time/birthtime filters use explicit tolerances and provenance. They do not substitute EXIF capture time or Unix ctime. |
| AC-046 | Inject an intentionally colliding mock hash result for different file bytes, including the MD5/SHA-1 code paths. Final byte comparison blocks removal. |
| AC-047 | Two supported images decode to identical full-resolution pixels but differ in descriptive metadata. Normalized comparison can match them, exact-file mode cannot, and cleanup requires manual review. |
| AC-048 | Different ICC/orientation/high-bit-depth inputs would look equal only after lossy normalization. The engine respects the declared comparison policy and does not claim exact equality by discarded precision. |
| AC-049 | Photo-only modes expose ID3 as not applicable/documented, not a checkbox with no implementation. Unsupported container normalization returns a typed capability result. |
| AC-050 | Two hard-linked paths and an independent copy exist. Object identity recognizes aliases; default accounting and removal suggestions do not count links as independent full copies. |
| AC-051 | A group has preferred and protected keeper folders. Folder protection wins over age/path ranking and cannot be bypassed with select-all or forged IPC. |
| AC-052 | Choose remove newest/oldest by birthtime and by modified time. Suggestions match the selected field and direction, with an explanation for each keeper. |
| AC-053 | Creation dates are unknown or tied. The engine requests review unless a documented explicit tiebreaker exists; it does not invent an original. |
| AC-054 | Only the preferred keeper is offline. The app refuses to quarantine the only accessible verified copy. Cached previews and offline backups do not satisfy the keeper invariant. |
| AC-055 | A JPEG is duplicated but its RAW/motion/sidecar companion is unique. The unique component is not removed as part of automatic bundle cleanup. |
| AC-056 | Modify bytes while restoring original size/mtime before quarantine. Fresh full comparison detects the new content even if the old fingerprint cache looked valid. |
| AC-057 | Quarantine a reviewed group and restart the app. Entries can be restored through a new reviewed plan; used disk space is not falsely reduced by the same-volume move. |
| AC-058 | Future visual-similarity feature: A~B, B~C, A!~C. Grouping preserves pairwise evidence and never claims the whole connected component is exact. This test is required before enabling that later mode. |

## 6. Integrity and storage

| ID | Scenario and expected result |
| --- | --- |
| AC-059 | A truncated image header is readable but full decode fails. Quick and deep results differ, with coverage shown. |
| AC-060 | RAW embedded preview is healthy but RAW image data is damaged or unimplemented. Preview success does not count as a passed full RAW validation. |
| AC-061 | A decompression-bomb/oversized-dimension fixture exceeds limits. Worker is bounded/stopped, UI remains responsive, and no destructive response is automatic. |
| AC-062 | A baseline digest changes after a legitimate edit. Label changed-since-baseline separately from decode corruption and retain previous evidence. |
| AC-063 | A generated tree has known 100/200/300-byte non-image entries plus media. Largest-file order and recursive totals match selected media-only/all-files scope. |
| AC-064 | Hard links, sparse files and clone-capable volumes are present. Logical, unique-object, allocated and potential-reclaim metrics remain distinct; unsupported exact values are unknown. |
| AC-065 | Treemap zoom/filter and accessible folder table show the same scope/totals. Excluded and unreadable entries are disclosed; partial scan is not called complete. |

## 7. Backup, restore, map and distribution

| ID | Scenario and expected result |
| --- | --- |
| AC-066 | For each advertised local/SMB/SFTP/FTPS/FTP destination, back up fixtures and catalog snapshot, remove only the temporary source, restore to a new directory, and compare full contents. Record actual server/OS/version. |
| AC-067 | SFTP unknown/changed host key fails closed; explicit trust workflow shows a fingerprint. FTPS invalid certificates fail. Plain FTP requires a clear warning. |
| AC-068 | Interrupt backup during transfer, disconnect server, exhaust quota and expire credentials. Session stays failed/partial/canceled as appropriate; no fake success or surprise unlock/prune. |
| AC-069 | Disconnect one included root before backup. Result identifies missing source scope; it does not silently count that root as successfully protected. |
| AC-070 | Delete/quarantine a source after a successful backup. Earlier snapshots remain recoverable until an explicitly reviewed retention action. |
| AC-071 | Differentiate structural repository check, partial sampled read, full read-data and actual restore comparison. A sampled pass never grants full-library verification status. |
| AC-072 | Restore a catalog after adding new local edits. Pre-restore backup and collision/migration review protect those edits. Absolute root paths are relinked rather than assumed on another OS. |
| AC-073 | Quit the app or sleep across a scheduled time. Behavior matches the documented running/tray/catch-up model; no fully-quit scheduling claim without an installed tested background agent. |
| AC-074 | Photos have valid zero coordinates, missing coordinates, identical coordinates and positions across the antimeridian. Missing GPS does not become (0,0); viewport results/clusters remain correct. |
| AC-075 | Clicking a map cluster and switching to timeline preserves date/root filters and selection. Large queries use bounded data; markers do not explode into one DOM node per library asset. |
| AC-076 | Go offline and inspect network logs. No unapproved photo metadata upload; map clearly reports unavailable tiles; attribution and provider cache/offline policy are respected. |
| AC-077 | Install on clean Windows/macOS machines without Rust/Node/Perl/Homebrew. Packaged metadata, thumbnails, optional codecs and backup helpers function for every advertised capability. |
| AC-078 | Release signing/notarization and updater verification are tested with actual credentials/artifacts. Missing credentials are reported as a release blocker, not called passed. |
| AC-079 | Keyboard-only and screen-reader review covers gallery/list selection, inspector, template preview, duplicate keep controls, map alternatives, treemap table and recovery dialogs. Color is never the sole error signal. |
| AC-080 | Production bundle has no mock-success backend, arbitrary filesystem/shell permission, test WebDriver server or exposed secret. Dependency/notice review matches the exact shipped binaries. |
| AC-081 | Concurrent scan, organizer and backup target overlapping roots. Locking prevents incompatible mutations, and app-generated watch events do not create phantom duplicates. |
| AC-082 | Export a CSV containing names starting with =, +, -, @ and unusual line breaks. Spreadsheet-oriented export escapes formula hazards while preserving a lossless JSON alternative. |
| AC-083 | Write catalog metadata to a supported XMP companion after an external edit. Conflict review prevents silent overwrites and preserves unknown namespaces and recognized ACR/JSON companions. |
| AC-084 | Kill the app during database migration or catalog backup. Recovery never opens a silently half-migrated schema and a previous valid catalog backup is available. |
| AC-085 | Release performance documentation labels objectives as targets until measured and records actual fixture, hardware, cache state, error counts and command evidence. |

## 8. Fixture requirements

Create deterministic image fixtures rather than copying private user photos. Include orientation variants, EXIF date/GPS combinations, ICC profiles, transparent images, high-bit-depth files, metadata-only variants, deliberately truncated/corrupt inputs, sidecar pairs, and identity/link cases. Native code paths need native temporary filesystem fixtures.

Use clearly licensed small RAW/HEIC test assets only when programmatically generating the required format is impractical. Record source, usage rights and expected decoder outcome. The supplied screenshots are not a test dataset for hashing, codec integrity or facial recognition.

Fault injection must be reproducible by named operation stage. Keep a matrix linking each stage to source/destination/catalog/journal postconditions. An operation executor cannot be enabled for real originals solely because the happy path passed.

## 9. Mandatory version 1.1 supplement

The 40 PS-001 through PS-040 scenarios in PRESERVATION_TESTS.md are additional release gates. They cover the owner's highest-priority preservation requirement, fixed-library sync, safe replacement, hidden/unsupported-file coverage, strict recovery evidence and transfer pacing. All 85 original AC scenarios remain applicable; none are removed. Total specified scenarios: 125. These counts describe the backlog, not executed or passing tests.

## M1 execution evidence

[M1_VALIDATION.md](M1_VALIDATION.md) maps AC-002–AC-022 to executed generated-fixture checks and identifies unverified native environments. The scenarios above remain requirements; an unavailable OS/provider is not a pass.

### Scan setup controls — owner revision, 2026-10-27

Verify tag add/remove/edit, duplicate suppression, Enter/Tab/blur, multiline paste, IME input and commas inside brace globs or folder names. Preview and Save include an unfinished value even without a blur. No From/Through date controls appear in Scan filters; opening a root with legacy scan bounds and saving/rescanning clears both bounds. Browsing date filters remain available. Changing switches or adding tags alone never saves or starts a scan. Generated native fixtures verify the persisted Managed mode, tag array and null scan dates, without modifying originals.

### Independent navigation scrolling — owner revision, 2026-10-27

With expanded scan settings in a generated library at 1180×780 and 660×560, scroll the main pane from top to bottom. Main content moves while the sidebar, logo and navigation retain their viewport positions; document and sidebar scroll offsets remain unchanged. All settings/actions remain reachable and no horizontal overflow appears. Saved-library gallery/list/Inspector scrolling and keyboard focus restoration continue to work.

### Streamlined selected-folder form — owner revision, 2026-10-27

- Sidebar shows the native application version and no tagline/local-design footer copy.
- Selecting a folder removes welcome heading/illustration/Import photos and moves keyboard focus into Library name; cancel restores the chooser.
- Advanced is a collapsed, full-width native accordion. Toggle it by pointer/keyboard and retain configured values. No Include patterns field appears; save/rescan clears legacy include-glob bounds.
- New File Extensions tags prefill the exact 33-suffix list in CODEX_SPEC.md. Display dots, normalise upper/lowercase input and deduplicate `.JPG`/`jpg`. Persist canonical suffixes and verify uppercase source filenames are indexed. Preserve saved custom extension scope.
- At normal and minimum window sizes, expanded form/accordion have no independent scroll range. All fields/actions remain reachable through the main pane, with sidebar geometry and document scroll unchanged. Browse gallery/list and Inspector keep their existing scrolling/focus behavior.

Discovery tests with generated undecodable files prove suffix coverage and unchanged source bytes; they do not prove RAW/HEIC decoding support.

### Jobs progress bar — owner revision, 2026-10-27

- A running scan shows a labelled indeterminate progress bar with current native counts and “total files not yet known”; equal indexed/found counts must not imply 100% while discovery is running.
- Pending cancellation remains active with “Stopping scan…” until the backend confirms a terminal outcome. Native `completed`, including an empty completed scan, shows a full bar. Partial/canceled/interrupted/failed scans stop activity and keep their explicit outcome without a completion percentage.
- At normal and 660×560 native window sizes, the progress track and existing Details/Cancel controls remain reachable without horizontal overflow. Reduced motion removes the activity animation. No real source library is a test fixture.

### Scan size unit selectors — owner revision, 2026-10-27

- Minimum size and Maximum size each have a labelled KB/MB/GB selector. Empty fields default to MB. Decimal units are stated in the interface; 1.5 MB saves/previews as `1500000` bytes and 2 GB as `2000000000`. Changing a selector applies its new scale to the entered amount independently of the other field.
- Empty clears a restriction; zero stays zero. Reopening and saving byte-exact legacy limits, including values beyond JavaScript Number precision, must not round or alter them.
- Negative, malformed, fractional-byte and above-u64 drafts display accessible errors and block both Save and Preview. A valid edit or unit change can correct the draft. Disable both controls while busy; backend minimum/maximum validation remains effective.
- At 1180×780 and 660×560, controls fit the Advanced grid without horizontal overflow or a nested form scrollbar. Generated native fixtures verify canonical byte persistence and unchanged sources. Browsing filters remain outside this scan-settings refinement.

### Chronological photo canvas — latest owner revision, 2026-10-27

This replaces the earlier gapless/no-checkbox requirements; their historical evidence remains in IMPLEMENTATION_LOG.md.

- At 1180×780 and 660×560, Timeline preserves complete photo ratios, uses consistent 4 px gutters and day headings, and leaves incomplete final rows left-aligned. Sparse date groups retain distinct headings. No horizontal overflow or overlapping pagination/navigation appears.
- Filename search stays outside the scrollable collection. Navigation collapses to a named icon rail. Root/date/size/extension/tag/favorite filters remain real catalog queries, with removable active chips. Thumbnail presets change density; uniform-grid fit/crop is explicit.
- Actual filtered year/month counts drive keyboard/pointer navigation. A month/year seek reaches the correct date without fetching intervening pages. Recently added uses first-added dates. Undated stays distinct; other sorts hide chronology. First-visible date excludes virtual overscan. Returning from a following page preserves a prior month seek.
- Hover/focus and selected tiles show checkboxes above both tiny and clear preview layers in Timeline and Uniform grid. A visible checkbox must win pointer hit testing over the image. Shift selection uses stable IDs in visible order; date selection says N shown. View/size changes retain selection and anchor. All-results selection freezes backend membership and a late freeze cannot apply to a changed query.
- Contextual actions state scope. Add tags retains existing tags and rejects an entire batch if any merged asset exceeds 64 tags. Failed catalog writes remain failed. Compare opens only 2–4 explicitly selected assets; its copy denies proof of exact duplication. Unsupported edit/export/album/grouping controls have reasons and no fake success.
- Viewer opens as a real modal, supports fit/100–400% zoom/pan, filtered previous/next beyond the page, optional filmstrip and information. Escape closes information first, then the viewer. Closing restores the invoking photo focus and exact gallery scroll offset, including after catalog annotations. Cached preview limits are disclosed. Offline originals can show cached images without claiming accessibility; unavailable/failed images have explicit states.
- Text list mounts no image elements and sends no nonempty thumbnail batches. Opening a viewer is an explicit separate image request; gallery preview work pauses while the viewer owns the bounded queue.
- Generated source manifests remain unchanged through browsing, comparison and annotation. No raw original mutation command, schema migration or permissions expansion is introduced. Windows/VoiceOver/full-resolution support require their own evidence. Similarity grouping stays an explicit M3 gate.

### Settings and indexed library locations — owner revision, 2026-10-28

- Settings is keyboard accessible from expanded/collapsed navigation, including before the first folder is added. Its heading receives focus; controls fit 1180×780 and 660×560 without horizontal overflow, with normal main-pane scrolling and fixed navigation.
- Library locations show actual catalog roots and their availability/path/mode. Change location routes an opaque native picker selection and the existing root ID through identity-checked relinking. Cancellation preserves roots; a different identity is refused. Add photo folder opens selected-folder setup without scanning before explicit Save. Originals stay in place.
- Save defaults persists through a fresh native preference-store read. The default registered library, view, density, fit, sort and navigation state apply to newly opened Photos. Selecting a view in Settings does not submit the form. An unavailable saved library is labelled instead of silently displaying All libraries.
- New-folder defaults apply to subsequent setup, including pending extension/exclusion tags and size thresholds. Existing root mode/filter settings remain unchanged. Saving defaults starts no scan, changes no annotations and preserves source manifests.
- Scan settings is an inline Settings group with a registered-library selector, saved per-root mode/filter controls and explicit Save & Rescan / Discard scan changes actions. No separate setup screen or dialog opens. Gallery links select and focus the requested root's group. Changing the selector never scans and does not carry unsaved edits between roots.
- Save & Rescan sends the selected stable root ID through the existing mode/scan commands, retains the Settings page and app-default drafts, reports failures without success and preserves scan drafts for correction. Disable duplicate submission while starting/running; busy scan startup disables conflicting defaults/location actions. Scan persistence and source preservation are checked with generated fixtures; saving scan settings must not write app defaults. Forms remain separate and valid, with normal main-pane scrolling at desktop/minimum sizes.
- Discard scan changes restores the selected root's saved mode/filter, clears pending tags/size validity/sample preview and feedback, and never saves defaults or starts a scan. Saving defaults continues to leave existing root settings unchanged. With no roots, explain that a photo folder must be added first.
- Reject invalid/default-root IDs, enum/version/filter/size errors, unknown fields, oversized files and linked preference files. Failed/corrupt/future-version settings remain preserved; failed reads and writes do not display success or silently fall back. Retry and draft discard behave explicitly. Discard restores saved amount/unit/validity and clears pending tags, sample/preview state; the shared Advanced controls reset to collapsed. Catalog identity/schema and registered root IDs remain stable.

### Condensed gallery and scroll loading — owner revision, 2026-10-28

- Resting Photos contains title/search/Filter, the Jobs icon and the canvas/timeline; no header Add folder, stacked filter/display/count bars or pagination footer. Folder registration remains available in Settings. The batch toolbar appears only for a nonempty selection.
- Filter opens a named native modal right panel. At 1180×780 and 660×560 its independently scrollable body exposes all query/sort/display/facet/availability controls, with Close and Reset/Apply reachable. Escape/Close cancels drafts and returns focus to Filter; Apply commits valid drafts together. Reset clears filters without changing app defaults. Facet buttons never submit the form.
- Select all freezes the applied query only; unapplied query drafts disable it with a reason. Availability is explicit in the panel and unavailable tiles remain labeled in the canvas. Similar groups stays unavailable.
- Scrolling loads subsequent native chunks (maximum 240 per IPC) with no Previous/Next buttons. Keep at most three chunks/720 summaries; evict the distant end, preserve the visible stable-ID/offset and retrieve earlier rows on upward scrolling, including scrollbar movement, wheel and keyboard navigation. Programmatic anchor restoration must not trigger reverse loads. Query/date changes reset the window. Stale replies cannot repopulate it; duplicate edge events cannot start concurrent continuations.
- Failed continuation retains displayed rows and exposes Retry. Catalog-revision refresh is bounded. Filter/viewer/Compare pause background paging and previews. Text list starts no nonempty preview work and contains no images.
- Native generated tests traverse 960 synthetic catalog observations in both directions, assert the 720-summary bound and preserve 32 generated source entries. They do not establish real-library throughput, physical wheel/picker gestures, Windows or VoiceOver behavior.
- Native seek tests compare every supported sort against ordinary filtered queries, including exact size keys beyond JavaScript safe-integer precision, cursor resumption, empty boundaries and rejected malformed IDs/limits. No schema migration, original write or production test capability is introduced.

### Library removal and indexed preview cleanup — owner revision, 2026-10-28

- Settings exposes root-specific Remove library and all-library Clear preview cache. Forgetting/catalog cleanup requires native review plus explicit confirmation. Clicking Remove first automatically stops that root's scan and queued/running preview jobs, including initial preparation, then builds fresh counts. Show stopping progress and disable confirmation until jobs drain. Review identifies root label/path/count and warns about lost notes/history; originals/sidecars stay unchanged. Cancel/Escape retains the library and restores invoking control focus, leaving jobs stopped; completed last-root removal returns to onboarding.
- Only an opaque, unexpired native plan can execute; reject invented/replayed/stale plans before stopping more jobs. Revalidate revision/expiry after draining; new changes require review. A 30-second stopping deadline retains the library/cache and releases admission for retry. Other-root jobs are not canceled; active unrelated work and cache-only maintenance retain their existing conflict behavior. Test cancellation of real queued/held preview workers, scan handoff/admission, failed stop, other-root isolation and stale/invented confirmations. Other Settings actions remain disabled during review/execution. Errors never show success. A failed post-commit refresh reports completed work with refresh failure, never retries a mutation.
- Cache cleanup targets only indexed keys, batches at most 256 keys and uses the held owned directory descriptor. Reject traversal, symlinks, hardlinks, nonregular/unexpected content and replacement. A replaced directory cannot redirect cleanup. Unknown/unindexed files are retained. Original roots are never enumerated/deleted for either action, including offline roots.
- Partial cleanup or catalog conflict retains the root and cache index for retry; missing indexed cache files are idempotent. Catalog deletion removes only the target's records/issues/history and dependent token membership, with atomic rollback on database error. Other roots/notes/previews remain. Promote a remaining base root with unchanged UUID; last-root removal survives reopening. Foreign-key check is clean.
- Cache-only clearing preserves root configuration and favorites/ratings/tags; previews regenerate for accessible originals. Workers cannot republish cleared results after the cleanup barrier. Removed default-root IDs resolve effectively to All libraries without a preference-file rewrite; no original writes or schema migration occurs. Generated native/core fixtures verify source manifests unchanged, actual cleanup, confirmation/cancel, persistence and desktop/minimum layouts.

### Folders explorer — owner revision, 2026-10-28

- Folders starts with named registered libraries. Opening one shows immediate child folders/files; opening a child excludes descendants until entered. Back, Up and accessible breadcrumbs navigate the hierarchy; breadcrumb overflow cannot consume the content viewport. Navigation history stays bounded.
- Icons mode presents named folder/file glyphs and real cached media thumbnails. Details has no image nodes/nonempty preview batches. Opening media uses the existing immersive viewer constrained to the exact native folder, then restores invoking focus/scroll; non-media files show catalog-only File details with no external launch.
- Show only indexed content admitted by saved scan filters, including empty directory observations and indexed non-media files. Filename filtering keeps folders visible. Explicitly identify blocked, retained/partial observations, offline originals, failed/unsupported previews and failed reads. An inaccessible library is never presented as an empty filesystem. Retry preserves scope and late replies cannot replace a newer location.
- Native pages return at most240 entries, directories first and stable native-byte keyset order, with counts and Previous/Next items. No full-tree renderer list, raw SQL/path execution or live directory enumeration is added. Reject cross-folder/root/search cursors, invalid IDs and out-of-range limits. Distinct native path bytes retain distinct IDs even with identical lossy labels; this is not proof the host filesystem supports creating those names.
- Native exact-directory media queries exclude nested files and bypass whole-root inventory count shortcuts. Existing recursive folder filters remain compatible. Other Photos/Settings/removal workflows retain their preservation gates. Generated source manifests, catalog UUID/schema/root IDs stay unchanged through browsing. Windows, physical gestures, screen-reader/deep-path/large-folder performance need their own evidence.

### Header Jobs dropdown and sidebar Settings — owner revision, 2026-10-28

- Jobs has no dedicated route or sidebar entry. Every screen header (onboarding, Photos/Tags, Folders and Settings) offers an icon button named Jobs with expanded state, an active-count badge and accessible count description. Settings stays in the sidebar footer above the version in expanded/collapsed layouts.
- Click or keyboard activation opens a nonmodal dropdown with all running/cancel-requested scans from native status. Preserve current gallery query, selection, mode and scroll position. Status continues to poll; completion removes the active badge and retains the outcome in collapsed Recent jobs. Connecting and No running jobs are distinct states.
- Reuse the truthful indeterminate progress/counts and explicit terminal outcomes. Cancel uses only the backend-issued job ID, disables while stopping, refreshes native status and exposes failure rather than claiming success. Issue details load only on request, escape untrusted text, ignore stale results after closure, and remain recoverable after a failed request.
- Opening focuses the dropdown. Escape or Close dismisses and returns focus to Jobs; outside pointer or focus dismisses without stealing focus. No modal backdrop or focus trap blocks the underlying workspace. The overlay escapes page clipping and remains within the viewport, including after resize, with its own bounded scroll area.
- Native checks at1180×780 and660×560 cover expanded/collapsed navigation, running/recent jobs, keyboard dismissal, reachable controls and unchanged generated originals. Settings/footer remains anchored while main content scrolls. No schema/IPC/scanner or original-file-write feature is introduced.

### P1 performance regression gate — 2026-10-28

The bounded catalog-read update must satisfy P1-01 through P1-07 in [PERFORMANCE_ACCEPTANCE_TESTS.md](PERFORMANCE_ACCEPTANCE_TESTS.md): exact field/order/selection semantics, indexed boundary seeks, bounded fresh summaries, actual release measurements and independent source preservation. Functional native tests do not establish release frame or Windows performance. Results and unrun gates belong in [PERFORMANCE_P1.md](PERFORMANCE_P1.md).

### M5 read-only native format slice — 2026-10-28

| ID | Required behavior | Current evidence |
| --- | --- | --- |
| M5-F01 | Packaged HEIC/HEIF, TIFF/GIF/BMP and declared camera samples produce bounded cached previews with provenance. | Passed on the recorded arm64 macOS host, 18 expected cases; see [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md). Other OS/cameras not run. |
| M5-F02 | No original/sidecar changes; independently inventory sources before/after including failures. | Passed for retained packaged and native fixture manifests. No real library tested. |
| M5-F03 | Retain input/dimension/output/deadline bounds, reject invalid ICC, preserve alpha/orientation, and monitor native child memory. | Unit/integration and packaged negative cases pass; sampled RSS is not a whole-process-tree quota. |
| M5-F04 | New policy retries old unavailable failures without schema migration and preserves compatible indexed M1 positive previews offline. | Passed in isolated catalog regressions and packaged HEIC legacy-failure retry. |
| M5-F05 | Native gallery/viewer loads HEIC, returns to the same query and keeps original mutations disabled. | Passed macOS debug in a separate native test invocation; independent 36-file manifests match. |
| M5-F06 | Complete AC-083 clean-machine format/OS, color, sidecar/metadata and full-validation gates before broad release claims. | Not run for this slice; remains open. Preview success is not full RAW validation. |

### Balanced chronological gallery — owner revision, 2026-10-28

- A mixed sequence with a panorama between two short landscape runs keeps the panorama in its own row without reordering, cropping or omitting assets. A dense eight-landscape case avoids a tiny final orphan. Rows may exceed target height by at most25%; sparse final rows never exceed target. Preserve4px gutters, finite positive geometry and no viewport overflow, including missing/extreme dimensions and the720-summary retained window.
- Day headings retain date provenance and exact shown-photo selection membership. Compact visible captions keep full accessible names/title, including historical years and unknown dates. Suppress secondary shown counts when there is insufficient heading width; retain the count in the checkbox label.
- Multiple resize notifications before a frame trigger one layout using the latest width and preserve the existing visible-ID/offset anchor. Cancel pending work on unmount. Gallery/viewer return, date seeks, ordered range selection and true image-free Text list remain covered by native regression checks.
- Check1180×780 and660×560 in the real Mac test app and independently compare generated-source manifests before/after, including failed runs. Record unrun OS/accessibility/performance gates. Article benchmark numbers are not acceptance evidence for Stillharbour.

### Progressive preview slice — owner revision, 2026-10-28

[PRV-01–PRV-05](PROGRESSIVE_PREVIEWS.md#acceptance) cover separate durable thumbnail jobs, explicit backfill/cancel/recovery, large generated JPEG/PNG/TIFF, per-tier failures, actual-load reveal/reduced motion and independent original manifests. Thumbnail jobs never reconcile missing files or update last-scan history; maintenance must reject concurrent publication. Keep existing text-only, offline, viewer-position, orientation/ICC and retained positive-cache acceptance. Platform and broader P2 gates require their own evidence.

### Gallery scrollbar and focus — owner revision, 2026-10-28; clarified 2026-10-29

- For chronological results, the timeline is the only visible vertical scroll control; the native gallery scrollbar is hidden. Other sorts retain a normal scrollbar. A single dense month, equal timestamps and Undated-only results keep an enabled handle that follows scrolling within retained rows. Wheel input over photos keeps normal viewport scrolling, and wheel input over the date rail forwards pixel/line/page distances without hijacking Control-wheel zoom.
- A focused date handle follows subsequent visible-date and within-month scroll changes. Dragging within the current month moves its retained rows before release; another month issues one indexed seek on release, including outside the rail, even when some of its rows are already loaded. Read the release coordinates even without an intervening move event; cancellation performs no catalog seek. Arrow/Page/Home/End keys remain usable. Upward wheel input at the retained top requests earlier rows even when scrollTop cannot change. Keep the three-page/720-summary bound and clarify that fractional positions cover retained month rows, not global asset ranks.
- Clicking/returning to a photo never draws a focus outline around the whole photo viewport. The viewport remains keyboard reachable and individual photo/selection/date controls retain their focus indicators. Existing viewer return, chronology, selection, paging, resize and source-manifest tests still pass.

- Native hidden/restore resizing must update row width even when animation frames are suspended; visible resize bursts still coalesce and retain the photo/pixel anchor. Canceling a Settings review must restore its invoking-control focus without waiting for a painted frame.

### Header navigation toggle — owner revision, 2026-10-28

- One icon-only collapse/expand button appears before the current header title near the sidebar divider, with no duplicate or text collapse row under the logo. It controls the same sidebar, reports its expanded state and retains an accessible action name/tooltip.
- Expanded and collapsed layouts work at desktop and minimum native window sizes; the control remains available across onboarding and application sections. Search, Filter, Jobs, navigation footer, gallery position and saved-default behavior remain functional.

## Live library reconciliation — owner revision, 2026-10-29

- In generated native roots, externally add/modify/move/remove files and add/remove a folder. Gallery/Folders update without manual Rescan; old confirmed locations leave normal views. Moves with unambiguous native identity retain asset ID, first-added time and catalog notes. Ambiguous/cross-root locations do not merge by name/hash.
- At the gallery beginning, incoming leading photos appear. Older windows retain stable anchors and the existing 720-summary bound. A removed last folder-page entry recovers rather than falsely declaring the whole directory empty. An open removed folder remains navigable through Up/Back.
- Journal Added/Moved/Updated/Removed/Reappeared observations transactionally after an initial baseline. Ordinary unchanged rescans create no false changes; canceled/incomplete/excluded scopes do not manufacture removals. Lossless path locators distinguish native names with identical display strings.
- Disconnect/invalidate a root during or between scans: retain inventory/history and show offline/inaccessible/relink state. Reconnect/restart and overflow/missed-notification fallback reconcile through the same scanner. Read/cache writes do not create scan feedback; bursts remain coalesced and only one scan is admitted.
- Jobs → Library changes loads bounded pages only while open, shows old/new paths as escaped text, monitoring state and observed time, and supports Older/Latest/error/retry. Late responses cannot overwrite a new scope or closed history. History survives catalog restart and root forgetting; removal review discloses retained path history. Cache clearing retains history. Malformed/oversized/newer journal payloads fail explicitly.
- Independent manifests match before/after each read-only phase around explicitly simulated external fixture changes, and restored original file paths/bytes/native identities/metadata match. Retain parent-directory mtime differences caused by the test’s external move/restore actions explicitly. Production excludes test controls and no original mutation API is added. Record actual native OS evidence and limitations; do not claim network-event completeness or Windows scanning from notification-library support.

## Shared read-only resources — owner stage continuation, 2026-10-29

[P3-01–P3-08](PERFORMANCE_ACCEPTANCE_TESTS.md#p3--shared-read-only-resource-stage) cover atomic bounded admission, native volume grouping, priority/fairness, queued cancellation/error recovery, scroll-aware preview requests and independent source manifests. Retain all earlier offline, stale-revision, preview-budget, maintenance and original-preservation acceptance. Fixture contention never passes real backup, transfer pacing, physical-device topology, NAS/HDD or restore tests; dispositions live in [PERFORMANCE_P3.md](PERFORMANCE_P3.md).

### Aligned sticky dates and faster cached previews — owner revision, 2026-10-29

- Every consecutive date starts its own justified row; full-width headings share one left edge. The current heading stays at the photo scrollport top and yields to the next date without duplicated controls. Test normal/minimum layouts, push/replacement and retained shown-group selection. Preserve Grid, image-free Text list, timeline seeks, viewer return and bounded rendering.
- Owner revision 2026-10-31: only 32 px fallback images receive an 8 px blur; larger cached fallbacks remain clear. Keep the tiny image visible until the larger image actually loads, then cross-fade and sharpen the larger image from 2 px to zero blur over 400 ms without changing geometry. Clip blur inside each tile, preserve terminal-failure fallbacks/retry and already-loaded clear DOM/source through status refresh. Reduced motion and occluded windows reveal immediately. Check native scrolling for seams; generated native screenshots must not substitute for the user's exact source/driver reproduction.
- During scroll, only tiny keys may schedule source work; clear keys use typed cache-only lookup. An uncached online observation with an inaccessible synthetic source reports pending without starting/canceling/advancing the work queue. Validate 64-key bounds, optional legacy compatibility and malformed flag rejection.
- Settled clear admission resumes after 150 ms, pending readiness is polled at 200 ms, and visible-first/overscan order and480-key/240-photo/decoded-pixel/cache bounds remain. Record actual warm-scroll observations and source manifests, including trace loss/coverage. Do not claim decode/physical I/O or full P4 acceptance from these timing controls.

### Timeline endpoint feedback — owner correction, 2026-10-29

- With a short final month still below an earlier visible month, scrolling to the actual gallery bottom must set the timeline value exactly to its maximum and thumb to 100%. Scroll back to the catalog beginning and verify 0. Keep first-visible restoration anchors.
- Verify capture/Recently added/Unknown date, nonfinal labels, subpixel rounding and page boundaries. More results available prevents exact-end feedback; a fitting tail seek with earlier pages available reports the end, while a wholly fitting collection starts at 0. Keep stable dates, sticky sections, renderer/IPC limits and read-only source manifests.

### Oriented gallery proportions — owner correction, 2026-10-29

- A generated 600×450 JPEG with EXIF orientation 6 renders as a portrait preview and a 3:4 Timeline tile; natural decoded proportions and tile geometry agree within 1 display pixel. No landscape-shaped side padding from the unrotated sensor pair.
- Existing catalog rows with retained EXIF orientations 1–8 return display dimensions consistent with rotation/mirroring through the common page/detail/neighbor/folder projection. Stored source dimensions/raw metadata remain unchanged by these reads, with no catalog migration, source re-read or required rescan.
- Already-oriented dimensions do not transpose twice. Missing/invalid JSON, missing sensor dimensions, unknown orientation and incomplete stored dimensions retain their geometry without query failure. Strict numeric strings from retained metadata are accepted; malformed orientation strings are not coerced.
- Keep original bytes/paths/metadata unchanged through native generated-fixture browsing. Uniform grid Fit/Crop, selection, sticky dates, bounded paging and unavailable states remain intact. Native Windows and physical-input evidence must be reported separately.

### High-resolution JPEG preview correction — 2026-10-29

- A generated tall JPEG and the specifically authorized8192×12288 attachment produce bounded native file previews, including32px placeholder and2048px viewer tier, without original changes or a Rust full-resolution raster.
- Enforce16384px maximum JPEG side and144000000 source pixels independently in header/native/frame guards; reject over-axis/over-pixel inputs. Preserve PNG/TIFF/byte-path geometry limits, malformed/ICC/writable-handle checks, read/output caps, timeout, child RSS watchdog and shared admission.
- Limit failures are Unsupported preview-budget results, distinct from genuine WorkerFailed decode errors; backend causes stay private. Obsolete p2-v2 negatives retry under p2-v3 on the next request while positive prior-policy caches remain usable and historical job outcomes remain truthful.
- Keep the supplied photo out of Git and all normal test fixtures; use generated committed regressions and independent manifests for the authorized disposable copy. Local worker/native results do not establish Windows, cold-cache or full process-tree evidence.

### P4 local stress evidence — 2026-10-29

[P4-01–P4-06](PERFORMANCE_ACCEPTANCE_TESTS.md#p4--local-stress-evidence) require bounded segmented traces, source preservation, separate current release-core/worker and debug-native scale/failure results, full checks and honest unavailable-gate dispositions. Production release builds must continue to reject automation. Local synthetic stress cannot pass backup, physical endpoint/pacing, NAS/HDD, Windows, complete WebContent/GPU memory or native release presented-frame acceptance.

### P4 continuation — empty filtered reads, 2026-10-29

- A warm exact no-match page in all five sort orders and both page/timeline request orders stays below64KiB logical pager misses on the10k regression fixture. Retain correct empty rows, no continuation and current catalog revision, with invalid cursor/limit/filter failures unchanged.
- Actor and external catalog writes invalidate proven emptiness even without a library revision bump; a newly matching observation appears in both page and timeline. Root counters set to zero while10k rows remain must still show240 page rows and all timeline counts.
- Release million-row measurements retain before/after raw samples, positive-match controls, executable/SQLite/lock provenance and independent unchanged sources. Native literal no-match search returns to populated photos. [The evidence report](PERFORMANCE_FILTERED_READS.md) records actual checks and unavailable gates; this does not complete P4 or authorize original operations.

### P4 continuation — positive timeline totals, 2026-10-30

- After a positive filtered timeline, a page and stable-ID seek in each of the five sorts add at most 64 KiB logical pager misses over the identical already-count-cached projection on the 10k fixture. Compare complete page fields and seek identity/totals. Existing expensive ordered projections are accounted separately rather than relabeled as bounded seeks.
- Keep positive tag/root scopes distinct and invalidate after actor/external changes without revision bumps. A complete positive timeline must not replace lagging unfiltered inventory counters; include-missing exact counts remain exact. Undated and added-date summaries retain existing semantics and validation failures.
- Retain separate page-first/timeline-first release million-row samples, empty/narrow/broad controls, binary/SQLite/lock provenance and unchanged lossless source manifests. Full repository/native/package checks and unrun gates are recorded in [PERFORMANCE_POSITIVE_TOTALS.md](PERFORMANCE_POSITIVE_TOTALS.md). No cold-filter latency, full P4, platform or preservation gate passes solely through this cache reuse.

### P4 continuation — selective filename projections, 2026-10-30

- With an exact count already cached, the selective filename page on the 10k fixture reads under 2 MiB logical pager bytes after clearing only its connection pager. Keep cold-count measurements separate. Preserve complete pages, all five sorts, keyset continuation, both stable-ID seek directions, bucket totals and viewer neighbors against the original SQL.
- Test exactly 1,024 and 1,025 raw matches, plus a small visible count with over-limit excluded/non-media raw names. Fall back without truncating results. Cover mixed root/directory/tag/favorite/date/size/extension/folder filters, missing states, escaped literal text, native i64 rowids and writes without revision bumps. Retain all original query validation, cache freshness/bounds and inventory semantics.
- Release comparisons include early/late narrow and broad controls, both request orders, actual/reference plan labels, binary/lock/SQLite provenance and independent unchanged sources. Retain the eager-probe broad regression and reject that policy. Record full repository/native/package outcomes and open gates in [PERFORMANCE_FILENAME_READS.md](PERFORMANCE_FILENAME_READS.md); no full P4 or production mutation authorization follows.

### First library preview preparation — owner revision, 2026-10-30

- Scanning artwork provides separate dark/light palettes: inherited dark surfaces and mint accent by default; a data-theme="light" ancestor yields white frames, pale sage scenery and a deeper green lens. Verify resolved frame/accent colours in both themes, restore the prior theme afterward, and visually inspect the artwork on matching dark/light backgrounds. Do not switch artwork to light solely because the OS is light while the app shell is dark.

- An empty collection during scoped queued/running/stopping discovery shows “Scanning your photos”; initial preview preparation shows “Preparing your photos”. Show a decorative photo-card/searching-glass illustration instead of settled-empty artwork or no-match wording. Animate queued/running jobs only; paused/stopping states and reduced-motion preferences remain still. Real rows replace the waiting view; errors and settled empty/filtered results do not show scanning artwork. Check responsive fit and computed motion in generated native desktop/minimum-window fixtures; no source or preview requests are needed for the SVG.

- On a generated first scan, raw catalog observations may exist while no pending/placeholder photos mount in the collection. Each introduced ready photo has an owned generated preview; failures remain explicit unavailable outcomes. Count/date/page/cursor/neighbor/selection scopes agree. Existing completed catalogs, inventory lag and cache-only behavior retain their previous contracts; no root/catalog identity or schema migration occurs.
- Discovery has no percentage/remaining claim while the total is unknown, even when found equals indexed. Once the native preview inventory exists, generated/cached/unavailable outcomes produce an accurate processed count and nonnegative remaining count. Queued and zero-inventory running states stay indeterminate. Stop/cancel/failed/interrupted outcomes never imply completion.
- The normal preparation banner fits one row (at most 54px high) at desktop and minimum window sizes: actual scanned/processed count, inline progress bar and stop/pause control. Discovery shows native discovered-file counts as “N files scanned”; preview work shows actual “N previews processed”, including unavailable outcomes. No unknown-remaining summary or invented total/percentage is displayed. library-content computes to zero padding; the banner meets its top/left/right bounds while its content child preserves 24px desktop / 16px narrow horizontal padding. Progress retains its accessible description. Detailed counts stay in Jobs; unavailable outcomes, paused messages and action errors remain visible when applicable. No horizontal overflow or pending photo tiles are introduced.
- Pause the queued initial job before handoff and during work; automatic admission/reconciliation cannot undo cancellation. Restart retains deferred observations, and explicit resume reuses the durable job/cache. Inventory/revision changes prevent false initial completion. Failed requests/status refresh and late replies after changing folders are clear and recoverable.
- Native desktop/minimum layouts retain the canvas, header Jobs, selection/anchors, image-free Text list and unchanged generated sources. Test-only bounded holds/automation remain absent from production. Record actual evidence, failures and unsupported platform/performance gates in [INITIAL_LIBRARY_PREPARATION.md](INITIAL_LIBRARY_PREPARATION.md).

### Page-scoped viewer and cached-request recovery — owner revision, 2026-10-30

- Previous/Next display left/right arrow icons with accessible names and hover titles. Preserve keyboard navigation, disabled boundaries and 40 px pointer targets.

- Open a loaded photo from Photos/Tags or Folders. Its measured thumbnail expands to an aspect-preserving fitted frame within the page; sidebar controls remain visible and hit-testable. The covered underlay is inert, the nonmodal dialog is labelled, and focus starts on X.
- Clicking the photo/background does not close it. X/Escape reverse the transition before unmounting; Escape closes Info first. Preserve filtered neighbors, zoom/pan, annotations, image-free Text list and gallery focus/pixel return. Missing/offscreen targets use a local fallback; navigation/unmount cancels animation and Reduced Motion/hidden windows skip spatial movement.
- While thumbnail/high-detail transport or image decoding remains pending, show a diagonal loading-only glare and textual status over the existing smaller pixels. Stop it on actual load or terminal failure; pause while hidden and provide a static Reduced Motion state. No hover effect or endless failure animation.
- A single image-element cache failure gets exactly one fresh owned-cache request. Exhaustion falls back explicitly. Gallery Retry is a separate accessible control, clears old ready responses and revalidates through existing bounded native preview commands. Source/revision/tier changes reset attempts; no automatic scan, purge or original repair. Failed native protocol responses are non-storable.
- Use generated native fixtures and independent manifests. Verify desktop/minimum page/menu bounds, no click-to-dismiss, real navigation/notes/return and release test exclusions. Identify held/mock loading screenshots honestly; do not infer production performance or the cause of every private screenshot failure.

### Original bytes and favorite heart — owner revision, 2026-10-30

- A generated JPEG wider than 2,048 px returns identical original bytes and original dimensions; admitted PNG/WebP return identical bytes and correct MIME. Independent source manifests remain unchanged through reads. IDs/revisions/policy govern access; invalid/stale/offline/replaced/linked/unsupported/over-budget data never becomes an arbitrary source URL or silent success.
- Only static JPEG/PNG/WebP within 32 MiB and 32,000,000 pixels are admitted. Validate actual geometry in the supervised child, bound chunk parsing and original concurrency, honor maintenance/cancel_all and retain all helper/source guards. Original responses and errors are no-store; no general filesystem permission is added.
- Loaded originals remove the old routine cached-preview banner and replace preview pixels only after load. Original readiness independently enables zoom. Offline/unsupported/failure states retain an explicit preview fallback and Retry original. Glare ends on actual load/terminal failure and honors hidden/reduced-motion states.
- X appears at the top right. The bottom heart has an accessible favorite name, aria-pressed, 44 px target and busy/error state. Fill/animate only after a confirmed catalog favorite, support unfavorite, skip entrance animation for existing favorites/navigation and ignore late animation feedback for a different photo. No original metadata write.
- Native generated-fixture checks confirm original-protocol pixels, absence of the old banner, right-side X, persisted heart toggles, navigation and gallery return. Production instrumentation exclusions and packaged helpers still pass. Do not claim unrun platform/format or color/memory certification.

### Floating viewer icon controls — owner revision, 2026-10-30

- Zoom out/in, Fit image, Info and Filmstrip are icon buttons in one labelled Photo controls group, with hover titles, accessible names and 40 px-wide buttons. Preserve slider/percentage, zoom limits and pressed toggle states.
- Info/Filmstrip leave the header and remain reachable in the floating surface over the photo, including at 660×560 and with the Info panel/filmstrip open. The existing favorite heart immediately precedes Info in visual and keyboard order. The bar is absolutely positioned inside the photo bounds, outside the pan/scroll canvas; all controls and navigation arrows pass native pointer hit testing, with the control icons in one row at the minimum size. Stack a bounded scrolling Info panel below the photo/filmstrip at narrow widths. Preserve focus return, original readiness, zoom/pan, page/menu bounds and per-photo zoom reset; annotations do not reset zoom.
- Use real generated native fixtures for layout/navigation checks; original access, fallback, source preservation and all existing milestone/platform limits stay unchanged.

### P4 continuation — cold filename summaries, 2026-10-30

- On the generated 10k catalog, cold empty/early/late selective filename counts use no more than 64 extra SQLite VM steps over a media-then-filename reference predicate. Preserve the original excess-work failure; this is a statement-work check, not a wall-clock or physical-I/O target.
- Retain filename threshold/escaping/native-key and all-sort page/seek/neighbor parity, missing/excluded/non-media and mixed scopes, exact bucket totals, initial-preview filtering, actor/external invalidation and lagging inventory counters. All query validation and SQL value binding remain intact.
- Compare current and legacy cold count/timeline statements on the same release million-row fixture, with empty/narrow/broad/late controls, real plans, VM steps, logical pager bytes, raw samples, binary/SQLite/lock provenance and unchanged independent lossless source manifests. Record full repository/native/package outcomes and open gates in [PERFORMANCE_COLD_SUMMARIES.md](PERFORMANCE_COLD_SUMMARIES.md). Do not turn a debug/native regression pass into full P4, platform, memory, release-presentation or preservation acceptance.

### P4 local closeout before M2 — 2026-10-30

- Reproduce under-capacity producer loss and retain the failed prototype. A held preferred lane can route an event through another bounded lane; a real eight-producer 1,024-event batch retains every event in the regression. Full capacity/all-lane contention still reports exact losses. Keep 4,096 events total, sixteen private aliases, consuming reads, production no-ops and isolated test instances.
- An owned child paused between fork/exec cannot extend a closed catalog’s lifetime lock. SQLite closes before explicit unlock; genuine live catalogs still reject another owner. No source operation, open retry or lock bypass. Retain the before/after OS35 regression and earlier historical uncertainty.
- Fresh 10k/100k/1m debug native routes retain full 120-step/60-second trace, source and collector evidence. Reports keep collection statuses distinct from incomplete WebContent/GPU/XPC ownership; missing/failed/truncated samples cannot pass complete collection. [PERFORMANCE_LOCAL_CLOSEOUT.md](PERFORMANCE_LOCAL_CLOSEOUT.md) records exact results and all unrun external/platform/preservation gates. M2 remains held for this task.

### P4 final acceptance record — 2026-10-30

The [current P4 status](PERFORMANCE_P4_STATUS.md) maps all six scenarios to exact current-source release/native/worker/contention/regression evidence and missing prerequisites. Current release catalog and packaged-worker source stat fields round-trip losslessly through JavaScript while older artifacts remain unchanged. Local passes never promote blocked P4-06 results; M4-dependent backup and endpoint tests, complete memory, release presentation and target-platform results require their actual environments/adapters. Keep original-file mutation disabled.

### HEIC original-viewer regression — owner fix, 2026-10-30

- Generated 3,072×2,048 HEIC, HEIF alias and uppercase HEIC originals return identical source bytes and image/heic through ID/revision/policy authorization. Native WKWebView displays the original at its real dimensions with no original-unavailable warning, hides the preview fallback and restores the gallery/query on close.
- Reject sequence brands/timed tracks, malformed/overflowed/oversized FileTypeBoxes, minor-version brand confusion, truncated/oversized/count-exceeding top-level boxes and unsupported codecs. Preserve source/helper/32 MiB/32-million-pixel limits and stale/offline/invalid-identity/policy failures. Unsupported installed webviews/variants retain honest preview/retry.
- Packaged original loading confirms exact HEIC bytes/dimensions and unchanged generated sources; no personal library, source conversion, dependency/schema change, cache purge or Windows/minimum-OS claim. See ORIGINAL_IMAGE_VIEWING.md for scope and evidence.

## Approved restrained polish regression scope — 2026-10-30

For the existing shell/gallery/Settings/viewer scenarios, verify the [approved polish](INTERFACE_POLISH.md) on desktop and the native 660×560 minimum. Standard field/select rows remain aligned, at least 36px high, readable and within their parent; primary actions/viewer tools keep 44px targets with actual pointer access. Preserve the 4px photo gutters, aspect ratios, bounded views, focus return, independent scrolling and unavailable-feature reasons. Confirmed favorites retain catalog persistence/failure/Reduced Motion behavior with no overshooting easing. Organization rows show actual library names, retain inspectable IDs and format exact byte counts without Number rounding; unsafe names and execution remain blocked. Existing source manifests must match their stated coverage. Native Windows, broad accessibility/release and later milestone gates remain open.


### Windows NTFS scan continuation — 2026-10-30

Partial target-platform coverage for AC-001–013 and AC-015–022: a generated NTFS folder registers through real native IPC, Save & Start scanning is enabled only after file/volume verification, actual worker-generated previews decode in the Windows WebView, and all fixture sources remain unchanged. Exercise nested JPEG/PNG/WebP, native UTF-16 names, more than one directory buffer, hidden/excluded scopes, root overlap and wrong captured volume, missing/replaced roots retaining inventory, junction escape refusal, read cancellation/retry and cache destination collisions. A mocked unsupported-selection UI case verifies the highlighted accessible alert and zero jobs; it does not substitute for the real scan case. Date provenance remains filesystem-based when capture metadata is unavailable. Other filesystems/transports, full Windows metadata/format coverage, large-folder performance, clean-machine/signed packages and original mutation/recovery remain unpassed. See ADR-0021 and IMPLEMENTATION_LOG.md for exact evidence and limits.

## Companion filename confirmation — 2026-10-31

Saved draft review explicitly inventories the direct native folder, independently of gallery filters. ADR-0026 records scope/bounds and metadata-only descriptors. Verify hidden/unindexed entries, exact native IDs for unrepresentable names, changed revisions even with restored mtime, replaced/offline roots, 4,096-entry truncation and unobserved indexed companions. Confirm only after full-scope acknowledgement; reject omitted/unsupported hints, unknown/duplicate IDs, stale fingerprints and conflicting targets. Identical retries retain one decision; reopening history keeps source/draft identity and no execution authority. Confirmed records do not silently alter drafts or exports. Full executable-plan integration and M2/M3 gates remain open.

Compiled-copy manifest coverage, 2026-10-31 (partial AC-023/024/034): require exactly one explicit confirmed decision per draft photo; reject duplicates, foreign/changed decisions and modified inventories. Test overlap merging without lost memberships, conflicting target/source assignments, file/folder conflicts, exact large-byte sums and stable order. Fresh identical retries return the same immutable manifest; retained opening recomputes reconciliation without claiming live validation. Native fixture compilation uses real IPC and independent preservation receipts. Copy approval/execution and all full M2/M3 gates remain pending.
