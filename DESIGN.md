---
name: Stillharbour
description: Neutral native photo workspace with the unchanged multicolour owner mark
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
---

# Stillharbour design system

Photographs lead; cool neutral surfaces, compact Inter text and consistent controls
support browsing and review. The owner's unchanged aperture mark supplies the
identity. The source of truth is [src/theme](src/theme) and the owning components,
not a historical screenshot or duplicated token table.

## Appearance and typography

Semantic tokens provide light/dark canvas, panels, fields, text, borders, focus
and selection. Appearance follows the system until a saved manual choice takes
precedence. Use the semantic role rather than mode-dependent component ternaries.
Inter Variable ships locally with native system fallbacks; tabular numbers keep
counts and metadata aligned.

Preserve readable text, visible focus, reduced-motion behavior and textual error/
availability states. Color alone cannot communicate a safety or job outcome.
Only actual metadata, counts and previews belong in the production interface.

## Layout and controls

- The native window defaults to 1180 × 780 with a 660 × 560 outer minimum.
  Windows decorations can leave a smaller content viewport.
- The expanded sidebar uses 240 px, narrows to 184 px at its component breakpoint,
  and collapses to a 72 px icon rail. The 64 px brand/header rows align.
- Shared fields and buttons use 36 px geometry with minimum icon-button width,
  consistent padding and grouped header actions. Specialized viewer targets keep
  their own accessible geometry.
- Gallery rows and virtual extents share 6 px gutters. Preserve aspect ratio,
  chronological order, bounded feeds and visible-anchor restoration.
- The date rail reserves 32 px and stacks above thumbnail controls. Its overlaid
  month label hides at rest while drag/keyboard navigation stays available.
- Display options owns view mode, thumbnail size and grid fit, including compact
  widths. Filter owns query state. Both panels keep draft Apply/Cancel behavior.
- Minimum/maximum size fields each embed their unit select inside the amount
  control, with validation and busy behavior preserved.
- Thumbnail checkbox/heart targets are 36 px with centered 28 px translucent dark
  marks and white glyphs. They align at the same inset; narrow portraits stack the
  targets to prevent overlap. Favourite fill follows confirmed catalog state.
- Browsing reveals photo controls on hover/focus. Explicit selection and album
  pickers retain visible selection controls. Selected actions float in a bounded,
  wrapping workspace overlay.

## Style ownership

Use Chakra v3 props, local `css` objects and colocated recipes. Keep shared
semantic tokens and document defaults in the theme. Dynamic virtual geometry
stays inline; dependency font/MapLibre stylesheets remain imported. Avoid
reintroducing application CSS files or global component-selector dumps.

Use native semantics, portalled overlays, labelled icon buttons and Field wiring.
Do not remove focus indicators, source/identity caveats, disabled reasons or retry
states to simplify a composition. Illustrative structure examples are labelled;
simulation and disabled execution are never presented as a real file operation.

See [coding conventions](docs/TYPESCRIPT_STRUCTURE.md),
[current behavior](docs/CODEX_SPEC.md) and [verification](docs/ACCEPTANCE_TESTS.md).
Prior design directions and their scoped screenshots/evidence remain in the
[changelog](docs/changelog/2026-11-09/DESIGN.md).
