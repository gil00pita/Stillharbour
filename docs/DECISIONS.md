# Current implementation decisions

This page summarizes decisions reflected in the build. Detailed numbered ADRs
and their original alternatives/evidence are preserved in the
[architecture archive](changelog/2026-11-09/docs/decisions/README.md).
Historical status text is not a new validation result.

| Decision                        | Implementation                                                                                                                        |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Native desktop, modular backend | Tauri 2 shell; React/TypeScript renderer; Rust domain/adapters; local SQLite                                                          |
| Backend safety authority        | Typed IDs/data, root and revision checks, restricted capabilities; no arbitrary renderer shell/SQL/filesystem authority               |
| Read-only default originals     | Catalog annotations, bounded cache work and reviewed new exports; mutation and recovery execution gates stay closed                   |
| Bounded resources               | Supervised media/helper processes, bounded queues/admission, virtualized feeds and versioned cache policy                             |
| Catalog organization            | Stable IDs, version checks, atomic album membership, retained tag history and consistent catalog outputs                              |
| Presentation                    | Chakra v3 component-owned styles/recipes, semantic neutral colors, locally bundled Inter and next-themes system/manual appearance     |
| Map privacy                     | Bundled geometry/places by default; fixed NASA overview requests only after explicit Satellite selection                              |
| Structure review                | Whole-library bounded native previews and session presets; simulation is labelled and performs no filesystem operation                |
| Experimental separation         | Empty default Cargo features; copy/backup workflows and fixture instrumentation require nondefault builds                             |
| Evidence                        | Generated originals, native failure/recovery tests, exact artifact scope and current-head both-platform CI; no implied signed release |

See [Architecture](ARCHITECTURE.md), [Preservation](PRESERVATION_CONTRACT.md),
[Development](DEVELOPMENT.md) and [Release status](RELEASE_STATUS.md) for the active
contracts. Existing decisions retain their original numbers and must not be reused.
Record a material new architecture decision under [docs/decisions](decisions/README.md)
and update this summary only when its implementation is reflected in code.
