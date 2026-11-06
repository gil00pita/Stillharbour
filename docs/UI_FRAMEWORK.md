# Stillharbour UI framework

The owner requested Chakra UI using [this example repository](https://github.com/gil00pita/ts-eslint-nextjs-boilerplate). [ADR-0083](decisions/ADR-0083-chakra-ui-design-system.md) records the inspected commit and architecture decision. npm stable tags verified on 2026-11-02 resolve to Chakra UI **3.37.0** and Emotion **11.14.0**, both pinned exactly with registry integrity in package-lock.json.

## Automatic theme types — 2026-11-05

`npm run build` and `npm run typecheck` run `npm run theme:types` before TypeScript.
The local, lockfile-pinned `@chakra-ui/cli` **3.29.0** generates the installed
Chakra declarations from `src/theme/system.ts`, including custom RadioCard
presentation variants. No global CLI, transient download or committed generated
node_modules declarations are required. `npm ci --ignore-scripts` is supported:
the normal build creates the types after installation.

The renderer remains Chakra **3.37.0** with Emotion **11.14.0**. The CLI is a
build-only MIT tool; version 3.29.0 declares Zod 4 support and the lock resolves
patched Zod 4.6.7. The initially tried CLI 3.37.0 required affected Zod 3 and is
not retained. [Theming/type generation](https://chakra-ui.com/docs/theming/overview)
and [GHSA-mrhx-c7xc-m3w5](https://github.com/advisories/GHSA-mrhx-c7xc-m3w5)
explain the generated-type requirement and the dependency constraint. Node 24
matches CI; newer development Node versions may emit upstream CLI dependency
engine warnings and are qualified only by the actual checks recorded in the log.

## Structure

- `src/theme/system.ts`: raw product tokens, semantic colors and button/input/textarea recipes. Uses `defaultBaseConfig` with preflight disabled, the existing dark palette and native system fonts. No remote assets or new theme preference.
- `src/theme/recipes.ts`: shared field recipe, also used by the styled native select.
- `src/components/ui/Provider.tsx`: the single production/test styling provider. Mounted outside the app and instrumented-only generated lab in `src/main.tsx`.
- `src/components/ui/controls.tsx`: standard Button, Input, Select and Textarea. Import these in ordinary product forms to retain native refs/events/validation and shared recipes. Keep explicit type=button on non-submit form actions; an omitted type intentionally keeps existing HTML submit behavior. Numeric size maps to htmlSize.
- `src/styles.css`: product layout and specialized controls; existing CSS variables resolve to Chakra semantic tokens, with same-value startup fallbacks. Generic field/button rules exclude their Chakra counterparts.
- `src/test/render.tsx`: Testing Library render with the actual provider, including custom test wrappers and rerenders. Import render/screen/fireEvent from this module in component tests.

The adoption converts 369 ordinary controls in 61 product modules: setup, shared tag/size fields, Settings, library forms/toolbars, catalog metadata, inspection and backup flows. It keeps native dialogs and specialized photographic canvas, virtual-row, map-marker/zoom, checkbox/switch/range controls. Their tested selection, focus and geometry remain in existing adapters. Further adoption should use semantic tokens and verify the affected interaction rather than silently changing its DOM or approval behavior.

The reference's provider/token/recipe separation informs this design. Stillharbour remains a Tauri/React/Vite application. No Next.js, cloud service, extra state library, Storybook, external fonts or next-themes is added.

## Checks and limits

```sh
npm run typecheck
npm run lint
npm test
npm run desktop:test:map
npm run build
npm run release:check
```

Run instrumented native tests before the final production frontend rebuild; they share build output. The map/metadata test scans only constructor-owned generated images/sidecars. Its native theme assertions check actual class/variable resolution and 36px fields, not just mocked component output. It also checks generated catalog edits, minimum dialog width and an independent unchanged-source manifest. Desktop evidence applies only to the recorded native host; Windows and full manual screen-reader review remain separate gates.

Two shared-control regressions cover lossless Unicode/line-break form values, implicit/explicit submit behavior, native refs, numeric input size and disabled approvals. The existing component suite exercises all migrated form states through the real provider. Initial default-size TypeScript incompatibility was resolved by mapping the native size attribute to htmlSize. A test-only barrel export warning was resolved with explicit exports rather than suppressing lint.

No Rust behavior or catalog schema changes in this UI adoption. The earlier schema-16 full Rust checkpoint remains 472 passes / 10 host metadata-preservation failures / 15 ignored; this UI change does not resolve those failures or claim a new Rust-suite pass. M5 export/interoperability and M4/M6 preservation/release gates remain open. Actual final native, audit, bundle and React Doctor outcomes are recorded in the implementation log.

## Final local checkpoint — 2026-11-02

457 component tests in 70 files pass through the real provider, along with strict types, ESLint, binding validation, formatting and the production build/resource checks. The native macOS map/metadata round trip passes in 6.9s (`.context/chakra-native-select.log`): actual semantic colors, 13px buttons, 36px fields, visible native-select arrows, real catalog edits and a 660×560 dialog with 607px content/client width. All nine generated source/sidecar entries remain unchanged. Retained JSON, screenshots and instrumented build receipt are under `.context/chakra-native`; final organization/minimum images were inspected.

Native testing exposed and fixed the legacy unlayered font rule overriding a Chakra recipe. Visual review exposed a select-arrow regression; using backgroundColor instead of a shorthand and explicitly retaining the factory control's product class fixes it. The native test now asserts arrow visibility. No assertion was weakened.

Production main JavaScript is approximately 758 kB (223 kB gzip), with the existing lazy basemap approximately 1.21 MB and worker 519 kB; the large-chunk warning remains. Production npm audit is zero; 33 development advisories remain. No Windows, startup benchmark or whole-process network interception is claimed.

React Doctor reports the same two existing Settings maintainability findings with identical messages/complexity. Its score changes from 67 across 19 changed files before adoption to 62 across 121 afterward; this is not described as a passing or improved score. `.context/chakra-doctor-comparison.json` retains the comparison. No rule suppression, automatic React Doctor installation or unrelated component rewrite is added.
