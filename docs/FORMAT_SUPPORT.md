# Read-only format previews — M5 slice

The owner's 2026-10-28 HEIC/CR2/common-preview request authorises this bounded M5 format slice. Maps, metadata export, original mutations and later performance phases remain outside it. The existing gallery, folder thumbnails and viewer use derived 256/512 px PNG previews. There is no full-resolution RAW developer, original conversion or source metadata write.

## Verified capability

These are local packaged macOS results, not universal camera/OS support. Baseline JPEG/PNG/WebP retains the Rust decoder. ImageIO/ColorSync handles the additional native formats; malformed explicit baseline ICC profiles fail explicitly. Native rendering uses sRGB RGBA8 SDR, orientation once, no upscaling, the primary image or first animation frame. It does not validate complete RAW data, preserve HDR, play animations or establish duplicate equality.

| Input                              | Evidence                                                                   | Result                                |
| ---------------------------------- | -------------------------------------------------------------------------- | ------------------------------------- |
| JPEG, PNG, WebP                    | Existing baseline regression suite; generated JPEG/PNG in new packaged run | Retained                              |
| HEIC / HEIF suffix, uppercase HEIC | Generated HEIC bytes, HEIF alias; real native gallery/viewer HEIC test     | 96 × 48 previews, no upscaling        |
| TIFF, GIF, BMP                     | Generated fixtures                                                         | 96 × 48 previews                      |
| Display P3 tagged PNG              | Generated profile conversion; independent known-color patch checks         | sRGB output within declared tolerance |
| CR2 — Canon EOS 5D Mark III        | CC0 archive sample 771                                                     | 512 × 341                             |
| CR3 — Canon EOS R                  | CC0 archive sample 4611                                                    | 512 × 341                             |
| NEF — Nikon Z6                     | CC0 archive sample 3581                                                    | 512 × 341                             |
| ARW — Sony ILCE-7M3                | CC0 archive sample 2414                                                    | 512 × 342                             |
| DNG — Ricoh GR III                 | CC0 archive sample 3115                                                    | 512 × 341                             |
| RAF — Fujifilm X-T4                | CC0 archive sample 3918                                                    | 512 × 341                             |

Camera provenance, HTTPS URLs, licenses and SHA-256 pins are committed in [camera-fixtures.json](../scripts/formats/camera-fixtures.json). Photo bytes remain outside version control in owned diagnostic fixtures. One HEIC encoding under two suffixes is not proof of every HEIF codec. Exhaustive native-format ICC/HDR validation, other camera variants, ORF/RW2/CRW/PEF and AVIF remain unverified even where the native adapter can attempt them. Windows retains baseline decoding; native Windows, Intel Mac, oldest supported macOS, clean-machine, signing and broader camera/color/sidecar/full-validation gates remain open. Do not equate an indexed extension with successful decoding.

## Limits and compatibility

Retain 32 MiB input, 12000 px maximum dimension, 128 MiB source RGBA estimate, 2 MiB encoded PNG, 8 MiB worker JSON, 12-second deadline, two workers and 64 queue entries. Larger files remain catalogued with an explicit unavailable preview. A direct-child RSS watchdog samples every 25 ms against 512 MiB, fails closed on monitoring errors and kills/reaps the worker on failure. It is not an instantaneous allocation quota or complete XPC/process-tree accounting. No safety limit was raised to pass RAW fixtures.

The backend reads identity/revision-guarded original bytes and passes only bounded bytes plus a fixed numeric format hint into the isolated worker. It retains no-replace cache publication and independent reopening/read-back. Cache policy is `m5-v2-oriented-native-srgb8-preview`. Legacy unsupported failures become retryable on read through a policy envelope in the existing error column; current failures stay suppressed. Indexed compatible M1 positive previews remain usable in the catalog's offline state. No catalog migration, automatic cache clearing, root/UUID change or original rewrite is needed. A regression fixture verifies this offline-state cache compatibility; it is not a physical network-disconnection test.

## Actual evidence — 2026-10-28

Host: Apple M1 Max, 10 cores, 32 GiB, Mac13,1, arm64; macOS 27.0.1 build 26A434; SDK 27.0, Apple clang 21.0.0 (clang-2100.3.34.2). ImageIO 3.3.0/2851, ColorSync 4.13.0/3929, CoreGraphics 2.0/2050. Rust 1.99.1, Node 26.3.1, npm 11.16.0. Deployment flag macOS 13 is provisional, not minimum-OS test evidence. SQLite remains 3.54.1/schema 2. The only Cargo graph change adds an owned build edge to already locked cc 1.10.0 (MIT OR Apache-2.0); no dependency version or npm lock change.

Source-bearing fixture run `2963a851-ed3f-4bca-ba41-df1418cd4439` contains 18 files. Final packaged run `51bee2e4-7d2d-4838-a6f7-516aa0f94215` reuses them read-only; all 18 expected outcomes pass, including malformed HEIC, over-input and over-dimension rejection, and legacy HEIC failure retry. Independent before/after manifests match. Reports, source manifests and previews are retained under `.context/formats/<run>/`. Worker SHA-256: `9c10a1416571b6969593b1712b5762256d7d51bf092c9dc5c0664807b5a7bc31`; release diagnostic probe: `6781055fad11f003ad2d2af377c7510905b12ae8ed9adbd9cf739f9653dc44fa`.

Each timing below is one request-to-cached-result observation, including dispatch/polling; it is not a warm/cold benchmark, percentile or SLO. Direct-child observed RSS is sampled, partial and excludes other processes. The NEF trace dropped one event; fast negative cases can have startup-only or no worker samples.

| Sample | Elapsed ms (n=1) | Observed worker RSS bytes |
| ------ | ---------------: | ------------------------: |
| HEIC   |          378.610 |                  24379392 |
| CR2    |          373.718 |                  60981248 |
| CR3    |          319.956 |                  62341120 |
| NEF    |         5356.361 |                  51183616 |
| ARW    |          345.291 |                  57982976 |
| DNG    |          345.406 |                  68059136 |
| RAF    |          319.170 |                  63717376 |

Native macOS WKWebView debug checks run separately: 13 existing desktop regressions pass; the generated HEIC gallery/viewer case passes with 36 independently inventoried fixture files unchanged. It confirms loaded images, viewer dimensions, return query and disabled original mutations. Screenshots: `.context/screenshots/native-heic-gallery.png` and `native-heic-viewer.png`. Unit/integration checks cover all eight orientations, alpha/channel ordering, no upscaling, bad ICC, dimensions, fixed worker arguments, RSS monitoring, failure-policy retries and compatible old offline previews. Full validation outcomes are recorded in [IMPLEMENTATION_LOG.md](IMPLEMENTATION_LOG.md).

Retained failures: the first camera runs rejected Sony ARW because byte-only ImageIO identified it as generic TIFF with missing dimensions; the fixed allowlisted RAW hint resolves it without a path interface. The initial Python download failed certificate trust; system curl uses host trust with verification enabled, pinned SHA-256 and HTTPS-only redirects. A combined native spec invocation reused the same test catalog, contaminating the baseline 32-file assertion with the 36-file format fixture. The format suite now has a separate configuration/invocation; assertions and timeouts were not weakened. These failed artifacts remain available; they are not format passes.

## Reproduction

Use generated and explicitly licensed public fixtures only. The runner requires macOS, a 4 GiB free-space reserve, fixed built executables and UUID-owned fixture directories; it never cleans previous runs or accepts a personal library path. Downloaded sources are immutable after the independent manifest. Reuse reads the original owned source tree and creates only new diagnostic data/cache/results. Synthetic catalog benchmarks are separate.

```sh
npm run formats:build
npm run desktop:build
npm run formats:test -- --packaged --camera-samples
# Reuse the retained source-bearing run without another download or source copy:
npm run formats:test -- --packaged --reuse-fixtures 2963a851-ed3f-4bca-ba41-df1418cd4439
npm run desktop:test:build
npm run desktop:test
npm run desktop:test:formats
npm run check
npm run desktop:build
npm run release:check
```

Run the native suites as separate CLI invocations, serially; do not rebuild `dist` while a native driver is active. Executable diagnostics: [runner](../scripts/formats/run.py), [catalog/worker probe](../crates/core/examples/format_probe.rs), [native viewer spec](../tests/formats/native.e2e.mjs), [separate config](../wdio.formats.conf.mjs), [independent manifest tool](../scripts/performance/manifest.py). Read [ADR-0010](decisions/ADR-0010-native-format-previews.md) for the adapter decision and retained safety gates.

## Large-image progressive renditions — 2026-10-28

The owner's large JPEG/PNG/TIFF request adds the bounded macOS handle-backed downsampling path in [PROGRESSIVE_PREVIEWS.md](PROGRESSIVE_PREVIEWS.md) and [ADR-0011](decisions/ADR-0011-progressive-renditions.md). Its explicit file/read limits replace the earlier full-source estimate for those three types only; other adapters retain prior limits. Tiny placeholders are prepared as a separate background job and clear previews remain demand-driven. Old M5-v2/M1 positive caches remain usable; old failures retry under the new policy. Existing evidence above remains historical, not proof of the changed path; current runs are recorded in the new document.
