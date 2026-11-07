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
private OrganizationTemplateFields child. OrganizationPresets owns shared date-based
preset data, SVG examples and its slot recipe; SegmentedControl owns the typed
segmented radio-group adapter and recipe.

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

## Complete presentation migration — 2026-11-07

Applicable elements throughout the component tree now use Chakra Box, Text,
Heading, Image, Button/IconButton, List and Table compound components; semantic
sections/forms/fieldsets use the Chakra factory. Use unstyled recipe components
where established native CSS still owns specialized geometry. Native asChild
children, dialog/details/summary, SVG, bidirectional filenames and checkbox/range
controls are intentional exceptions. The retained numbered list uses List.Root
asChild around ol to preserve its native start offset.

SearchField shares the Photos/Map search frame with Albums and Folders. BackupScreen
owns snapshots; SyncScreen owns the former combined page's comparison workflow.
StructureFormatter owns whole-library rule configuration, cancellable paged
native preview state and result paging. It consumes no photo IDs or gallery
query. OrganizationTemplateFields is shared through the OrganizationPreview
barrel; the separate selected-photo preview keeps its existing planning contract.
The migration adds no runtime dependency or theme token. The detailed inventory
is in [UI_WORKSPACE_AUDIT.md](UI_WORKSPACE_AUDIT.md).

Chakra Table.Caption defaults to bottom independently of its recipe. Set
captionSide="top" when migrating native captions so ordering and focused preview
results retain their previous behavior. Native SVG image dimensions use
htmlWidth/htmlHeight on Chakra Image.

The expanded Structure Formater groups 25 choices in OrganizationPresets.
StructurePathBuilder owns its chip vocabulary, pure path-parts helper and recipe.
Metadata context for synthetic examples is generated from Rust's
OrganizationMetadata schema; retainedTemplateExamples validates and preserves
it. Native source context is catalog-owned and cannot be supplied by the rule
form. Register the builder recipe in the theme and typegen after recipe changes.
