> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# Neutral gallery workspace — 2026-11-07

The owner requested a copy of `feat/app-ui-redesign-audit`, then a professional
redesign using inline Chakra UI v3 styles and the attached neutral gallery brief.
The current branch fast-forwards to `589a531`, retaining the audit work, Structure
workflow, separate Backup/Sync pages and native safety behavior. Its name remains
`feat/app-ui-redesign-audit-v2`.

## Presentation

Cool neutrals replace the earlier charcoal/sage palette. Dark uses `#121419`
canvas, `#0e1014` sidebar and `#191c23` panels; light uses `#f8f8fa`, `#f1f2f5`
and `#e6e8ed`. The original multicolour owner logo is unchanged. `prism.*` is
reserved for small colour-tag indicators; ordinary controls and selection use
neutral semantic tokens. Existing errors retain explicit text and status roles.

Inter Variable is bundled locally, with native system fallbacks and tabular
numbers. Controls use 36 px height and 8 px corners; viewer targets remain 44 px.
The 64 px page header includes a labelled appearance toggle. Appearance follows the system
by default; explicit light/dark choices persist locally through next-themes. Chakra semantic tokens theme
the document, portal dialogs, native controls, photographic states and timeline.
The client-rendered app keeps next-themes' SSR bootstrap inert as application/json;
the provider effect applies appearance without allowing inline executable scripts
or weakening Tauri CSP. Unset appearance follows later OS changes; a saved manual
choice takes precedence. The header retains its two-state light/dark toggle.

Photo rows and virtual extents share a 6 px gutter. The header uses separate
36 px icon buttons for Display options, Filter and selection. Filter keeps an
active-count badge, accessible name and hover title. Display options opens a
right-side panel matching Filters, with Timeline/Uniform grid/Text list choices,
the thumbnail-size slider and grid image fit. Display changes remain a draft
until Apply display; Cancel, Close and Escape discard them and restore focus.
The slider stays available at compact widths and hides with image fit in Text
list. Applying display changes remembers the visible anchor and leaves the query
and selection intact. Filters contains only query/sort, folder/tag, availability
and all-results selection controls; its reset/apply behavior stays independent.
Hover/focus reveals selection and
catalog-favourite actions without moving thumbnails. A favourite fills only
after native catalog confirmation; failed writes remain visible and retryable.
Album creation and Add photos are explicit pickers: their selection controls
remain visible even with no selected photos, including when opened from a
photo context menu. Ordinary Photos retains its hover/focus reveal behavior.
The thumbnail checkbox and heart share a 36 px invisible click target and top
inset, with centred 18 px glyphs on 28 px dark translucent discs. The outer button
has no visible background, including when favourited. Portrait tiles narrower
than 86 px stack the controls along their left edge to keep targets separate. Photo-specific semantic
roles keep white glyphs and a 70% dark backing in both app appearances, increasing
to 85% on hover; contrast therefore follows the photograph rather than the app
surface. A checked box gains a fill and tick; a confirmed favourite gains a fill.
The native checkbox retains its accessible name, keyboard behavior and busy
blocks; its decorative mark ignores pointer events. Keyboard focus has a light
ring with a dark outer edge, and forced-colour styles use system colours.
The white glyph against the 70% backing over a white photograph computes to
approximately 6.75:1, above the [3:1 non-text contrast threshold](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html);
this calculation is not a full accessibility certification.
Batch actions float within the photo workspace when a selection exists, retain
keyboard navigation and wrap/scroll within a bounded area. The header selection
toggle is a 36 px checkbox icon button with a labelled hover title and pressed
state; its accessible name changes from Select to Done selecting while active.

The existing Inspector continues to show actual catalog metadata, tags and notes.
No histogram, camera/lens values, smart-album counts or additional navigation
destinations are fabricated to imitate the example. Production has no sample
photo collection. Native visual checks use the existing disposable generated
media, including explicit unavailable/unsupported cases.

## Control refinements from owner screenshots

Shared button recipes own the 36 px control minimum, including at least 36 px
width for icon buttons; primary actions no longer carry per-screen 44 px sizing
or 28 px top margins. Intentional viewer targets retain their larger geometry.
Header action groups sit together beside the appearance button, with one title/
search spacer. Fields use a softer neutral border and filled input surface;
keyboard focus retains a strong separate ring. Switches share a light thumb,
muted off track and distinct checked track in both appearances. Onboarding's
Library icon explicitly strokes its paths, matching the remaining navigation.
The Folder structure Layout type segments prefix their labels with 16 px
calendar, location-pin, tag, camera and adjustment icons. Icons inherit the
segment's semantic colour and remain decorative; text labels, radio semantics
and keyboard/disabled behavior stay authoritative.

Minimum and maximum file sizes each use Chakra's [InputGroup with an embedded
NativeSelect](https://chakra-ui.com/docs/components/input#select), following the
owner's example. The dropdown sits inside the amount field with its own labelled,
keyboard-operable control and visible indicator. Existing exact-byte conversion,
validation, unlimited blank, zero and disabled behavior remain unchanged.

The scanning artwork is bounded to 160 px wide, retains its aspect ratio, and
respects reduced motion. The date rail reserves exactly 32 px at desktop and
compact widths. Year markers and the full-height range target remain in that
lane. The rail stacks above thumbnail controls so its month label can overlay
photos without being obscured; the label appears to its left while hovering, focusing,
dragging or scrolling, and hides at rest so photos retain their space.

Failed/canceled inspections expose an explicit retry. Reopening a failed run
does not automatically repeat it. Draft scope validation and offline-root blocks
still apply; retrying baseline establishment opens settings and resets its
acknowledgement. Failed saved-report reads have a separate reload action that
does not start a scan, and failed history reads can be retried in place.

## Style ownership

All five owned application stylesheets and the StructureFormatter CSS-module
bridge are removed. Component layouts, states, responsive rules and specialized
photo/native geometry live in their owners' Chakra props or `css` objects.
Reusable variants remain in colocated recipes, registered in `src/theme/index.ts`.
`src/theme/global-css.ts` contains document defaults and native element semantics;
`keyframes.ts` owns shared animation definitions. CSS custom properties still
carry virtual coordinates and shared measurements. High-frequency computed
positions retain React `style` values or existing imperative native geometry;
they do not generate a new Emotion rule for every scroll/pointer position.
Classes remain stable for native integration, state selectors and diagnostics.

The remaining CSS imports are dependency-owned MapLibre styles and the locally
bundled Inter font faces. Neither is an application component stylesheet. The
timeline's MIT attribution moves with its styles into DateTimeline. Inter's OFL
and next-themes' MIT notices ship under `public/licenses/`.

## Validation and limits

MapLibre's bundled basemap uses concrete neutral palette values for ocean,
land and outlines. Its style parser does not resolve Chakra token expressions
or CSS variables; Chakra continues to own the surrounding controls and surfaces.
See [MapLibre color types](https://maplibre.org/maplibre-style-spec/types/#color).

Run strict types, ESLint, the frontend component/consumer suite, pinned Chakra
typegen, frontend build and release policy checks. The shared migration affects
all UI consumers, so run the generated native desktop suite, including
`tests/desktop/appearance.e2e.mjs`, control consistency, embedded size fields,
timeline navigation, page chrome, Foundation, viewer, utilities,
metadata and structure flows. The appearance case records both modes at
1180×780 and 660×560, checks native favourite persistence, header containment,
the real font/gutter and unchanged generated originals. Run native drivers and
frontend/native builds sequentially with an exclusive executable and unused
port; keep required helper resources beside a copied test binary.

Actual outcomes and failed attempts belong in IMPLEMENTATION_LOG.md. Browser or
component checks do not qualify native Windows, installed packages, screen-reader
coverage, backups or preservation gates. Both-platform CI remains required before
merge. Rust, IPC, catalog schema, root identities and mutation authorities are
unchanged by the redesign.
