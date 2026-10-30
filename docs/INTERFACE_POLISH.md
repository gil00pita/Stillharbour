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
