<div align="center">

<img src="assets/branding/stillharbour-logo.svg" alt="Stillharbour logo" width="96" />

# Stillharbour

**A home for the moments you keep.**

A local-first desktop photo organizer that keeps originals in ordinary folders.

[User guide](docs/USER_GUIDE.md) · [Development](docs/DEVELOPMENT.md) · [Documentation](docs/README.md)

</div>

Stillharbour uses Tauri 2, React, Chakra UI and a Rust backend with a local SQLite
catalog. It browses indexed folders, stores annotations and albums in the catalog,
and performs read-only inspections. No account or telemetry is enabled.

The application is in development. Production copy, rename, move, duplicate
quarantine, snapshot, restore and scheduling execution remain disabled. Signed
distribution and clean-machine acceptance are incomplete. See
[build capabilities](docs/CODEX_SPEC.md) and [release status](docs/RELEASE_STATUS.md).

## What you can use

- Browse Photos as a timeline, uniform grid or image-free text list; navigate
  indexed folders and filter by filename, date, place and catalog properties.
- View admitted originals or cached previews with zoom, pan, filmstrip and Info;
  compare selected photos without treating visual similarity as byte equality.
- Save favourites, ratings, tags, captions, reject flags, virtual albums and saved
  searches without rewriting originals or source sidecars.
- Create reviewed portable-catalog and XMP exports in new, separately selected
  outputs. A catalog export does not contain the original photos.
- Browse indexed GPS on a bundled offline map, use local place search, or
  explicitly enable NASA satellite overview imagery.
- Run read-only Duplicates, Integrity, Storage and Sync comparisons; inspect
  retained reports and background jobs.
- Preview folder structures across the catalog, review proposed paths and
  simulate a plan. Actual restructuring remains disabled.

Format and platform coverage is bounded. See [format support](docs/FORMAT_SUPPORT.md).
An indexed file or cached thumbnail is not proof that its original is accessible,
intact or backed up.

## Run from source

Use Node.js 24, npm 11.16.0, the Rust toolchain pinned in
[rust-toolchain.toml](rust-toolchain.toml), Python 3 and your platform's
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
node scripts/backup-helpers.mjs --download
node scripts/backup-helpers.mjs --check
npm run desktop:dev
```

Helper preparation verifies the repository's pinned resources; it is a build-time
step, not an application download. See [Development](docs/DEVELOPMENT.md) for
platform setup, validation and isolated native test builds.

`npm run dev` starts the frontend server alone. Native filesystem and catalog
features require the Tauri application. Use generated disposable media for
development tests; never use a personal library as a fixture.

## Repository

| Path                 | Contents                                                     |
| -------------------- | ------------------------------------------------------------ |
| `src/`               | React/TypeScript UI and generated IPC bindings               |
| `src-tauri/`         | Native shell, command permissions and media protocols        |
| `crates/core/`       | Catalog, domain rules, filesystem adapters, jobs and workers |
| `scripts/`, `tests/` | Build tools, checks and generated-fixture harnesses          |
| `assets/`, `vendor/` | Reviewed branding, map/place data and helper resources       |
| `docs/`              | Current behavior, contributor guidance and release limits    |
| `docs/changelog/`    | Previous specifications, plans, decisions and dated evidence |

Start at the [documentation index](docs/README.md). Previous milestone plans and
the full earlier implementation history are preserved in the
[documentation changelog](docs/changelog/README.md).
