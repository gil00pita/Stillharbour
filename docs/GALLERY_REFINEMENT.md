# Gallery row balancing — 2026-10-28

The owner requested gallery improvements informed by [Building the Google Photos Web UI](https://medium.com/google-design/google-photos-45b714dfbed1). The useful principles for this existing app are choosing row breaks across a sequence rather than greedily, keeping rendering bounded, and avoiding repeated resize work. These are design inputs, not Stillharbour performance measurements or permission to implement every future performance phase.

## Resulting behavior

- Timeline photos preserve chronological order, existing aspect-ratio bounds and 4 px gutters. A shortest-path layout evaluates alternative row breaks in the retained renderer window. It balances deviation from the selected thumbnail height and excessive whitespace, with a stronger penalty for extremely short/tall rows. Panoramas can occupy their own row instead of shrinking into an already crowded row.
- Normal rows can stretch up to 125% of the chosen target. A sparse last row never stretches above that target. Exceptional aspect ratios may leave whitespace rather than produce an excessively tall row. Tiny final orphans are discouraged, not forbidden when the photo sequence requires one. Date boundaries, unknown dates and explicit shown-group membership remain intact; no photo is reordered or hidden by layout.
- Narrow date headings use a compact month/day/year caption, retaining the full date in the heading's accessible name and title. Today/Yesterday/Unknown date keep their meaning. Counts appear only when the heading has room. The date checkbox still names the exact number of shown photos; it never selects unseen catalog matches.
- ResizeObserver notifications coalesce through one requestAnimationFrame callback, using the latest width and existing stable-ID/pixel-offset anchor. Unmount cancels pending work. This avoids repeated calculations during a resize burst without adding an arbitrary settling delay.

The existing TanStack virtualizer, two-row overscan, bounded three-by-240 feed, filtered month seeks, viewer/filmstrip, text-only mode, preview ownership and error/offline states remain. There is no full-catalog renderer array, predictive original read, new rendition/cache policy, shared resource scheduler, schema/IPC/dependency change or original-file operation. No 60fps, latency or memory improvement is claimed without measurements.

## Implementation and checks

[justifiedLayout.ts](../src/library/justifiedLayout.ts) implements a backwards shortest-path pass with at most 64 candidate successors per photo: O(n × 64) work and O(n) storage for the retained window, not an enumeration of all layouts. It accepts only geometry supplied by [model.ts](../src/library/model.ts), which normalizes metadata ratios and available width/target. Missing dimensions remain bounded placeholders. Day membership accumulation now appends to its owned array instead of repeatedly copying a dense day.

[usePhotoViewport.ts](../src/library/usePhotoViewport.ts) retains the existing anchor and virtualizer seam. [AssetBrowser.tsx](../src/library/AssetBrowser.tsx) chooses concise date captions from computed heading width without reading each tile's DOM geometry. There is no framework replacement.

Regression tests in [model.test.tsx](../src/library/model.test.tsx) and [AssetBrowser.test.tsx](../src/library/AssetBrowser.test.tsx) cover panorama isolation, dense days, finite ordered geometry through all 720 retained summaries, missing/extreme dimensions, narrow/large windows, compact date labels, true text-only requests and resize coalescing/anchor retention. The old row-height assertion is revised because modest upward stretch is now intentional; it still enforces the explicit 125% cap and unchanged aspect/gutter/coverage assertions. The visible-date test derives its expected first asset from actual row grouping instead of assuming the old greedy row boundary.

Native [foundation.e2e.mjs](../tests/desktop/foundation.e2e.mjs) uses generated source files and real Rust IPC. It checks 1180×780 and 660×560, chronology, no horizontal overflow, 4 px row/tile gutters, month jumps, selection, image-free Text list and viewer return. A separate Python SHA-256 inventory is captured before scanning and in suite teardown, including failures. Evidence lives under `.context/gallery/native-<UUID>/`; new canonical captures are `.context/screenshots/balanced-gallery-desktop.png` and `balanced-gallery-minimum.png`. Existing preserved M5 camera fixtures are not rescanned.

```sh
npm test -- src/library/model.test.tsx src/library/AssetBrowser.test.tsx
npm run desktop:test:build
npm run desktop:test
npm run check
npm run desktop:build
npm run release:check
```

Build/test invocations must run serially because they share `dist`. Actual results and limitations are appended in [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md). Native checks on this Mac do not establish Windows, VoiceOver, physical gesture, production release frame-rate or million-row performance evidence. Existing preservation, M5 format and later performance-phase gates remain unchanged.

## Recorded validation

Full npm run check passes118 frontend tests and57 Rust tests, formatting, ESLint, strict types, generated IPC bindings, rustfmt, denied-warning clippy and the production frontend build. Native13 cases pass18.3s on macOS; run5820b395-2860-45ca-bd1b-60e4f47ea225 has32 independent source entries unchanged. The strengthened resize test confirms a30px offset retained when the anchored photo moves to a different balanced row. Mechanical design scan has zero findings; full React Doctor remains73/100 with ten incumbent warnings (changed-only mode skipped the untracked scaffold). The unsigned macOS application builds33.86MiB and production exclusions/helper-manifest checks pass. No new frame-rate/latency/memory benchmark or Windows evidence is claimed.
