> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0098: Neutral Chakra presentation and local appearance

Date: 2026-11-07. Status: accepted for the owner's requested UI redesign.

The audit branch already uses Chakra v3 while unlayered application CSS still
owns much of the layout and geometry. The owner requests inline Chakra styles
and supplies a neutral photo-workspace brief. Preserve the Tauri 2 application,
existing native workflows, owner mark and all original-file safety gates.

Move owned component CSS into local Chakra props/objects and colocated recipes.
Keep document/native defaults and shared keyframes in the theme; keep virtual
coordinates in existing dynamic inline values to avoid unbounded generated CSS.
Retain dependency MapLibre/font stylesheets. A central TypeScript dump of all
component selectors would keep global ownership problems, so it is rejected.

Use semantic light/dark cool-neutral tokens, self-hosted Inter Variable 5.2.8
(OFL-1.1) and next-themes 0.4.6 (MIT). The latter is Chakra's documented appearance
composition, not a second application state store. The owner selected system
appearance as the default; saved manual light/dark choices retain precedence. Keep its SSR script inert in
this client-rendered desktop; theme effects set the document class. No CSP or
native capability widening is needed. Preserve both dependency notices in the
frontend distribution. Lockfile integrity pins both packages.

Keep real catalog favourites, existing display sizes and metadata rather than
the brief's Electron implementation or fabricated sample data. No runtime,
catalog/schema, credential, root, helper or file-operation migration occurs.
Validate actual portals, minimum-width layout, keyboard/focus behavior, native
favourites and generated-source preservation; report Windows/CI/package gates
separately. See [the delivered contract](../NEUTRAL_UI.md) and implementation log.

Primary references: [Chakra CSS variables](https://chakra-ui.com/docs/styling/css-variables),
[Chakra colour mode](https://chakra-ui.com/docs/components/concepts/color-mode),
[next-themes source](https://github.com/pacocoursey/next-themes),
[Inter source and license](https://github.com/rsms/inter).
