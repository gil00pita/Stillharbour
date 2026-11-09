> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

---
name: Stillharbour
description: Cool neutral desktop photo workspace with a preserved multicolour identity
colors:
  neutral-50: '#f8f8fa'
  neutral-100: '#f1f2f5'
  neutral-150: '#e6e8ed'
  neutral-200: '#d8dbe2'
  neutral-300: '#b4b9c5'
  neutral-400: '#969cab'
  neutral-500: '#626978'
  neutral-600: '#4b5260'
  neutral-700: '#292d36'
  neutral-800: '#20232b'
  neutral-850: '#191c23'
  neutral-900: '#121419'
  neutral-950: '#0e1014'
  prism-yellow: '#F5CD2A'
  prism-orange: '#F78B24'
  prism-pink: '#FF1E56'
  prism-violet: '#5D24E6'
  prism-blue: '#1D84D8'
  prism-green: '#7AC724'
typography:
  headline:
    fontFamily: '"Inter Variable", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: 'clamp(24px, 3vw, 31px)'
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: '-0.02em'
  title:
    fontFamily: '"Inter Variable", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: '18px'
    fontWeight: 600
    lineHeight: '24px'
    letterSpacing: '-0.02em'
  body:
    fontFamily: '"Inter Variable", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif'
    fontSize: '14px'
    lineHeight: 1.5
  control:
    fontSize: '13px'
    fontWeight: 500
    lineHeight: '20px'
  label:
    fontSize: '12px'
    lineHeight: '16px'
rounded:
  photo-grid: '4px'
  control: '8px'
  panel: '12px'
spacing:
  photo-gutter: '6px'
  compact: '8px'
  group: '16px'
  section: '24px'
  major: '32px'
  control: '7px 12px'
components:
  button-primary:
    backgroundColor: '{colors.neutral-200}'
    textColor: '{colors.neutral-950}'
    rounded: '{rounded.control}'
    padding: '{spacing.control}'
    height: '36px'
  button-primary-light:
    backgroundColor: '{colors.neutral-800}'
    textColor: '{colors.neutral-50}'
  button-secondary:
    backgroundColor: '{colors.neutral-850}'
    textColor: '{colors.neutral-100}'
    rounded: '{rounded.control}'
    padding: '{spacing.control}'
    height: '36px'
  button-secondary-light:
    backgroundColor: '{colors.neutral-100}'
    textColor: '{colors.neutral-900}'
  field:
    backgroundColor: '{colors.neutral-900}'
    textColor: '{colors.neutral-100}'
    rounded: '{rounded.control}'
    padding: '{spacing.control}'
    height: '36px'
  field-light:
    backgroundColor: '{colors.neutral-50}'
    textColor: '{colors.neutral-900}'
  navigation-current:
    backgroundColor: '{colors.neutral-700}'
    textColor: '{colors.neutral-100}'
    rounded: '{rounded.control}'
    padding: '12px'
  navigation-current-light:
    backgroundColor: '{colors.neutral-150}'
    textColor: '{colors.neutral-900}'
  scan-list-chip:
    backgroundColor: '{colors.neutral-700}'
    textColor: '{colors.neutral-100}'
    rounded: '{rounded.photo-grid}'
    padding: '0 0 0 8px'
  settings-section:
    textColor: '{colors.neutral-100}'
    padding: '32px 0'
  selected-actions:
    textColor: '{colors.neutral-100}'
    rounded: '{rounded.panel}'
    padding: '16px'
---

# Design System: Stillharbour

## Overview

**Creative North Star: "Photo canvas inside the native workspace"**

Stillharbour is an Operate workspace: photographs lead, while cool neutral surfaces, compact Inter text and precise controls support browsing and review. The unchanged multicolour owner aperture is the identity; ordinary actions and selected states stay neutral. First-run setup remains spacious, while galleries, utilities and Settings use the density their tasks require.

This code-led refresh records the delivered 2026-11-07 redesign. It supersedes conflicting dated charcoal/sage, system-only typography, four-pixel gutter, header and navigation instructions from the previous DESIGN.md. Functional contracts and preservation gates remain in the product documentation. Source authority is `src/theme/` and the owning components; [NEUTRAL_UI.md](docs/NEUTRAL_UI.md) records implementation scope and validation limits. Native gallery captures in `.context/redesign-native/screenshots/` show both appearances at 1180×780 and 660×560; generated media and preparing-preview states are test evidence, not product artwork or a bundled library.

**Key Characteristics:**

- Photographs and the unchanged owner mark carry colour; interface state uses cool neutrals.
- Bundled Inter Variable and tabular numbers keep compact controls and data aligned.
- Six-pixel gallery gutters, real date context and restrained final rows preserve image rhythm.
- Loading, unavailable, offline, selected and failed states remain explicit.
- Chakra semantic tokens cover document, native controls and portalled surfaces in both appearances.

## Colors

Cool neutral layers frame the collection. The frontmatter records the actual primitive scale, not the illustrative values in the supplied brief. `src/theme/semantic-tokens.ts` assigns these primitives by appearance:

| Role                          | Light       | Dark        |
| ----------------------------- | ----------- | ----------- |
| Canvas / toolbar / input      | neutral-50  | neutral-900 |
| Sidebar                       | neutral-100 | neutral-950 |
| Panel / surface               | neutral-100 | neutral-850 |
| Hover / navigation / selected | neutral-150 | neutral-700 |
| Primary text                  | neutral-900 | neutral-100 |
| Secondary text                | neutral-600 | neutral-300 |
| Muted text                    | neutral-500 | neutral-400 |
| Divider                       | neutral-200 | neutral-700 |
| Field boundary                | neutral-500 | neutral-500 |
| Primary action                | neutral-800 | neutral-200 |
| Primary action text           | neutral-50  | neutral-950 |
| Focus ring                    | neutral-600 | neutral-300 |

Primary actions are neutral inversions. There is no ordinary blue or sage action accent. `prism.*` supplies small colour-label indicators; its presence does not establish additional navigation or smart-album features. Error and danger surfaces also use neutral semantics, with readable messages, icons and status roles.

**The Neutral Frame Rule.** Keep ordinary chrome, focus and selection neutral; reserve prism hues for small colour-label indicators and preserve the original logo fills.

**The State Has Words Rule.** Selection, availability, job progress and errors need textual or semantic states alongside colour.

## Typography

Bundled **Inter Variable** supplies body and headings, with the native system fallbacks recorded in the frontmatter. It is self-hosted rather than fetched at runtime. Tabular numbers apply at the document root. Use the title, body, control and label roles above; onboarding retains its balanced responsive headline. Page titles use the shared title role; metadata values remain stronger than their labels. Date/count captions and version text use compact supporting sizes where their components require them.

The header identity shows Stillharbour, the native runtime version and “Your media library organizer.” Keep the mark unchanged at its existing desktop/compact sizes. Paths and raw metadata wrap; filenames truncate within constrained rows and preserve bidirectional isolation. User text remains selectable where needed, including errors; chrome titles/navigation retain their existing selection behavior.

## Layout

The Tauri window defaults to 1180×780 with a 660×560 minimum. A 240px rail narrows to 184px at the existing 850px media boundary or collapses to a 72px icon rail. The document stays fixed; the main pane and navigation scroll independently. Jobs, Settings and Collapse remain anchored in the footer while long navigation labels truncate without changing row height.

PageHeader and the brand share a fixed 64px row. Titles, flexible filename/date/place search and right-aligned actions share 36px control geometry. Desktop headers use 24px horizontal insets and a 136px title lane where search exists; the 760px compact rule relaxes the title lane and uses 16px insets. Appearance remains reachable. The thumbnail scale disappears below 1024px; the existing Filter display controls remain available. The theme’s general breakpoints are recorded in the sidecar; component media rules retain their observed boundaries.

Layout padding follows the existing 8/16/24/32px rhythm with explicit compact exceptions. Timeline and Grid use six-pixel horizontal and vertical gutters, including virtual row extents. Justified rows preserve aspect and chronological order; final rows stay left-aligned and restrained. Day headings derive from real query dates. Uniform grid retains filenames and Fit/Crop options; Text list uses image-free 54px rows and a shared horizontal scroll surface for selected columns. The bounded feed keeps at most three native 240-item pages with visible rows and overscan mounted; density/view changes preserve the visible asset anchor.

A persistent 96px chronology lane reserves space for real year/month labels at desktop and minimum widths. It uses actual filtered chronology and ordinal navigation, with keyboard/pointer seeking and explicit undated handling. No fabricated distribution stands in for catalog data.

Filters is a right-side full-height dialog with independent body scrolling and reachable header/footer actions. Opening creates a draft; Apply commits, Close/Escape discards. Search in the header applies directly. Settings remains a flat divided page with 40px desktop insets, compact 20px/16px insets and natural page scrolling through expanded forms. Selected-folder setup and inline scan settings expand without inner scrollbars. Existing amount/unit fields retain their shared baseline and two-column layout.

Folders uses Back/Up and horizontally scrolling breadcrumbs over bounded immediate-child catalog pages; Icons and image-free Details retain their distinct presentations. Viewer opens in-page while navigation remains usable, restores gallery position/focus on close, and keeps floating controls and optional filmstrip/Info. Narrow layouts retain reachable controls and wrapping metadata.

## Elevation & Depth

Flat tonal layers and thin dividers carry the shell, Settings and utility workspaces. Floating controls and selection actions gain restrained depth: BatchToolbar uses the appearance-aware elevated surface, 12px backdrop blur and `0 8px 24px rgb(0 0 0 / 16%)`; ViewerControls uses `0 6px 24px rgb(0 0 0 / 0.28)`. Portalled dialogs preserve their existing dimming, focus and dismissal contracts. Do not turn these local floating treatments into shadows on every panel.

Visible two-pixel focus outlines remain distinct from selected/pressed fills. Shared recipes use `brand.focusRing`; existing native/photographic controls retain semantic neutral focus styling. Buttons use 140ms colour feedback; thumbnail actions reveal over 150ms. Existing preview transitions and confirmed-favourite feedback respect reduced motion and hidden-document behavior. A favourite fills only after native catalog confirmation.

## Shapes

Regular fields and actions use gently rounded eight-pixel corners. Floating panels use twelve-pixel corners. Timeline tiles and preview wells use four-pixel corners, as do Grid preview wells. Native checkboxes remain compact sixteen-pixel inputs, and switches retain pill tracks with circular thumbs. Viewer targets stay 44px; standard controls are 36px. Preserve actual component geometry rather than rounding all photographic surfaces to match buttons.

## Components

**Buttons and fields.** Shared Button defaults to a flat neutral secondary fill despite the recipe’s `outline` variant name; solid actions invert foreground/background. Disabled controls use 55% opacity. Labels, help, validation and visible focus remain attached to real fields. Native selects and checkboxes share appearance-aware defaults. Do not replace meaningful disabled reasons with fake success.

**Appearance.** Page headers expose a named light/dark toggle. next-themes follows the system appearance by default, including later OS changes. Explicit light/dark choices persist locally under `stillharbour-appearance` and take precedence. Portals share semantic appearance. The header exposes a two-state manual toggle; an unset preference follows the OS. The client-rendered provider keeps its SSR bootstrap inert under the existing Tauri CSP.

**Navigation and Jobs.** Photos, Folders, Albums / Tags and Map lead the saved-library rail. Utilities groups Structure Formater, Duplicates, Integrity, Storage, Backup and Sync. These destinations reflect actual app routes, not a list inferred from the brief. Jobs opens a full main-pane page with Back while preserving the preceding page mounted, hidden and inert. Active-job counts are real. Collapsed icons retain accessible names/current states. Runtime version belongs beside the wordmark, not a fabricated footer label.

**Photo tiles and selection.** Hover or keyboard focus reveals the separate selection checkbox and catalog-favourite action without shifting the image. Active selection keeps checkboxes visible. Checked/favourite states remain visible and semantic; selected tiles have an inset neutral ring. Favorite failures appear as bounded overlay alerts with retry through the action. Right-click/Shift+F10 opens the existing photo menu; Select and Add to album retain their real catalog behavior. No decorative tile dots are needed. Day selection means shown photos for that date; an all-results selection is a distinct frozen native query scope.

**Batch actions.** A selection reveals a floating, wrapping toolbar within the photo workspace, inset 24px and capped at half its height with internal scrolling. Existing comparison, favourite, tag, album and preview actions keep their admission rules and keyboard navigation. A disabled action remains honestly disabled. Clearing selected IDs and leaving selection mode remain distinct operations.

**Inspector and viewing.** Show actual catalog metadata, captions, ratings, tags and explicit provenance. Preserve optimistic writes and unsaved drafts. Never fill missing EXIF fields or a histogram to imitate a reference. The viewer’s bounded supported-original route and labelled cached fallback retain their source/error limits; visual Compare is inspection of explicit selections, not proof of duplication. Floating zoom/favourite/Info/filmstrip controls retain accessible names and tooltips.

**Setup and Settings.** Choosing a folder is separate from saving and scanning. Managed/Read only, independent hidden options, removable exclusion/extension tags and exact decimal size inputs retain the existing native read-only scan workflow. Saved browsing/new-folder defaults do not rewrite existing roots. Change location reconnects identity; removal/cache cleanup uses reviewed scope, explicit note-loss warnings and focus return. Preserve originals/sidecars wording and completed-operation feedback even if a later refresh fails.

**Catalog, Albums and errors.** Albums / Tags owns Catalog and Saved plans; actual virtual albums, saved filters and tag history retain their catalog contracts. Catalog organization and dialogs keep fixed headings/actions, independently scrolling content and narrow stacked comparisons. The bottom-centred persistent error toast wraps/selects the full backend message, supports explicit dismiss/reconnect and adds no layout height; field validation, job outcomes and blocking recovery stay at their decision points. Query/directory failures must not become empty results.

**Utilities and Structure.** First-use utility setup, real read-only scan progress, saved/history reports and explicit unavailable evidence remain distinct. Duplicates uses actual cached comparisons and illustrated matching choices; Integrity and Storage keep their specific findings. Backup and Sync are separate pages whose execution/restore gates are unchanged. Structure Formater operates over the indexed library: five category tabs, illustrated rules, custom path chips and separate filename/missing-date/case choices. Chip drag, keyboard and click alternatives remain accessible. Editing/review/completion preserve focused stage headings, bounded results, explicitly labelled simulation and disabled actual restructure. Existing examples stay labelled illustrative source data. Technical detail is disclosed where needed, not added as chrome.

**The Owned Styles Rule.** Keep component styling in Chakra props, css objects and colocated recipes; keep global theme styles limited to document/native defaults and shared keyframes.

The only imported CSS is dependency-owned MapLibre styling and bundled font faces. Preserve dynamic virtual/pointer coordinates in existing React style values or imperative geometry, plus stable integration/state classes. [PRODUCT.md](PRODUCT.md), specifications and dated evidence govern capability claims; this document supplies no new mutation or release authority.

## Do's and Don'ts

### Do:

- **Do** preserve Stillharbour’s British spelling and the unchanged owner SVG.
- **Do** use semantic neutral tokens across both appearances, including portals and native controls.
- **Do** keep six-pixel gallery geometry and virtual extents in agreement, preserving the visible photo anchor.
- **Do** preserve keyboard focus, explicit selection scopes, retryable failures and return focus after dialogs/viewing.
- **Do** distinguish catalog edits, cached previews, accessible originals and independently proven recovery.
- **Do** reuse shared control recipes and keep component-specific styles beside their owner.

### Don't:

- **Don’t** restore the superseded sage palette, system-only typography or four-pixel gallery gutters.
- **Don’t** recolour the logo, use reference screenshots as shipped assets or invent photographs, histogram values, metadata or smart-album counts.
- **Don’t** present missing, offline or failed observations as successful empty results.
- **Don’t** infer duplicate equality, original integrity or recovery from visual comparison or cached previews.
- **Don’t** enable original-file operations, restore or release claims through a presentation change.
- **Don’t** add application CSS files or generate Emotion rules for every scroll/pointer coordinate.
