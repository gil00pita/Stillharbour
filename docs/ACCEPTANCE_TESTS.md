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
| AC-001 | Build and launch a real Tauri window on each target OS. Native picker and a typed Rust round trip work. A browser-only shell does not pass. The initial screen shows "Start by selecting your Photo library folder." and "Choose a folder", without promotional supporting copy, milestone/database diagnostics or footer notices. Selection waits for native readiness; failures remain actionable. In M0, selecting a folder reports "Not scanned" without indexing or modifying it. Its card shows "Save & Start scanning" disabled, with the visible accessible explanation "Saving and scanning aren’t available yet." and no native identity message. No save/scan success is fabricated. In authorised M1, the button becomes functional only with a native selection and saves/starts the actual scan; failures retain the selection and show an error. |
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

## 3. Browsing, dates and performance

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

## 5. Duplicate matching and removal policy

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
- Selecting a folder removes welcome heading/icon/Choose a folder and moves keyboard focus into Library name; cancel restores the chooser.
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
- Hover/focus and selected tiles show checkboxes. Shift selection uses stable IDs in visible order; date selection says N shown. View/size changes retain selection and anchor. All-results selection freezes backend membership and a late freeze cannot apply to a changed query.
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

- Settings exposes root-specific Remove library and all-library Clear preview cache. Neither starts work before a native review plus explicit confirmation. Review identifies root label/path/count and warns that catalog notes/history are forgotten; originals/sidecars stay unchanged. Cancel/Escape performs no mutation and restores invoking control focus; completed last-root removal returns to onboarding.
- Only an opaque, unexpired native plan can execute; reject invented/replayed/stale plans. Active scans block maintenance. Other Settings actions remain disabled during review/execution. Errors require a new review and never show success. A failed post-commit refresh is reported as completed work with refresh failure, never retried as a mutation.
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
