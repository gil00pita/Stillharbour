# Reference timeline navigation

Owner correction, 2026-11-05: implement the behavior of [alumbo/react-timeline-vertical-navigation](https://github.com/alumbo/react-timeline-vertical-navigation), not the previous month-spaced approximation. See [ADR-0090](decisions/ADR-0090-reference-timeline-navigation.md) for the native paging decision.

## Reference contract

The React wrapper at 0273e0197be86eb5b62110e931e71af392f3f4cc wraps [relyfe's Stencil navigator](https://github.com/relyfe/timeline-vertical-navigation/blob/9e7849dba58e6e21779ce334d362d4f47f7beaa9/src/components/timeline-vertical-navigation/timeline-vertical-navigation.tsx). Its index scale is photo count, not elapsed time or equal month slots. Press selects immediately and movement selects continuously. Hover previews without selecting. The gallery's first visible photo updates the selected position. Year labels use their first photo's rank; filler dots are 4px with 12px spacing between year labels. These dots do not assert that a month or photo exists at each marker.

Stillharbour now follows this contract over filtered native pages. Month counts provide a bounded summary of positions/preview labels; seek_index selects the exact stable photo ordinal, including targets beyond the first 240/720 photos of one month. Chronological page startIndex metadata and visible asset identity keep the rail synchronized while ordinary scrolling or paging. Duplicate dates retain the existing date/id tie order. Undated remains explicit. A single photo disables scrubbing instead of dividing by zero.

## Interaction and geometry

- The 56px inner lane sits beside the gallery. The invisible native vertical range covers the lane with 24px end insets. Match the source's 40×1px selected line, 72×2px preview underline, 24px dark date label, 4px gray dots, blue dark-mode lines and compact dark year labels. The date label's **bottom**, rather than its center, aligns with the underline. Use the app's system font and keep desktop chrome outside the rail.
- Press and drag navigate live across loaded and unloaded dates. Keep the gallery/rail mounted while seeking; maintain the requested position until the replacement page arrives. One native replacement is active, with only the latest pending target retained. Stale responses cannot replace a newer target or filter.
- Release ends dragging. Capture/document listeners handle outside movement/release. Escape, blur, cancellation or lost capture stop further tracking; they cannot roll back a photo position already selected by live dragging.
- Hover moves only the preview label/underline. Leaving restores the actual selected label. Arrow keys move one photo, Page keys move roughly a tenth of the range, Home/End reach exact endpoints. Accessible year buttons remain keyboard-operable. Wheel forwards to the gallery; Ctrl+wheel retains platform behavior.
- Hover, focus, dragging and gallery scrolling reveal the rail immediately; active opacity never waits on an occluded WebView animation. After scrolling, a three-second idle delay precedes its fade. Reduced Motion removes the transition. All hidden visuals retain their keyboard semantics and pointer hit area.

Separate arrow presses must also advance through photos that share one loaded gallery row. Keep the requested ordinal while that row's observed first index is unchanged; retire it when the gallery position/page changes or focus leaves. Otherwise keyup can reset the next press to the same row's first photo indefinitely. Page replacement and later scrolling must not reactivate an old requested ordinal.

## Native boundary and limits

seek_index accepts only the existing validated chronological query plus a u32 ordinal and a page limit from 1 to 240. It returns existing stable-ID summaries and optional startIndex metadata, not arbitrary paths, SQL or a full-library date array. Packed page v1 retains all 27 tuple fields. Renderer retention remains at most three pages/720 summaries. Prefix/ordinal SQL may scan the filtered ordered index; no million-photo timing or P4 acceptance is claimed. No originals, sidecars, database schema, helpers or production mutation gates change.

The reference's MIT notice is distributed at public/licenses/timeline-vertical-navigation-MIT.txt, copied into dist/licenses and embedded with the native frontend. Upstream font downloads and React 17 dependencies are not imported. Preserve the owner's unchanged logo and all other studio/duplicate/heart work.

## Validation

The initial regression fails on equal-month weighting (max 3 instead of 52 for 53 photos) and absence of press-time navigation. Component/helpers cover density, preview coordinates, live pointer/keyboard operation, pending-page continuity, outside release, cancellation and single-month/Undated inputs. Feed tests verify the retained rail and one-active/latest-pending requests. Rust fixtures compare ordinal seeks, returned ranks, tied dates and filters against ordinary keyset traversal, including ranks beyond a page/window and invalid/empty scope. Wire tests validate optional ordinal metadata without changing older fixtures.

Run npm test, npm run typecheck, npm run lint, npm run bindings:check, npm run rust:check, npm run build, npm run test:release, npm run release:check, format:check and git diff --check. tests/desktop/timeline-navigation.e2e.mjs seeds only generated media and a disposable 960-photo catalog. It asserts an actual seek while the pointer is down, stable rail identity, bounded retained pages, exact endpoint IDs, native line/label geometry at 1180×780 and 660×560, Escape/outside release and unchanged generated sources. Actual outcomes and failures are recorded in IMPLEMENTATION_LOG.md. Browser screenshots do not establish Windows or physical accessibility acceptance.
