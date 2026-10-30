# Stillharbour - M1 - Read-only library vertical slice

## Objective

A person can select base/additional folders, index them without modifying originals, and browse a persistent chronological gallery or a true text-only list.

## Prerequisites and references

M0 decisions and evidence; docs/CODEX_SPEC.md sections 6-8, 16-20; acceptance AC-002 through AC-022.

## Work

Implement root registration, read/write-mode storage, nonoverlap checks, volume identity and relink flow. New roots default to Managed per the owner’s 2026-10-27 settings revision, with a Read only switch for referenced mode. Preserve existing saved modes. Both modes only scan/read originals in M1. Store catalog/cache locally and keep app-managed directories out of scan scope.

Implement the shared filter model, streaming enumeration, batched database records, metadata extraction, revision-aware thumbnail cache and background job progress/cancellation. Integrate real Rust and filesystem code; no production mock catalog. Handle unreadable files, links, offline volumes and placeholders explicitly.

Create the screenshot-inspired shell with Photos, Folders, Map placeholder with an honest disabled reason, Albums/Tags, Utilities and Jobs access. Build the real gallery, uniform grid and no-thumbnail text list, date provenance, stable keyset pagination, date-bucket navigation, inspector, and root/name/date/size filters.

The latest owner canvas brief supersedes the earlier gapless/no-checkbox presentation: justified 4 px rows with date headings, hover/range selection, fixed filename search, collapsible navigation, actual month seeks, density controls, modal cached-preview viewer and explicit 2–4-photo Compare. Retain a true no-thumbnail Text list. Batch tags merge into existing catalog tags; all actions remain catalog-only. Similarity analysis and original edits/export stay separately gated. See docs/decisions/ADR-0003-photo-canvas.md.

Display partial scan results early. Preserve filter/selection/anchor on view changes. Keep cached browsing after disconnection and implement reconciliation on reconnect/restart. Add initial catalog favorites/ratings/tags without writing originals.

Owner Settings revision (2026-10-28): provide persistent browsing/new-folder defaults and indexed photo-folder location controls using the existing native picker, identity-checked relink and explicit registration/scan flow. Existing root IDs/mode/filters remain intact; defaults never rescan roots. Keep preferences app-owned, typed, bounded and versioned, with explicit failed-load/save and coherent Discard states. Settings uses its own main-pane scrolling while navigation stays fixed. Catalog/cache relocation remains outside this confirmed scope; see docs/decisions/ADR-0004-settings.md.

## Exclusions

No original-file rename, move, deletion, metadata write or import execution. A visible future action is disabled with a reason. Do not wire it to simulated success.

## Gate

Pass applicable AC-002 through AC-022 with fixture checks proving original bytes/names/sidecars unchanged. Record warm/cold performance observations and unsupported formats. Validate gallery/list and offline states on both target OSes where available; missing environments remain explicit. Stop before M2 mutation work.

## Version 1.2 integration

N1 extends this catalog/scanner with Recently added and authoritative inventory reports; N2 extends catalog save/open. Preserve stable identity/date provenance now. Read docs/LIBRARY_FEATURES_ADDENDUM.md.

## Owner gallery refinement — 2026-10-28

Implement the annotated condensed canvas: search/Filter header, modal side panel for all query/sort/display/availability controls, contextual selection toolbar and bounded bidirectional scroll loading without a pagination footer. Follow [ADR-0005](../docs/decisions/ADR-0005-filter-panel-feed.md) and the matching acceptance criteria. Native fixture rows are synthetic unavailable catalog observations; originals stay read-only. This does not authorize M2/M3 or close Windows/accessibility/performance gates.

Owner inline Settings refinement (2026-10-28): embed saved-root scan controls in Settings itself, with a registered-root selector and independent Save & Rescan / Discard scan changes. Gallery links navigate to the group rather than replacing Settings with setup. Reuse the existing native read-only workflow; keep app defaults, error handling, stable IDs and preservation gates independent.

Owner removal/cache refinement (2026-10-28): add explicitly confirmed root-catalog forgetting and all-library indexed preview cleanup in Settings. Follow ADR-0006 for bounded native approval, scan/worker exclusion, descriptor-relative app-cache ownership, atomic catalog cleanup and failure recovery. No real user library/cache is a test fixture; this never authorizes original-file deletion or M2/M3.

Owner Folders refinement (2026-10-28): implement immediate-child explorer navigation over registered roots/catalog observations, with Back/Up/breadcrumbs, Icons/Details, bounded native-byte cursor pages and exact-folder viewer scope. Reuse the existing media pipeline, preserve legacy recursive filters and keep all original operations read-only. Follow ADR-0007 and matching acceptance criteria; no real-library scan is a development test.

Owner Text list refinement (2026-10-28): default to indexed Modified Date without repeated provenance subtitles. Provide session-local optional catalog columns/reset and existing native sort choices/header shortcuts. Preserve read-only behavior, bounded paging, image-free text mode and shared horizontal header/cell alignment at narrow widths. Effective capture fallbacks remain distinguishable through optional Date Source and Inspector.

## P1 catalog read follow-up — 2026-10-28

The owner approved the next bounded performance phase after P0. Implement compact versioned display pages, existing-index plan requirements for simple inventory seeks, and connection-owned bounded count/timeline caching; preserve existing gallery/list/viewer/selection behavior and source read-only access. No schema/dependency migration, startup-validation reduction, P2–P4 work or production mutation is included. Acceptance: P1-01–P1-07 in docs/PERFORMANCE_ACCEPTANCE_TESTS.md. Actual checks, measured/unrun scopes and the next gate are in docs/PERFORMANCE_P1.md; architecture in ADR-0009.

Owner Jobs refinement (2026-10-28): remove the Jobs route/sidebar entry and show a header Jobs icon with active count and nonmodal running-jobs dropdown on each screen. Reuse real status/progress, ID-only cancellation and on-demand issue details; retain terminal history under collapsed Recent jobs. Preserve gallery mounting/query/selection/scroll and keyboard/outside dismissal. Keep Settings/version in the footer and all preservation/performance gates unchanged. See the matching header-Jobs acceptance criteria.

Owner gallery balancing refinement (2026-10-28): use the linked Google Photos article to refine row breaks in the existing bounded gallery, improve narrow date captions and coalesce anchored resize updates. See docs/GALLERY_REFINEMENT.md and its acceptance criteria. Preserve source read-only access, chronology/selection,720-summary feed cap, existing virtualizer, viewer return and text-only behavior. No whole-catalog renderer, new preview cache/scheduler, P2–P4 completion or later milestone expansion.

Owner live-library refinement (2026-10-29): automatically invalidate/reconcile registered roots for external changes and expose durable, bounded Library changes history in Jobs. Follow [LIVE_LIBRARY_UPDATES.md](../docs/LIVE_LIBRARY_UPDATES.md) and [ADR-0013](../docs/decisions/ADR-0013-live-library-reconciliation.md). Reuse the safe scanner and existing schema-2 journal, retain offline/incomplete scopes and unambiguous move identity, hide confirmed missing normal entries and preserve live feed anchors. No original-file operation, schema migration or P3 implementation; native platform/fallback limitations remain explicit.

Owner gallery follow-up (2026-10-29): separate justified rows by date with aligned full-width sticky headings, remove filtered-thumbnail seams and reuse loaded/cached clear pixels during scrolling. [GALLERY_REFINEMENT.md](../docs/GALLERY_REFINEMENT.md) records scope/evidence; preserve original-file access, resource bounds, text-only/offline states and all outstanding platform/preservation gates.

Owner initial-scan revision (2026-10-30): publish raw catalog observations early but introduce Photos/Tags collection entries after tiny-preview publication or explicit unavailable outcomes. Add a temporary top preparation banner with truthful discovery/preview phases, known remaining counts, stop/pause and durable resume. Preserve completed libraries, bounded feeds, image-free Text list, source guards and milestone gates. Follow [INITIAL_LIBRARY_PREPARATION.md](../docs/INITIAL_LIBRARY_PREPARATION.md) and ADR-0016; this supersedes immediate first-gallery placeholders and the earlier blanket removal of scan banners.

Owner viewer revision (2026-10-30): single-photo viewing expands within the page while the sidebar remains usable. X/Escape reverse the expansion; image/background clicks do not dismiss. Add loading-only glare with textual/reduced-motion/hidden states and bounded cached-request retry plus explicit revalidation. Preserve preview safety, filters, navigation, annotations, text-only behavior and return focus/scroll. See [VIEWER_PRESENTATION.md](../docs/VIEWER_PRESENTATION.md) and ADR-0017; supersedes whole-window single-photo modality, with all milestone/preservation gates retained.

Owner original-viewing revision (2026-10-30): use a guarded backend original-byte route for supported static JPEG/PNG/WebP within 32 MiB/32 million pixels, preserving identity/revision/resource checks and explicit preview fallback. Remove the routine cached-preview banner for loaded originals, put X on the right and a confirmed catalog-backed animated heart at the bottom. Follow [ORIGINAL_IMAGE_VIEWING.md](../docs/ORIGINAL_IMAGE_VIEWING.md) and ADR-0018. No original mutation, renderer path authority, broad format claim or later-milestone expansion.

Owner floating-controls revision (2026-10-30): group zoom out/in, Fit, Info and Filmstrip as labelled icon actions in a floating bar over the photo, with the favorite heart immediately before Info. Keep the overlay outside the pan/scroll canvas; stack the bounded scrolling Info panel below the photo/filmstrip at narrow widths. Preserve minimum-window/panel access, zoom state, keyboard/focus behavior and existing original-viewing limits. See docs/ORIGINAL_IMAGE_VIEWING.md; no milestone expansion or native/source changes.

Owner HEIC viewer fix (2026-10-30): remove the original-format restriction causing valid HEIC images to fall back. Admit detected HEIC/HEIF still-image bytes through the existing guarded macOS route on compatible WebKit; preserve all source/helper/32 MiB/32-million-pixel and fallback limits. Require full-size native pixels, exact packaged bytes and unchanged generated sources; no source conversion, schema/dependency change or cache purge. Follow ORIGINAL_IMAGE_VIEWING.md and ADR-0018's follow-up; other milestones/platform/preservation gates remain separate.
