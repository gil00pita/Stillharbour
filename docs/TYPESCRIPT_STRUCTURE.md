# TypeScript and Chakra conventions

The 2026-11-06 refactor applies the component and hook layout in AGENTS.md to the
existing desktop frontend. The application remains Tauri/React/Vite with strict
TypeScript. Feature data models, IPC adapters and pure planning utilities retain
their existing boundaries; there is no catalog or native permission change.

## Components and hooks

Application components live in `src/components/<Name>/<Name>.tsx`. Each folder
has an `index.ts` with explicit exports. Consumers import
`@/components/<Name>`; private children use the owning folder's `components/`
directory. Tests for a component sit beside its implementation. The Vite alias
and TypeScript `paths` both resolve `@` to `src`, including instrumented profiling
builds. `src/main.tsx` remains the bootstrap, `src/test/render.tsx` the test
provider adapter, and `src/components/ui/Provider.tsx` the existing Chakra
provider.

Shared hooks live in `src/hooks/use-*.ts`, with their existing regression tests
beside them. Feature-specific helpers can stay with their owner. Pure feature
models and cross-component integration tests remain in their existing feature
modules. This reorganization does not introduce a state library.

Settings owns private LibraryLocations, PreviewCacheSettings, BrowsingDefaults,
FolderDefaults and SettingsHeading components. Its parent retains draft,
approval, busy and focus-return ownership. OrganizationPreview similarly owns a
private OrganizationTemplateFields child and colocated preset data.

## Chakra v3

The system is composed in `src/theme/index.ts`. Raw colors and fonts live in
`colors.ts` and `fonts.ts`; semantic colors live in `semantic-tokens.ts`. Button,
Input and Textarea recipes are colocated with their components. NativeSelect
owns its recipe and reuses the Input field recipe. Shared focus styling lives in
`theme/focus-ring.ts`. The existing dark palette and deliberately limited
`defaultBaseConfig` remain in use; this refactor adds no theme preference.

Ordinary forms import Button, Input, NativeSelect and Textarea from their named
component barrels. Their native ref, event, value, numeric size, validation and
form-submission contracts remain explicit. Non-submit actions use
`type="button"`; the Button adapter retains HTML's default submit behavior.
NativeSelect composes Chakra's `NativeSelect.Root` and `NativeSelect.Field` with
`asChild`, preserving a single native select, the OS arrow and keyboard behavior.
It accepts native option children and is distinct from Chakra's collection-based
Select.

The migrated ordinary labeled controls use Field.Root and Field.Label. Existing
label layouts use `asChild` and `unstyled` to retain their established CSS and
native label associations. FileSizeField uses Field.ErrorText for automatically
associated error feedback and keeps native custom validity. Its unit picker has
a separate Field context so an amount error does not mark the unit invalid.
Specialized photographic selection, switches, ranges, map and virtualized
controls retain their native adapters and established geometry.

ESLint rejects legacy Chakra imports and v2 prop names, Stack `spacing`, and
imports through `@/components/<Name>/...`. Direct local recipe imports remain
valid for system registration. New components follow AGENTS.md; existing
specialized CSS is not evidence that a broad visual redesign has been accepted.

## Checks

Run `npm run check`, then `npm run desktop:test:build` and
`npm run desktop:test` sequentially. Build the production frontend again after
instrumented native checks. Use generated fixtures only.

After theme edits, run the pinned one-shot command:

```sh
npx --yes @chakra-ui/cli@3.38.0 typegen ./src/theme/index.ts
```

Generated installed declarations are not a build prerequisite. Verify strict
types and the build against stock declarations after `npm ci --ignore-scripts`.
No CLI dependency or automatic build hook was added.

React Doctor diagnostics and their evidence are tracked in
[REACT_DOCTOR_REVIEW.md](REACT_DOCTOR_REVIEW.md). Scans disable telemetry and
supply-chain queries; this frontend refactor does not change dependencies.
Scanner counts do not establish native behavior, performance or preservation
qualification.
