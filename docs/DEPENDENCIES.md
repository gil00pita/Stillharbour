# Dependency and helper inventory

Recorded 2026-10-27. Exact Rust/JavaScript dependency graphs and registry checksums are in Cargo.lock and package-lock.json. Owned application source is MIT; dependencies retain their own licenses. License metadata collected locally covers 268 resolved macOS Rust packages and 761 npm entries in `.context/evidence/license-inventory.json`. This is not a completed distribution notice/legal audit.

| Component                                      | Pin                                        | License / purpose                                                                                                                                                                                   |
| ---------------------------------------------- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust toolchain                                 | 1.99.1                                     | MIT/Apache-2.0; rustfmt and clippy included                                                                                                                                                         |
| Tauri / tauri-build / dialog                   | 2.12.5 / 2.7.4 / 2.8.3                     | MIT/Apache-2.0; native shell, explicit command manifest and Rust-owned picker                                                                                                                       |
| React / React DOM                              | 19.3.1                                     | MIT; local renderer                                                                                                                                                                                 |
| Vite / TypeScript                              | 8.3.5 / 6.0.3                              | MIT / Apache-2.0; strict renderer build                                                                                                                                                             |
| rusqlite / libsqlite3-sys                      | 0.41.0 / 0.39.1                            | MIT / SQLite public domain; bundled runtime reports **3.54.1**                                                                                                                                      |
| image                                          | 0.25.10                                    | MIT/Apache-2.0; defaults off, JPEG, PNG and WebP enabled                                                                                                                                            |
| Tokio                                          | 1.54.1                                     | MIT; bounded admission and supervised child processes                                                                                                                                               |
| ts-rs                                          | 12.0.1                                     | MIT; generated IPC declarations                                                                                                                                                                     |
| windows-sys                                    | 0.61.2                                     | MIT/Apache-2.0; Windows Foundation/FileSystem/Security, System IO and Wdk FileSystem bindings for local NTFS reads and owned catalog snapshot publication; system Kernel32/Ntdll, no bundled binary |
| serde / serde_json                             | 1.0.229 / 1.0.154                          | MIT/Apache-2.0; typed contract data                                                                                                                                                                 |
| thiserror / uuid / fs2 / tempfile / libc       | 2.0.21 / 1.29.0 / 0.4.3 / 3.27.0 / 0.2.191 | MIT/Apache-2.0 (fs2 MIT/Apache-2.0; libc also BSD-2-Clause); errors, IDs, locks, owned staging, native primitives                                                                                   |
| WebdriverIO / Tauri service and plugins        | 10.1.0 / 1.5.1                             | MIT; development/test only                                                                                                                                                                          |
| ESLint / typescript-eslint / Vitest / Prettier | 10.13.0 / 8.72.1 / 5.0.5 / 3.9.10          | MIT; development checks                                                                                                                                                                             |

Native build flags live in Cargo.toml: edition 2024, exact direct versions, thin release LTO and stripped symbols. M1 runs the packaged app executable in an early isolated stdin-worker mode for previews, before Tauri starts. The standalone media-worker binary remains a development probe. The original supplied SVG is unchanged; desktop icons were rasterized with `tauri icon ... --fit contain`. Reference screenshots are excluded from frontend imports and bundle resources.

## Advisory review

`cargo-audit 0.22.2` checked the lockfile against RustSec commit `99670cfc0bc574433034b75876a4b791ccaa2efb`: no vulnerability entries; informational warnings remain for unmaintained `proc-macro-error 1.0.4` and unsound `glib 0.18.5` in the cross-platform lockfile. Review the Linux/GTK dependency chain before any Linux support; it is outside the selected release targets. No advisory IDs were ignored.

`npm audit --omit=dev` found zero production advisories. The full scan initially found 19 high-severity package entries. Pinning the compatible patched `basic-ftp 6.2.3` override reduced this to **9 high-severity development entries**, rooted in `extract-zip 2.0.1` through WebdriverIO/browser-download tooling. Upstream lists [symlink traversal](https://github.com/advisories/GHSA-jmr9-qjv8-65gv) and [symlink file writes](https://github.com/advisories/GHSA-7pqw-9j4j-h8q3); no patched extract-zip release was available in the checked registry. Do not use these tools to extract untrusted archives. Native tests use an already-built binary and disable automatic driver downloads. This is a recorded development-tool risk, not a claim that the whole dependency tree is clean. Recheck before CI/release expansion; do not force npm's suggested incompatible WebdriverIO 8 downgrade.

## Development helpers actually exercised

Direct upstream release downloads stalled, so the local probes used Homebrew's checksummed arm64 macOS 27 bottles. `brew fetch --force-bottle` verified them; their SHA-256 values were also compared with Homebrew formula metadata. Only disposable `.context/helpers` copies were extracted. M0 bundled none of these helpers. M1 vendors the reviewed ExifTool code and never downloads helpers at runtime.

| Helper   | Version / architecture                  | Verified bottle SHA-256                                            | License                                   |
| -------- | --------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------- |
| ExifTool | 13.55 (bottle revision 1), darwin arm64 | `b2be112059660150c63351003a49503472acca8ff37cea2d14f16fad106d9909` | Artistic-1.0-Perl OR GPL-1.0-or-later     |
| restic   | 0.19.1, darwin arm64, Go 1.27.1         | `a5a348df1e0bc9d1b6048141e5138442178c54c624c1be9a175a6cafce59901c` | BSD-2-Clause plus bundled Go dependencies |
| rclone   | 1.75.2, darwin arm64, Go 1.27.2         | `ec1a9edc575668f7e270d6d85031043bbc9e864ed2bc319131f70fc2e3297b80` | MIT plus bundled Go dependencies          |

The extracted ExifTool launcher needed Homebrew's documented interpreter placeholder relocated to `/usr/bin/perl`, and an explicit private module path. That development arrangement does **not** prove standalone packaging or the provisional macOS 13 target. The website listed ExifTool 13.59; that source download was not validated or executed. Do not substitute its version in evidence for the 13.55 executable actually tested.

Observed local executable SHA-256 after development relocation:

```text
ExifTool 28c9e97fe540b5e32252ee2ef2028c5551c833643dffdbd5d482690df5488989
restic   d703334cda5470dfd0bc3c61630988d6be310b4fc5c6a31486d71f7f35c2d24c
rclone   9fc6a483ccc55f98436a952710d27f816b034f7c54f772e0e130b41997eaf8b5
```

## Packaged helper plan and release matrix

Use a reviewed manifest per target triple: `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`; optionally add `x86_64-apple-darwin` only after native tests. Each entry must pin upstream version, URL/source commit, archive and executable checksum, architecture, deployment target, compiler/build flags, license and complete notices. Build-time acquisition must verify these values; never download tools at first launch or trust PATH in production.

Put approved executables under Tauri's target-suffixed `binaries/<name>-<target-triple>` convention and list only reviewed artifacts in `bundle.externalBin`. Resolve the immutable bundled path in Rust; invoke explicit argv with bounded output/time and a controlled working directory/environment. Keep all shell/process permission out of the renderer. Sign nested helpers before the enclosing app. A missing helper is an explicit unavailable capability, never silent PATH fallback.

| Adapter        | macOS Apple Silicon plan                                                                                                      | Windows 11 x64 plan                                                        | Current gate                                                                                                                               |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Decoder worker | App executable has an isolated early stdin-worker mode; sign the enclosing app                                                | Build and package same worker; restricted job/process policy               | JPEG/PNG/WebP generated fixtures and native preview protocol exercised on macOS and Windows local NTFS; clean-machine/distribution pending |
| ExifTool       | Bundle reviewed Perl code with a private relocatable runtime or reviewed standalone launcher; preserve private library layout | Official Windows distribution with its complete required runtime directory | M1 bundles checked Perl code; hostile-name/config and native stdin metadata probes pass locally; clean machine and minimum OS pending      |
| restic         | Pinned upstream darwin arm64 binary plus notices/SBOM, deployment-target check                                                | Pinned Windows amd64 binary plus notices/SBOM                              | Local repository/check/restore passed; independent recovery and every remote transport pending                                             |
| rclone         | Pinned darwin arm64 binary with allowlisted adapters/config channel                                                           | Pinned Windows amd64 binary with same contract                             | Local inventory only; newline encoding and protocol/fidelity tests pending                                                                 |
| RAW            | Evaluate pinned LibRaw adapter/embedded-preview pipeline, preserve camera matrix                                              | Build same reviewed provider for MSVC                                      | No provider compiled, packaged or claimed supported                                                                                        |
| HEIC/HEIF      | Evaluate native ImageIO with tested color/orientation and capability detection; compare explicit libheif option               | Evaluate libheif plus reviewed decoder dependencies, notices and licensing | No HEIF provider or camera fidelity claim; licensing and fixtures block enablement                                                         |
| libvips        | Defer until measured baseline-worker bottleneck justifies packaging cost                                                      | Same evidence requirement                                                  | Not a dependency                                                                                                                           |

The provisional deployment target is macOS 13.0, but only macOS 27.0.1 arm64 was exercised. Current Homebrew bottles cannot establish older-OS compatibility. Intel, broad Windows distribution and signed/notarized installers are unverified; the local NTFS read-only Windows scope is recorded in M1_VALIDATION.md. M1 resources include the reviewed ExifTool code and its licenses; restic/rclone and reference images remain excluded.

M4 must run real servers for each advertised SFTP/SMB/FTPS/explicit legacy-FTP or other transport, verifying identity/TLS/host-key failures, interruption/uncertain commits, restored bytes, metadata and pacing limitations. A local restic success does not unlock source mutation or advertise any of those protocols. Restic/rclone may exceed application-owned source-disk pacing; upload limits cannot be presented as physical-I/O caps.

Sources: [Homebrew restic](https://formulae.brew.sh/formula/restic), [rclone](https://formulae.brew.sh/formula/rclone), [ExifTool](https://formulae.brew.sh/formula/exiftool), [restic releases](https://github.com/restic/restic/releases/tag/v0.19.1), [rclone releases](https://github.com/rclone/rclone/releases/tag/v1.75.2), [Tauri sidecars](https://v2.tauri.app/develop/sidecar/), [LibRaw](https://www.libraw.org/), [libheif](https://github.com/strukturag/libheif).

## M1 additions

- `@tanstack/react-virtual` **3.14.23**, MIT: bounded visible rows, without a second state-management framework. React Compiler is not enabled; the mutable virtualization adapter is not memoized by it.
- `globset` **0.4.20**, MIT/Unlicense: compiled bounded include/exclude rules.
- `blake3` **1.8.7**, CC0-1.0/Apache-2.0/Apache-2.0 LLVM exception: observed revision/cache keys and full-byte relink samples. This is not an integrity-baseline feature.
- `chrono` **0.4.45**, MIT/Apache-2.0, default features off with std/clock: validated capture-local dates and explicit UTC catalog events.
- `rustix` **1.1.6**, Apache-2.0 WITH LLVM-exception/Apache-2.0/MIT: Unix handle-relative no-follow directory traversal.

`vendor/exiftool-manifest.json` pins ExifTool 13.55 provenance, bottle hash and every resource SHA-256. `npm run release:check` verifies those bytes. Original module notices/README/Changes are retained, with full Artistic/GPL texts rendered from system Perl 5.34 POD. `/usr/bin/perl` is an explicit macOS runtime dependency, with no PATH fallback. Minimum-OS and distribution review remain required. See [ADR-0002](decisions/ADR-0002-read-only-library.md) and [M1 validation](M1_VALIDATION.md).

M1 advisory rerun: zero Rust vulnerability entries at the recorded RustSec database commit and zero production npm advisories. The same two informational Rust warnings and nine high-severity development-only npm entries remain; no advisory was ignored. Logs are `.context/evidence/m1-cargo-audit.json`, `m1-npm-production-audit.json` and `m1-npm-audit.json`.

## Live notifications — 2026-10-29

Add exact notify 8.2.1 (MIT OR Apache-2.0), with default macOS FSEvents support and follow-symlinks disabled where the backend supports that option. Its new locked adapters are notify-types 2.2.0, objc2-core-services 0.3.2, inotify 0.11.5/inotify-sys 0.1.8 and kqueue 1.2.1/kqueue-sys 1.1.2. Registry SHA256 checksums and platform selection remain in Cargo.lock. These bind native OS facilities; no helper executable or decoder is downloaded. Existing package versions/checksums and the npm lock remain unchanged; this is an additive notification dependency, not an upgrade of the existing stack. Version/feature selection follows the [primary documentation](https://docs.rs/notify/8.2.1/notify/). New dependency metadata does not replace the historical advisory/distribution audit above.
