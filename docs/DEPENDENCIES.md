# Dependencies and reviewed resources

The exact JavaScript and Rust graphs are pinned in
[package-lock.json](../package-lock.json) and [Cargo.lock](../Cargo.lock).
Direct declarations are in [package.json](../package.json),
[Cargo.toml](../Cargo.toml), [core](../crates/core/Cargo.toml) and
[shell](../src-tauri/Cargo.toml). Update declarations and lockfiles together.

## Main implementation dependencies

| Component                    | Repository pin         | Purpose                                                   |
| ---------------------------- | ---------------------- | --------------------------------------------------------- |
| Rust toolchain               | 1.99.1                 | rustfmt, Clippy and native compilation                    |
| Tauri / tauri-build / dialog | 2.12.5 / 2.7.4 / 2.8.3 | Desktop shell and native picker                           |
| React / React DOM            | 19.3.1                 | Renderer                                                  |
| Chakra UI / Emotion          | 3.37.0 / 11.14.0       | Component styling, semantic tokens and recipes            |
| next-themes / Inter Variable | 0.4.6 / 5.2.8          | Local appearance and locally shipped fonts                |
| Vite / TypeScript            | 8.3.5 / 6.0.3          | Frontend build and strict types                           |
| MapLibre GL JS               | 6.16.0                 | Bundled-map and explicit satellite rendering              |
| rusqlite                     | 0.41.0                 | Bundled SQLite, backup API and query functions            |
| image                        | 0.25.10                | Baseline decoder with only JPEG/PNG/WebP features enabled |
| Tokio / ts-rs                | 1.54.1 / 12.0.1        | Native work orchestration and generated IPC types         |
| WebdriverIO / Tauri service  | 10.1.0 / 1.5.1         | Development-only native automation                        |

Rust release builds use thin LTO and stripped symbols. Test fixture and workflow
features are nondefault; release preflight checks the resolved feature graph.
Do not install a second styling/state framework or change native helpers without
an explicit need and review.

## Helper and data provenance

| Resource                      | Canonical inventory                                                                                                                     |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| ExifTool 13.55                | [vendor/exiftool-manifest.json](../vendor/exiftool-manifest.json): exact reviewed files, hashes, runtime and Artistic/GPL license terms |
| restic 0.19.1 / rclone 1.75.2 | [vendor/backup-helper-manifest.json](../vendor/backup-helper-manifest.json): host-specific archives/executables, hashes and licenses    |
| Helper notices                | [vendor/backup-licenses](../vendor/backup-licenses) and [ExifTool notices](../vendor/exiftool)                                          |
| Font/appearance notices       | [public/licenses](../public/licenses)                                                                                                   |
| Natural Earth geometry        | [assets/maps/manifest.json](../assets/maps/manifest.json) and [license](../assets/maps/NATURAL_EARTH_LICENSE.md)                        |
| Local places                  | [assets/places/manifest.json](../assets/places/manifest.json)                                                                           |
| Optional NASA imagery         | [assets/maps/satellite-provider.json](../assets/maps/satellite-provider.json)                                                           |
| Owner mark                    | [assets/branding/stillharbour-logo.svg](../assets/branding/stillharbour-logo.svg)                                                       |

Prepare helpers with `node scripts/backup-helpers.mjs --download`, then verify
with `--check`. Build-time downloads use pinned checksums; application startup
does not download replacements. Do not trust PATH or silently substitute a
changed helper. ExifTool's system-Perl arrangement on macOS retains its
clean-machine/minimum-OS qualification limits.

## Advisory and distribution checks

CI runs `npm audit --audit-level=low` and pinned `cargo-audit 0.22.2` against the
locked graphs. Dependency-policy regressions validate reviewed npm overrides.
No historical zero-advisory count is a claim about a later registry database or
shipped package. Refresh advisory/license/resource review for actual release
artifacts and report informational or unsupported-platform warnings explicitly.

`npm run release:check` verifies the default dependency/features and reviewed
resources. It is a preflight, not signed/installed-package certification.
Earlier acquisition experiments and audit results are retained in the
[archived inventory](changelog/2026-11-09/docs/DEPENDENCIES.md).
