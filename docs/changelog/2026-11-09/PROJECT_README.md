> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

<div align="center">

<img src="../../../assets/branding/stillharbour-logo-no-padding.svg" alt="Stillharbour logo" width="96" />

# Stillharbour

**A home for the moments you keep.**

A local-first photo library that keeps your originals in ordinary folders.

[Features](#features) · [Get started](#get-started) · [Development](#development) · [Roadmap](#roadmap) · [Documentation](#documentation)

</div>

**Stillharbour** is a native desktop application built with Tauri, React, Chakra UI and Rust. Browse photos across folders, inspect metadata, and organise favourites, ratings, tags and virtual albums in a local SQLite catalog. Your originals stay where they are, and annotations stay in the catalog. No account, telemetry or cloud service is enabled.

> [!IMPORTANT]
> Stillharbour is in development. Native library evidence covers **macOS Apple Silicon**, with a bounded **Windows local NTFS** scan/preview and EXIF/GPS implementation. Windows network drives, other filesystems and broader formats remain unavailable or unverified. Signed installers and clean-machine release validation remain pending. M2 organization, M3 read-only inspection and M4 comparison/copy/snapshot integration are present. Production copying, renaming, moving, deletion and backup remain gated while native preservation acceptance is collected. See [M4 validation](docs/M4_VALIDATION.md) and the [implementation log](docs/IMPLEMENTATION_LOG.md) for actual checks and open gates.

## Features

- **Browse your collection.** A chronological photo canvas, uniform grid, text-only list, folder explorer and date navigation, with filename search and filters.
- **Keep a local catalog.** Save multiple nonoverlapping photo folders, configure scan filters, follow background jobs, and retain indexed records and available cached previews when a root goes offline.
- **Inspect and annotate.** View metadata and date provenance; add favourites, ratings and tags individually or in batches without rewriting originals or sidecars.
- **Organise in the catalog.** Save captions and reject flags in Info; use Catalog or Add to album for virtual albums, saved searches and explicit tag categories. Preview and apply exact tag rename/merge/removal, undo retained tag changes, and explicitly import observed keywords. Export reviewed portable catalogs or XMP into new files in a separately selected folder. Source files stay unchanged. See [metadata scope and evidence](docs/M5_METADATA.md).
- **Compare backup scope.** Review the base library against a separate destination dataset using quick or full-byte comparison, save immutable comparison profiles, review historical destination edits and inspect read-only previews. This read-only M4A increment cannot copy, replace or establish recovery evidence. See [M4 status](docs/M4_BACKUP_SYNC.md).
- **Look closer.** Navigate photos with zoom, pan, filmstrip and Info, or visually compare 2–4 selected photos. Comparison does not establish duplicate equality.
- **Explore locations.** Browse retained photo locations on an offline world overview, filter by folder/date, select thumbnail clusters and show the same area in the library. Existing folders need an explicit rescan to acquire previously unextracted GPS; Windows reads bounded EXIF/GPS through the supervised worker. Format and platform limits remain explicit in [map scope and evidence](docs/M5_MAP.md).
- **Plan organization.** Select up to 240 individual photos and try date, event and sequence templates. Save immutable template versions with up to four validated examples and destination-bound drafts; review existing target conflicts and selected source folders. Reopen Saved plans and view or save new JSON/CSV review files. Future-name checks, complete companion bundles, execution, import and undo remain pending. Originals stay in place.

Preview support includes JPEG, PNG and WebP, with tested HEIC/HEIF, TIFF and selected RAW camera samples on macOS. The viewer can display admitted JPEG/PNG/WebP originals and supported HEIC/HEIF originals on compatible macOS WebViews, within a 32 MiB / 32-million-pixel limit. Other cases retain a labelled preview or an explicit unavailable state. See [format coverage](docs/FORMAT_SUPPORT.md) and [original-image viewing](docs/ORIGINAL_IMAGE_VIEWING.md) for exact limits.

## Get started

These instructions run the application from source. Signed installers and clean-machine platform support remain release gates.

### Prerequisites

- **Node.js 24 or newer compatible version**, with **npm 11.16.0**, as recommended by the [development guide](docs/DEVELOPMENT.md).
- **Rust via rustup.** The repository pins **Rust 1.99.1**, including rustfmt and Clippy, in [rust-toolchain.toml](../../../rust-toolchain.toml).
- **Native Tauri prerequisites** for your operating system: follow the [Tauri setup guide](https://v2.tauri.app/start/prerequisites/). Functional library testing currently requires macOS Apple Silicon.

From the repository root:

```sh
npm ci
npm run desktop:dev
```

This launches the native desktop app. `npm run dev` starts only the frontend server; native features require Tauri, and the browser reports the missing native connection.

### Explore a library

1. Choose a photo folder, review its scan settings, then select **Save & Start scanning**.
2. Follow discovery and preview preparation in the progress banner and **Jobs**. Browse Photos, Folders or Tags once the collection is ready.
3. Open a photo to inspect it, or select photos to add catalog annotations and preview organization templates.

The built-in File Extensions preset includes `.mp4`; existing saved filter lists retain their scope. Discovery does not establish video preview or playback support.

Use generated, disposable media for development tests. Debug catalogs are isolated under `.context/native-data`; production builds use the local application-data directory. Even folders marked **Managed** currently only read originals.

> [!NOTE]
> A catalog holds references, metadata and annotations. It is not a backup of your photos. Original-file operations remain gated on reviewed plans, recovery evidence and native failure tests; selecting a folder does not authorize those operations.

## Development

See [TypeScript and Chakra conventions](docs/TYPESCRIPT_STRUCTURE.md) for component folders, hooks, imports and theme recipes.

| Command                                 | Purpose                                                                                                              |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `npm run desktop:dev`                   | Start the native application in development mode                                                                     |
| `npm run check`                         | Check formatting, lint, strict types, component tests, IPC bindings, Rust formatting/Clippy/tests and frontend build |
| `npm run desktop:test:build`            | Build the instrumented native test application                                                                       |
| `npm run desktop:test`                  | Run native WebView tests with owned, generated fixtures                                                              |
| `npm run desktop:build`                 | Build the normal unsigned macOS `.app` bundle                                                                        |
| `npm run release:check`                 | Check production permissions, instrumentation exclusions and reviewed helper resources                               |
| `npm run release:smoke -- <executable>` | Probe a built executable's decoder and in-memory SQLite without opening a user catalog                               |
| `npm run test:release`                  | Run production-preflight and dependency regressions, also included in `npm run check`                                |

Run the native test build and suite sequentially after `npm run check`; they share frontend build output. Test instrumentation must stay out of production. Additional fixture, format, helper and performance probes are documented in [Development](docs/DEVELOPMENT.md).

### Project layout

The React/TypeScript interface uses Chakra UI 3.37.0 with a Stillharbour token/recipe layer; see [UI framework](docs/UI_FRAMEWORK.md). It sends typed requests to the Rust backend, which owns filesystem access, catalog writes, workers and safety checks. Business rules remain separate from I/O adapters; IPC bindings are generated from Rust-owned types.

```text
src/                 React/TypeScript interface and generated IPC bindings
src-tauri/           Tauri desktop shell, permissions and native protocols
crates/core/         Rust domain rules, SQLite catalog, jobs and media workers
scripts/             Development, validation and diagnostic tools
tests/               Native integration tests and generated-fixture harnesses
assets/branding/     Owner-supplied Stillharbour logo
vendor/exiftool/     Reviewed metadata helper resources
docs/                Specifications, decisions and validation evidence
tasks/               Bounded implementation milestones
templates/           Data-only example presets and manifests
```

## Roadmap

Work proceeds through reviewed milestones. Individual later slices, such as macOS format previews, do not complete their parent milestone.

| Milestone                                                                    | Scope and current position                                                                                                                                         |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [M0 — Foundation](tasks/M0_FOUNDATION.md)                                    | Implemented foundation with recorded local evidence; cross-platform gates remain                                                                                   |
| [M1 — Read-only library](tasks/M1_READ_ONLY_LIBRARY.md)                      | Implemented native browsing and catalog annotations; platform and performance gates remain                                                                         |
| [M2 — Organization and import](tasks/M2_SAFE_OPERATIONS.md)                  | In progress: previews, companion review, destination preflight, saved template versions and immutable drafts; execution/import/undo gates open                     |
| [M3 — Duplicates, health and storage](tasks/M3_DUPLICATES_HEALTH_STORAGE.md) | Read-only Duplicates, Integrity and Storage; production quarantine awaits M4 recovery                                                                              |
| [M4 — Backup and restore](tasks/M4_BACKUP_RESTORE.md)                        | M4A accepted; copy, retained history and snapshots integrated, with production execution gated on native preservation acceptance                                   |
| [M5 — Maps, metadata and formats](tasks/M5_MAP_METADATA_FORMATS.md)          | Development gate complete; Windows/macOS native map/metadata/export acceptance passes; unverified formats and release limits in [M5 closeout](docs/M5_CLOSEOUT.md) |
| [M6 — Release hardening](tasks/M6_RELEASE_HARDENING.md)                      | In progress: production preflight, safety review and 125-scenario evidence index; signing, clean-machine and release acceptance remain blocked                     |

Catalog portability, broader health checks, editing, provider imports and mobile are separately gated [library extensions](docs/LIBRARY_FEATURES_ADDENDUM.md). They are planned work, not current capabilities. [Performance status](docs/PERFORMANCE_P4_STATUS.md) records the remaining P4-06 gate.

## Documentation

- [User guide](docs/USER_GUIDE.md), [M6 readiness](docs/M6_RELEASE_READINESS.md) and [release evidence matrix](docs/M6_EVIDENCE_MATRIX.md) — current workflows, disabled capabilities and release gaps.

- [M6 dependency hardening](docs/M6_DEPENDENCY_HARDENING.md) — reviewed development overrides, regression evidence and current advisory results.

- [Product specification](docs/CODEX_SPEC.md) and [decisions](docs/DECISIONS.md) — requirements and implementation choices.
- [Preservation contract](docs/PRESERVATION_CONTRACT.md) — original-file safety and recovery requirements.
- [Development guide](docs/DEVELOPMENT.md) and [dependency inventory](docs/DEPENDENCIES.md) — setup, commands and helper boundaries.
- [M1 validation](docs/M1_VALIDATION.md) and [M2 previews](docs/M2_ORGANIZATION_PREVIEWS.md) — implemented behavior and limits.
- [Acceptance scenarios](docs/ACCEPTANCE_TESTS.md) and [implementation log](docs/IMPLEMENTATION_LOG.md) — required tests and recorded outcomes.
- [Architecture decisions](docs/decisions/README.md) and [product identity](docs/PRODUCT_IDENTITY.md) — design rationale and branding conventions.
