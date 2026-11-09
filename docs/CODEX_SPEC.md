# Build capabilities

This is the current behavior reference for the code in this repository. The
original proposal and subsequent milestone specifications are preserved in the
[changelog snapshot](changelog/2026-11-09/README.md).

## Default application

Stillharbour is a native Tauri 2 desktop application with a React/TypeScript UI,
Chakra UI v3 styling, a Rust core and a local SQLite catalog. Originals remain in
their selected folders. Catalog annotations and derived previews live outside
source roots. No account, telemetry or cloud library integration is enabled.

| Area                    | Implemented behavior                                                                                                                                                                        |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Library                 | Native folder selection, nonoverlapping root registration, scan filters, discovery, preview preparation, pause/resume/cancel, reconnect and retained offline observations                   |
| Photos                  | Timeline, uniform grid, image-free text list, folder browsing, bounded feeds, indexed date navigation, filename/date/place search and query filters                                         |
| Presentation            | Bundled Inter, neutral Chakra styles, system appearance on first use, saved light/dark override, separate Display options and Filter panels, thumbnail scale and floating selection actions |
| Viewer                  | Admitted original bytes or explicit preview fallback, zoom/pan, filmstrip, Info, filtered navigation and return to the invoking photo                                                       |
| Catalog organization    | Favourites, ratings, tags, captions, reject flags, virtual albums, tag-derived albums, saved searches and reviewed tag-history operations                                                   |
| Exports                 | Reviewed new portable SQLite catalog and XMP outputs with native destination checks; no original media in a catalog export and no source-sidecar rewrite                                    |
| Map                     | Bundled offline world geometry, local city/country search, indexed GPS clusters, explicit transfer to Photos and optional NASA satellite overview                                           |
| Utilities               | Read-only duplicate evidence, integrity inspections, storage accounting and source/destination Sync comparison, with retained reports and explicit retry/error states                       |
| Structure               | Whole-library paged path preview, date/location/tag/camera/custom rules, session presets, review filters and a revalidated simulation that changes no files                                 |
| Selected-photo planning | Bounded template previews, companion/destination review, retained template versions and immutable plan drafts; saving or exporting a draft grants no execution authority                    |
| Jobs and Settings       | Actual job state/history, supported job controls, library settings, preview-cache maintenance and saved browsing/import-destination preferences                                             |

The [user guide](USER_GUIDE.md) describes these workflows. Platform and decoder
limits are in [Format support](FORMAT_SUPPORT.md). A missing or unsupported field
is not filled with invented metadata or production sample data.

## Closed execution gates

The shell reports `originalMutationsEnabled: false`. Both owned Cargo packages
have empty default feature sets. Production copy, in-place rename/move,
quarantine/purge, backup snapshots, restore and recurring execution remain
disabled pending their preservation and native acceptance gates.

`copy-workflow` and `backup-workflow` contain experimental implementation.
`desktop-test` additionally enables generated fixtures and WebDriver
instrumentation. These features are not accepted default production build inputs.
Their tests do not authorize operations on a real user library.

Basic image editing, provider-account imports and mobile applications are not
implemented product capabilities. Their earlier plans live only in the archive.
No release, filesystem or format claim follows from a navigation label, a fixture,
a responsive page or an enabled experimental compile flag.

## Binding constraints

- Follow the [preservation contract](PRESERVATION_CONTRACT.md). Read-only work
  leaves original bytes, names and descriptive metadata unchanged.
- Rust owns filesystem authority, database writes, credentials, worker admission
  and validated native identities. Renderer commands carry typed data and IDs.
- Keep queues, concurrency, feeds, cache keys and worker resources bounded.
  Offline and inaccessible scopes must not be interpreted as empty.
- Preserve typed failures, retained baselines and truthful coverage. A checksum
  mismatch proves change, not damage; a thumbnail proves neither integrity nor recovery.
- Keep test ports and fixture adapters out of default production builds.
- [Release status](RELEASE_STATUS.md) governs unqualified distribution, recovery,
  performance and accessibility claims. Historical acceptance obligations remain
  preserved; this documentation reorganization does not relax them.
