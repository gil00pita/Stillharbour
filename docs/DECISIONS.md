# Stillharbour - Defaults, owner decisions and feasibility gates

These are proposed defaults. Record changes as short ADRs rather than silently changing the specification.

| Area | Default | Required gate |
| --- | --- | --- |
| Product name | Stillharbour, selected by the owner; British harbour spelling | Public trademark/domain clearance remains outstanding; see PRODUCT_IDENTITY.md |
| Application type | Local-first single-user desktop app | No hosted account service in v1 |
| Catalog | One local SQLite catalog with multiple nonoverlapping roots | Native write/locking and backup tests |
| Primary platforms | Windows 11 x64; macOS Apple Silicon | Clean-machine CI/manual gates on both |
| Minimum macOS | Set during M0 | Validate every packaged dependency; document exact minimum |
| Intel Mac | Desired secondary build, not implicit support | Same tests and codec/helper availability as ARM build |
| Desktop/UI | Tauri 2 + React/TypeScript + Vite | Real map/virtualized gallery and native IPC spike |
| Rust DB layer | rusqlite behind database actor | Migrations, concurrent reads and consistent backup |
| Metadata | Pinned bundled ExifTool | Safe argument/config behavior and clean-machine execution |
| Image decoding | image worker for baseline; adapters for RAW/HEIC | ICC, orientation, format coverage and memory limits |
| libvips | Evaluate; not mandatory | Adopt only for measured benefit and viable distribution |
| Full format promise | Per-format capabilities, not universal RAW/HEIC support | Publish tested fixtures/format and camera coverage |
| Backup | restic snapshots + rclone transport adapters | Real backup/restore for each advertised transport |
| Legacy FTP | Explicitly insecure option | Warning, credential handling and round-trip tests |
| Map provider | Configurable; no production demo tile endpoint | Provider/license/privacy/caching decision by M5 |
| Offline map downloads | Off initially | Only add with legally usable offline data/provider rights |
| Privacy | No telemetry; no cloud AI | Opt-in design before enabling external processing |
| Metadata edits | Catalog first; explicit sidecar export | Unknown-field preservation and conflict tests |
| Permanent deletion | Not provided by default | Separate approved purge design or OS-only deletion |
| Scheduling | While app/tray runs; catch up on restart | Separate platform agent before claiming fully-quit execution |
| AI | Optional later module | Model license, hardware budget, validation and privacy |
| Licensing/business model | Undecided | Review all component/binary/model licenses before distribution |

## M0 evidence, not guesswork

The first milestone must identify which capabilities can be demonstrated in its environment and which require a native machine, test server, certificate, or owner choice. It must not mark those items verified based on memory or a Linux-only build.

Outstanding public brand clearance and optional AI choices do not block a read-only library implementation. Missing safe no-replace/identity/recovery primitives block enabling original-file mutations. Missing codec distribution evidence blocks claiming that codec is included. Missing transport restore evidence blocks advertising that transport as supported.

The owner can reduce first-release scope explicitly after reviewing a gap, but Codex must not quietly redefine v1 to conceal incomplete functionality.

## Version 1.1 owner-priority update

Preservation of every file is the overriding requirement. See PRESERVATION_CONTRACT.md. Default to source-retaining organized copies; gate in-place original mutations on exact-version independent recovery evidence. Add a source-fixed readable Safe copy workflow alongside encrypted snapshots. No automatic mirror deletion, two-way source writes, permanent purge or history expiry.

Quiet transfer defaults are proposed at 10 MiB/s and one active transfer, with bounded buffers and comparison/verification I/O accounting where enforceable. These are starting values, not measured optimal settings. Unsupported helper/physical-I/O caps are disclosed. Full-library backup scope is separate from gallery visibility and includes hidden/unrecognized regular files unless explicitly excluded.

M2/M3 production mutation remains disabled until a tested recovery-evidence provider is available. M4 is split into compare, safe copy, retained-version replacement and snapshot/restore integration gates. None is marked implemented by this specification update.

## Version 1.2 owner feature extension

Adopt the six features in LIBRARY_FEATURES_ADDENDUM.md as proposed scope. Integrity reports and catalog recovery are core safety refinements. Recently added uses catalog first-added dates. Basic editor saves recipes, tags save catalog annotations, and export creates protected copies/sidecars. Initial cloud imports consume Google Takeout/Apple original exports; provider APIs are optional, native-tested and fidelity-labeled. Single-file catalogs are consistent snapshots without originals or secrets; active catalogs stay local. Mobile editing is an explicit future N5 gate, not an implied desktop capability. No safety requirement is relaxed.

## Authorised M1 implementation — 2026-10-27

The owner authorised the next phase after M0. [ADR-0002](decisions/ADR-0002-read-only-library.md) records the schema-2 migration, handle-relative scanner, bundled metadata resources, shared media-worker admission, keyset pages, virtualized views and cache protocol. [M1 validation](M1_VALIDATION.md) records actual evidence and remaining gates. No original-file mutation capability is enabled by saving managed mode.

## Owner-approved scan settings revision — 2026-10-27

New library folders default to Managed, replacing the earlier referenced default. Read only is an explicit switch that stores referenced mode; existing saved modes are retained. Managed still requires the reviewed future operation/executor/recovery gates and never makes M1 scanning write originals.

Hidden-file and hidden-folder inclusion use independent switches. “Include hidden folders” is the user-facing name for permitting traversal into them. Pattern, excluded-folder and extension lists use removable tags rather than comma-separated text; literal commas must survive valid globs/folder names. Scan date restrictions are removed from this screen and cleared on Save/Rescan so they cannot silently limit indexing. Browse date filters remain useful for the indexed collection. No schema or catalog identity migration is needed.

## Owner-approved Advanced setup revision — 2026-10-27

Replace sidebar footer prose with the actual running application version. After selection, show only the selected-folder form; cancel restores the welcome. Form/accordion expand naturally and delegate scrolling to the main pane.

Rename Scan filters to a full-width Advanced accordion. Remove Include patterns and clear its legacy bounds on Save/Rescan. New folders use the exact 33-photo-suffix File Extensions preset in CODEX_SPEC.md; existing saved scopes remain intact. Interpret the owner's ambiguous “non case-insensitive” wording as case-insensitive matching (the existing engine rule), stated during implementation: `.JPG` matches `.jpg`. Display leading dots but store canonical lowercase values. This is a discovery preset, not a format-decoder expansion; mutation/platform/color gates do not change.

## 2026-10-27 — Chronological canvas within M1

[ADR-0003](decisions/ADR-0003-photo-canvas.md) records the owner-approved justified rows, real month navigation, read-only viewer and catalog-only batch actions. This supersedes the prior gapless/no-checkbox presentation, not its safety gates. New month/neighbor/additive-tag IPC uses schema 2 without a catalog migration. Similarity analysis remains unavailable pending M3; visual Compare makes no duplicate claim. Cached viewer previews remain limited to 512 px.

On 2026-10-28 the owner explicitly confirmed completing the gallery/viewer while leaving similarity grouping unavailable. This resolves the scope question without authorising M3 analysis.

## Settings — 2026-10-28

[ADR-0004](decisions/ADR-0004-settings.md) records persistent app defaults and indexed-photo-folder location management. The owner explicitly excluded catalog/cache relocation from this request. Existing root identity/preservation gates and catalog schema 2 remain unchanged.

## Gallery side panel and bounded scroll feed — 2026-10-28

[ADR-0005](decisions/ADR-0005-filter-panel-feed.md) supersedes explicit gallery pagination within M1. Keep search/Filter above the photo canvas; query, sorting, display and availability live in a draft/apply side panel. Bidirectional catalog seeking and a three-by-240 renderer window provide scroll loading without an unbounded array. Original access remains read-only; no dependency, schema or production capability expansion.

## Inline scan settings in Settings — 2026-10-28

The owner moved existing-root Scan settings into a group on Settings itself. Reuse RootSetup/ScanOptions in an embedded presentation and existing native update-mode/scan operations keyed by root ID. The library selector and Save & Rescan / Discard scan changes are independent from application defaults; gallery links target this group. No new dialog, backend command, schema or permission is introduced. This refines [ADR-0004](decisions/ADR-0004-settings.md), not the preservation/milestone gates.

## Indexed library removal and preview cleanup — 2026-10-28

[ADR-0006](decisions/ADR-0006-library-removal-cache.md) adds explicitly confirmed catalog forgetting and indexed preview-cache clearing within M1. Backend-owned expiring/revision-bound review, scan admission, publication quiescence, descriptor-relative cache deletion and atomic catalog cleanup preserve originals and surviving libraries. Unknown cache files remain retained; schema2 and all original-operation gates remain unchanged.

## Read-only Folders explorer — 2026-10-28

[ADR-0007](decisions/ADR-0007-folder-explorer.md) replaces Folders' flat media-filter presentation with immediate-child catalog navigation, native-byte directory identifiers, bounded two-way pages and exact-directory viewer scope. Preserve the legacy recursive filter, existing preview/viewer pipeline, schema2 and all original-file preservation gates.

## P0 performance instrumentation — 2026-10-28

[ADR-0008](decisions/ADR-0008-performance-baseline.md) authorises opt-in bounded numeric traces, generated catalog/media fixtures, independent source manifests and provenance/report scripts only. No production query/schema/decoder/cache-policy/resource-scheduler change is included. The normal SQLite metadata CLI flag uses an in-memory connection; the trace/seed renderer bridge is test-only. Later performance phases and preservation gates require separate approval/evidence.

## 2026-10-28 — P1 catalog read performance authorised

The owner's “Move to next plan” instruction after the P0 report authorises P1 only: compact pages, indexed seeks and bounded cached summaries. Keep the interface, schema, dependencies, original-file access and preservation gates. [ADR-0009](decisions/ADR-0009-catalog-read-performance.md) records the transport/index/cache decisions; [PERFORMANCE_P1.md](PERFORMANCE_P1.md) records actual checks and measurements. P0 evidence remains historical. Stop after P1; P2–P4 and real-library mutations still require separate approval.

## 2026-10-28 — Jobs moves to the header

The owner replaces the dedicated Jobs page/sidebar destination with a header icon and running-jobs dropdown. Include cancellation-requested work in the active badge, retain terminal outcomes in collapsed Recent jobs, and preserve existing progress/cancel/issue-detail behavior. Use a nonmodal portalled disclosure on each screen without unmounting its workspace; retain Settings/version in the footer. Reuse existing status polling and typed job commands. No scheduler, backend/schema/IPC, original-file operation or later performance phase is authorised. Acceptance and commands are in ACCEPTANCE_TESTS.md and DEVELOPMENT.md.

## Read-only native previews — 2026-10-28

The owner authorised the common-format portion of M5. [ADR-0010](decisions/ADR-0010-native-format-previews.md) selects macOS ImageIO/ColorSync in the existing isolated bounded byte worker, fixed RAW hints, versioned negative observations and compatible M1 positive cache reuse. [FORMAT_SUPPORT.md](FORMAT_SUPPORT.md) records actual camera/OS results and limits. No new codec framework, dependency upgrade, schema migration, original mutation or automatic cache clearing. The rest of M5 and P2–P4 remain gated.

## Bounded gallery row balancing — 2026-10-28

Reuse the existing photoRows/usePhotoViewport/AssetBrowser seam and virtualizer. A bounded shortest-path row-break helper replaces the greedy gallery partition; retain64 candidate successors per photo, the720-summary feed cap and unchanged selection/order/preview safety. Up to125% target height trades small local stretching for fewer abrupt row sizes/orphans. Compact date headings and frame-coalesced resize notifications refine the existing surface. [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md) records implementation, tests and scope; this is not P2–P4 completion or a new caching/scheduling system.

## Owner-authorised progressive preview slice — 2026-10-28

[ADR-0011](decisions/ADR-0011-progressive-renditions.md) records bounded file-backed macOS JPEG/PNG/TIFF downsampling, separate tiny-thumbnail jobs and per-tier derived failures without a catalog migration. [PROGRESSIVE_PREVIEWS.md](PROGRESSIVE_PREVIEWS.md) defines requirements, resource limits and executable checks. This extends only the authorised P2 rendition slice; preservation, platform and remaining roadmap gates stay intact.


## P2 gallery/rendition continuation — 2026-10-29

The owner confirms P2 as the next stage. [ADR-0012](decisions/ADR-0012-bounded-rendition-pyramid.md) adds bounded high-DPI/viewer tiers, explicit decoded-pixel admission, indexed smaller-cache fallbacks and binary worker framing. Retain the existing virtualizer, anchors, decoder/cache safety limits, schema and dependencies. [PERFORMANCE_P2.md](PERFORMANCE_P2.md) records evidence and remaining release/platform gates. P3 scheduling and all original-preservation operations remain separate.

## Live library observations — 2026-10-29

[ADR-0013](decisions/ADR-0013-live-library-reconciliation.md) adds native invalidation/periodic reconciliation over the existing scanner and durable, namespaced change observations in the existing schema-2 journal. History is distinct from filesystem events, integrity baselines and recovery evidence. Preserve originals and all prior gates; no P3 scheduler or user-catalog migration.

## P3 shared source admission — 2026-10-29

[ADR-0014](decisions/ADR-0014-shared-source-admission.md) replaces separate media admission with atomic source-volume/CPU/memory reservations, visible preference and background aging. Keep job/maintenance gates and current helper/source safety. Scroll-aware tiny-first requests reuse existing viewport signals. Synthetic read contention is explicitly separated from the missing production backup adapter, physical topology/pacing and native NAS/HDD evidence.


## Aligned sticky date sections and cached scrolling — 2026-10-29

The owner replaces side-by-side sparse dates with separate justified rows and full-width native sticky headings. Retain complete section extents around bounded virtual rows, without overlay copies of selection controls. Compatible cache-only thumbnail lookup can reuse clear renditions during scrolling while source work remains tiny-first and constrained by ADR-0014. Remove gallery image filters and retain loaded pixels across status refreshes. [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md) records the revised product behavior, evidence and limits; earlier layout/performance records remain historical. No new framework, decoder, dependency/catalog migration or original mutation.

## P4 local evidence stage — 2026-10-29

Reuse the existing fixture/probe/native infrastructure for [bounded local stress checks](PERFORMANCE_P4.md). Consume diagnostic event segments without increasing the recorder bounds or restarting observers. Keep the compile-time release automation prohibition and distinguish release core/worker evidence from debug UI diagnostics. Production optimization requires a measured reproducible bottleneck; outstanding P3 backup/physical-endpoint/pacing and platform/presentation gates remain intact.

## P4 continuation — exact empty display reads, 2026-10-29

The next-step request continues P4's measured filtered reads. Reuse the existing page and summary-cache boundaries: an exact zero count avoids the ordered projection and empty page/timeline results share evidence within the same query/epoch. Preserve inventory-counter lag, conservative invalidation and every validation/field. [PERFORMANCE_FILTERED_READS.md](PERFORMANCE_FILTERED_READS.md) retains the release reproduction and positive controls; no schema/index/dependency/IPC or safety change.

## P4 continuation — positive timeline totals, 2026-10-30

Reuse retained matching timeline buckets as exact count evidence for pages/seeks, with checked summation and existing cache epochs/bounds/fallback. Keep unfiltered inventory semantics and avoid eager timeline work. [PERFORMANCE_POSITIVE_TOTALS.md](PERFORMANCE_POSITIVE_TOTALS.md) records the reproduced redundant scan, both request orders and the remaining filtered-projection/aggregation and wider P4 gates. This refines ADR-0009's existing read seam without a new architecture, schema, dependency or IPC surface.

## P4 continuation — selective filename projections, 2026-10-30

Use a bounded existing-index lookup only after an exact small unscoped filename count is available. Apply a transient parameterized rowid predicate only when the complete name match fits; larger or mixed scopes retain normal planning. Reject the eager-count prototype after its late-broad regression. [PERFORMANCE_FILENAME_READS.md](PERFORMANCE_FILENAME_READS.md) records policy boundaries, before/after evidence and remaining gates. This stays within the existing SQL read adapter and schema; no INDEXED BY requirement, persistent rowid identity/cache, migration or public surface is added.

## First library preview preparation — 2026-10-30

[ADR-0016](decisions/ADR-0016-initial-preview-preparation.md) records the owner's initial-ready collection and temporary progress banner. Reserve one durable initial thumbnail job before discovery; the existing bounded executor takes over afterward. Withhold pending initial previews consistently in collection queries, retain paused state across restart/reconciliation, and require an explicit resume. Unknown discovery/queued totals stay indeterminate; actual thumbnail inventories report remaining work. Legacy completed libraries, catalog/root identity, original-file safety and all wider gates remain unchanged.

## Page-scoped single-photo viewer — 2026-10-30

[ADR-0017](decisions/ADR-0017-page-photo-viewer.md) replaces the single-photo full-window modal with a page-contained nonmodal dialog, measured thumbnail expansion/reverse close and a loading-only glare. The sidebar remains usable, the covered page is inert, and existing preview/navigation/focus safety remains. One bounded cache transport retry plus explicit native revalidation addresses the reproduced permanent image-error latch without claiming repair of private originals or all cache failures. [VIEWER_PRESENTATION.md](VIEWER_PRESENTATION.md) records the bounded M1 scope and checks.

## Bounded original-byte viewing — 2026-10-30

[ADR-0018](decisions/ADR-0018-original-image-viewing.md) permits original display only through a guarded ID/revision/policy protocol, preserving source identity, read/helper budgets and explicit fallback. Remove the routine 2,048-pixel banner for actually loaded originals, put X at the right and a catalog-confirmed animated heart at the bottom. [ORIGINAL_IMAGE_VIEWING.md](ORIGINAL_IMAGE_VIEWING.md) records the static JPEG/PNG/WebP 32-MiB/32-million-pixel scope, tested/unrun evidence and residual platform/format/preservation gates. No source mutation or broad filesystem exposure.

## P4 continuation — cold filename summaries, 2026-10-30

Place filename rejection after media classification and before presence/preparation checks in the existing shared predicate builder. Retain SQLite’s planner choice: measured forced-name-index and covering-name-join alternatives regress the all-matching control. No new cache, SQL hint, index, dependency, schema/IPC or source operation. [PERFORMANCE_COLD_SUMMARIES.md](PERFORMANCE_COLD_SUMMARIES.md) records the bounded read-adapter refinement and actual evidence; ADR-0009 and all earlier P4 limits remain in force.

## Local P4 evidence closeout before M2 — 2026-10-30

The owner explicitly chooses local P4 work before M2. [ADR-0019](decisions/ADR-0019-p4-local-evidence.md) keeps a fixed total trace budget through stable preferred producer lanes, separates aliases and shortens consuming reads without hiding losses. It also records explicit catalog lifetime-lock release after SQLite closes, with a generated fork/exec regression and unchanged strict acquisition. [PERFORMANCE_LOCAL_CLOSEOUT.md](PERFORMANCE_LOCAL_CLOSEOUT.md) separates observed memory collection from unavailable whole-app ownership and preserves all historical/external gates.

## Authorized M2 organization previews — 2026-10-30

The owner continued to M2 and selected organization templates/previews as the priority. [ADR-0020](decisions/ADR-0020-organization-template-previews.md) separates bounded pure template rendering from catalog snapshots and future executable plans. [M2 preview status](M2_ORGANIZATION_PREVIEWS.md) records current relative proposals, deterministic fingerprints and explicit missing/collision issues. No destination/approval/executor is enabled; P4-06 and exact-version recovery/native preservation gates remain open. Persistent destination-aware planning and companion review are next.


## M2 destination planning prerequisite — 2026-10-31

[ADR-0022](decisions/ADR-0022-organization-destination-review.md) adds temporary native destination identity/overlap preflight, bound to picker sessions and exact catalog previews. At that slice, schema 2 remained required pending retained Windows migration snapshots. Native target checks, complete companions and execution remain pending. The prior scan performance changes remain intact.

## Retained M2 planning — 2026-10-31

[ADR-0023](decisions/ADR-0023-retained-organization-drafts.md) supplies scoped Windows application-snapshot publication, transactional schema-3 migration, immutable template versions, fingerprint-bound draft records and append-only draft journal evidence. UI review/history and JSON/CSV text carry no execution authority. All original-file guards remain closed; M2 is in progress with native targets, complete companions, capacity/fidelity, executor/import/undo and recovery acceptance still open.

## Owner-approved Windows NTFS read adapter — 2026-10-30

[ADR-0021](decisions/ADR-0021-windows-ntfs-scanning.md) records the explicit Windows-backend continuation. Use local NTFS volume GUID/file identities, bounded held-handle enumeration and file-ID opens with native name/membership checks. Baseline worker dimensions/previews preserve filesystem date provenance; unsupported roots show the blocking alert. Generic original publication/durability, non-NTFS/network support, full Windows metadata/format packaging and release/preservation gates remain separate. No persisted identity/schema migration.

## Complete direct-folder companion review — 2026-10-31

[ADR-0026](decisions/ADR-0026-confirmed-companion-review.md) adds metadata-only live folder-name inventory and append-only decisions bound to retained drafts. Full reviewed public inventory, selected exact native locators, acknowledgement and targets are retained without changing original draft digests. Limits and incomplete observations are explicit. Future executable manifests must bind explicit decision IDs and revalidate; no original mutation or execution approval is introduced.
