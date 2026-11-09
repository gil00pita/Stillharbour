> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# Restrained interface polish

Date: 2026-10-30. The owner invoked Unslop and approved consistent controls/spacing, readable supporting text, quieter secondary actions and less bouncy feedback, preserving the current brand and photo layouts. This is a presentation refinement of existing workflows, not another milestone or an execution grant.

## Direction and audit

Product: a local photo library for personal collections. Character: calm, precise, quietly warm. World: a photographic contact sheet on a dark viewing table. Moments: photographs and the unchanged aperture logo carry color. Signature: quiet date labels and the existing 4px photo gutters.

The existing photo-first shell is coherent and calls for light polish. Native captures showed short macOS selects beside taller inputs, small labels and descriptor text, inconsistent form spacing, tinted secondary controls, repeated technical identifiers in organization rows and overshooting favorite feedback. Generated gradient fixtures are test media, not interface decoration. Keep the system typeface, graphite/sea-green palette, section order, availability states and current feature boundaries.

## Applied passes

1. Spacing: regularize navigation, form grids, labels, Settings sections and dialog actions to the existing 4px rhythm. Preserve sidebar widths, anchoring and all photo layout algorithms, targets and gutters.
2. Color: reuse existing neutral surfaces for secondary buttons and the existing preview neutral for hover. Keep sea-green for primary, selected and confirmed states; preserve error/availability semantics.
3. Icons: use the existing authored 18px/1.5px outline family for the disabled Map/Utilities entries, with aligned labels and a quiet Later hint. Keep tooltips explaining their unavailable state and preserve the owner's SVG.
4. Copy: shorten repeated Settings/organization help without dropping preservation, reconnection-validation or unchecked-destination explanations. Preview rows use actual source-library names, retaining stable IDs in their tooltip; removed/unlisted roots have an explicit fallback. Group byte totals using the existing BigInt formatting pattern without precision loss.
5. Type: retain native system fonts; use 14px base text, 13px controls/help/paths, 12px labels/meta and a legible 11px descriptor even at the native minimum. Use tabular figures. Small headings have ordinary tracking; the onboarding wording stays unchanged.
6. Layout: standard fields and selects are 36px tall with 8px corners. Native select menus retain their semantics, using appearance:none and an authored monochrome chevron for a consistent closed field. Primary actions and overlaid viewer tools retain 44px targets. Compact chip editors, range controls, checkboxes and the timeline remain purpose-specific exceptions.
7. Details: names lead over identifiers, supporting Settings paragraphs have a 72ch measure, and whitespace in organization paths remains visible. Preserve keyboard focus, native dropdown behavior and explicit review/disabled states.
8. Visuals: keep actual catalog imagery as the visual content. No generated assets, sample production library, new decorative surface or image prompt is needed.
9. Motion: use the existing ease-out curve for brief color feedback. Confirmed favorites use a 180ms response with only a 4% press; remove the overshooting easing. Keep Reduced Motion, occluded-window completion, confirmed catalog writes and the existing purposeful viewer/loading/preparation feedback. The completion guard recognizes both milliseconds and seconds because the bundled CSS may serialize `180ms` as `.18s`.

## Verification and limits

Existing native flows exercise desktop/minimum Settings, Filters, organization preview and viewer overlays, preserving generated-source inventories. Extend them to check native field/select alignment and readability, viewport fit and saved favorite feedback. Component coverage verifies library-name/identity presentation while preserving request IDs, stale results, errors and execution gating. Full repository checks, React Doctor and the normal unsigned build/release exclusions remain applicable. Actual outcomes are recorded in [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md).

The closed-control styling follows [MDN appearance](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/appearance); [native date input presentation](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/date) remains platform-dependent. An explicit height and WebKit date-editor padding reset address the observed local 40px/36px mismatch; Windows behavior is not inferred.

Use only generated media and temporary catalogs. No original, backend, catalog schema, dependency, Tauri capability or persistence change is intended. M2 destination/approval/executor/recovery work, P4-06 and platform/release gates remain open.

Workflow credit: [Unslop by Damir Sokolovsky (@heydamir)](https://heydamir.design/unslop), CC BY 4.0. This report applies the workflow to Stillharbour's existing product and brand.

## Final local results

The final full check passes 200 frontend/harness tests and 115 Rust cases, strict types, lint/format/bindings, rustfmt, warnings-denied Clippy and build. Changed/untracked React Doctor reports no new issues with score/telemetry disabled. All 16 native macOS cases pass in the final 35-second suite, including actual field alignment, viewer frame/tool containment, catalog favorite confirmation and unchanged generated-source inventories. The unsigned 34.63 MiB app builds; production exclusions/helper checksums pass and packaged in-memory SQLite reports 3.54.1 without a user catalog.

The favorite guard's CSS-time-unit regression is confirmed red then green for both `180ms` and `.18s`, with hidden-window completion preserved. Viewer captures now wait for the opening transition and fitted frame to settle; the animation itself is unchanged. One earlier gallery resize-settling run failed and was not reproduced in three focused repeats or subsequent complete runs. Its cause remains unclassified; no gallery resize/anchor algorithm was changed and no universal reliability claim follows. Earlier failed runs and intermediate captures remain under `.context/`; [the implementation log](IMPLEMENTATION_LOG.md) records their scopes and exact binary/source evidence.

## Screen critique — 2026-11-02

The owner switched the pending polish request to Impeccable critique. The independent design and detector review is archived at [.impeccable/critique/2026-11-02T13-34-40Z__src-app-tsx.md](../../../../.impeccable/critique/2026-11-02T13-34-40Z__src-app-tsx.md). Its provisional heuristic score is 26/40. Priorities are unsaved caption/review draft protection, a persistent organization-dialog header/exit, visible saved-search replacement context, simpler technical help and a photo-led no-location map state. These are recommendations, not implemented changes or expanded milestone authority. Evidence combines current source with retained native captures that predate the shared-control migration; fresh live/native verification remains required.

## Critique follow-through — 2026-11-02

The owner requested polish against all five findings in the archived critique. Caption/review drafts now belong to the current catalog UI session and survive photo changes, Info closure, viewer closure and section navigation. They retain the original optimistic version; reopening never silently rebases onto a newer saved value. Explicit Save clears the matching draft only after confirmation; failed saves/reloads and late saves cannot erase a newer draft. At most 240 unsaved photo drafts are admitted; existing drafts are never evicted. Drafts are memory-only: save before closing the app or switching catalogs, as disclosed beside the editor. This is not crash recovery or persisted autosave.

The organization dialog has a stationary title/Close area, keyboard-focusable section shortcuts and a separately scrolling body. Saved-search replacement displays saved and frozen proposed filters, with Keep saved filters or explicit Replace confirmation. Exact backend version checks remain authoritative. Tag explanations now lead with grouping photos; exact matching and limits remain in details. No usable location removes the empty geography and presents the unlocated photo count directly. Existing palette, typography, logo, source protections and milestone boundaries remain.

Validation commands: npm test; npm run typecheck; npm run lint; npm run desktop:test:map; npm run rust:check; npm run build; npm run release:check; scoped Prettier and git diff --check. Actual outcomes and any limitations are recorded in IMPLEMENTATION_LOG.md. Native tests use only generated fixture photos and assert unchanged originals.

## Quiet photographic studio — owner selection, 2026-11-05

The owner requested a more professional, expensive-looking interface and selected the quiet photographic studio direction. The existing brand becomes warmer and more deliberate through charcoal layers, warm white type, pale sage and clearer visual hierarchy. No downloaded fonts, new dependency, new production media or original-file behavior is introduced.

App navigation now groups collection destinations, library-care workflows and unavailable utilities, retaining anchored Settings/version and independent scrolling. The wider 240px rail has quieter inactive labels and a stronger current state. A panel glyph replaces the ambiguous navigation chevron. Photos gains recessed search with a search glyph, an outlined Filter action, and quieter Jobs/Catalog/Saved plans. Secondary toolbar text collapses into named icons with tooltips at narrow desktop widths, fixing action clipping at 660×560. First run uses the approved product tagline as its headline and explains folder choice and in-place originals beneath it; selecting a folder still does not scan it. Settings keeps flat sections with stronger headings and more space. Standard controls use 6px corners and shared 36px sizing; primary actions retain 44px. Text selection, carets and native scrollbars use the same palette. Existing photo geometry, viewer motions, reduced-motion handling, safety wording and backend gates remain.

The custom Chakra system registers the pinned default Field slot recipe. Initial CLI regeneration exposed missing required-field typing when Field was absent from the custom recipe registry; registering that used recipe restores strict checking without changing required-field behavior. Chakra primary references: [theming system](https://chakra-ui.com/docs/theming/overview), [Field composition](https://chakra-ui.com/docs/components/field).

Browser captures in .context/studio-*.png use an isolated, ignored Vite configuration and synthetic catalog responses; example imagery comes from system wallpaper thumbnails copied only to .context. These captures establish presentation only. They are not shipped sample content, actual indexed libraries, provider-fidelity evidence or native acceptance. The legacy Playwright wrapper could not resolve its executable; the installed agent-browser CLI supplied the isolated browser checks. Full outcomes and native prerequisite failures are recorded in IMPLEMENTATION_LOG.md.

The final targeted macOS native smoke passes against a separate hash-matched instrumented executable while the independent Conductor development run remains active. It uses real Rust IPC, a disposable catalog/generated library and unchanged-source verification at desktop/minimum sizes. See IMPLEMENTATION_LOG.md for the exact binary, initial failed/interrupted attempts and the separate full-suite outcome.

Final native evidence: all five repository desktop specs pass across the isolated main run and the two affected M5 reruns (26 macOS cases, three existing platform skips), plus the targeted one-case UI smoke. Earlier attempts, the corrected old-palette assertions, the independent serial rustdoc retry and Windows/release limitations remain recorded in IMPLEMENTATION_LOG.md.

## Illustrated duplicate choices — 2026-11-05

For the owner’s later request, Duplicates replaces match-mode, file-scope and property-combination dropdowns with radio cards. Three generated paper-and-sage illustrations explain file equality, property candidates and static PNG8 pixels. Algorithm choices move into Advanced; existing typed settings and all evidence limits remain. This explicitly requested artwork extends the earlier pass’s no-new-assets scope only for these controls. Full interaction, asset and validation details: [DUPLICATE_MATCH_CARDS.md](DUPLICATE_MATCH_CARDS.md).

## Favorite heart animation — owner revision, 2026-11-05

The owner supplied the Prototypr Twitter-heart CSS article and requested that animation on the existing heart. This explicitly supersedes the earlier restrained 180ms favorite press: a bounded 800ms rose heart pop, expanding ring and seven paired particle rays acknowledge a confirmed favorite. Keep existing catalog confirmation, failure behavior, navigation, hidden-window completion, Reduced Motion and 44px target. See [ORIGINAL_IMAGE_VIEWING.md](ORIGINAL_IMAGE_VIEWING.md) for source attribution, choreography and checks.

## Sidebar collapse control — 2026-11-05

The owner requested moving collapse into the navigation menu. The sidebar Workspace actions now contains Jobs, Settings and Collapse navigation, in that order. Expanded navigation shows the panel icon and label; collapsed navigation keeps the same control as an icon with an Expand navigation accessible name and tooltip. The control remains mounted during onboarding and page changes, retaining keyboard focus and the existing temporary override/settings default behavior. Page headers no longer contain it; shared sidebar button sizing and focus styling apply.

## Sidebar scrollbar alignment — 2026-11-05

The navigation scroll area extends through the sidebar’s trailing padding to the vertical divider. Matching internal padding preserves the menu-item alignment and width. Expanded and collapsed navigation use their existing 16px/10px insets; the anchored footer stays in place. Only the scrollbar’s trailing gap changes.

## Version in the app title — 2026-11-05

Display the native-reported application version beside the sidebar’s Stillharbour name (currently Stillharbour 0.1.0), without a Version prefix or a separate footer entry. Keep the descriptor beneath it. Hide the title/version together when navigation is collapsed. This changes presentation only; the displayed version remains the actual application version, not the illustrative 1.0.1.

## Jobs page — 2026-11-06

Jobs above Settings now opens a page in the main pane, with a heading and Back action. It keeps the live count, progress, recovery/history and native controls. The previous screen stays mounted but hidden and inert, retaining its draft and selection for Back. Sidebar navigation also leaves Jobs; Escape and outside clicks do not dismiss a page. The page uses the normal main scroller at desktop and minimum widths.

## Header alignment and relocated actions — owner revision, 2026-11-06

All page title rows and the sidebar brand share 64 px height, including compact navigation, with 18 px/24 px title typography. Remove the legacy compact sidebar's 24 px top offset. Keep descriptions and changing status outside the row. Catalog and Saved plans now sit on Albums / Tags; Photos has Filter and Select beside search. Photo context menus replace dots, with checkboxes visible only in explicit selection mode. Suppress native text selection/context menus on sidebar/title chrome while preserving normal input editing and keyboard focus. This supersedes earlier toolbar-placement/hover-selection descriptions.
