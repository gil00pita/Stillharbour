# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

React renders inside a native Tauri desktop window. The product targets Windows and macOS; this value describes the interface technology, not a browser-only distribution. Mobile is separately gated.

## Stack

The owner-approved starter specifies Tauri 2, a Rust core, strict React/TypeScript, Vite and local SQLite.

## Users and purpose

People organising personal photo and video folders need to browse, understand and protect their collections while keeping originals in ordinary folders. Preservation outranks convenience, speed and space savings.

## Capabilities and constraints

docs/CODEX_SPEC.md and its preservation/library addenda are the normative product record. docs/CODEX_START_PROMPT.md authorises M0 only. The owner subsequently authorised M1: save selected native folders, scan read only, browse a persistent gallery/grid/text list, inspect metadata and save catalog-only favorites, ratings and tags. The initial screen focuses on selecting a photo library folder; developer diagnostics stay out of the interface. Selection remains explicitly unscanned until Save & Start scanning. New folders default to Managed with an optional Read only switch; both modes scan without changing originals. Selected-folder setup replaces the welcome with a naturally expanding form and full-width Advanced accordion, hidden switches and exclusion/extension tags. New folders prefill the 33-photo-extension list; there are no include-pattern or scan-date restrictions. Sidebar footer shows the running app version; browsing can still filter dates. Original-file operations, map and backup remain unavailable. No telemetry, sample library or cloud provider is enabled.

## Brand commitments

Use Stillharbour, British spelling, the descriptor Photo & Video Library, and the unchanged owner-supplied aperture SVG. Reference screenshots establish a restrained dark desktop workspace, not shippable media. Use system UI typography for native familiarity.

## Evidence on hand

The specification package, logo and two visual references predate implementation. Generated fixtures and recorded local tests provide implementation evidence; they do not establish Windows support, independent recovery, codec coverage or release signing.

## Accessibility

Keyboard operation, visible focus, semantic controls, textual status and explicit errors are required. Gallery, screen-reader and complete workflow review are later milestone gates.

## Current gallery surface — 2026-10-27

The photo canvas is an Experience surface inside the Operate shell: aspect-preserving justified rows with 4 px gutters and day headings, a fixed filename search, collapsible navigation and a real year/month rail. Hover/focus selection, ordered ranges and catalog-only batch notes support work without covering the images at rest. Timeline, Uniform grid and true Text list share query/selection/anchor. The immersive viewer uses clearly limited cached previews with zoom/pan, navigation, optional filmstrip/Info and scroll/focus return. Compare is visual inspection of 2–4 explicit selections, never proof of duplication. Similarity analysis remains an M3 gate; originals stay read-only. This latest brief supersedes the earlier gapless/no-checkbox gallery request.

## Settings surface — 2026-10-28

Settings is an Operate surface for choosing indexed photo folders and persistent browsing/new-folder defaults. Library locations distinguish existing native root availability from saved paths; changing location reuses identity-checked relink, while a different photo folder uses Add photo folder and explicit setup/scan. Default library/view/density/fit/sort/navigation and new-folder mode/Advanced filters are saved explicitly. Existing roots and originals stay unchanged by defaults. Catalog/cache relocation is outside this owner-confirmed scope.

## Gallery refinement — 2026-10-28

The owner's annotated screenshot condenses the resting canvas to fixed filename search and Filter above the photos. All library/query/sort/display/facet/availability controls and result selection live in a right-hand draft/apply panel; folder management remains in Settings. Scroll loading replaces the page footer while retaining at most 720 summaries in three native 240-row chunks. Selection actions appear contextually. Existing chronology, read-only previews, explicit unavailable states and similarity gates remain intact.

## Inline scan settings — 2026-10-28

Settings contains an inline Scan settings group for registered photo folders, with a library selector and explicit Save & Rescan / Discard scan changes. Gallery links target the same group. Root mode/filters remain distinct from saved application/new-folder defaults; opening the page or choosing a root never starts a scan. Reuse the existing read-only scan workflow, with independent feedback and ordinary page scrolling.

## Library removal and preview cleanup — 2026-10-28

Settings offers Remove library for each indexed folder and Clear preview cache for all libraries. An inline review precedes confirmation, names the scope and warns about forgotten catalog notes/history. Originals/sidecars stay in place. Cache-only cleanup preserves notes and settings; offline previews require originals reconnected to regenerate. Cleanup is limited to index-owned previews, with stale/scan/failure handling and no recursive original-folder deletion.

## Folders explorer — 2026-10-28

Folders is a read-only explorer of indexed library roots and immediate child folders/files. Back/Up and breadcrumbs navigate the hierarchy; Icons shows named folders and media thumbnails, Details is text-only. Photos open the existing exact-folder viewer; other indexed files show catalog details. Scan settings determine indexed coverage, so this is not a live OS filesystem browser. Offline/partial/retained states and source preservation remain explicit.

Header Jobs (2026-10-28): a Jobs icon in the header opens a nonmodal dropdown on onboarding, Photos, Tags, Folders and Settings. Its count includes running and stopping scans; active jobs lead with real progress, counts, cancellation and issue details, while terminal history stays under collapsed Recent jobs. Opening Jobs preserves the current page, query, selection and scroll position. Settings remains in the bottom navigation footer above the runtime version; the main menu and page content scroll independently. The separate Jobs page and scan banner are removed.

Text list defaults to actual Modified Date without repeated provenance subtitles. Readers can choose catalog columns and existing native sort orders; Name remains available and narrow windows scroll across selected fields. Choices survive view switches for the current library screen, without changing saved defaults or originals.

## Balanced gallery refinement — 2026-10-28

Keep the established photo-first canvas and dark native shell. The owner-requested engineering refinement balances alternative justified row breaks in the bounded retained window, makes panorama presentation less cramped, keeps sparse final rows restrained, and uses compact fully named date captions when space is narrow. Resize updates coalesce per frame while retaining the visible photo anchor. This refines existing browsing, not preservation workflows or the deferred similarity/cache/scheduling scope.

## First scan preview preparation — 2026-10-30

Owner first-scan revision (2026-10-30): the initial Photos/Tags collection waits for generated tiny previews or explicit unavailable outcomes, while raw catalog discovery remains early. A temporary single-row top banner shows remaining media, an inline progress bar and stop/pause; unknown totals read “Remaining unknown”. Details stay in Jobs, with unavailable outcomes and paused/error feedback visible when needed. Pausing/restarting retains the indexed collection and requires explicit resume; existing completed libraries and image-free Text list behavior remain intact. This supersedes the earlier removal of scan banners only for active/paused preparation; header Jobs and its history remain unchanged. Details: docs/INITIAL_LIBRARY_PREPARATION.md and ADR-0016.

## Single-photo viewing — 2026-10-30

Open photos within the page while keeping navigation usable. Expand from the thumbnail, close with a reverse transition from X/Escape, and show a loading-only glare. Keep explicit cached-preview failures and bounded recovery; this does not certify original integrity or authorize original changes. See docs/VIEWER_PRESENTATION.md.

## Original viewing and favorites — 2026-10-30

Single-photo viewing loads actual original bytes for supported static JPEG/PNG/WebP images within the bounded 32-MiB/32-million-pixel envelope, retaining a clearly labelled preview fallback for offline/unsupported/error cases. The routine cached-preview banner is removed for loaded originals. X is at the top right; the bottom heart fills after a confirmed catalog favorite and respects Reduced Motion. See docs/ORIGINAL_IMAGE_VIEWING.md; this is not broad format, integrity or archival-fidelity certification.

Viewer controls follow-up (2026-10-30): a floating icon bar overlays the photo and groups Zoom out/in, Fit, the favorite heart immediately before Info, and Filmstrip, retaining accessible tooltips, the slider and percentage. Keep the photo/menu visible and all existing source/favorite safety behavior.

HEIC viewer fix (2026-10-30): detected HEIC/HEIF still-image originals also open through the guarded route on compatible macOS webviews, within the existing 32 MiB/32-million-pixel limits. Preserve unchanged source bytes, loading/error fallback and source safety. No conversion, rescan or cache purge is required. See docs/ORIGINAL_IMAGE_VIEWING.md; this does not imply support for every HEIC variant or older webview.

## Organization template previews — 2026-10-30

M2 begins with a catalog-only preview from the Photos selection toolbar for 1–240 individually selected photos. Preserve the existing Operate shell, system type, neutral surfaces and protected dialog focus. Editable session presets/custom templates show exact relative proposals, missing-date policy, conflicts and unconfirmed companion candidates. Destination checks, saved/versioned templates, export, approvals and all copying/original operations remain unavailable. See docs/M2_ORGANIZATION_PREVIEWS.md for behavior and gates.

## Restrained polish — owner approval, 2026-10-30

Retain Stillharbour's current photo-first brand and layouts while making shared controls, spacing and supporting text consistent. Use neutral secondary actions, 36px standard fields/selects, 44px primary/viewer targets, native system type, readable help text and restrained confirmed-favorite feedback. Library names lead over UUIDs in organization previews; identities remain available for inspection. No backend capability, milestone or original-file permission changes. See docs/INTERFACE_POLISH.md.
