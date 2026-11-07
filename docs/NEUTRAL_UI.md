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

Photo rows and virtual extents share a 6 px gutter. The header's thumbnail scale
uses the existing compact/comfortable/large display state and remembers the
visible anchor. At widths below 1024 px it hides to protect search space; the
existing Filter controls remain available. Hover/focus reveals selection and
catalog-favourite actions without moving thumbnails. A favourite fills only
after native catalog confirmation; failed writes remain visible and retryable.
Batch actions float within the photo workspace when a selection exists, retain
keyboard navigation and wrap/scroll within a bounded area.

The existing Inspector continues to show actual catalog metadata, tags and notes.
No histogram, camera/lens values, smart-album counts or additional navigation
destinations are fabricated to imitate the example. Production has no sample
photo collection. Native visual checks use the existing disposable generated
media, including explicit unavailable/unsupported cases.

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

Run strict types, ESLint, the frontend component/consumer suite, pinned Chakra
typegen, frontend build and release policy checks. The shared migration affects
all UI consumers, so run the generated native desktop suite, including
`tests/desktop/appearance.e2e.mjs`, page chrome, Foundation, viewer, utilities,
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
