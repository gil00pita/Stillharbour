# TypeScript and Chakra conventions

The frontend is React/TypeScript with Vite and Chakra UI v3. Keep strict
TypeScript and the existing feature boundaries. [AGENTS.md](../AGENTS.md) contains
the detailed coding standards; [architecture](ARCHITECTURE.md) describes native
authority and IPC ownership.

## Components and imports

Application components live in `src/components/<Name>/<Name>.tsx`, with an
`index.ts` barrel. Consumers import `@/components/<Name>`. Colocate recipes,
component tests and stories; private children live in the owner's `components/`
directory. Shared hooks use `src/hooks/use-*.ts`, with their tests nearby.
Pure feature models and integration tests stay in their existing feature modules.

`src/main.tsx` bootstraps the app. `src/components/ui/Provider.tsx` and
`color-mode.tsx` compose Chakra and next-themes. `src/test/render.tsx` supplies the
same provider for component tests. Vite and TypeScript both resolve `@` to `src`.

## Theme and styles

`src/theme/index.ts` composes the system from `defaultBaseConfig` and the product
configuration. Raw colors/fonts live in their own files; semantic colors in
`semantic-tokens.ts` define light/dark meaning. Use tokens before inventing values.
Appearance follows the system until a saved manual choice takes precedence.

Use inline Chakra props, local `css` objects and colocated recipes. Register
component recipes in the system rather than centralizing implementation in a
global selector dump. Keep document defaults/keyframes in the theme. Dynamic
virtual coordinates stay inline; dependency-owned MapLibre/font stylesheets stay
imported. Do not add application CSS files for ordinary component styles.

Use v3 compound components, `colorPalette`, `gap`, plain boolean state props,
details-object callbacks, `Field.Root` and `asChild`. Keep overlays portalled.
Do not use Chakra v2 imports/props, `sx`, removed icon packages or Emotion styled
wrappers. ESLint rejects legacy imports/props and deep component-barrel imports.

After theme edits, use the pinned type-generation command:

```sh
npx --yes @chakra-ui/cli@3.38.0 typegen ./src/theme/index.ts
```

Compilation must also work against stock installed declarations after a clean
install. No generated `node_modules` declaration is a hidden build prerequisite.

## Native form and photo contracts

Ordinary controls use the shared Button, Input, NativeSelect and Textarea
adapters. Keep refs, controlled values, disabled states, validation and native
submission behavior. Non-submit actions explicitly use `type="button"`; the
Button adapter otherwise preserves HTML's submit default.

NativeSelect composes v3 Root/Field around one native select. It accepts native
options and is separate from collection-based Select. FileSizeField uses
InputGroup with an embedded amount/unit control and keeps amount errors associated
through Field. Icon-only buttons need accessible names and visible focus states.

Keep specialized checkboxes, ranges, SVGs, dialogs/details/summary and dynamic
photo/map geometry semantic. Reduced-motion, selection/busy guards and truthful
preview/error states are part of behavior. An album picker explicitly enables
selection before any photo is chosen; ordinary Photos uses hover/focus reveal.

## Verification

Follow [Development](DEVELOPMENT.md#validation-and-ci): types, lint, relevant
component/consumer cases, build for output changes and focused native interaction
checks. Shared theme changes broaden the consumer scope. Browser styles and
component tests do not establish native focus, Windows or release acceptance.
