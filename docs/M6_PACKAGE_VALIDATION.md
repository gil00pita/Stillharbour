# M6 native package and executable evidence — 2026-11-04

This is a fresh local developer-host validation on **macOS 27.2 arm64** after [M6 dependency hardening](M6_DEPENDENCY_HARDENING.md). No signed distribution, clean-machine installation or Windows package is certified. The default application keeps original-file mutations and experimental copy/backup execution disabled. No user catalog or personal photo library is used by these probes.

## Normal production bundle

`npm run desktop:build` passes with the normal feature set: release compile 1m35s, then one `.app`, reported by the bundler as 153.66 MiB. `npm run release:check` passes against the production frontend and default dependency graph. The full native UI suites ran separately in the instrumented debug build; they are not reported as packaged production UI execution.

| Observed property  | Result                                                                                                                                                                |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bundle             | `target/release/bundle/macos/Stillharbour.app`                                                                                                                        |
| Application        | Version 0.1.0, unchanged identifier `local.stillharbour.foundation`                                                                                                   |
| Executable         | Mach-O 64-bit arm64                                                                                                                                                   |
| Binary SHA-256     | `9ab4a94f0535de03a62c8ff29c8b897032437a05bcd1357803a9aa82cdc08396`                                                                                                    |
| Exact inventory    | 273 regular files, 161,112,188 logical bytes                                                                                                                          |
| ExifTool resources | All 265 files match the reviewed file manifest; exact membership, not just count                                                                                      |
| Backup resources   | Host restic/rclone executable hashes and bundled manifest match their source pins; both are arm64                                                                     |
| Backup notices     | Both bundled license files match the reviewed source copies byte-for-byte                                                                                             |
| Resource boundary  | Exact expected binary/plist/icon/helper/manifest/license membership; no links, special files, unreviewed paths, source maps, node_modules, reference or context trees |

Restic SHA-256 is `06582569ff2f10e1935a6f12187c76db02f0bae99e6e54098f2cdc374766d768`; rclone is `af0257730173748dc9706ad3c83288bf3a0c5d5e9768fef20235e375e9d835e1`. The read-only checker and full per-file inventory are `.context/m6-continue/check-bundle.py`, `bundle-check.log` and `bundle-inventory.json`. The receipt pins its checker source; it does not replace independent distribution/legal review.

The unsigned bundle's main executable has a linker-generated ad-hoc signature. **Strict deep bundle verification exits 1**, reporting `code has no resources but signature indicates they must be present`. Preserve this result in `signature-verification.stderr`; successful compilation and resource checks do not constitute a signature pass.

## Separate ad-hoc resource-sealing diagnostic

Create a fresh copy under `.context/m6-continue/adhoc/Stillharbour.app`, retaining the original bundle unchanged, then run:

```sh
codesign --force --sign - --timestamp=none \
  --identifier local.stillharbour.foundation \
  .context/m6-continue/adhoc/Stillharbour.app
codesign --verify --deep --strict .context/m6-continue/adhoc/Stillharbour.app
```

Both commands exit zero. This uses an anonymous ad-hoc identity, no owner certificate, private key, timestamp service or notarization. `--force` replaces only the existing signature in this new owned build copy; it grants no file-operation override or original-library authority.

The copied executable SHA-256 becomes `f908960f948da68472b1e7117d86847d90bd5b23d7e0ddc4c5c460142cef264d`. A complete content comparison confirms the sole changed original file is `Contents/MacOS/stillharbour`; the sole added file is `Contents/_CodeSignature/CodeResources`. Every helper, notice, icon, plist and other resource is unchanged. Both helper pins still match, and the original unsigned binary retains its original hash. `adhoc-receipt.json`, signing/verification stdout/stderr and the original inventory retain these distinct artifact identities.

This closes a local resource-sealing experiment only. It is not Developer ID trust, notarization, Gatekeeper acceptance, clean-machine installation or evidence that a future owner-signed helper pipeline has passed.

## Actual bundled SQLite and generated-library execution

The in-memory diagnostic runs against each artifact without opening a user catalog:

```sh
node scripts/performance/packaged-sqlite.mjs
node scripts/cargo.mjs run --locked --release -p stillharbour-core \
  --features fixture-probes --example packaged_library_probe -- \
  target/release/bundle/macos/Stillharbour.app
target/release/examples/packaged_library_probe .context/m6-continue/adhoc/Stillharbour.app
```

The normal-artifact SQLite receipt is run `76b4f953-143e-4275-9134-b7b5347ef313`. Both binaries report **SQLite 3.54.1**, source ID `2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76`, and `userCatalogOpened: false`. Compare the actual source ID with the [official release history](https://sqlite.org/changes.html); the [3.51.3 WAL-reset fix](https://sqlite.org/releaselog/3_51_3.html) predates this runtime. The [official vulnerability list](https://sqlite.org/cves.html) was rechecked on 2026-11-04. This scoped runtime/source review is not permission to execute imported SQL or skip catalog-schema defenses.

Each packaged-library run independently creates its own fixtures, indexes **24 media**, exercises the actual packaged metadata worker and baseline JPEG/PNG/WebP previews, and verifies **36 source entries unchanged**. JPEG and HEIC original bytes compare exactly; HEIC geometry is 3,072×2,048. Original JPEG/PNG/WebP/HEIC routes succeed. The generated capture date `1964-02-29T13:15:20.1234` and `+05:30` offset come from bundled metadata execution. The two runs are separate fixture sets, not one combined library or a claim of all-format support.

Logs: `production-app-build.log`, `release-check-final.log`, `packaged-sqlite.json`, `packaged-library.log`, `adhoc-receipt.json` and `adhoc-packaged-library.json` under `.context/m6-continue/`. No universal RAW/HDR/color/camera, minimum-macOS, Intel, clean-machine, Windows or signed-release claim follows. Full application accessibility, performance attribution, preservation certification, independent recovery and trusted distribution retain their existing gates.

## Current-main Windows executable follow-up — 2026-11-04

The isolated current-main continuation builds ordinary Windows release output with npm run tauri -- build --no-bundle (2m27 native compile). This produces an executable, not an installer. No production app/IPC/permission/source-operation feature is enabled. The test-only ZIP peer pin leaves all 183 production npm package records unchanged. Earlier legacy-checkout executable experiments are not evidence for this source.

Actual executable SHA-256 is 95f93c5a8560496fe56f2ae5fb474dbce8bd2efd7b1c0be74ebd157183d47b30. Read-only Authenticode inspection returns NotSigned. The new reusable npm run release:smoke -- target/release/stillharbour.exe command tests only early worker/in-memory SQLite modes: generated two-pixel PNG decoding/output, typed malformed input, invalid worker sizes/names and rejected SQLite path arguments. It accepts only an executable, bounds child time/output, verifies the unchanged executable hash before emitting a receipt, launches no normal GUI and opens no user catalog. No library source path is accepted or created.

The current executable passes all these checks. SQLite reports 3.54.1, source ID 2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76 and userCatalogOpened=false. Raw output is .context/m6-ci/windows-release-smoke.json. Normal production frontend/resource/helper/default-graph preflight passes afterward; ordinary frontend output is retained. The existing Vite large-chunk warning remains. This does not certify a Windows installer, installed resources, clean-machine/runtime prerequisites, archival color fidelity, trusted signatures or backup/restore through the production executable.

Separately, the actual instrumented Windows desktop build passes all five default specs: twelve applicable cases pass, seventeen platform cases skip, in three minutes. This includes generated operation review/copy/undo, M4 application snapshot/read/restore and M5 map/metadata exports. The map and metadata receipts independently retain nine unchanged source/companion entries each and originalMutationsEnabled=false under .context/m5-native/map.json and .context/m5-native-metadata/metadata.json. These are debug native tests, not a signed production GUI or screen-reader run.

The owner explicitly confirms signing/notarization credentials, clean native machines and owner test endpoints are unavailable. Hosted macOS ARM64 capacity also cancels the earlier queued job without execution. M6 remains blocked at the documented release gates; no self-signing substitute, release publication or production mutation enablement follows from these Windows development results.
