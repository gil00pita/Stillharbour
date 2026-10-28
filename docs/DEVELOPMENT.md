# Development

M1 implements the read-only library on macOS Apple Silicon. Read [M1 validation](M1_VALIDATION.md) and [ADR-0002](decisions/ADR-0002-read-only-library.md) for behavior, limits and platform gates. Original-file mutations and production backup remain unavailable.

Install Node 24 or newer compatible Node, npm 11.16.0, Rust via rustup, and the [native Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). The repo pins Rust 1.99.1 with rustfmt/clippy. This session installed rustup without editing shell profiles; Node scripts explicitly include `~/.cargo/bin` in child PATH.

```sh
npm ci
npm run desktop:dev
```

The native command uses Conductor's allocated port when present and updates both Vite and Tauri's dev URL. Vite binds loopback and fails on port collisions. `npm run dev` alone opens a frontend development server; outside Tauri, the UI truthfully reports the missing native connection. It has no successful browser fallback.

Conductor configuration lives in `.conductor/settings.toml`. Each worktree gets its own debug catalog under `.context/native-data`, so desktop development can run concurrently across workspaces. Conductor's Mac client reads shared settings after they reach the remote default branch; the commands can be run directly immediately. Native test sessions own fresh temporary catalogs and generated source folders; they do not share the development or production catalog.

Production builds use Tauri's local application-data directory. The directory picker returns a display path and opaque session ID; its native path stays in Rust. Selecting a directory only opens its handle and reads directory metadata, never enumerates its children. Canceling preserves the previous selection. Save & Start scanning explicitly registers that selection and begins a read-only scan. It is not permission for original-file mutations.

The initial screen focuses on the folder-selection heading and button. Selecting a folder replaces its welcome heading/icon/chooser with the form alone; Cancel returns to the chooser. The form and Advanced panel grow naturally without internal scrollbars; the main pane handles page scrolling. The sidebar footer displays the running Rust package version instead of tagline copy. Native/database diagnostics and milestone/footer notices are omitted from the product interface. Connection readiness still gates the button, errors remain visible, and a selected folder is explicitly unscanned. Native smoke tests verify catalog status through real Rust IPC rather than relying on visible developer diagnostics.

The selected-folder card defaults new folders to Managed, with a Read only switch that saves referenced mode. Existing roots retain their mode. The full-width Advanced accordion uses independent Include hidden files / Include hidden folders switches and removable tags for exclusion patterns, excluded folder names and File Extensions. Include patterns is removed; setup clears legacy include-glob restrictions on Save/Rescan. New folders start with the 33-photo-extension preset exported from Rust; displayed dots and uppercase input normalise to lowercase suffixes. Existing custom filters remain intact. Enter, Tab and blur add a value; multiline paste adds several; commas remain literal. Save and Preview also include a pending value without needing blur. Scan date fields are removed, and save/rescan clears any legacy scan date bounds. Browsing date filters remain available. Size filters and the pure relative-path preview remain. Save persists the root and starts a real background scan. Results, cancellation and per-file issues come from Rust. Root mode, filters and annotations survive restart. Reconnecting or relinking reconciles retained inventory; an offline root is never treated as empty.

The library uses bounded keyset pages (240 assets), virtualized gallery/grid/text rows and direct year seeks. Text list never requests nonempty preview batches. Select all results freezes membership in SQLite; favorite/rating/tag actions update the catalog only. The inspector labels date provenance and unknown offsets. Missing codecs, invalid or unsupported ICC inputs and oversized files stay catalogued with explicit unavailable previews. The approved macOS native preview slice and its per-camera evidence are documented in [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md).

The current photo canvas supersedes the earlier gapless gallery: justified 4 px rows preserve photo ratios, day headings show date provenance through details, and hover/focus checkboxes support explicit/page-range selection. Thumbnail density presets target 150/220/300 px; Uniform grid offers fit/crop and Text list requests no images. A fixed filename search, removable filter chips and collapsible navigation keep controls available. Actual filtered month buckets back the right-hand scrubber and indexed seeks for capture/Recently added sorting. Scroll loading replaces explicit paging: three native chunks of at most 240 assets retain no more than 720 summaries, with virtual rows and stable-ID anchors. See [ADR-0003](decisions/ADR-0003-photo-canvas.md) for controller, selection, preview ownership and new typed IPC.

The modal viewer navigates within the same filtered query, including outside the loaded page, with optional filmstrip/Info and 100–400% fit-relative zoom/pan. It uses the existing 512 px cached previews, not a new full-resolution decoder. Closing restores focus and the original gallery offset. Compare accepts only 2–4 explicit selections and establishes no duplicate evidence. Batch Add tags merges atomically into existing catalog tags; favorites/ratings/tags never rewrite originals or sidecars. Similarity analysis, albums, original editing and export remain unavailable. Native dialog behavior follows [MDN showModal](https://developer.mozilla.org/en-US/docs/Web/API/HTMLDialogElement/showModal), the accessible scrubber uses a [native range input](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/range), and virtualization uses the [official TanStack API](https://tanstack.com/virtual/latest/docs/api/virtualizer).

The header Jobs dropdown renders scan progress from the existing polled native status. Streaming discovery has no fixed total, so running/cancel-requested bars are indeterminate and retain actual found/indexed/skipped/issue counts. Only a native completed scan fills its bar; other terminal states show a stopped neutral track and explicit outcome. Reduced motion disables the activity sweep. This is presentation only; it adds no scanner, filesystem, schema or IPC changes.

Advanced scan Minimum size/Maximum size use independent KB/MB/GB selectors (decimal units, 1 KB = 1,000 bytes). Empty inputs start in MB; saved limits select a suitable unit and retain every byte. FileSizeField uses exact BigInt conversion to the existing nullable byte-string fields; blank clears a limit, zero does not. Switching units changes the scale of the entered amount. Invalid drafts show a field error and block Save/Preview through form validation. Rust still validates the size range. The browsing Filters byte inputs and backend schema are unchanged.

Desktop-test builds expose generated-source and synthetic-catalog fixture commands and a read-only frontend request-observation event, plus the existing picker-presentation registry. The suite bypasses only the OS picker interaction with a server-issued selection for an owned generated folder; saving, scanning, queries, annotations and cached PNG loading use actual Rust IPC/protocols. Normal builds exclude all test routes. Native OS picker automation remains unverified because Accessibility access is unavailable.

| Command                                       | Purpose                                                                                                    |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `npm run check`                               | Formatting, ESLint, strict types, component tests, IPC drift, Rust formatting/clippy/tests, frontend build |
| `npm run bindings` / `npm run bindings:check` | Generate / verify the Rust-owned IPC schema                                                                |
| `npm run fixtures:smoke`                      | Owned temporary catalog, volume, publication and interrupted-copy probes (macOS only for publication)      |
| `npm run media:smoke`                         | Real decoder-child failure/timeout/replacement tests                                                       |
| `npm run desktop:test:build`                  | Build a debug native app with test-only WebdriverIO instrumentation                                        |
| `npm run desktop:test`                        | Drive native WebView with owned generated sources; native library IPC                                      |
| `npm run desktop:build`                       | Build the normal unsigned local macOS app bundle                                                           |
| `npm run release:check`                       | Check narrow production permissions and default dependency graph                                           |

Rust equivalents are `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --features stillharbour-core/fixture-probes -- -D warnings`, and `cargo test --workspace --features stillharbour-core/fixture-probes`. Do not use `--all-features` for a release; it deliberately enables the forbidden test server. `fixture-probes` is enabled only by explicitly instrumented desktop tests, never the normal application.

The helper probe is developer-only and requires explicitly supplied, checksum-reviewed helpers:

```sh
python3 scripts/helper-smoke.py \
  --restic /absolute/path/to/restic \
  --rclone /absolute/path/to/rclone \
  --exiftool /absolute/path/to/exiftool \
  --perl-library /absolute/path/to/exiftool/lib
```

It creates its own temporary root, PNG and companion fixtures, rejects an unwanted ExifTool config, backs up/restores a local restic repository and compares restored bytes. It removes only its asserted generated source folder. It never accepts a real photo root. A local test does not prove remote protocols or metadata fidelity. See DEPENDENCIES.md for the exact development helper versions and packaging gaps.

Run `npm audit`, `npm audit --omit=dev`, and `cargo audit` when updating lockfiles. Review advisories rather than applying a forced downgrade. Dependency license metadata is an inventory input, not a finished distribution legal review.

The GitHub workflow defines macOS/Windows checks and native smoke tests. A checked-in workflow is not evidence of a completed run. Windows filesystem publication/identity and the packaged helper/codecs/signing matrix remain gates. M1 is authorised; stop before M2. Windows scanning stays unavailable pending its native identity/traversal adapter and tests.

To repeat the owned synthetic catalog and packaged-code probes on macOS:

```sh
node scripts/cargo.mjs run --release -p stillharbour-core --features fixture-probes --example library_probe
node scripts/cargo.mjs run --release -p stillharbour-core --features fixture-probes --example packaged_library_probe -- target/release/bundle/macos/Stillharbour.app
```

The packaged probe accepts only an application-bundle location. It creates its own photo fixtures and catalog, invokes the bundle's decoder executable and bundled ExifTool resources, and checks unchanged generated source bytes/names. It accepts no user-library path.

## Documenting requested changes

Every requested repository change includes the relevant documentation updates and a dated entry in [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md), including documentation-only requests. Update the existing documents that own the affected requirements, behavior, commands or paths. Approved product changes must also update the applicable specification and acceptance criteria; milestone and preservation gates remain in force.

The log records the request, resulting change, affected files, actual validation and remaining limitations or next gate. Keep small entries brief and preserve earlier evidence. A documentation-only change needs relevant formatting/link checks; a code change needs checks appropriate to its behavior. Describe unrun checks accurately. Material architecture decisions belong in `docs/decisions/ADR-*.md`.

The app shell fills the viewport. Sidebar and main content have independent vertical scroll areas, so long onboarding/settings pages do not move the identity rail. The collection retains its virtualized scroll region; settings use main-page scrolling. Selecting a new folder moves keyboard focus into Library name. Native tests compare sidebar/brand/navigation positions before and after scrolling the main pane at normal and minimum desktop sizes.

## Settings and preferences

The sidebar Settings screen manages indexed photo folders; it does not relocate catalog/cache storage. Change location reuses the native picker/identity-checked relink workflow, and Add photo folder reuses explicit setup/Save & Start scanning. Use generated disposable libraries for all location/scan tests. A relink may reconcile the selected folder; never invoke it against a real library as a development test.

Preferences are generated from crates/core/src/preferences.rs and use the existing foundation_request permission boundary. The backend saves preferences.json only inside its private app-data directory (worktree-local for debug, temporary for native tests). It reads at most 64 KiB, validates the version/enums/shared scan filters and registered default-root ID, rejects links and unreadable/future data, then stages/syncs/atomically replaces its own preferences file and syncs the parent. This is app-owned configuration, not an original-file executor or catalog migration. Publication follows the pinned [tempfile persist API](https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist); contents and directory require explicit synchronization. Windows preference saving remains gated pending native durability tests.

Settings defaults cover new Photos sessions and new folder setup. ScanOptions/scanForm, switches, tag fields, FileSizeField and BrowserControls are shared; existing root settings take priority over defaults. A corrupt preferences file is preserved with an explicit failed-load/retry state; automatic repair/reset of invalid files is not implemented. See [ADR-0004](decisions/ADR-0004-settings.md). Run npm run check, then desktop:test:build and desktop:test sequentially because general/test builds share dist output. Use the documented supported Node/npm versions; the installed Node 26.3.1/npm 11.16.0 toolchain is available through ~/.nvm/versions/node/v26.3.1/bin.

## Gallery side panel and scroll feed — 2026-10-28

Photos has fixed filename search, Filter and Jobs in its header. The native dialog side panel owns query/sort/display controls, folder/tag facets, active chips, total count, explicit availability/relink/settings and applied-query Select all. Apply commits drafts; Close/Escape cancels. The independently scrolling body preserves access to fixed actions at the minimum window. Add folders through Settings. A contextual batch toolbar appears only after selection.

`useAssetFeed` serializes continuations, rejects stale-query responses and retains at most three 240-row keyset chunks. Forward loading evicts the oldest chunk; scrolling upward near the top (including scrollbar movement), wheel and keyboard navigation retrieve the preceding chunk. Programmatic anchor restoration is distinguished from user scrolling to avoid before/after oscillation. Capture the visible stable ID/offset before committing each window. Timeline seeks stay direct. Catalog revision refresh rereads at most three chunks from the first retained ID, falling back to the query head if the window disappears. Failed continuation retains rows and requires Retry. Native `seek_asset` uses an allowlisted sort and SQLite-native sort key (including u64-sized byte values), validates IDs/limits/query, and filters every returned row. It reads only the catalog. See [ADR-0005](decisions/ADR-0005-filter-panel-feed.md).

The desktop-test-only `test_scroll_catalog` accepts no paths/counts and requires a verified generated library fixture. It inserts 960 explicitly unavailable synthetic catalog observations, not original files. Its only purpose is native bidirectional window/anchor validation in Text list. Production commands/capabilities exclude both fixture routes. This is not a large-photo decoding or full-library performance benchmark.

## Inline library scan settings — 2026-10-28

Settings now includes ScanSettingsGroup after Library locations. It embeds the existing RootSetup/ScanOptions form with a registered-root selector, saved per-root mode/filter values and independent Save & Rescan / Discard scan changes actions. It has no surrounding setup screen or dialog and no nested form. Gallery Scan settings links navigate to Settings and focus the group for that root. Per-location launch buttons are removed. Add photo folder still opens explicit onboarding; Change location still uses identity-checked relinking.

The frontend `useFoundation.rescan` reuses typed update_mode and scan operations, then refreshes native status without navigating away or saving defaults. Failure refreshes status and rejects to the inline error, retaining the draft. Native mode publication and scan start remain separate existing operations; a scan failure is not proof the preceding mode update rolled back. Discard reloads the current native root values and resets field-local drafts/validity/preview. App defaults remain in a separate named form, so pending tags and identical scan field names never cross forms. Ordinary polling does not reset an unsaved scan draft. Select another library to load its saved values; discard pending edits from the previous root.

Native regressions use generated sources to exercise gallery routing, per-root save/rescan, restart-of-the-page persistence, unchanged app defaults, valid form structure and desktop/minimum layouts. They do not authorize a personal-library rescan or original-file changes.

## Remove library and clear preview cache — 2026-10-28

Settings LibraryMaintenance uses typed prepare_maintenance/apply_maintenance through the existing local-main-window library command. A single five-minute opaque plan records target and catalog revision; apply takes the scan permit, cancels queued/inflight preview work and waits for the cache write barrier. Worker publication and protocol reads take the same read barrier. Native status/recovery is serialized with cleanup. No renderer destination/path or SQL is accepted.

CacheCleanup holds the owned thumbnails directory descriptor and unlinks only index-owned 64-hex PNG names using openat/O_NOFOLLOW and unlinkat. It rejects linked/nonregular/invalid PNG-signature files and normal identity replacement; it does not claim immunity to a hostile writer changing owned files between checks. Cleanup is not recursive. Unindexed orphan/staging/unknown files are retained rather than claimed as owned from their suffix. File lists stream in 256-key batches; no unbounded renderer/worker list.

The existing cache index remains until files are removed and the directory synced. A crash/error before catalog commit retains a retryable ownership list; already missing files are safe to retry and previews can regenerate. Final SQL transaction checks the revision again, removes target records/dependencies, promotes a surviving base UUID and bumps catalog revision; schema remains2. Cache-only cleanup leaves catalog notes/roots intact. Forgotten annotations are intentionally not restored by re-adding; no secure erasure/Undo/catalog portability feature is claimed. Native preference reads neutralize a removed default-root ID only in the returned effective defaults, preserving the app-owned file and unrelated preferences.

See ADR-0006 and matching acceptance criteria. Validation is restricted to generated libraries and temporary app-data/cache directories; development never removes a real registered library or its cache. Native platform gates remain for Windows; POSIX descriptor behavior is exercised on macOS.

## Folders explorer — 2026-10-28

App routes Folders to FolderExplorer rather than the chronological collection. useDirectoryPage reads a typed browse_directory page for the selected root/directory/search/cursor; stale replies are ignored and failed reads remain explicit. ExplorerCanvas virtualizes one bounded page in responsive Icons or image-free Details. Back history is capped64; breadcrumb text scrolls horizontally rather than expanding a deep path into an unusable viewport. Photos keeps its own three-chunk scroll feed.

The native directory adapter queries existing directory/asset observations only. It returns folders first plus immediate files (including non-media records) in at most240 entries, with bidirectional root/directory/search-bound cursors and a joined asset projection rather than per-file lookups. Identifiers encode native relative-path bytes and are catalog keys, not filesystem execution inputs. The root uses the empty native path. Root IDs are validated; unknown/excluded/blocked directory navigation fails. Old observations are labeled retained; they do not prove live presence. Empty/indexing/offline states remain distinct. Saved scan settings still determine what has been indexed.

AssetQuery accepts optional directoryId for exact folder_native equality. The renderer never reconstructs an execution path from a display name. This scope constrains the existing photo viewer and its neighbors; the legacy folder string remains recursive. Exact-directory queries bypass the root media-count fast path. Non-media files use FileDetails and no external application. All original access uses the existing read-only preview path; navigation itself is SQL-only. No schema/dependency/capability expansion.

Tests distinguish generated filesystem hierarchy evidence from synthetic catalog observations for non-text native bytes: this Mac/APFS volume refused creating invalid UTF-8 names, so no native creation support is claimed. Directory-query performance on huge/deep real libraries remains unmeasured; bounded IPC/DOM is not proof of the provisional latency targets. See ADR-0007 and M1_VALIDATION.

Header Jobs revision (2026-10-28): Settings/version remain in the sidebar Workspace actions footer; Jobs is no longer a Section or page. App supplies one JobsDropdown through the existing header slots for onboarding, Photos/Tags, Folders and Settings. It consumes the existing useFoundation status polling, ScanJobProgress, job_details and cancel commands; no backend/IPC changes. Running/cancel-requested jobs lead, recent terminal jobs are collapsed. The nonmodal native dialog is portalled to document.body to escape main-pane overflow, positioned from its opener and bounded to the viewport on resize/scroll. Escape/Close restores opener focus; outside pointer/focus closes without navigation. Details use per-entry cleanup to ignore late responses; errors stay explicit. This preserves gallery mounting, query, selection and position.

Text list uses shared native SortControl choices and a session-local Columns menu. textColumns owns the bounded allowlist/order/widths; TextList renders accessible headers/rows and indexed metadata only. Modified Date is AssetSummary.modifiedAt, not the effective capture-date fallback. The default table omits repeated provenance; optional Date Source and the inspector retain it. Name is pinned, Reset restores the six defaults, and column changes do not publish queries or clear selection. Sort changes reuse the controller/native keyset query (and its existing selection reset). The table shares a horizontal scroll surface for header/body rather than hiding later fields at narrow widths. No preference, IPC or database schema is added.

## P0 performance diagnostics

Read [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md), the [P0 checklist](PERFORMANCE_ACCEPTANCE_TESTS.md) and [ADR-0008](decisions/ADR-0008-performance-baseline.md). P0 does not approve later optimisations. Scripts use Python3 plus the pinned Node/Rust tools and generated sources only. The release catalog probe creates a fresh UUID-owned directory under .context/performance and retains its catalog/sources/manifests. It rejects unsupported sizes and requires a four-GiB free-space reserve. It never accepts a photo-library path, purges caches or modifies production catalog settings.

```sh
npm run desktop:build
node scripts/performance/packaged-sqlite.mjs
npm run perf:catalog:build
npm run perf:catalog -- --rows 10000 100000 1000000
STILLHARBOUR_REACT_PROFILE=1 npm run desktop:test:build
STILLHARBOUR_PERF_ROWS=100000 npm run perf:desktop
STILLHARBOUR_PERF_ROWS=1000000 npm run perf:desktop
python3 scripts/performance/summarize.py --catalog-run UUID_FROM_THE_CATALOG_RUN
```

Do not build while taking a quiet measurement, and do not run two native drivers together. A six-step smoke run uses STILLHARBOUR_PERF_STEPS=6; it cannot pass the60-second scenario. The embedded evaluator has an independent30-second timeout, so the seed harness submits one owned promise and polls completion with a180-second bound (600seconds for the separate million-row debug setup). The optional React profiling alias affects only desktop-test when STILLHARBOUR_REACT_PROFILE=1; it never changes production React or dependencies. Profiler actualDuration measures React render work, not browser layout/commit/presentation.

Core traces and renderer traces each stop retaining after4096 numeric observations and report drops. Core recording uses nonblocking admission;16 opaque logical filesystem-volume aliases stay separate from source/cache/catalog roles. Alias0 means unknown/overflow. SQLite pager bytes and process I/O are separate from payload bytes; no universal physical-device/transfer cap is implemented. Raw device IDs, request contents, paths, GPS and secrets are not exported. Python's500ms sampler records only numeric core/observed-descendant data, capped at4096 samples/identities, and discloses omitted XPC/GPU/short-lived children. No telemetry upload is provided.

The normal app supports `--diagnostic-sqlite` with no additional arguments. It reports the actually linked SQLite version/source/options using an in-memory connection and exits before application/user-catalog setup. The private trace/fixture command remains desktop-test-only and cannot ship in release. Rebuild the default app/frontend before release:check, which verifies the normal bundle excludes the renderer harness and test capability.

## P1 catalog-read diagnostics

The owner approved P1 after P0. Read [ADR-0009](decisions/ADR-0009-catalog-read-performance.md) and [PERFORMANCE_P1.md](PERFORMANCE_P1.md). The normal app now uses compact, versioned native page transport and a per-database-worker bounded count/bucket LRU. Regenerate/check the Rust-owned IPC types with npm run bindings / npm run bindings:check; keep the shared tests/fixtures/catalog-page-v1.json contract fixture in sync only through a reviewed wire-version change. Existing UI state stays AssetPage.

```sh
npm run perf:catalog:build
npm run perf:p1 -- --rows 10000 100000 1000000
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes boundary_neighbors_use_a_bounded_index_seek -- --nocapture
npm test -- --run src/pageTransport.test.tsx
```

P1 uses the existing independently manifested, freshly seeded owned fixtures and retains phase=P1 in baseline.json. No user catalog paths are accepted. It measures uncached summaries separately from cached summaries; reset_for_probe clears only this owned connection's derived summaries. One PRAGMA shrink_memory run resets only its SQLite pager, not the OS filesystem cache. The first/last-neighbor and exact payload measurements supplement P0; they do not rerun or replace P0 full-reopen/selection baselines. Do not compare a cached P1 summary with an uncached P0 summary without labeling that difference. Run without concurrent builds. Existing native perf:desktop and desktop:test commands exercise the real wire decoder as part of the app.

Native evidence can be labeled with STILLHARBOUR_PERF_PHASE=P1; it defaults to historical P0. The harness overwrites one bounded progress.json per owned run. Summaries must match phases:

```sh
python3 scripts/performance/summarize.py --phase P1 --catalog-run UUID_FROM_RUN
python3 -m unittest discover -s scripts/performance -p 'test_*.py'
```

The phase regression rejects cross-phase catalog input and excludes native observations from other phases. Missing/unmeasured values remain null. Preserve P0 reports; never use a later P1 native result to retroactively improve a P0 baseline.

Header overlay rendering uses the existing React DOM dependency: [createPortal](https://react.dev/reference/react-dom/createPortal) preserves the React tree while allowing the Jobs dialog to escape page overflow. No popover framework or extra polling loop is introduced.

## Common-format preview diagnostics

See [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md) for exact reproduction commands, format matrix, owned public fixtures and native test isolation. `npm run formats:build` builds the release fixture probe; `npm run formats:test -- --packaged --camera-samples` drives the real packaged worker, with independent source manifests. `npm run desktop:test:formats` runs the generated HEIC gallery/viewer case separately from `npm run desktop:test`. Build once with `npm run desktop:test:build` before those two serial invocations; the pinned driver shares a catalog within a single invocation.

The owned C11 adapter uses installed ImageIO/CoreGraphics/CoreFoundation/ColorSync frameworks and already locked cc 1.10.0. Input is bytes plus a fixed numeric RAW hint, not a path. The native format adapter is macOS-only; Windows baseline remains unchanged. Preview failure records are policy-versioned in the existing column, and compatible indexed M1 cache entries remain available offline. No user catalog migration, codec download or cache purge is required. Provisionally compiling for macOS 13 does not pass its native compatibility gate.

## Gallery balancing

[GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md) documents the bounded row-break algorithm, compact headings and anchored frame-coalesced resize behavior. Reuse photoRows for geometry; keep raw catalog/decoder work outside the renderer. Native foundation tests now record independent Python source manifests under `.context/gallery/native-<UUID>/` and capture balanced-gallery desktop/minimum states. Run desktop-test build, native suite and full check serially; do not rebuild shared dist during an active driver.

## Git publication hygiene

Commit application sources, specifications, tests, pinned lockfiles and reviewed vendor resources. Keep .context evidence/attachments, build output, installed dependencies and local settings excluded through .gitignore. .gitattributes enforces LF for owned text on both Windows and macOS regardless of core.autocrlf, while preserving vendor bytes without line-ending conversion or upstream whitespace rewriting, so helper manifests remain authoritative; Markdown hard breaks are intentional. Owned-file validation still rejects accidental whitespace. Check the staged diff before publication and use a normal push on the existing branch.

## Node download-cache cleanup

To reclaim space from shared npm/npx download caches with the installed npm11.16 toolchain, inspect the configured location, then use npm's own cleanup commands:

```sh
npm config get cache
npm cache clean --force
npm cache npx rm --force
```

These remove downloaded package/execution caches, not the project's node_modules, Node installations, source files, lockfiles or photo preview cache. Later installs and one-off npx tools may download again. Keep installed dependencies and running development processes in place when only reclaiming download-cache space; inspect cache paths for links/ownership and active one-off npx tools before cleanup. Do not substitute a broad process kill or recursive deletion of unrelated build/library data.
