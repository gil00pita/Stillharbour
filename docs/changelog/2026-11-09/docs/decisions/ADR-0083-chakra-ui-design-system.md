> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0083 — Chakra UI and the Stillharbour design system

Date: 2026-11-02. Status: implemented; native/platform evidence remains qualified.

## Context

The owner requested the latest Chakra UI and supplied [ts-eslint-nextjs-boilerplate](https://github.com/gil00pita/ts-eslint-nextjs-boilerplate) as the example. Inspected its provider, theme, semantic tokens, recipes and package manifest at commit `c1c41fefe4fa4731d4610534db6705b10957b09d`. Its relevant pattern is Chakra as the implementation engine with a product-specific theme layer. Its Next.js application and other dependencies are outside this request.

## Decision

Pin `@chakra-ui/react` 3.37.0 and `@emotion/react` 11.14.0, verified against npm's stable tags on this date. Use the existing React/TypeScript/Vite frontend in Tauri. No Next.js, next-themes, Storybook or additional application-state library is required for this desktop adoption.

Create one application `ChakraProvider` using `src/theme/system.ts`. Raw Stillharbour colors, system fonts, sizes and radii feed semantic tokens and recipes. Use Chakra's documented `defaultBaseConfig` to keep the existing product scale without importing the default design theme. Disable preflight so the native dialog, image canvas and browser form defaults are not globally reset. Prefix generated variables with `stillharbour`; bridge the existing CSS variables to those tokens with same-value initial-render fallbacks. Keep the present dark appearance rather than adding a new theme preference.

Standard buttons, text inputs, selects and textareas use `src/components/ui/controls.tsx` across 61 product modules. Chakra Button/Input/Textarea and the styled native-select factory retain existing DOM classes, refs, native events, labels, values and validation. Preserve HTML's implicit submit behavior explicitly: Chakra Button normally defaults to type=button, while existing product forms rely on type=submit. Map numeric input/select size to Chakra's htmlSize property.

Keep photographic canvas/virtual-row controls, map marker/zoom controls, switches, checkboxes, ranges and native modal dialogs in their specialized adapters. Existing CSS owns their geometry and state. Adoption does not require replacing tested selection/focus or native form behavior with a different interaction model. The core continues to own all permissions, catalog writes, filesystem operations and safety gates.

Tests render the actual provider through `src/test/render.tsx`; no mock Chakra components or successful browser-native fallback is introduced. Native generated-fixture checks inspect actual Chakra classes, resolved colors/control sizes, modal fit and real catalog persistence.

## Consequences

This is an owner-requested UI dependency with 129 added lockfile packages, justified by a shared theming/control implementation rather than repeated screen-specific styling. The production build is tree-shaken; exact bundle/audit/native outcomes are in [UI_FRAMEWORK.md](../UI_FRAMEWORK.md) and the implementation log. No native executable, external font, map provider, remote stylesheet, schema migration, IPC permission or original-file mutation is added. Keep pinned dependency checksums and review package licenses before distribution.

Official API references: [Vite installation](https://chakra-ui.com/docs/get-started/frameworks/vite), [customization and base config](https://chakra-ui.com/docs/theming/customization/overview), [recipes](https://chakra-ui.com/docs/theming/customization/recipes), [Chakra factory](https://chakra-ui.com/docs/styling/chakra-factory). This adoption does not complete M5 interoperability, M4 preservation or M6 release acceptance.
