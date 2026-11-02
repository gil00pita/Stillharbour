# M5 implementation and acceptance evidence

Date: 2026-11-02. Request: “Finish M5”. Implementation covers offline maps, catalog organization, reversible tags, explicit keyword import and reviewed portable-catalog/XMP export. Full milestone acceptance still depends on the explicitly recorded native/platform gates below. M4 preservation authority and M6 release scope remain separate.

## User-visible behavior

Open **Map** for bundled offline geographic browsing and its keyboard-accessible photo list. Open **Catalog** for albums, validated saved searches, category relationships, reviewed exact-label tag rename/merge/remove and conflict-checked undo. The Info inspector retains captions/rejects and offers explicit observed-keyword import; removing a tag persists across rescans unless the user chooses to restore removed labels.

Catalog export creates an independently openable `.stillharbour` SQLite database. It preserves organization and read-only evidence, with roots unbound and credentials, schedules, approvals and native authority absent. It is a portable organization copy, not the existing full recovery snapshot. No in-app portable-catalog import flow is claimed.

XMP export creates new, independently verified files in a separately selected folder. The review lists exact generated names and sizes; backend approval binds their digests. Approval acknowledges private metadata/association disclosure. Supported mappings are `dc:subject`, `dc:description` with `x-default`, `xmp:Rating` (reject uses -1), and Stillharbour's rating/favorite/reject/stable-ID properties. Original XMP and other possible companions remain verbatim separate artifacts; association JSON maps them to the original relative names. Unknown namespaces/editing instructions stay in the merged packet where the supported structure permits it. Unsupported or ambiguous cases fail without source modification.

Output names are deliberately distinct from source basenames; this is explicit portable export, not an automatic editor-folder sync. No source XMP, original or existing destination is overwritten. Interrupted stages/outputs remain retained. **Export completion and retained partials** shows the latest twenty receipts; incomplete is never labelled completed and old approvals never automatically resume.

## Bounds and conflict handling

- Plans bind exact catalog revision, native folder identity/volume, picker session and app instance; expire after one hour and are single-use.
- Catalog exports admit at most 512 MiB of source pages, with a 30-second SQLite snapshot deadline. Export streams are hashed and independently read back. SQLite internal page I/O and metadata calls are not separately byte-metered; owned stream I/O uses the shared 10 MiB/s budget.
- XMP selection: at most 200 media, 32 possible companions per photo, 32,768 direct-parent entries, 1,000 artifacts and 64 MiB total. XML source/merged limits are 1/2 MiB, depth 64, 50,000 events and 128 attributes per element.
- Fresh native directory inventory includes hidden, excluded and unindexed companions. Symlink, placeholder, special-file, ambiguous variant, unsupported filename and changed-source cases block export. Sidecars receive full-byte digest checks; media receive identity/revision witnesses, not a claimed full-photo-byte verification.
- Tag previews/undo bound work to 50,000 assets, 512 categories, sixteen parent levels and 16 MiB retained review JSON. Exact case/Unicode/slash labels stay distinct; category-only history and monotonically increasing undo versions are covered.

## Reproduction and local evidence

Use only constructor-owned/generated or pinned licensed fixtures. No command here accepts a personal library as a test target. Run native suites serially and rebuild the production frontend after instrumentation.

```sh
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes,copy-workflow,backup-workflow catalog_metadata::
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes,copy-workflow,backup-workflow metadata_export::
node scripts/cargo.mjs test --locked -p stillharbour-core catalog::
npm run desktop:test:map
npm run desktop:test:metadata
npm run typecheck
npm run lint
npm test
npm run bindings:check
npm run rust:check
npm run build
npm run desktop:build
npm run release:check
```

Current checks: frontend **472/472 in 75 files** (61.80s), including explicit keyword restore consent and no stale success after failure; strict TypeScript and ESLint pass. React Doctor changed-scope scan stays at 62/100 with the same two existing settings complexity/size warnings and no new issues. Catalog migration tests pass 27/27, including 45 actual exit boundaries through schemas 2–16. The full workspace attempt passes the desktop's two unit tests; core records 487 passed / 10 failed / 15 ignored in 275.19s, with the ten inherited native-metadata/replacement failures described below. The workspace command stops at core failure; later integration targets are checked separately where recorded.

The initial export-process fixture required canonical `/private/var` paths; the first new history component assertion incorrectly expected text across a `bdi` boundary and was corrected to assert its list item. Initial compile/Clippy issues were corrected without removing preservation checks. Native attempts initially stopped on a blank map: diagnostics prove hidden document visibility and a paused animation-frame callback while the bundled worker responds. The attempted worker-format change and foreground action did not fix that condition and were removed. Neither foreground activation, temporary keep-awake nor instrumented-only disabled background throttling restored native animation frames. Those workarounds were removed; scheduling/CSP/worker remain unchanged. The separate native metadata suite uses the real photo-list alternative and asserts no GPU/foreground-map capability. The final native metadata run passes in 7.2s (`.context/m5-finish-native-metadata-final.log`). Actual native IPC creates albums/search/categories, saves Unicode captions/rejects, previews/applies/undoes tags and saves catalog plus selected-media XMP copies. Independent audit reopens the schema-17 portable database, verifies absent native/credential/schedule/approval authority, checks two retained original/merged unknown-field XMP packets, preserves the destination-only neighbor and verifies all nine source image/companion entries unchanged. Eight new output artifacts are audited across three exports. Production original-mutation capability remains false. At 660×560 the dialog content/client width is 624px and the header/Close stay in bounds. Native focus and GPU capability are explicitly **not asserted** in this occluded-host suite. Screenshots and constructor receipt are under `.context/m5-native-metadata`.

Final review also fixes undo's comparison of category graphs by stable ID rather than label-sorted display order; a rename that moved across another category previously caused an unwarranted conflict. A backend regression covers undo and undo-of-undo across that order change. The frontend keeps Undo disabled until parent refresh, current history and revision finish. Owned export workspaces are created exclusively with Unix mode 0700; the diagnostic reopens the canonical owned parent while retaining no-follow on the leaf. Final focused tag/metadata cases: **17/17** (0.67s); export/parser/actual-process-exit cases: **10/10** (2.37s), including the independent auditor and private-directory assertion. These follow the full-suite attempt above; no newer full-suite pass is claimed.

The production/package/format results are recorded below. Earlier evidence remains in [M5 map](M5_MAP.md), [metadata](M5_METADATA.md), [format support](FORMAT_SUPPORT.md) and [UI framework](UI_FRAMEWORK.md), at its original schema/build scope.

## Remaining acceptance gates

This Mac cannot certify Windows native behavior, Intel/oldest-supported macOS, clean target machines or a complete manual screen-reader review. Those formats/platforms remain explicitly unverified; an indexed RAW extension is not full-media validation. Native macOS checks use generated fixtures and the local WebView, not a signed release or clean-machine install.

Ten inherited native-metadata/replacement tests fail on this host because freshly created files carry `com.apple.provenance`, which the existing no-extra-stream preservation policy rejects. The unchanged-main reproduction is retained. No extended attribute was ignored or production policy weakened to obtain a pass. Full M4 preservation acceptance and production original-file mutations remain gated. M6 signing, release hardening, mobile/cloud import and editing remain out of scope.

## Production and packaged format checkpoint

The default production macOS `.app` build passes (153.65 MiB in the initial final-build log); its default dependency graph and frontend resource checks exclude desktop-test instrumentation, and pinned ExifTool/restic/rclone resources match their manifests. The release artifact is a local build, not a signed/clean-machine release claim. Vite still reports the existing >500 kB chunk warning; no startup or throughput claim is inferred.

Packaged format refresh `dd2a0b54-3263-46a6-86e9-f6f60f549f34` passes all eighteen expected outcomes, including generated HEIC/HEIF/TIFF/baseline/P3 cases, bounded rejections and six pinned licensed RAW samples. All eighteen source entries are independently unchanged. [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md) records scope and hashes. The executable's read-only in-memory diagnostic confirms bundled SQLite 3.54.1 and its official source ID without opening a user catalog. [DEPENDENCIES.md](DEPENDENCIES.md) records the WAL-reset/current advisory review.

Strict types/lint, binding drift and workspace/all-target Clippy with warnings denied pass again after the fixes. The final frontend suite passes **472/472 in 75 files** (61.80s). Full worker/OS network interception and manual screen-reader/focus review were not collected; earlier main-WebView/source/CSP evidence keeps its recorded boundary. Final rebuilt package SHA-256: `bc7608d31dc720f3a23824148e9d861e7aedf7f794c83f74647336a6ffe00058`. The retained-fixture run `25c57a61-1656-4700-b1dc-0d8927229c62` passes **18/18** again with the exact rebuilt worker hash and all eighteen original source entries unchanged. This rebuild includes the final keyword failure-notice fix. `npm run release:check` passes again. Final formatting and whitespace checks pass after formatting the newly expanded tag regression; no source or assertion was removed.

## PR preparation checkpoint

The complete workspace PR also includes newer notes-draft admission-limit and no-location map presentation changes. Fresh frontend checks pass **473/473 in 75 files** (37.51s), strict TypeScript/ESLint, production frontend build, release resource exclusions and unchanged React Doctor findings. Native/package receipts above retain their earlier exact build scope; this preflight does not certify fresh foreground graphics, native focus or platform acceptance. The known full-Rust preservation failures remain disclosed in the PR.
