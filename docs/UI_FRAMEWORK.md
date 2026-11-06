# Stillharbour UI framework

The owner requested Chakra UI using [this example repository](https://github.com/gil00pita/ts-eslint-nextjs-boilerplate). [ADR-0083](decisions/ADR-0083-chakra-ui-design-system.md) records the inspected commit and architecture decision. npm stable tags verified on 2026-11-02 resolve to Chakra UI **3.37.0** and Emotion **11.14.0**, both pinned exactly with registry integrity in package-lock.json.

## Clean-install custom recipes — 2026-11-06

Current main consumes InspectionChoiceCards' colocated recipe with
`useSlotRecipe({ recipe })` and applies the typed slot styles directly. Custom
presentation variants therefore compile with the published Chakra declarations.
The Storage components follow the same direct `useRecipe` pattern. No CLI
dependency, generated node_modules declarations or automatic type-generation
step is required by `npm run build` or `npm run typecheck`.

The earlier integration added an automatic CLI to repair the missing custom
variant declaration. Main's direct recipe fix supersedes that workaround, so
the extra dev dependency and build hooks are removed rather than retaining two
solutions. Runtime Chakra **3.37.0** and Emotion **11.14.0** stay unchanged.
Validate a fresh `npm ci --ignore-scripts`, direct strict TypeScript with stock
declarations and the ordinary build. Historical CLI/advisory experiments remain
in IMPLEMENTATION_LOG.md; they are not current build prerequisites. Follow the
owner's separate theme-type generation guidance when editing the theme, rather
than using casts around missing component props.

## Structure

The current layout and form contracts are documented in
[TYPESCRIPT_STRUCTURE.md](TYPESCRIPT_STRUCTURE.md).

- `src/theme/index.ts`: system composition and recipe registration, with raw
  colors/fonts and semantic colors in separate modules.
- `src/components/{Button,Input,NativeSelect,Textarea}/`: named native-contract
  controls and colocated recipes. NativeSelect composes Chakra v3's Root/Field
  with `asChild` to preserve one native select.
- `src/components/ui/Provider.tsx`: the existing production/test provider.
- `src/components/<Name>/`: application components, explicit barrels, private
  child components and colocated component tests.
- `src/hooks/use-*.ts`: shared hooks and their existing tests.
- `src/styles.css`: established product layouts and specialized native controls.
- `src/test/render.tsx`: Testing Library render through the actual provider.

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

## Component refactor checkpoint — 2026-11-06

[TypeScript conventions](TYPESCRIPT_STRUCTURE.md) now own component folders,
barrels, hooks, colocated recipes and ordinary Field composition. Clean-install
strict types, lint, 561 frontend cases, Rust checks and the final thirteen-spec
macOS desktop run pass. The native run catches a unit-field container regression;
a normal bounded container fixes it without weakening the existing assertion.
Production assets are restored and release preflight passes.
[React Doctor review](REACT_DOCTOR_REVIEW.md) records the 153-to-151 full-scan
change, the current cleanup false positive and remaining warnings. These checks
do not qualify Windows, manual screen-reader coverage or wider release gates.

### Shared application header and context actions — 2026-11-06

`PageHeader` composes the existing semantic header through Chakra v3 `Box asChild` and a colocated recipe. The theme's `appHeader` size is shared with the brand; title line height is a token. Settings/Jobs descriptions and Storage status live below the fixed header. `CatalogActions` owns the existing catalog/plans dialogs on Albums / Tags; query callbacks route to Photos. Native context-menu suppression is scoped to the sidebar and header h1, preserving input editing.

`PhotoAlbumActions` retains one controlled portalled `Menu.Root` for virtualized photos, positioned from the context event or keyboard anchor. Its provider exposes explicit selection mode to image/text rows and day headings. There is no menu button per tile. Isolated album pickers retain their own checkbox selection behavior. Busy/frozen selection and versioned membership IPC remain unchanged. `tests/desktop/page-chrome.e2e.mjs` measures native header geometry and exercises menu/selection/relocated dialogs against generated fixtures; the instrumented app must remain isolated from the development catalog.

### Photo context-menu anchor repair — 2026-11-06

The pinned menu implementation does not apply the supplied root `anchorPoint` to its ordinary positioning effect. Use `positioning.getAnchorElement` with a stable virtual element whose rect is the viewport context point, `strategy: fixed` and zero gutter. Keep the virtual element identity stable while the menu is open to avoid observer recursion. A changed context point remounts only the menu root so a second right-click repositions an already-open menu and resets its item highlight. Floating UI retains edge flipping/shifting. Keyboard anchors still derive from the focused photo row; the provider, focus restoration and catalog membership flow are retained. The native smoke must assert actual menu bounds, not just item presence.
