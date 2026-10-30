# Stillharbour - Codex implementation specification

All repository paths in this document are relative to the repository root.

Specification version: 1.2.1
Naming revision: 2026-10-26 (no functional scope change)
Preservation and Backup & Sync supplement: 2026-10-23
Original research date: 2026-10-20
Library feature/provider research: 2026-10-26
Status: proposed product and engineering specification; not an implemented application
Product name: Stillharbour (selected by the owner; trademark/domain clearance not established)
Product identity: docs/PRODUCT_IDENTITY.md
Primary targets: Windows and macOS desktop

Read AGENTS.md and docs/PRESERVATION_CONTRACT.md before implementation. Read docs/LIBRARY_FEATURES_ADDENDUM.md for v1.2 refinements and separately gated optional work. Version 1.1 preservation requirements take precedence over weaker earlier defaults. Read docs/BACKUP_SYNC_ADDENDUM.md and docs/PRESERVATION_TESTS.md for the new workflow and mandatory safety gates. Read docs/RESEARCH.md for market rationale and docs/SOURCES.md for primary-source references. Bracketed identifiers such as [R01] refer to that source register. Normative requirements in this document are proposed product decisions, not statements that the application already supports them.

## 1. Product outcome and boundaries

Build Stillharbour, a local-first photo organizer that lets a person understand, organize, verify, and back up their real photo folders without surrendering control of those folders. The application combines a chronological photo browser, a folder organizer with dry-run previews, a duplicate review tool, a storage analyzer, a map browser, and recoverable backups.

The differentiator is safe, explainable file management: show what will happen, why it will happen, which copies survive, and how to recover. Do not compete initially on professional image development, cloud sharing, or a mandatory account.

An existing folder can be indexed without moving, renaming, rewriting, or deleting anything. The user's original directories remain ordinary directories. The catalog is an index and a store for annotations, not an opaque container for originals.

First release means milestones M0 through M6 pass their acceptance gates. M1 is a read-only vertical slice, not the completed first release. Do not silently drop later milestones and describe the application as feature-complete.

Original M0-M6 first-release boundary: mobile applications, a hosted server, multi-user concurrent catalogs, two-way device synchronization, a full editing/develop module, general-purpose audio management, autonomous deletion, custom cryptography, and training an AI model are excluded. Version 1.2 explicitly adds a basic recipe editor, large cloud-library imports and a future mobile basic editor as separately approved N3-N5 extensions; it does not claim they are delivered or silently change platform scope. Integrity reporting, Recently added, catalog files and tags refine existing M1/M3/M5 features through N1/N2. Optional local face recognition, semantic search, OCR, GPX geotagging and smart culling remain later extensions.

## 2. Technology decision

Use Tauri 2 for the desktop shell, React with strict TypeScript and Vite for the interface, Rust for the application core, and SQLite for the local catalog. Use MapLibre GL JS for the map. Rust is selected for efficient native processing, explicit resource control, and memory/thread-safety benefits, not a claim that one language is fastest for every workload. Tauri uses platform WebViews, so Windows and macOS still require separate visual and integration testing. [R01, R02]

Use a modular monolith, not microservices. Business rules must be independent of Tauri and directly testable in Rust. The frontend is a client of typed commands; it is never the authority for filesystem safety or duplicate deletion.

Recommended implementation components, to be verified and pinned during M0:

| Concern | Initial choice | Implementation rule |
| --- | --- | --- |
| Desktop | Tauri 2 | Narrow capability set; explicit custom-command permissions |
| Frontend | React, TypeScript, Vite | Strict types; no filesystem or subprocess authority |
| UI primitives | Accessible reusable components; Radix or equivalent after M0 review | Keyboard and screen-reader support, not visual-only controls |
| Large results | Virtualized gallery rows and table rows; TanStack Virtual/Table candidates | Do not mount the full collection in the DOM |
| Database | SQLite via rusqlite | Dedicated database actor; bounded read connections; migrations |
| Job orchestration | Tokio | No blocking image decoding or hashing on async executor threads |
| CPU work | Bounded worker pools; Rayon where appropriate | Coordinate decoder and hashing parallelism to avoid oversubscription |
| Exact hashing | BLAKE3-256 by default; SHA-256 option | Versioned cache; fresh evidence before destructive actions |
| Legacy hash options | MD5 and SHA-1 | Compatibility/reporting only; byte comparison required before removal |
| Metadata | Bundled, pinned ExifTool behind an adapter | Read-only in initial milestones; safe argument protocol |
| Basic images | Rust image crate in an isolated media-worker | Enable only tested formats; explicit color-management policy |
| Broader images | LibRaw; HEIF adapter; libvips if packaging/performance spike justifies it | No claim that a single crate handles every RAW/HEIC/ICC case |
| Backups | Bundled restic; bundled rclone for transport | Use established snapshots and encryption, not a new backup format |
| Filesystem watching | notify candidate plus reconciliation | Notifications are hints; rescans are authoritative |
| Secrets | OS-backed credential adapter | macOS Keychain and Windows credential storage; never frontend state |
| Mapping | MapLibre GL JS | Local photo coordinates; configurable tile provider |
| Tests | Rust tests, frontend unit/component tests, desktop WebdriverIO | Instrumented test builds only; no automation server in production |

ExifTool supports broad metadata extraction; image provides common-format native Rust decoders. LibRaw explicitly supports RAW data and embedded previews. libvips is a candidate when its performance and memory behavior justify extra packaging and licensing work. These are capabilities to evaluate, not interchangeable dependencies to add unconditionally. [R18, R19, R20, R21]

Pin exact versions and checksums after checking compatibility and security advisories. Commit Cargo.lock and the JavaScript lockfile. Record native binary architecture, source, license, notices, and build flags. Do not embed unreviewed nightly dependencies or download executable tools at first launch.

## 3. Platform and distribution assumptions

Initial mandatory release targets: Windows 11 x64 and macOS Apple Silicon. Add an Intel macOS build only after it passes the same clean-machine tests; do not advertise support based only on compilation. Determine the minimum supported macOS version during M0 from all chosen dependencies and actual test machines. Windows ARM64 and Linux are subsequent targets, not implicit promises.

Prefer direct signed desktop distribution initially. Produce a Windows installer and a signed/notarized macOS application in a DMG. App Store sandboxing and store-specific entitlement behavior are a separate delivery project. Tauri documents platform-specific signing requirements. [R30, R31]

Users must not need Rust, Node, Perl, Homebrew, a terminal, or a system-installed rclone/restic to run a release. Package and sign the required helpers. Test startup, folder selection, metadata extraction, previews, backup, and restore on clean machines without developer tools.

## 4. Domain vocabulary

**Catalog:** one local database containing roots, file observations, photo metadata, annotations, jobs, and recovery history.

**Library root:** a user-authorized directory on a particular filesystem/volume. One root is the preferred import destination. Extra roots can be local folders, removable disks, or mounted network shares.

**Referenced root:** indexed in place and read-only from the application's perspective. Enable the Read only switch to select this mode. New roots default to Managed per the owner's 2026-10-27 scan-settings revision; existing saved modes are preserved.

**Writable root:** a root for which the user has explicitly enabled application-managed import or organization. This does not authorize arbitrary mutation: each operation still needs a reviewed plan.

**File entry:** a specific directory entry. Two different paths can be two entries referring to one hard-linked filesystem object.

**Asset:** a logical photo or supported media item. It may have confirmed associated files, such as a Live Photo motion component or an XMP sidecar.

**Bundle:** a confirmed collection of related files that must be considered together during operations. RAW/JPEG variants and motion pairs are related content, not automatically duplicate waste.

**File revision:** an application-observed version of an entry, with recorded identity, size, timestamps, and optional content digest. A path is not a permanent identity and a timestamp is not proof of content.

**Operation plan:** an immutable, versioned list of proposed effects and preconditions, created without changing original files. Approval authorizes that plan only.

**Quarantine:** an application-managed recoverable holding area, normally on the source volume. It is not a backup and does not immediately reclaim space.

**Backup snapshot:** a restic snapshot with recorded source scope and verification state. It is not merely a successful network transfer.

## 5. Non-negotiable safety invariants

S-01. Scanning, searching, inspecting duplicates, calculating sizes, and generating previews never intentionally change original bytes, names, or embedded/sidecar metadata. Ordinary filesystem reads may update OS access times; do not misrepresent this as a content or descriptive-metadata edit.

S-02. All imports, sync transfers/replacements, renames, moves, quarantines, restores, external-file metadata writes, and backup-retention actions have an explicit backend-created plan. The frontend cannot invent a destination or bypass a precondition at execution time.

S-03. A confirmation applies to a specific plan ID, revision, and digest. Any changed target, keep rule, template, scope, or material source evidence creates a new plan and requires confirmation again.

S-04. No silent overwrite. New publication uses no-replace primitives. Explicit destination replacement requires a retained, verified previous version and tested conditional publication against the reviewed revision; otherwise keep both or block. A prior existence check alone is insufficient.

S-05. A hash match is evidence for an exact-match candidate, not permission to delete. First-release exact-duplicate quarantine requires fresh full-byte equality against a surviving accessible keeper. Approximate, attribute-only, or normalized-pixel matches never trigger automatic removal.

S-06. Keep at least one accessible, byte-verified copy of each exact content group outside quarantine. Offline roots, cached previews, expired backups, or unavailable repositories do not qualify as the sole survivor. Protected roots are never auto-selected for removal.

S-07. Never drop essential bundle components. An exact match of two JPEGs does not prove that either attached RAW, motion clip, sidecar, or edit file is redundant. Ambiguous bundles block automatic cleanup.

S-08. Cancellation is cooperative and leaves a recoverable state. A crash is recovered from a durable journal. Do not claim that a SQLite transaction makes a filesystem operation atomic.

S-09. Never follow a symbolic link, junction, or reparse point outside authorized roots. Default scanning does not follow them. Revalidate containment and identity during execution, not only in the UI.

S-10. Unknown, unsupported, denied, corrupt, missing, and offline are different states. Do not turn an error or a disconnected root into an empty directory and then delete its catalog records or backup history.

S-11. Backups include originals, confirmed companions, and a consistent catalog snapshot plus portable settings and root mapping. No live database file is copied blindly.

S-12. No default permanent deletion or automatic quarantine expiry. Purge is a separate, explicitly approved feature with new evidence and a preview. First release may leave permanent deletion to the OS rather than offer a purge command.

S-13. Original image bytes are never changed to compute a duplicate comparison that ignores descriptive metadata.

S-14. Production paths and real user libraries are never test fixtures. Generated fixtures and disposable temporary directories are mandatory during development.

S-15. Safety guarantees assume a functioning filesystem and no undetectable hostile concurrent rewrite. Hold handles and use available sharing restrictions; detect normal concurrent changes and stop. Do not claim protection against all disk failures, other applications, administrator actions, or kernel-level attacks.

S-16. Preservation is the highest-priority requirement. Read-only discovery and preview never intentionally mutate sources; failure pauses affected work instead of bypassing safety.

S-17. Organizing/importing defaults to verified copy with source retained. In-place rename, source relocation, duplicate quarantine and explicit source metadata writes require exact-version, independent verified recovery evidence under docs/PRESERVATION_CONTRACT.md.

S-18. Safe one-way sync retains destination-only files and previous destination versions. No mirror deletion, automatic history expiry or two-way source writes in this extension.

S-19. Quick comparisons, completed uploads, skipped files and sample restores never count as full verified protection. Report precise scope, revision, method, time and any fidelity limitations.

S-20. Transfer controls account for owned-engine comparison, hashing, copy, verification and history I/O. Unsupported helper or physical-I/O ceilings are explicitly disclosed, not simulated by UI controls.

## 6. Onboarding and library management

### 6.1 First-run flow

Show the value proposition and a clear statement that indexing leaves files in place. Ask the user to choose a base folder with the native directory picker. Default new folders to Managed and provide a Read only switch instead of a mode dropdown. Enabling Read only saves referenced mode. Existing folders retain their saved mode when settings open. Saving Managed records the mode for future approved operations; it does not execute a write or bypass plan, preservation or recovery gates. Let the user add extra roots and label each root.

Ask whether hidden files/folders should be included; default to excluded. Show default excluded system/package/cache directories. Explain catalog and thumbnail cache locations and approximate configurable cache limits. Begin the scan immediately after authorization and show partial results without waiting for the entire scan.

Store the active catalog and thumbnail cache on a local writable disk, outside indexed roots and outside cloud-synced database locations by default. Do not put the active WAL database on SMB: SQLite documents that WAL is not supported over network filesystems. Use its backup API for a consistent catalog backup. [R14, R15]

### 6.2 Root identity and reconnection

Store a stable root UUID, native path representation, platform, mount/volume identity when available, root mode, and observed filesystem capabilities. Drive letters and mount paths are location hints, not identity.

Reject duplicate or overlapping root selections in the first release. Show the existing owner root and suggest a folder filter instead. This avoids double indexing and conflicting mutation ownership. A later implementation may support overlaps only with explicit ownership rules and tests.

Distinguish online, offline, inaccessible, scanning, and needs-relink. Keep cached metadata and thumbnails when a root is unplugged. Disable original-dependent actions with a reason. Relinking requires matching volume identity where possible and validating sample relative paths/content; a reused drive letter alone is insufficient.

Removing a root from the catalog does not delete its files. Explain whether locally stored annotations will be retained/exported. Scan failures keep the last successful inventory and display its age.

Treat Apple Photos library packages as opaque, not normal writable folders. Do not reorganize inside application packages. A future Apple Photos integration must use an explicit export/API workflow.

### 6.3 Filters

Provide one shared, versioned filter model for scans, imports, duplicates, health checks, and storage views. Backup scope is separately saved and displayed; never silently inherit a UI-only filter into a backup.

Support folder basenames and root-relative path globs, file-extension sets, minimum/maximum logical size, date intervals in the shared model, hidden file inclusion, and hidden folder inclusion. The full-width Advanced accordion uses independent switches labelled "Include hidden files" and "Include hidden folders". The latter controls traversal into hidden folders; hidden files retain their separate rule. Exclude patterns, excluded folder names and File Extensions use removable tag inputs, added with Enter, Tab or blur; multiline paste adds multiple values. Commas remain literal so brace globs and folder names with commas survive. Include patterns is removed from this setup screen; saving/rescanning clears any legacy include-glob restriction, just as it clears removed scan dates. New folders prefill the File Extensions photo preset; matching ignores letter case and accepts displayed leading dots, storing canonical lowercase suffixes without dots. Existing saved extension choices, including an empty unrestricted list, are retained. Save and Preview include unfinished input. Scan filters omit From/Through date controls and clear legacy date restrictions on save/rescan; date filtering remains available when browsing the indexed library. Use slash-separated logical paths in saved patterns; never use string-prefix containment checks.

Rules: exclusions override inclusions. An excluded directory prunes traversal. A nonmatching parent is not pruned merely because a descendant could match an include pattern. Empty inclusion rules mean include all permitted candidates. Hidden detection uses dot names plus platform hidden flags when supported. Unsupported attribute criteria are reported, not treated as false or equal.

Advanced scan settings expose Minimum size and Maximum size as independent decimal amount inputs with KB/MB/GB dropdowns, defaulting to MB for an empty field. These are decimal byte units: 1 KB = 1,000 bytes, 1 MB = 1,000,000 bytes and 1 GB = 1,000,000,000 bytes. Changing a dropdown applies its new scale to the current amount. Empty means unrestricted; zero remains an explicit zero-byte threshold. Save and Preview use exact integer-byte strings in the existing shared schema, without floating-point rounding. Existing saved byte limits display exactly in a suitable unit and survive an untouched settings save. Reject negative, malformed, fractional-byte and above-u64 values visibly; block Save and Preview until corrected. The backend retains minimum/maximum ordering validation. Busy forms disable both amount and unit controls. This refinement applies to scan setup; browsing filters retain their existing byte inputs.

Example excludes: `**/.git/**`, `**/@eaDir/**`, `**/node_modules/**`, `**/.DS_Store`. App-managed cache, staging, quarantine, and backup directories are always excluded from indexing; explain these locked exclusions. Escape and validate patterns, limit pathological input, and provide a "Test filter on these paths" preview with a reason for every inclusion/exclusion.

Cloud placeholders must not be silently hydrated by a scan. Show unavailable-original status and ask before an operation downloads placeholder content. Never treat a placeholder as corrupt solely because it is not locally readable.

### 6.4 Settings — owner revision, 2026-10-28

Provide Settings in the sidebar before and after adding a library. The owner clarified that Library location means **photo folders being indexed**, not catalog/cache storage. Show each saved photo folder, availability, indexed count and mode. Change location uses the native picker and existing identity-checked relink/reconciliation workflow for the same moved folder; adding a different folder uses Add photo folder followed by explicit Save & Start scanning. Neither flow moves photos or silently maps unrelated files onto existing catalog records. Picker cancel retains the previous root, annotations and defaults. Existing roots retain their IDs and saved filters/mode.

Let users save the default browsing library (All libraries or a registered root), view, thumbnail density, grid fit, sort and initial navigation-collapse state. Defaults apply when opening Photos; ordinary browsing controls remain temporary overrides. New-folder defaults reuse the same Read only/Managed control and Advanced filters as folder setup, including extension tags, exclusion rules, hidden switches and exact size units. Existing roots keep their saved scan settings; changing defaults does not rescan them. Save defaults is explicit, with busy, success, failed-save, discard and failed-load/retry states. Editing view buttons must not submit the form. No telemetry or new original-file operation is introduced.

Owner inline-scan revision (2026-10-28): show a Scan settings group directly within Settings, between Library locations and Browsing defaults. A Library to configure selector chooses one registered root; seed its saved mode/filters rather than new-folder defaults. Reuse the existing folder-mode, Advanced, tags, exact byte units and preview controls. Save & Rescan explicitly updates that root and starts the existing read-only scan; it stays on Settings and reports pending/success/error without saving application defaults. Discard scan changes resets only that root's draft, including amount/unit/validity, pending tags and preview. Switching libraries discards unsaved scan edits and loads the other root's saved configuration. Disable another submission during startup or an active scan. Keep root scan and app-default forms separate, without nested forms or an internal group scrollbar. Gallery Scan settings links navigate to and focus this inline group for the requested root. Remove per-location Scan settings launch buttons; Change location and Add photo folder retain their existing workflows. Opening Settings or selecting a root never starts a scan. No catalog/schema/backend permission change is required.

The native backend owns a versioned, bounded preferences.json in its existing private app-data directory, separate from schema-2 catalogs. Generated typed IPC validates enum/filter values and the registered default-root ID. Persist only validated preferences through a flushed staged atomic replacement and parent-directory flush on the validated macOS implementation. Corrupt, oversized, linked or future-version settings are preserved and reported, never silently overwritten with defaults. Native durability/support gates remain explicit on other platforms. This revision does not relocate the catalog/cache or implement N2 catalog portability.

### 6.5 Remove indexed library and clear previews — owner revision, 2026-10-28

Settings offers Remove library per indexed folder and Clear preview cache for every library. Both require an inline backend-issued review and explicit confirmation. Removal shows the saved label/path, indexed-record count and preview count/bytes, and warns that its catalog favorites, ratings, tags and scan history are forgotten. Originals and sidecars remain untouched; re-adding does not restore forgotten notes. Cache-only clearing retains roots, catalog metadata and notes, and warns that offline previews require reconnecting originals to regenerate. Counts describe the reviewed observation; the fixed scope is that root or all indexed previews.

Use one bounded, opaque native review plan, expiring after five minutes and bound to catalog revision. Reject missing, stale, replayed or expired approval and active scans. Quiesce/cancel thumbnail work and exclude publication/reads during cleanup. Delete only cache-index-owned validated keys in bounded batches, relative to an already-open owned cache-directory handle. Refuse links, hardlinks, nonregular/invalid files and unexpected changes; retain unindexed files. Never enumerate or delete original-folder contents or recursively remove a path supplied by the renderer.

Retain the cache index as a durable retry list until deletions and cache-directory sync succeed. Partial failure can leave some regenerable previews absent; retain library records and require another review. Finish catalog cleanup atomically with foreign keys enforced, invalidating selections containing forgotten assets and promoting a remaining base root without changing surviving IDs. Last-root removal returns Photos to folder onboarding. Missing removed default-root IDs resolve to All libraries on native preference reads without silently rewriting the preference file or changing other defaults. Catalog identity/schema and original-mutation gates remain unchanged. See [ADR-0006](decisions/ADR-0006-library-removal-cache.md).

## 7. File discovery, metadata, and image pipeline

Use a streaming discovery pipeline: enumerate permitted entries -> record stat/identity -> identify format -> metadata and companion association -> schedule thumbnails -> optional integrity/hash work. Discoveries are committed in bounded batches. A bad file must not terminate the scan.

Treat filesystem notifications as invalidation hints. Debounce bursts and do a reconciliation scan after restart, root reconnection, missed notifications, or a user request. Scope disappearance checks to a successfully completed directory scan; do not mark an entire root missing after one error.

Store the observed extension and independently detected format. A misleading extension is a warning, not automatic corruption. Preserve original metadata values and their provenance before deriving normalized fields.

First vertical slice must support tested JPEG, PNG, and WebP fixtures. First-release format gates include selected HEIC/HEIF and common RAW fixtures relevant to user libraries, plus TIFF. Declare actual capabilities per format: discovery, metadata, thumbnail, full preview, full validation, and normalized-pixel comparison. Do not use a single supported/unsupported boolean.

A RAW embedded preview can serve browsing but does not prove the RAW sensor data is intact. An unimplemented RAW decoder is "Unsupported validation," not "Corrupt." LibRaw can extract both RAW data and previews, but output quality and format coverage must be validated. [R20]

Use isolated workers for image decoding and metadata processing, bounded by memory, pixel dimensions, time, output size, and number of concurrent jobs. A worker crash yields a typed file error and worker restart, not application termination. Isolation is not a claim of a complete security sandbox; implement OS resource controls and document residual risks.

Apply orientation correctly and exactly once. Define and test an ICC/color-management path for previews; do not silently discard profiles and claim color accuracy. Display-profile handling and HDR need explicit test coverage and capability labels. Thumbnail output is a documented display color space; original profiles remain unchanged.

Generate multiple useful thumbnail sizes, with a bounded on-disk least-recently-used cache. Key cache entries by file revision, processing version, requested size, orientation/color policy, and any preview edit state. Stale thumbnails cannot be reused as current after a content change. Prioritize visible rows and cancel obsolete off-screen requests. Serve small images via an ID-based scoped protocol, not base64 blobs of entire originals in JSON.

ExifTool arguments are backend-authored and allowlisted. Do not allow arbitrary tag assignments, expressions, user configuration, or shell fragments. Handle filenames beginning with hyphens or containing newlines safely, including any persistent argument protocol. Disable untrusted local ExifTool configuration; validate exact invocation behavior during M0. [R18]

## 8. Timeline, list, search, and annotations

### 8.1 Date semantics

Default to newest-to-oldest by effective capture wall-clock date/time. The provenance chain is: user catalog override -> embedded original capture time or format equivalent -> trustworthy embedded media-creation time -> filesystem modified time -> filesystem birth time -> Undated. Every fallback is visible in details. User may instead sort by imported time, modified time, name, or size.

Preserve raw date strings, fractional precision, UTC offset when present, source, confidence, and whether the timezone is unknown. A local EXIF timestamp without an offset is not silently labeled UTC. Display capture-local order by default; an explicit UTC sort must label entries that have no known offset. Changing the current machine timezone must not rewrite capture metadata or change template output implicitly.

Store filesystem creation/birth and modified times separately from capture time. Do not use Unix ctime as a creation-time substitute. Creation timestamps may not be available on all filesystems, as the Rust metadata API documents. Missing values stay missing. [R22, R23]

### 8.2 Library views

Provide a chronological justified thumbnail gallery, a compact uniform-grid option, and a true text-only list. Preserve the same selection, filter, scroll anchor, and sort when switching views. The list must not request thumbnails.

Owner Text list revision (2026-10-28): default columns are Name, Modified Date, Size, Dimensions, Folder and Status. Modified Date reads the indexed filesystem modification timestamp, with Unknown when absent; do not relabel effective capture time as modification time. Remove repeated date-source subtitles from default rows. A Columns menu can hide optional fields or add Capture Date (the effective date, including documented fallbacks), Date Source, Added Date, File Format, Camera, Rating and Tags. Name remains visible; Reset columns restores the defaults. Column choices last for the current library-screen session, survive view/filter changes and do not alter query, selection or saved app defaults. Keep selected columns reachable through horizontal table scrolling at narrow widths.

Offer the existing native sort orders directly in Text list and from supported column headers: filename A–Z, size largest first, and capture/modified/added dates newest first. These sort the matching catalog, not just loaded rows; other headings remain labels. Show active order accessibly and disable sorting while loading/batch work blocks it. Sorting retains existing query-change selection-reset behavior. Keep raw date/offset details in the inspector and full indexed date timestamps in cell tooltips. Browsing remains read-only; no new format/similarity or original-mutation capability is implied.

The current Timeline groups by day and keeps month/year context in the navigation rail. Provide a right-hand year/month navigator backed by date-bucket counts and indexed queries. Clicking a period seeks to a date cursor; do not load all preceding records to reach it. Optional month/year grouping and sticky headings remain follow-on presentation work; do not imply controls that are not implemented.

Owner chronological-canvas revision (2026-10-27; supersedes the earlier gapless/no-checkbox gallery revision): Timeline uses justified rows that preserve the complete image aspect ratio, 4 px gutters and day headings. Compact/Comfortable/Large target heights are 150/220/300 px; the final incomplete row stays left-aligned without enlarging a lone photo. Sparse consecutive dates can share a row with separate headings. Uniform grid offers fit/crop; Text list renders metadata without image nodes or nonempty thumbnail requests. Keep bounded virtualization: scroll loads native keyset chunks of at most 240 assets, retaining at most three chunks (720 summaries) in the renderer. Evict the far end and preserve the visible stable-ID/row offset; scrolling back retrieves earlier chunks. No full-library array or pagination footer. See the 2026-10-28 revision below.

Keep filename search fixed above the collection, with collapsible navigation, library/date/size/extension/tag/favorite filters and removable active-filter chips. Timeline and Recently added sorting use actual filtered year/month buckets and indexed seeks, without loading preceding pages. The rail tracks the first visible row, excludes overscan and keeps Undated distinct. Hide chronology for other sorts. Day selection explicitly says how many **shown** photos it covers; it never silently selects unloaded date/group members. Hover/focus reveals individual checkboxes; selected checks stay visible and above all preview layers, with pointer access in Timeline and Uniform grid. Shift range selection follows the loaded ordered window. Select all results explicitly freezes matching stable IDs in a backend selection token. Batch favorites and additive tags affect the catalog only; tag merges are atomic and retain existing tags (maximum 64 per asset). Compare shows 2–4 explicitly selected photos with no exact-duplicate claim; token selections cannot enter Compare.

Opening a photo uses an immersive modal cached-preview viewer with fit/zoom/pan, filtered previous/next navigation beyond the loaded page, optional filmstrip and optional information panel. Close returns focus and the exact gallery browsing offset. Preview limits remain 256/512 px; zoom is relative to fit and does not imply full-resolution original decoding. Explicitly distinguish missing/offline originals, unsupported previews, pending work and failures, with retry where applicable. Browsing never rewrites originals or sidecars. Album/export/edit actions remain disabled with reasons until their separate workflows exist.

Owner filter-panel revision (2026-10-28): keep the title, fixed filename search, a right-hand Filter button and the header Jobs icon in the gallery header. Remove Add folder from this header; indexed folder management remains in Settings. Move library, sort, date/byte/extension/tag/favorite/missing filters, folder/tag facets, active-filter chips, view/density/fit controls, result count, root availability/relink/settings and Select all results into a right-hand modal side panel. Its body scrolls independently with fixed Close and Apply/Reset actions. Draft edits apply together; Close/Escape discards them. Select all uses the applied query and is unavailable until query drafts are applied. Show the batch toolbar only with a selection. Preserve explicit offline/unsupported/failed tile states and the year/month rail. Use scrolling instead of Previous/Next buttons or a page footer. Transient loading/retry states must retain already loaded rows. Settings defaults remain distinct from temporary gallery choices. [ADR-0005](decisions/ADR-0005-filter-panel-feed.md) records native bidirectional seeking and bounds; original-file permissions and milestone gates do not change.

The owner confirmed on 2026-10-28: complete the gallery/viewer and keep similarity grouping unavailable. M1 has no similarity-group engine or persisted similarity data. Keep Similar groups visibly unavailable until a separately authorised M3 implementation supplies evidence and membership. Visual comparison, name companions and exact duplicates are distinct concepts. Future grouping must expose membership/count/expansion and explicit action scope, without automatically applying actions to hidden members.

Show multi-file/burst stacks with a count and an expand control. Show offline, backup, integrity, rating, and favorite states without permanently obscuring the image. Keyboard selection, range selection, select-all-in-query, and batch actions operate on stable IDs or a backend selection token, not just loaded rows.

Owner Folders explorer revision (2026-10-28): Folders is a distinct read-only explorer, starting with the registered library folders. Open a library or child folder to list only its immediate indexed children, with folders before files, Back/Up and root-to-folder breadcrumbs. Include empty indexed directories and non-media files admitted by the saved scan settings. Icons mode shows named folder/file icons and existing cached photo thumbnails; Details is text-only and starts no nonempty preview batches. Filename filtering affects direct files while retaining folder navigation. Show catalog counts, availability and partial/retained states; offline is not empty. Failed reads show Retry rather than a successful empty result.

Use bounded virtualized 240-entry native pages with explicit Previous/Next items controls for Folders; Photos keeps its existing scroll feed. Native directory identifiers preserve path bytes and remain scoped to a validated root, never renderer execution paths. Root/directory/search-bound keyset cursors access both directions without OFFSET or whole-tree renderer arrays. Photo viewer neighbors use exact native-directory scope, not the recursive logical folder filter. Closing returns focus and browsing offset. Non-media files open read-only catalog details, not an external application or arbitrary decoder. Folder opening does not scan, rename, move, delete or rewrite originals; thumbnails use the existing read-only pipeline. No schema migration, original-file permission or new format support follows. See [ADR-0007](decisions/ADR-0007-folder-explorer.md).

### 8.3 Search and catalog metadata

Provide structured filters for date, root, folder, filename, extension/detected format, size, dimensions, camera/lens, rating, favorite, tags, GPS presence, integrity status, and backup status. Text search uses a documented grammar or a simple query plus filter chips; SQL is never accepted from the frontend.

Include favorites, 0-5 ratings, reject flags, captions, hierarchical tags, virtual albums, and saved searches in the first release. Albums do not move files. Save annotations in the catalog by default. Provide later/explicit XMP export or synchronization with conflict review; do not rewrite originals automatically while users browse.

Unknown XMP namespaces and existing editing instructions must be preserved on any supported metadata round trip. Read-only roots can have catalog annotations, but their sidecars cannot be written. Associated XMP, ACR, JSON, and other recognized companion files are preserved even when the app does not interpret all of their fields. Adobe documents XMP interoperability and additional ACR companions, so a sidecar policy must not assume that `.xmp` is the only relevant file. [R29]

## 9. Organization templates and dry-run preview

Current M2 implementation (owner continuation, 2026-10-30): [organization template previews](M2_ORGANIZATION_PREVIEWS.md) implement the first catalog-only slice for 1–240 explicit photo IDs. Editable session presets/custom DSL produce relative names and typed issues with deterministic snapshot fingerprints. Destination-aware persistent plans, saved templates, trees/export, confirmed companions, approvals and execution remain pending. This slice does not relax sections 9–10 or preservation/native acceptance gates.

### 9.1 Template model

A saved template has UUID, name, description, schema version, template version, path pattern, enabled token set, missing-value policy, date policy, filename case policy, collision policy, sequence policy, and sample inputs/outputs. Ship editable copies of presets; editing a preset does not silently alter an existing plan.

Use a deliberately limited token DSL, not JavaScript, a shell, or a general code evaluator. Proposed syntax:

```text
{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}
{capture:%Y}/{capture:%m}/{capture:%Y%m%d_%H%M%S}_{sequence:04}.{ext}
{event}/{capture:%Y-%m-%d}/{original_stem}.{ext}
{camera_make}/{camera_model}/{capture:%Y}/{original_stem}.{ext}
```

Implement an allowlist of date directives. Reject newline/tab-producing directives and unrecognized tokens. Optional named tokens: event, original_stem, original_folder, ext, camera_make, camera_model, lens, sequence, and an explicitly computed short digest. Geographic tokens use already-approved cached metadata only; building a preview must not secretly send GPS to a network geocoder.

Missing capture date defaults to `Undated/{original_stem}.{ext}` unless the user explicitly chooses a displayed fallback. Missing event/camera values have explicit fallback text or block the plan; never manufacture values.

Sequence assignment is deterministic within a persisted plan. Sort by a documented stable key and record allocations. Existing no-op paths keep their names. Preview and execution use the exact same resolved targets, including collision suffixes. Reapplying a template to unchanged files is idempotent.

### 9.2 Portable destination validation

Render tokens into individual path components, sanitize each component, then apply destination-filesystem validation. Reject absolute paths, parent traversal, NUL/control characters, path separators embedded in values, reserved device names, empty components, illegal suffixes, and unsupported lengths. Detect case-only collisions and Unicode normalization collisions using destination capabilities and native checks, not universal lowercasing of all filesystem paths. Windows naming rules require explicit handling. [R24]

Default collision policy: add a stable suffix or require resolution. Never overwrite. The preview labels every collision and its exact final resolution. Store the original names and target names in the journal.

### 9.3 Preview and execution flow

The screen shows a before tree, an after tree, and a searchable per-file diff table. Display counts for unchanged, rename, same-volume move, cross-volume copy, blocked, conflict, companions, and unknown non-photo files. Show destination-space needs and whether a later source removal is involved. Provide JSON and CSV plan export; CSV export must mitigate spreadsheet formula injection.

The default organizer creates a verified organized copy of selected media and confirmed companions in a separate nonoverlapping destination, retaining the source. In-place rename/move is explicitly selected and requires the preservation recovery gate. It never blindly renames a parent folder containing excluded/unrelated content. A literal whole-folder rename requires a separate plan that inventories all affected descendants, including non-photo files. Unknown descendants must be visible, not discarded.

Only application-created empty directories may be automatically tidied after successful operations. Existing empty directories need separate opt-in approval. On referenced roots, show an explanation and require a write-grant change before an execution plan is approvable.

## 10. Safe operation executor and recovery

Implement this subsystem before enabling any original-file mutation. The planner is pure over a recorded snapshot of inputs; validation and execution are separate services.

Plan-level states: draft -> ready -> approved -> running -> completed / completed_with_errors / canceled / needs_recovery / invalidated. Blocking conflicts prevent approval. A changed precondition invalidates the affected operation and pauses dependent work; never silently regenerate the plan.

Each operation stores source file/revision, root identities, destination, expected content evidence, required companions, expected destination absence, operation kind, dependency order, retry state, timestamps, and an audit reason. Approval includes plan digest, scope summary, and mandatory exact-version recovery evidence for source mutation under docs/PRESERVATION_CONTRACT.md; read-only and source-retaining copy operations use their applicable lesser preconditions.

For same-volume renames, journal intent durably, use native no-replace primitives, then durably record outcome and update catalog paths. Stage case-only renames and cyclic renames through reserved collision-proof names under the same authorized volume. The temporary stage is journaled before use. Do not assume Rust's ordinary rename function is a no-overwrite operation.

For cross-volume transfers: create an exclusive temporary destination -> stream copy and hash -> flush where supported -> reread/verify destination -> publish without replacement -> record completion -> only then propose removal of the source. Import defaults to retaining the source. Unsupported metadata preservation is reported; do not claim ACLs, alternate streams, resource forks, hard-link structure, or birth times survived unless tested.

A cross-volume move cannot be an atomic rename. Leave the source intact on transfer failure. A source-removal phase is separately journaled and is allowed only after verified destination publication and the approved policy. Prefer source quarantine over permanent deletion.

Serialize overlapping mutating operations by affected roots/paths and lock planned keepers. Backup and organization of the same roots must not race. Scan events caused by app operations are reconciled against the journal rather than creating duplicate asset records.

Recovery inspects both paths, file identities, staged objects, and content evidence before deciding whether a step completed. Never replay a potentially completed destructive action blindly. Offer resume, safe rollback, or manual resolution. Undo is a new reverse plan: if the original path is now occupied or files changed, show the conflict and do not overwrite the newer data.

## 11. Duplicate detection and keeper selection

### 11.1 Separate match modes

Exact, property-rule and explicitly supported normalized-pixel comparisons are first-release scope. Perceptual visual-similarity review is post-v1 by default; its contract below applies when that later mode is explicitly authorized. Do not show a functioning similarity mode before it is implemented and validated.

| Mode | Meaning | Allowed cleanup behavior |
| --- | --- | --- |
| Exact content | Same complete bytes, confirmed by streaming comparison | Reviewed quarantine after fresh verification and keeper checks |
| Same rendered pixels | Same dimensions and canonical decoded pixels under a documented pipeline | Manual review; preserve metadata and higher-fidelity variants |
| Visually similar | Perceptual similarity above a threshold | Suggestions/stacks only; never automatic deletion |
| File-property rule | Chosen filename, extension, size, date, or attribute predicates match | Candidate report; not proof of duplicate content |

Expose the mode before advanced checkboxes. Selecting "ignore descriptive metadata" switches from whole-file exactness to normalized image comparison; it cannot make two different whole-file hashes identical.

For normalized pixels, decode the supported original, honor orientation and color interpretation, and compare a canonical full-resolution representation. Include width, height, channel layout, precision, alpha semantics, pipeline version, and decoder version in the signature. Do not quantize high-bit-depth files and call them exactly equivalent; unsupported precision remains unsupported. ICC and orientation affect appearance and are not discarded as irrelevant metadata. RAW preview equality does not establish RAW equality. Animated/multipage images and HDR auxiliary images are excluded from normalized equality until the versioned policy compares all required frames/pages/components; a matching first frame is not whole-asset equality.

Approximate search uses bounded candidate generation or an index, not unconditional all-pairs comparison across the library. Keep pairwise scores and evidence. Similarity is not transitive: A similar to B and B similar to C does not prove A similar to C. Group UI must not imply all group members are identical. Thresholds require validation on a labeled test set; no invented universal perfect threshold.

### 11.2 Advanced criteria requested by the user

Support base filename, extension, logical size, file content, filesystem last-modified time, filesystem creation/birth time, and supported filesystem attributes. Content options: BLAKE3, SHA-256, SHA-1, MD5, and direct byte comparison. BLAKE3 is the normal hashing path; legacy choices remain available for compatibility but cannot bypass byte verification. BLAKE3's primary implementation provides optimized native hashing; do not advertise a fixed cross-machine speed advantage without measuring. [R16]

Choose all selected criteria (AND) by default. An explicit any-rule (OR) mode is available only as an advanced candidate search with clear labeling. Exact-content preset does not require matching names/extensions, so renamed copies can still be found.

Name comparison can ignore case, selected literal characters, whitespace, or a defined Unicode comparison normalization. Show the normalized comparison keys. These options affect comparisons only, not original filenames. Empty normalized names never match everything by accident.

Date comparison has an explicit resolution/tolerance and field name. Attribute comparisons use portable booleans where meaningful and platform-specific fields otherwise. Missing/unsupported fields are unknown, not equivalent values; show the unknown count and never silently relax a rule.

ID3 is predominantly associated with audio/media tagging, not an ordinary JPEG/HEIC photo setting. Present it as not applicable to the first-release photo modes, not as a fake functioning toggle. A later audio/container plugin can implement it with its own testable semantics. [R28]

Hard-link default: recognize identical filesystem object IDs on the same volume and collapse them for content reading/storage accounting while still listing their paths. Offer "exclude hard-linked objects from this search" and an expert "show linked paths" view. Linked directory entries are not independent redundant copies. Do not auto-convert files to hard links.

### 11.3 Exact scan algorithm

Enumerate eligible entries -> identify/handle hard-link aliases -> group by size -> optionally compare small beginning/end samples -> compute full selected digest only for remaining candidates -> group by digest/size -> offer results. Sampling only rejects candidates; it never proves equality. Skip unreadable/unstable files with explicit results.

Cache fingerprints with algorithm, algorithm version, observed revision, identity, length, timestamps, and calculation time. Filesystem metadata alone cannot detect all content changes; an integrity refresh rereads data. Before quarantine, verify each selected file directly against an accessible keeper from open handles and recheck for changes. This applies even if cached hashes matched.

### 11.4 Review and keep policy

Show groups with side-by-side previews and a text table containing root/path, size, dimensions, actual format, capture-date provenance, birth/modified times, hash method, match evidence, metadata differences, bundle membership, availability, and backup state. A rule explanation accompanies every suggested keeper/removal.

Support ordered preferred-keeper folders, folders that must never be selected for removal, keep oldest/newest filesystem birth time, keep oldest/newest modified time, keep shortest path, and manual overrides. For non-exact groups also show dimensions/format/metadata as review aids, never assume larger bytes always means better quality. "Original" is a policy label, not a claim that timestamps identify the camera-original file.

Unknown dates or unresolved ties require review unless an explicit deterministic tiebreaker was selected. Protected folders override date rules. Users can keep multiple copies; they cannot approve zero surviving accessible exact copies.

An inaccessible protected keeper does not justify removing the only online copy. Selecting all removable rows is still validated server-side. An exact duplicate of one component in a bundle does not authorize deleting unique companions. Bundle-level automatic suggestions require complete component-level evidence; otherwise block and explain.

Quarantine rather than permanently delete. If same-volume quarantine cannot be made safely, skip and explain; do not fall back to a direct unlink. Merge catalog album/tag/favorite information only via a previewed metadata plan, with conflict handling. Preserve unknown metadata and sidecars instead of automatically stripping differences.

## 12. Imports

Choose one or more source folders, whether to recurse, filters, destination writable root, and a saved organization template. Source selection is a separate authorization from destination permission.

Show existing exact duplicates, new assets, unsupported files, missing companion risks, name collisions, expected bytes, and template output before import. Compare against selected catalog scope. Duplicate detection must not depend on matching filenames. Offer "skip verified exact duplicates" as a default; when skipped, do not delete source files.

Copy by default. Preserve original file bytes and confirmed companion files. Do not require a full source scan before the first safe preview results, but final approval binds the complete selected manifest. Prioritize browsing of already copied assets.

Track an import session and per-file outcomes. Copy to exclusive temporary names, verify, and publish. Retry is idempotent: already verified targets are reused by recorded evidence, not copied repeatedly. Unexpected destination content causes a conflict. A disconnected source pauses the session.

Imported filenames and capture-time formatting use the selected saved date policy, not the runtime computer timezone. No source-folder cleanup or automatic card erase in first release.

## 13. Backup & Sync and restore

Version 1.1 adds a required readable Safe copy / one-way sync workflow with the base library selected automatically. See docs/BACKUP_SYNC_ADDENDUM.md for compare, retained-version replacement, contextual errors, throttling and M4A-M4D gates. Keep this separate from restic snapshots. Both obey docs/PRESERVATION_CONTRACT.md.

### 13.1 Engines and transport

Use restic for encrypted, versioned backup snapshots. Use its documented rclone backend for the requested SMB, SFTP, FTP, and FTPS transports. A local/mounted-folder destination can use the local repository backend. Prefer SFTP or a secured SMB configuration; FTPS is preferable to plain FTP. Do not implement transport protocols or a new encrypted repository format from scratch. The restic-to-rclone integration and rclone transport support are documented upstream. [R10, R11, R12, R13]

Explain the format choice: a restic repository is not a browsable mirror of photo filenames. It requires restore through this application or restic and a repository secret. Provide a recovery-key export/reminder flow; losing the secret can make backup data unrecoverable. Readable Safe copy / one-way sync is now a required separate feature; label its actual retained-version policy without implying that it uses the encrypted snapshot repository or offers independent disaster protection on the same disk. [R10]

Do not enable plain FTP silently. Require an explicit insecure-transport warning: restic encrypts backup payloads, but plain FTP credentials and connection metadata are still exposed to network interception. Validate FTPS certificates. Enforce SFTP known-host verification; a changed host key fails closed and requires an explicit trust-update flow. rclone's configurable host checking must not be left at a permissive default. [R11, R13]

Bundle signed, verified helper binaries. Invoke fixed absolute executables with backend-authored arguments, never shell concatenation. Configure rclone/restic through tightly scoped process environment or restricted short-lived configuration, never command-line passwords or JavaScript state. Do not treat reversible password obscuring as secure secret storage. Redact secrets and sensitive paths from exportable logs.

### 13.2 Backup job semantics

A profile stores destination type and nonsecret settings, credential references, repository identity, included roots, excludes, schedule policy, bandwidth/concurrency limits, and verification/retention policy. Destination validation checks read/list/write behavior in an isolated test area and never deletes arbitrary existing contents.

A backup session captures a source manifest and a consistent catalog snapshot. Suspend application-managed mutations to affected roots while files are read. External changes must be detected where possible and surfaced; a file-level backup is not advertised as an atomic whole-filesystem snapshot. Do not back up only the changing live SQLite main file. [R15]

Manifest entries record asset/file IDs, root ID plus relative path, actual selected file revision, companion links, bytes, and digest where verified. The backup record maps these entries to the concrete snapshot ID. Root paths in restore metadata are portable mappings, not assumed drive letters on another OS.

Classify results as completed, completed_with_errors, canceled, failed, or partial_offline_sources. A restic exit status indicating unreadable files or a partial snapshot is not success. Save structured helper output and errors; a progress bar reaching 100% is not a success criterion.

Retry from established snapshot/repository state. Do not claim byte-offset resume on every transport; successful chunks/files may be reusable by the engine, and in-progress work may restart. Honor repository locking; never automatically force-unlock, repair, forget, or prune because a timeout occurred.

Keep "included in snapshot," "repository structure checked," "data read verified," and "restored and compared" as separate evidence levels with timestamps and scope. A full library is not marked verified because one sample restore passed. The basic restic check does not read all stored data; full verification uses its documented read-data option or an explicit scoped restore comparison. [R17]

### 13.3 Retention and scheduling

Deleting or quarantining a source photo must not automatically erase older snapshots. First release defaults to retaining snapshots until an explicit retention policy is reviewed. Retention preview shows snapshots kept/removed and warns about unique historical file versions. Pruning is a separate serialized, cancellable-with-engine-semantics maintenance job; follow restic recovery rules, not invented rollback.

Display backup age and destination reachability. Distinguish an offline source root from an intentionally removed root. Do not silently shrink a profile's scope when a drive is disconnected.

Initially, schedules run while the application/tray process is running, with catch-up after sleep or restart and clear UI disclosure. Jobs while fully quit require separately implemented and tested platform background agents; do not imply they exist. Store schedule timezone and daylight-saving policy. Optional application reminders must not upload photo metadata.

### 13.4 Restore

A restore browser selects destination profile, snapshot, roots/folders/photos, and target directory. Default to a new folder, not overwriting an active library. Show expected bytes, path sanitization, platform metadata limitations, and collision handling before restore.

Reject unsafe or escaping paths from repository manifests. After restoration, compare bytes/digests against the backed-up manifest for the selected scope and show results. Restoring the catalog is a separate action with a pre-restore catalog backup and migration checks. Credentials are not restored from a plaintext settings export.

Release gates require an end-to-end backup -> remove temporary source -> restore -> byte comparison on every advertised transport. A connection test alone does not establish support.

## 14. Integrity checks and storage analysis

### 14.1 Health modes

Quick check: format signature, header/container sanity, metadata warnings, truncation indicators where supported. Deep check: decode the relevant full image data within safety limits; for supported multi-page/animated images state whether all pages/frames were checked. RAW deep checks must inspect RAW data, not only an embedded JPEG preview.

Checksum baseline check: compare newly read bytes with a previously recorded verified digest. A changed digest means "changed since baseline," not necessarily corruption; combine with metadata/content history to explain it. A successful decode alone cannot prove the photograph has never been altered.

Use distinct statuses: not_checked, quick_pass, deep_pass, unsupported, inaccessible, offline, changed_during_read, decoder_warning, suspected_corrupt, and confirmed_decode_failure. Record decoder/version, depth, time, and error details. Do not automatically delete files with errors. A thumbnail may render while deeper image data is damaged; report exact coverage.

### 14.2 Size views

Provide a descending largest-files table with root/path, format, dimensions, logical bytes, allocated bytes when available, and health/backup states. Provide a zoomable treemap and an accessible sortable folder table. Area represents the explicitly selected size metric. A SpaceSniffer-style treemap is an interaction reference, not a code-copy request. [R09]

Offer "indexed media and companions" and "all eligible files in selected roots" as separate scopes. Scanning all files for storage size does not turn the application into an audio organizer. Show excluded/unreadable/unknown space and the last scan time.

Distinguish total logical bytes, unique known filesystem-object bytes, allocated bytes where reported, potential duplicate logical bytes, and estimated reclaimable physical bytes. Hard links must not be counted as independently stored full copies. Filesystem compression, sparse files, clones, snapshots, or links outside the scan can make exact reclaim estimates unavailable; show unknown/estimate rather than a false exact number.

Moving to same-volume quarantine does not free the data blocks. Do not subtract it from used disk space. Only label a proposed saving as "potential after permanent removal," with appropriate filesystem caveats. A storage dashboard summarizes each actual volume; do not display a fictional aggregate 100 GB quota from the visual reference.

## 15. Map browser

Use the supplied map image as a layout reference: dark basemap option, thumbnail markers, numerical clusters, zoom controls, and unobtrusive settings. Keep the same application shell, root filter, date filter, and selection semantics as the library.

Map only valid known GPS coordinates. Preserve coordinate source and precision. Values at latitude/longitude zero are valid when explicitly present; missing values are not converted to zero. Missing-location photos appear in a separate filter/count, not as a pile at (0,0).

Use a spatial index and viewport/zoom-aware queries. Bound payloads and render clusters instead of thousands of DOM thumbnails. Smaller datasets can use MapLibre's built-in clustering; larger datasets need backend spatial aggregation or another measured bounded strategy. Handle identical coordinates, world wrapping, and antimeridian-spanning viewports. [R25]

Click a cluster to zoom or open a filtered filmstrip; click a marker to preview the photo. Provide optional selection by visible area, a date slider, and "Show these photos in library." Preserve filters when navigating back.

Photo metadata and original image files stay local. Explain that online tile requests disclose the viewed map area/IP to the tile service. Do not send filenames, thumbnails, or coordinates to an external geocoder by default. Reverse geocoding is an explicit opt-in integration with local caching and attribution.

Do not ship a production dependence on demo map tiles. Provider configuration, attribution, network behavior, cache limits, and offline rights must be explicit. The OSM standard raster tile policy prohibits bulk offline prefetch, so an offline-map download feature requires appropriately licensed/self-hosted data. In offline mode without such data, show cached/permitted coverage or a clear basemap-unavailable state; the photo list still works. [R26]

Manual geotagging is optional after the initial map view. Store a catalog override first, with original coordinates preserved and a separately previewed metadata-export action.

## 16. UX structure and reference adaptation

Treat docs/reference/timeline-reference.png and map-reference.png as user-supplied inspiration, not assets for product redistribution. Do not reproduce branding, identify people in photos, or copy their sample image collection into the application.

Main navigation: Photos, Folders, Map, Albums/Tags, and Utilities. Utilities contains Organize, Duplicates, Imports, Backup & Sync, and Health & Storage. Show an always-available Jobs/Activity panel and a distinct Recovery/Quarantine view. Avoid a sidebar with fifteen equally prominent destinations.

Use a dark neutral default inspired by the references, with optional light/system theme. Keep a persistent top search/filter bar, collapsible left navigation, the content area, and an optional inspector. The desktop side menu stays anchored to the window while page content scrolls independently, including long onboarding and scan-settings pages. If navigation exceeds the window height, it has its own scroll area. Timeline gets a right date rail; Map gets map controls and a bottom or side filmstrip. Use the full available content width rather than recreating the large empty areas in the example.

Organization preview is not a toast: it is a dedicated before/after review workspace. Duplicate review is not a gallery with a delete-all button: it is a compare workspace with keeper decisions and evidence. Backup state is visible per asset and per job.

List/table columns are configurable, sortable, resizable, and keyboard-accessible. Selection survives pagination and mode switching. All destructive confirmation text includes the actual scope and keeper summary. Warning severity is conveyed with words/icons as well as color. Support high-DPI displays, platform shortcuts, visible focus, reduced motion, and screen-reader labels.

Provide empty, scanning, partially available, offline, permission-denied, unsupported-format, no-results, interrupted-job, and recovery-needed states. Do not present placeholder/mock success states in production builds.

In M1 Jobs, show a progress bar for each scan alongside its native state and found/indexed/skipped/issue counts. While streaming discovery is running or cancellation is pending, use an indeterminate activity bar: the total number of files is not yet known, and indexed/found is not an overall completion percentage. Only native `completed` fills the bar. Partial, canceled, interrupted and failed scans retain a neutral stopped track with their explicit outcome; never animate stopped work or imply successful completion. Give the progress bar an accessible library label and textual description, respect reduced motion, and fit the minimum desktop window. Continue using the existing status refresh and cancellation behavior.

Owner navigation revision (2026-10-28; supersedes the earlier Jobs footer/page): Settings stays in the bottom sidebar footer above the running version. Jobs is an icon in the current screen header, available before and after folder registration; it is not a navigation destination. Its active count includes running and cancellation-requested jobs. Opening a nonmodal dropdown shows every active scan with live native counts, indeterminate progress, cancellation and issue details. Retain terminal outcomes under a collapsed Recent jobs section. The dropdown preserves the current screen, query, selection and browsing position. Escape/Close returns focus to the opener; outside pointer/focus dismisses without stealing focus. Connecting, empty, stopping and failed requests remain explicit. Keep the dropdown contained at the minimum desktop window and keep originals read-only. Primary library navigation/page scrolling and Settings/version footer anchoring remain unchanged.

The sidebar footer shows the actual running app version instead of the tagline/local-design copy. The initial library screen uses the heading "Start by selecting your Photo library folder." and one "Choose a folder" action. Keep the logo, Library navigation and Your library toolbar. Omit promotional supporting copy, milestone labels, native/database diagnostics, technical status panels and footer notices from this empty state. Keep actionable connection/picker errors and disable selection until the native connection is ready. During M0, a selected folder is explicitly "Not scanned" and scanning remains unavailable; simplifying the screen does not authorise indexing or file operations.

After a folder is selected, remove the welcome heading, folder icon and Choose a folder button; show the selected form alone and move keyboard focus into Library name. Cancel restores the chooser. The form expands naturally with no internal scrollbars, including when Advanced is open; the main page owns any needed scrolling. Advanced is a native full-width accordion with a right chevron and a full-row pointer/keyboard toggle. After selection, show "Save & Start scanning" at the lower right of the selected-folder card, with no native identity/debug message under the path. In M0 the action is disabled with the visible explanation "Saving and scanning aren’t available yet." linked as its accessible description. Selection remains session-only. Persistence and actual read-only indexing require the separately approved M1 implementation; the button must not report success before those functions exist.

M1 was authorised on 2026-10-27. Its real Save & Start scanning handler replaces the M0 disabled state after a native selection. It persists a Managed root by default (Read only is an explicit switch), starts a background scan, exposes issues/cancellation and opens persistent library browsing. Root filters and catalog annotations never write originals or sidecars. See docs/M1_VALIDATION.md for current platform and format limits.


## 17. Data model and IPC contract

### 17.1 Logical database entities

The following is a design contract, not a requirement to freeze every column before M0 experiments:

| Entity | Required information |
| --- | --- |
| roots | UUID, display name, native path, volume identity/capabilities, read/write mode, online state, filter profile, scan generation |
| directories | Root-relative native path, parent, direct/recursive counts and size observations, observation state/time |
| files | Stable app UUID, root/directory, native relative path, display path, OS object identity if known, stat fields, detected format, revision, presence state |
| assets | UUID, primary file, effective capture date/source/offset/precision, dimensions, camera fields, GPS and provenance, rating/favorite/reject/caption |
| asset_files | Asset/file relationship, role, association source/confidence, explicit confirmation |
| file_fingerprints | File revision, algorithm/version, digest, completeness, observation time |
| thumbnails | File revision, processing policy version, size, cache key and access time |
| tags / asset_tags | Hierarchical tags and assignments |
| albums / album_assets | Virtual album membership |
| saved_queries | Versioned typed filter/query definition |
| templates | Versioned token DSL and policies |
| jobs / job_items | Job type/state, per-item progress/errors, checkpoint and cancellation state |
| operation_plans / operations | Immutable plan/version/digest, approval, dependencies, before/after paths, preconditions, journal state |
| duplicate_runs / duplicate_groups / duplicate_members | Scope, match type, evidence, scores, keep decisions, scan revision |
| integrity_results | File revision, check coverage, decoder/version, outcome/error and time |
| backup_profiles / backup_sessions | Scope, transport config without secrets, credential reference, snapshot ID, result |
| backup_file_versions | Snapshot-to-file-revision mapping, digest evidence, verification coverage/time |
| quarantine_entries | Original/staged path, identity, related plan, restore state |
| annotations / metadata_conflicts | Catalog overrides, metadata provenance, conflict versions |
| audit_events | Event type, affected IDs, actor/source, timestamp, redacted explanation |

Use foreign keys, explicit constraints, migrations, and indexes. Dates preserve timezone uncertainty. Byte sizes use 64-bit integers. IPC represents integers beyond JavaScript's safe range as decimal strings. Preserve native filesystem path encoding; never round-trip a lossy display string into a filesystem action.

Use a root-aware comparison key only for lookup/collision behavior; preserve original path bytes. Do not deduplicate database rows globally by content hash: two independent copies remain two file locations. Use known object identity to account for hard links without pretending duplicate content has only one path.

Index effective capture date plus stable ID for keyset pagination; root/directory; size; tag joins; and backup/integrity status. Use SQLite FTS5 and R*Tree where available and verified in the bundled build, or document a tested fallback. Do not ship an unbounded OFFSET scan for deep chronological navigation.

### 17.2 Suggested command families

```text
catalog.open / catalog.status / catalog.export
roots.choose / roots.add / roots.update_permissions / roots.relink / roots.remove_index
scan.start / scan.status
assets.query / assets.get / assets.date_buckets / assets.update_annotations
thumbnails.request
filters.preview
organization.templates.list / organization.templates.save
organization.preview
imports.preview
operations.get / operations.approve / operations.execute / operations.undo_preview
recovery.list / recovery.resolve / quarantine.restore_preview
duplicates.scan / duplicates.groups / duplicates.keep_preview
integrity.scan / integrity.results
storage.summary / storage.tree / storage.largest
backup.profiles.list / backup.profiles.save / backup.connection_test
backup.preview / backup.run / backup.snapshots / backup.verify / backup.retention_preview
restore.preview / restore.run
map.query / map.photos_in_bounds
jobs.list / jobs.pause / jobs.resume / jobs.cancel
```

These are logical names; map them to valid typed Tauri commands. Version request/response contracts and generate frontend types from one authoritative schema. Return typed errors with code, safe message, affected IDs, retryability, and details. Suggested codes: ROOT_OFFLINE, ACCESS_DENIED, PATH_CONFLICT, SOURCE_CHANGED, DESTINATION_CHANGED, UNSUPPORTED_FORMAT, INSUFFICIENT_SPACE, HOST_KEY_CHANGED, VERIFICATION_FAILED, and PLAN_INVALIDATED.

Mutating command input uses backend IDs, a plan revision, and its digest; raw renderer paths are not accepted as authorization. Approval and execution are separate. Any command capable of writing must independently enforce root permissions, containment, and plan checks.

Events are bounded, batched, and sequenced: job.progress, scan.batch, root.availability, plan.invalidated, backup.verification, and recovery.required. Include sequence numbers so the frontend can refresh after dropped/reordered events. Do not send an event for every processed byte or preload the whole catalog into JavaScript.

## 18. Security and privacy

Treat filenames, metadata, thumbnails, tile responses, backup manifests, and imported templates as untrusted data. Render text safely; no raw HTML from metadata. Block navigation of privileged WebViews to untrusted pages. Remote map tiles/styles receive no Tauri command authority. Avoid broad asset-protocol access to a user's home directory.

Use explicit Tauri capabilities and custom-command authorization; the capability system does not replace safe Rust path validation. Tauri's documentation notes that custom commands and Rust code need their own security decisions. [R03]

Templates contain data only, never code. Do not expose arbitrary shell, arbitrary SQL, arbitrary filesystem write, or unrestricted fetch commands to the renderer. External editing opens a selected file with explicit user action; no metadata-provided program launch.

No telemetry by default. Crash reports require opt-in and redaction; GPS, people labels, filenames, credentials, and private paths are sensitive. Catalog and thumbnail cache are local and protected by OS permissions but are not advertised as encrypted without an implemented encrypted-storage feature. Allow the user to clear caches without deleting originals.

AI extensions must be optional, local by default, separately downloadable with model source/license/checksum disclosure, cancellable, and removable. Face labels are not uploaded or shared implicitly. AI similarity does not authorize destructive action.

Signed updates and manifests are required before enabling an automatic updater. Native helper updates go through the same reviewed distribution process. Development/test automation ports, mock backends, and debugging authority must be absent from production.

## 19. Performance objectives and measurement

These are proposed acceptance targets, not measured product claims. M0 defines reproducible reference machines; use a practical 4-core-or-better CPU, 16 GB RAM, and SSD for the baseline, and record exact hardware/OS/fixture characteristics.

Target a 100,000-asset catalog for the first release and test a 1,000,000-row synthetic catalog for query/selection/virtualization behavior. The latter is not proof that one million real RAW files have been decoded successfully.

Provisional warm-state targets: first visible cached gallery within 2 seconds of opening a known catalog; ordinary indexed page/filter query p95 below 250 ms; list-mode first page below 500 ms; cancellation acknowledged within 1 second with bounded completion of the current safe I/O step; responsive input while scanning. Track frames and long tasks during scrolling rather than promise 60 FPS on every machine.

Set explicit resident-memory and thumbnail-cache budgets. Initial target is under 1 GB steady-state for shell/core/visible thumbnails together on the reference workload, with separately reported bounded decoder/backup spikes. Very large images may be refused or decoded via a lower-memory path rather than violate the configured cap.

Bench discovery, metadata, sample hashing, full hashing, byte verification, thumbnail generation, decoder peaks, and network backup separately. Record cold/warm cache, bytes read, files/sec, cancellation latency, and errors. Tune concurrency per HDD, SSD, removable storage, and network root. Do not run simultaneous full-disk scans/hash jobs at unlimited concurrency.

Performance optimization follows profiling. Efficient sequential I/O, avoiding unnecessary rereads, caching, batching, and virtualization matter more than choosing a language by a generic ranking. Photo Mechanic's own engineering notes discuss disk I/O as a main bottleneck in its browsing workload; do not extrapolate that into a universal benchmark. [R07]

## 20. Test and release gates

Use docs/ACCEPTANCE_TESTS.md as the executable acceptance backlog. Every mutating feature needs unit tests, temporary-filesystem integration tests, crash/fault injection, and desktop UI tests before enablement.

Cover native filesystem behavior on both target operating systems, including case-only renames, Unicode paths, permission failures, removable volumes, no-replace publication, and actual network protocol endpoints. Container-only Linux tests cannot establish Windows/macOS support.

Use frontend browser tests for visual/component behavior and desktop integration tests for real picker/IPC/filesystem behavior. Current Tauri documentation describes WebdriverIO's embedded driver route across Windows and macOS; validate it in M0 and exclude its plugins from release builds. Direct tauri-driver has different platform limitations. [R27]

Run formatting, linting, strict type checks, tests, dependency/security/license checks, and git diff checks. Document exact commands once the repository exists. Never report a command or OS test as passed if it was not run. Missing signing credentials block signed release, not local development; report the distinction.

Release requires correct restore verification, a tested recovery journal, clean-machine packaged codec/helpers, accessibility checks, and a documented supported-format/OS matrix. No number of attractive screenshots substitutes for these gates.

## 21. Milestones and change control

M0: capability, packaging, and safety spikes. Confirm stack, licenses, operating systems, supported decoder paths, safe filesystem primitives, backup transports, and desktop test infrastructure. Produce decisions and a minimal shell.

M1: read-only library vertical slice. Base/additional roots, persistent catalog, scan/filter pipeline, metadata, thumbnails, chronological gallery, text list, offline reconnection, and jobs.

M2: safe operations. Immutable previews, journal/recovery, template organizer, verified copy import, sidecar/bundle preservation, and guarded undo/quarantine infrastructure.

M3: cleanup and diagnostics. Exact/property duplicate modes, normalized comparison on explicitly supported formats, keeper preferences, quarantine review, integrity checks, largest files, and treemap.

M4: real backup and restore. Restic/rclone integration, SFTP/SMB/FTPS and explicit legacy FTP, credentials, snapshots, restore verification, clear scheduling behavior, and retention preview.

M5: map and metadata completeness. Spatial view, clustering, shared filters, provider privacy, annotation/album/saved-query completeness, portability, and broad-format release gates.

M6: stabilization and distribution. Fault injection, large-catalog performance, accessibility, clean-machine tests, signed installers, release documentation, and capability matrix.

Each milestone has a task file. Implement one approved milestone at a time. Record decisions and actual evidence in docs/IMPLEMENTATION_LOG.md. Dependency or scope changes need a small architecture decision record explaining benefit, cost, alternatives, and safety implications. Do not replace the Rust core with a mock service to make a demo look complete.

## 22. First implementation instruction

Read AGENTS.md, this document, docs/RESEARCH.md, docs/ACCEPTANCE_TESTS.md, and tasks/M0_FOUNDATION.md. Inspect the repository before making changes. Implement M0 only, record the test evidence and blocking decisions, and stop at its gate. The next explicit task is M1_READ_ONLY_LIBRARY.md.

## 23. Version 1.1 preservation and sync extension

Mandatory additions: docs/PRESERVATION_CONTRACT.md, docs/BACKUP_SYNC_ADDENDUM.md, docs/PRESERVATION_TESTS.md and docs/BACKUP_SYNC_SOURCES.md. The new contract strengthens earlier defaults; preserve all unrelated requirements. The M4 task defines phased delivery. Use docs/CODEX_BACKUP_SYNC_PROMPT.md only with prerequisite inspection, not as permission to bypass M0-M3. All 85 original AC scenarios and 40 additional PS scenarios remain proposed tests until real evidence is recorded.

## 24. Version 1.2 library feature extension

Read docs/LIBRARY_FEATURES_ADDENDUM.md, docs/LIBRARY_FEATURES_DATA_MODEL.md, docs/LIBRARY_FEATURES_TESTS.md and docs/LIBRARY_FEATURES_SOURCES.md. This specifies six owner-requested capabilities: integrity report, Recently added, basic non-destructive image editor with mobile follow-on, large iCloud/Google Photos imports, save/open catalog files and photo/video tags.

The preservation contract still has highest precedence. Integrity checks and editor/tag saves never modify originals. Missing requires an authoritative successful enumeration; offline/incomplete is different. Added dates are catalog events, not capture/upload guesses. Catalog saves are consistent snapshots, not active-WAL file copies, and do not contain original media. Google Takeout and Apple original exports/local copies are initial importer paths; API/PhotoKit paths are capability-gated and fidelity-labeled. No fake provider/cloud completeness or mobile support claims.

Use docs/CODEX_LIBRARY_FEATURES_PROMPT.md with its prerequisite gate. N1/N2 extend existing modules, N3 adds the desktop recipe editor, N4 is separately staged provider importing, and N5 is the requested future mobile editor. Approve one slice at a time and retain the original M0-M6 release/evidence requirements.

### Default photo extension preset — owner revision, 2026-10-27

.jpg, .jpeg, .jfif, .heic, .heif, .hif, .png, .tif, .tiff, .webp, .avif, .gif, .bmp, .dng, .cr3, .cr2, .crw, .nef, .nrw, .arw, .sr2, .srf, .raf, .orf, .rw2, .raw, .pef, .srw, .x3f, .3fr, .fff, .iiq, .rwl

The 33-suffix preset controls photo-file discovery, not decoder capability. Unsupported formats remain catalogued with honest metadata/preview status; existing format, color and native-platform gates remain. Video suffixes remain recognised by the engine when a saved/custom filter permits them, but are not in this default photo preset.

### P0 performance evidence — owner authorisation, 2026-10-28

Implement only the bounded diagnostic/fixture/report slice in [PERFORMANCE_PLAN.md](PERFORMANCE_PLAN.md), with [PERFORMANCE_ACCEPTANCE_TESTS.md](PERFORMANCE_ACCEPTANCE_TESTS.md) and [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md). Preserve current UI, query semantics, schema2, original-file guards, FULL durability and recovery requirements. Diagnostics are opt-in, local, bounded and exclude sensitive content/paths. Distinguish release synthetic catalog results, real generated-media worker results and debug native UI observations. The standalone SQLite provenance flag opens no user catalog. Existing paging/virtualisation/caching are inspected implementations, not evidence that the proposed performance phases or targets passed. No P1–P4, backup adapter, schema/decoder/dependency replacement or original mutation is authorised by P0.

### Approved P1 catalog read revision — 2026-10-28

Gallery/list/seek pages remain bounded to240 rows and retain all display fields. Native page transport uses Rust-generated PackedAssetPage format1; src/pageTransport.ts restores the existing AssetPage at the IPC seam and rejects incompatible versions. Page SQL excludes native path blobs, raw metadata and other non-display columns. Unfiltered inventory navigation requires the existing scope/sort-specific ordered indexes; selective queries retain normal planning. Counts/timeline buckets use a per-worker LRU bounded to16 entries/512KiB encoded data, keyed by the full validated query and invalidated by connection writes/external commit observations. Frozen selection membership and integrity/recovery evidence are not cached. See [ADR-0009](decisions/ADR-0009-catalog-read-performance.md), [performance acceptance criteria](PERFORMANCE_ACCEPTANCE_TESTS.md) and [P1 measurements](PERFORMANCE_P1.md). This authorises no schema/dependency migration, source change or later performance phase.

## Read-only native preview slice — owner approval, 2026-10-28

The common-image/HEIC/CR2 request authorises the bounded M5 format slice in [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md) and [ADR-0010](decisions/ADR-0010-native-format-previews.md). On verified macOS, the existing isolated byte-input worker adds ImageIO/ColorSync previews for HEIC/HEIF, TIFF/GIF/BMP and tested camera samples CR2/CR3/NEF/ARW/DNG/RAF. Retain baseline decoding, original read-only access, bounded processing, 256/512 px derived PNGs and explicit unavailable states. Fixed native-extension hints never expose arbitrary paths/UTIs to the renderer. Retry legacy negative observations under the new processing policy while preserving compatible indexed M1 positive previews offline; no schema migration or cache purge. This supersedes candidate-only adapter wording for this slice, not the cross-platform/full-format release gate. Other M5 work and later performance phases remain separate.

## Gallery balancing refinement — owner request, 2026-10-28

Refine the existing chronological canvas using the owner's Google Photos engineering reference; see [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md). Choose row breaks across the bounded retained photo sequence, allow up to125% target height for ordinary rows, preserve aspect ratios/order/4px gutters and keep sparse final rows at or below target. Panoramas must not be forced into an already crowded row by a greedy threshold. Compact narrow date captions retain full accessible dates and exact shown-group membership. Coalesce resize notifications per animation frame while preserving the existing stable-ID/offset anchor. Keep the current bounded feed/virtualizer, timeline, selection, text-only behavior and read-only preview pipeline. No new cache/scheduler, catalog migration or later performance-phase completion is implied.

### Owner revision — progressive derived previews, 2026-10-28

Add the bounded P2 slice in [PROGRESSIVE_PREVIEWS.md](PROGRESSIVE_PREVIEWS.md): a separate cancelable thumbnail-generation job after completed scans, manual existing-library backfill through header Jobs, 32 px cached placeholders and demand-driven clear previews with an accessible blur-to-clear transition. Read-only macOS JPEG/PNG/TIFF downsampling may use an authorised inherited file handle under explicit input/read/output/deadline/memory-watchdog limits. Per-tier failures must not blank a usable cached preview. Preserve originals, offline states, retained compatible cache entries and all preservation gates; no schema/dependency migration or all-P2/P4 approval is implied.

### Owner revision — gallery scrolling and focus, 2026-10-28; clarified 2026-10-29

For chronological collections, use the right-hand timeline as the sole visible vertical scroll control; hide the extra native gallery scrollbar while retaining wheel, trackpad and keyboard scrolling. The timeline follows continuous scroll progress within the retained rows of the current month, including single-month and Undated-only results. Dragging within the current month scrolls its retained rows immediately; another month uses the existing indexed seek on release. Month fractions describe the bounded retained window, not a catalog-wide asset rank. Other sorts retain their normal scrollbar. The date handle follows browsing after focus, maps top-to-bottom consistently, commits a drag released outside its narrow bounds, and cancels interrupted gestures without navigating. Wheel input over the date rail scrolls the photo viewport. Remove the container-wide focus outline while retaining the viewport tab stop and visible keyboard focus on actual controls. See [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md#gallery-scrollbar-and-focus-repair--2026-10-28); original-file access and bounded paging stay unchanged.

### Owner revision — navigation toggle placement, 2026-10-28

Move the existing navigation collapse/expand control from the wide sidebar row to an icon-only button at the leading edge of the current page header, immediately before its title, matching the owner's annotated screenshot. Keep it available when navigation is collapsed and across Photos, Folders, Albums / Tags, Settings and onboarding; preserve accessible action names, expanded state, keyboard focus and existing preference behavior. The sidebar retains branding, destinations, footer Settings and version. This is a placement change only.


### P2 gallery/rendition continuation — owner confirmation, 2026-10-29

Extend the derived-cache pyramid to32/256/512/1024/2048px under [ADR-0012](decisions/ADR-0012-bounded-rendition-pyramid.md). Choose gallery tiers up to1024px from actual rendered geometry/DPR under bounded decoded-pixel admission; the explicitly opened viewer requests2048px while retaining smaller cached layers through actual load or failure. Backend fallbackEdge exposes an existing smaller indexed rendition without starting an extra request. Text-only behavior,720-summary window, selection, stable anchors and offline distinctions remain. The worker's bounded versioned metadata/PNG frame replaces numeric-array JSON for production decode responses; legacy diagnostic modes remain. Keep256MiB indexed cache retention, read-only originals, existing input/dimension/time/concurrency/RSS controls and per-tier failure/revision guards. [P2 evidence](PERFORMANCE_P2.md) separates measured local behavior from unavailable native release/platform gates. Stop before P3.

Prioritise visible rows before overscan for rendition admission without changing chronological DOM or selection order. Keep justified multi-image rows at least half the requested height; naturally short single panoramas remain valid. Resolve preview status through bounded cache-index and narrow status batches, preserving revision, policy, offline and maintenance checks.

### Owner revision — live library reconciliation, 2026-10-29

Keep Gallery/Folders current with debounced backend native notifications and bounded read-only reconciliation. Treat events as hints, revalidate roots/volumes and infer absence only from complete scopes; offline/inaccessible contents remain retained. Hide confirmed missing items/branches in normal views. Persist versioned observed changes transactionally in the existing journal and expose bounded Library changes history in the header Jobs dropdown, including old/new paths and monitoring state. Preserve unambiguous move identity, notes, bounded feeds and anchors; include newly arrived leading photos without a manual refresh. Record lossless native locators alongside display paths. Keep history across restarts and explicitly disclose retention of previous paths when forgetting a library. Native-watch failure/overflow and startup/reconnect require reconciliation; periodic fallback and latency limitations must be stated. Follow [LIVE_LIBRARY_UPDATES.md](LIVE_LIBRARY_UPDATES.md) and [ADR-0013](decisions/ADR-0013-live-library-reconciliation.md). No original writes, schema migration or later milestone is authorised by this refinement.

### P3 read-only scheduling — owner continuation, 2026-10-29

Coordinate metadata extraction and rendition work through shared backend-owned admission with bounded CPU/memory estimates, conservative source-volume concurrency, interactive preference and aging background service. Preserve source identity/revision checks, durable job outcomes, cache maintenance and decoder limits. During scrolling request visible tiny previews, then clearer tiers and bounded overscan after settling; retain existing displayed layers and layout. [ADR-0014](decisions/ADR-0014-shared-source-admission.md) defines exact scope and limits. No transfer-rate controls or complete physical-device guarantee are implied; backup adapters, accounted pacing and full native contention acceptance remain unavailable. [PERFORMANCE_P3.md](PERFORMANCE_P3.md) records measured and blocked gates.

### Owner revision — aligned sticky date sections and cache reuse, 2026-10-29

In chronological Timeline, begin each date on a separate justified photo row with one full-width heading aligned to the same left edge. Keep the active heading sticky in its date section; the next heading pushes it away. Preserve shown-group selection, capture-local/added/unknown date semantics, aspect ratios, bounded virtual rows and browsing anchors. Reuse already-loaded/cached clear previews during scrolling and remove the extra CSS blur filter. Visible tiny work remains bounded; missing clear work waits for a 150 ms settled signal. The optional typed cache_only thumbnail flag performs only owned-cache/catalog status lookup and LRU bookkeeping, never original access or work-queue mutation. Normal requests remain compatible. Details/limits and actual evidence are in [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md). No original operation, decoder/dependency upgrade, catalog migration or later performance phase is authorised.

### Owner correction — timeline endpoint feedback, 2026-10-29

When the gallery reaches the known matching-catalog end, the timeline must reach its exact maximum and report the final visible asset's period, including sparse final months, Recently added and Unknown date. Keep the first visible photo anchor for restoration. A loaded page with more matches is not the catalog end. Retained-month navigation and indexed seeks remain bounded; a wholly fitting collection starts at 0 and a fitting tail-only window with earlier pages available reports the end. See [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md) for endpoint semantics and evidence. No original-file or backend/schema change.

### Owner correction — oriented gallery proportions, 2026-10-29

Catalog display summaries must use the same oriented width/height as previews. For retained EXIF orientations 5–8, transpose dimensions only when the stored pair still matches the retained sensor width/height. Apply the same read projection to gallery pages, seeks/neighbors, details and folder browsing, including existing catalogs. Retain original stored metadata/dimensions; browsing must not migrate or rewrite a catalog or read originals to repair layout. Malformed/missing metadata and already-oriented dimensions retain their existing geometry. Timeline preserves composition; Uniform grid remains intentionally square with its explicit Fit/Crop control. See [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md).

### Owner correction — high-resolution JPEG previews, 2026-10-29

Allow read-only macOS file-backed JPEG downsampling up to16384px per side under the retained144000000 source-pixel ceiling, existing read/input/time/memory/admission/output limits and ICC/orientation/revision checks. Other adapters and byte/full-raster paths retain their current geometry bounds. Report true limit failures explicitly as unsupported preview budgets. A versioned policy retries obsolete negative observations on demand while retaining compatible positive caches and historical job outcomes; no original conversion, rescan, blanket purge, migration or new phase. Follow [ADR-0015](decisions/ADR-0015-tall-jpeg-previews.md).

### P4 local stress evidence — 2026-10-29

The next performance stage runs bounded generated-source stress/failure checks with explicit build and platform scope. Retain the production release automation exclusion; current release core/packaged-worker probes and debug native UI diagnostics are separate evidence. Diagnostic segmentation retains fixed4096-event buffers and loss counts, with independent lossless source manifests and executable/lock provenance. [PERFORMANCE_P4.md](PERFORMANCE_P4.md) records actual results; P3 backup/endpoint/pacing and full native release-presentation/memory/platform gates remain open. No new original-file operation, schema/dependency change or later milestone is implied.

### P4 continuation — exact empty filtered results, 2026-10-29

A validated display query with an exact zero count can return its empty page/revision without an ordered asset scan. Share proven emptiness between the page count and month summaries through the existing fully query-scoped bounded cache and conservative write/data-version invalidation. Inventory counters that may lag a scan never prove emptiness: preserve partial rows and timeline entries. Keep cursor/filter/limit validation, typed responses, schema, durability, original access and safety gates. [PERFORMANCE_FILTERED_READS.md](PERFORMANCE_FILTERED_READS.md) records the release reproduction, bounded fix and remaining positive-filter/P4 gates.

### P4 continuation — positive timeline totals, 2026-10-30

When a matching positive timeline is already retained in the existing summary cache, pages and stable-ID seeks may sum its bucket counts with checked arithmetic instead of repeating an exact count scan. Preserve full-query identity, epoch invalidation, retention bounds and SQL fallback after eviction; never compute a timeline eagerly for a page. Unfiltered visible inventory totals retain their existing possibly lagging counter semantics. Selection/safety evidence, query validation, projection and cursor behavior remain unchanged. [PERFORMANCE_POSITIVE_TOTALS.md](PERFORMANCE_POSITIVE_TOTALS.md) records both request orders and remaining cold-filter/aggregation/P4 gates; this is a bounded continuation, not a new milestone or migration.

### P4 continuation — selective filename projections, 2026-10-30

For an unscoped filename-only display query with an already-known exact total of 1–1,024, a bounded name-index lookup may narrow full-row page/seek reads and known-count month summaries. Retain at most 1,025 transient native rowids; apply their predicate only when all raw name matches fit, otherwise retain the original full scope. Preserve every validated predicate, order, cursor, stable UUID and count. Cold counts, unknown/large summaries, mixed scopes, viewer neighbors and frozen selection keep their existing plans. No persistent rowid cache, new index/schema/dependency/IPC or source operation. [PERFORMANCE_FILENAME_READS.md](PERFORMANCE_FILENAME_READS.md) records the rejected eager-probe regression, final measurements and outstanding gates.

### Owner revision — first library preview preparation, 2026-10-30

For a folder's first scan, keep catalog observations early but defer Photos/Tags collection entries until tiny preview publication or an explicit unavailable outcome. Do not fill the first collection with generation placeholders. Use a durable initial thumbnail job in the existing schema and a consistent typed ready-scope predicate for pages, counts, dates, seeks and selection. Existing completed libraries/cache/offline behavior remain unchanged. Keep decoding separate from discovery and originals read-only.

Show a temporary top banner: indeterminate discovery with found/indexed counts while the total is unknown, then counted preview progress and remaining eligible media. Include safe stop/pause and explicit resume/error states; preserve pending preparation across cancellation/restart and do not let reconciliation undo a pause. Successful/partial completion requires no eligible pending current revisions. Header Jobs/history remain available; Text list stays image-free and shares the collection scope. [INITIAL_LIBRARY_PREPARATION.md](INITIAL_LIBRARY_PREPARATION.md) and [ADR-0016](decisions/ADR-0016-initial-preview-preparation.md) supersede earlier immediate-first-gallery and removed-scan-banner wording for this bounded initial phase, without passing wider milestone/preservation/platform gates.

### Owner single-photo viewer revision — 2026-10-30

The single-photo viewer expands from its clicked thumbnail into the page container, keeping the sidebar visible and usable. Clicking the image/background does not dismiss it; X and Escape perform a reverse close transition (Info closes first on Escape). Retain filtered navigation, zoom/pan, catalog annotations and focus/scroll return. Respect Reduced Motion and bound/cancel animation work. Show a glare driven only by actual preview loading, with textual feedback and explicit unavailable/retry states. Recover transient cached-image request failures with one bounded transport retry; persistent failures remain explicit and can be retried through the guarded native pipeline. No automatic original mutation or cache purge. See docs/VIEWER_PRESENTATION.md and ADR-0017; this supersedes earlier whole-window single-photo modality only.

### Owner original-image viewer revision — 2026-10-30

Load the original image rather than only a 2,048-pixel derived rendition when its format and resource envelope are supported. Use a backend-authorized ID/revision/policy route, native identity/revision guards, bounded read/helper work and no source mutations or arbitrary renderer paths. Static JPEG/PNG/WebP originals up to 32 MiB and 32 million pixels are the initial scope; all other/offline/error cases retain an explicitly labelled preview and retry. Remove the unconditional cached-preview banner only when the behavior no longer depends on it. Put X at the toolbar's right and a confirmed catalog-backed animated heart at the bottom. Preserve Reduced Motion, loading/error states, page/menu containment, arrows/zoom/pan and return focus. Follow docs/ORIGINAL_IMAGE_VIEWING.md and ADR-0018; this is a bounded read-only slice, not a broad full-resolution/format or preservation-gate claim.

### Owner viewer control-bar refinement — 2026-10-30

Group single-photo Zoom out/in, Fit, Info and Filmstrip into one centered floating icon bar over the photo. Include the existing favorite heart immediately before Info; the bar overlays the image without a reserved footer. At narrow widths, stack the bounded scrolling Info panel below the photo/filmstrip to preserve control access. Preserve accessible names/tooltips, zoom slider/percentage, disabled limits, toggle states, Info focus return and minimum-window access. Keep original access/fallback, page/menu containment, arrows, X, favorites and all safety/milestone gates intact. See docs/ORIGINAL_IMAGE_VIEWING.md.

### P4 continuation — cold filename summaries, 2026-10-30

Keep the existing SQL planner and indexes, but evaluate filename matching immediately after media classification and before presence/initial-preparation checks in the shared validated query predicate. Nonmatching names can avoid later row checks while non-media rows retain their early rejection. Preserve parameter binding, escaped literal matching, all query scopes/sorts, capture/added/Undated summaries, exact filtered counts, inventory-counter lag and summary invalidation/bounds. No eager candidate probe, new index, schema/dependency/IPC change or original access. [PERFORMANCE_COLD_SUMMARIES.md](PERFORMANCE_COLD_SUMMARIES.md) records statement-work regression, reference/current release measurements, rejected index alternatives and remaining P4 gates; predicate order does not promise reduced physical I/O or universally faster filters.

### P4 local closeout before M2 — owner choice, 2026-10-30

Finish available local trace/memory/catalog-restart work before M2. Keep the recorder’s 4,096-event total, sixteen private volume aliases, nonblocking producers, explicit loss reporting and production no-ops. A fixed partitioning/stable producer preference may reduce contention without increasing event capacity; strict reports still reject any loss. Close SQLite before explicitly releasing its lifetime lock so an unrelated pre-exec child cannot prolong it; retain genuine-owner rejection with no lock bypass or open retry. Separate observed-process collection status from full WebContent/GPU/XPC coverage and keep privileged/native-platform gaps explicit. [PERFORMANCE_LOCAL_CLOSEOUT.md](PERFORMANCE_LOCAL_CLOSEOUT.md) records actual evidence; this does not authorize original mutation or declare full P4/M2 acceptance.

### P4 final evidence and cross-milestone gates — 2026-10-30

Consolidate current P4-01–P4-06 dispositions in [PERFORMANCE_P4_STATUS.md](PERFORMANCE_P4_STATUS.md), preserving earlier failures and exact run/build identities. Current release catalog/media, packaged-worker and synthetic-contention evidence must retain independent source inventories with lossless 64-bit stat transport. Separate completed read-only implementation/local verification from P4-06’s unfulfilled whole-memory, native release presentation, target-platform and M4 backup/endpoint/pacing evidence. No requirement is waived; later fixture-based M2/M3 development remains governed by the preservation contract, and real original-file mutation remains disabled.

### Owner HEIC original-viewer fix — 2026-10-30

Fix the original-format gate rejecting otherwise viewable HEIC images. Extend the guarded original-byte route to detected HEIC/HEIF still-image containers on compatible macOS webviews, with unchanged source bytes and image/heic MIME. Preserve all source identity/revision, helper, concurrency, 32 MiB/32-million-pixel and failure/retry limits. Validate bounded still-image brands/container extents and reject sequences/timed tracks. Existing indexed images need no rescan or cache clearing; bump the generated route policy without schema/dependency changes. Verify real full-size HEIC pixels and absent fallback warning in native WKWebView, exact bytes in packaged probes and unchanged fixture sources. Keep unsupported webviews/variants and broader platform/preservation gates explicit. See docs/ORIGINAL_IMAGE_VIEWING.md.
