> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# Illustrated duplicate matching choices

Owner request, 2026-11-05: replace duplicate-matching dropdowns with radio cards and coordinated illustrations within the quiet photographic studio direction.

Duplicates now leads with three selectable cards: **Identical file contents**, **Matching file properties**, and **Static PNG8 pixels**. Each has a bundled, decorative transparent illustration, a plain-language description and an evidence limit. Property matches remain candidates only. The existing normalized-pixel policy covers bounded static RGB8/RGBA8 PNG samples only; its results require manual review and establish no complete-file equality or removal authority. No perceptual similarity mode is introduced.

File scope uses two compact radio cards. The existing property-rule AND/OR combination also uses cards. Content algorithms move into an initially closed **Advanced matching options** disclosure, with BLAKE3, SHA-256, SHA-1, MD5 and direct-byte choices and the existing hard-link option. BLAKE3 and exact-content defaults, root selection, exclusions, property validation, typed IPC and job lifecycle are unchanged. Changing a choice does not start a job; the explicit Start action remains necessary. Retained inspection history remains a dropdown because its options are dynamic job records. Integrity and Storage retain their current controls.

## Interaction and presentation

Chakra UI v3 RadioCard supplies labelled radio groups, hidden native inputs, checked/disabled states and standard browser arrow-key selection. The selected card has a pale-sage outline, quiet sage surface and filled radio indicator. Keyboard focus adds a two-pixel sage outline. Entire labels are clickable. Artwork has empty alternative text; labels and descriptions carry all meaning. Controls disable during an active request/job. Short colour transitions respect reduced motion.

Three illustrated columns become horizontal illustrated rows below a 620px group width. Scope/combination cards stack below 420px. The main inspection area scrolls at the 660×560 native minimum; no horizontal overflow is permitted. Artwork cannot replace mode names, restrictions or error text. The owning components and slot recipe are colocated in src/components/DuplicateMatchOptions and src/components/InspectionChoiceCards; the existing custom theme registers the recipe.

## Artwork

assets/illustrations/duplicates contains exact-content.png, properties.png and pixels.png. They are generated for this request using OpenAI image generation: ivory photo prints, metadata tags and pixel tiles, in a coordinated paper-and-sage miniature style. Each bundled image is 640×426 with alpha; their total is approximately 920 KiB. They load locally without a network service, image decoder for originals or catalog access. They are decorative concepts, not user media or inspection evidence. The owner’s unchanged logo is still authoritative. Original generation outputs remain outside the repository; only resized assets ship.

## Checks

Component tests cover the initial choice, decorative artwork, explicit-start boundary, property logic, file scope and algorithm payload, disabled active-job controls and empty-criterion validation. tests/desktop/duplicate-cards.e2e.mjs uses an isolated native catalog and generated media, checks artwork/fit at 1440×1000, 1180×780 and 660×560, card selection and an actual exact-content job, then verifies unchanged fixture files. Run npm test, npm run typecheck, npm run lint, npm run rust:check, npm run build and npm run release:check. For native validation, build with desktop:test:build and run the duplicate-cards spec with the documented isolated target/config approach; see DEVELOPMENT.md. Actual receipts and limitations are recorded in IMPLEMENTATION_LOG.md.

Primary API reference: [Chakra UI RadioCard](https://chakra-ui.com/docs/components/radio-card). No dependency, backend, schema, codec, mutation, preservation or milestone gate changes.

The pinned embedded desktop driver simulates radio arrow keys by assigning checked and firing only change; Ark expects a click. Its synthetic keyboard result is not a physical WebKit keyboard test. Keep the standard Chakra behavior, check keyboard selection/focus in the real browser, and exercise native card clicks and real IPC separately. Do not add production event workarounds for this harness limitation.

The conditional property fieldset has a stable wrapper. In the inspected Chromium build, inserting it directly into the outer fieldset could collapse the preceding radio group and lose focus; the wrapper preserves the rendered choices and keyboard focus across all three modes. The card group uses normal block flow with an internal grid.

## Utilities integration — 2026-11-05

The illustrated matching choices remain inside the expandable duplicate inspection settings below the cluster workspace. Retain main's card assets, radio semantics, file scope, advanced algorithms, property-rule combinations and explicit native Start action. Search/group/keeper review is a distinct read-only workspace; no card or visual confidence preview approves removal. See [Utilities UI](UTILITIES_UI.md). Native card checks scroll the settings into view before inspecting their layout.
