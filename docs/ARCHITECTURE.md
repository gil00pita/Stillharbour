# Architecture

Stillharbour is a modular desktop application. The renderer presents catalog
state; the Rust backend is the authority for native access and safety decisions.

## Runtime boundaries

| Layer               | Implementation                                                                                               | Responsibility                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| Interface           | [src/components](../src/components), [shared hooks](../src/hooks)                                            | Typed requests, local drafts, navigation, accessible controls and bounded photo presentation                         |
| Desktop shell       | [src-tauri/src/main.rs](../src-tauri/src/main.rs), [capabilities](../src-tauri/capabilities/foundation.json) | Native startup, root selection, command admission, credentials and guarded protocols                                 |
| Domain and adapters | [crates/core/src](../crates/core/src)                                                                        | Catalog, business rules, read-only inspections, worker scheduling, operation planning and native filesystem adapters |
| Storage             | SQLite and derived caches                                                                                    | Stable catalog identity, observations, annotations, jobs and durable records; previews do not replace originals      |
| Workers             | Supervised native media/helper processes                                                                     | Bounded decoding and metadata work outside UI and async-executor threads                                             |

IPC bindings in [src/bindings.ts](../src/bindings.ts) are generated from Rust-owned
types. `npm run bindings:check` checks drift. Do not allow arbitrary shell, SQL,
executable paths or unapproved filesystem destinations from the renderer.

## Catalog and source ownership

The catalog is a local SQLite database. The backend owns writes, bounded request
admission and its lifetime lock. Catalog startup and recovery validate persisted
identity and activation history; an inaccessible credential or unknown catalog
must not become a new empty catalog. Keep catalog UUIDs, native application IDs,
root IDs and credential scopes stable when changing display names.

Registered roots retain identity, mode, filters and availability. Read-only
reconciliation retains uncertainty for inaccessible or incomplete scopes.
Metadata and preview observations are revision-bound. Derived cache publication
does not prove full-media validity or original accessibility. Root registration,
catalog-only edits and reviewed exports use different authorities.

New catalog/XMP exports require reviewed native destinations, source checks and
no-replace publication. They never rewrite embedded source metadata by default.
Portable catalogs omit credentials, live root permissions and original media.

## Interface and map

Chakra styles live with their components; semantic tokens live in `src/theme/`.
`next-themes` applies local appearance without an executable inline bootstrap.
Inter font files ship locally. Dynamic virtual coordinates remain bounded inline
values. The only retained application CSS imports are dependency-owned font and
MapLibre stylesheets.

Photos and other browsers share typed catalog scopes and bounded feeds. Display
options are independent of query filters. Photo selection and saved plans grant
no mutation authority. Structure Formater queries the whole catalog independently
of Photos' selection and uses bounded native result pages.

MapLibre uses bundled Natural Earth geometry. City/country lookup uses bundled
data. Only explicit Satellite selection admits the fixed NASA overview path;
the request transformer and CSP restrict that path. MapLibre receives concrete
palette colors rather than Chakra token-expression strings.

## Build boundaries

[Shell features](../src-tauri/Cargo.toml) and [core features](../crates/core/Cargo.toml)
default to empty. `copy-workflow` and `backup-workflow` contain experimental
execution. `desktop-test` enables those workflows plus constructor-owned fixtures
and WebDriver instrumentation. Default release preflight rejects experimental
features, test routes and instrumented resources.

Native test adapters accept only their generated fixture scopes. Passing those
tests does not enable real-library mutation or qualify a signed package.
See [Preservation](PRESERVATION_CONTRACT.md), [Development](DEVELOPMENT.md) and
[Release status](RELEASE_STATUS.md).
