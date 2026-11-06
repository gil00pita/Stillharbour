# M6 dependency hardening — 2026-11-04

The owner's “Continue” request continues M6. [ADR-0088](decisions/ADR-0088-development-dependency-advisory-remediation.md) records the exact development overrides and compatibility decision. Earlier M6 files and evidence remain retained. No production operation, root permission, schema, original-file capability or native helper changes.

## Change and reproduced failures

| Dependency               | Previous affected path                                                                                 | Exact override / compatibility                                                        |
| ------------------------ | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| glob                     | 13.0.6 and nested 10.5.0                                                                               | 14.0.3; async named API and CommonJS sync traversal retained                          |
| minimatch                | 10.2.6, 9.0.9 and 5.1.9                                                                                | 11.0.4; actual archive utility and Minimatch class retain Unicode/braces/exclusions   |
| Puppeteer browser helper | Nested 2.13.2 through Tauri service's WebdriverIO 9 utility; a separate top-level 3.2.6 already exists | 3.2.6 everywhere; remove extract-zip, retain every API imported by the actual utility |

`scripts/tests/dependency-policy.test.mjs` has four cases and joins `npm run test:release` within `npm run check`. Both safety regressions fail against the old lock: an escaping ZIP symlink is accepted, and glob performs nine reads after aborting its first directory read. All four pass against the updated graph: escaping ZIP rejection, one-read cancellation, the actual WebdriverIO consumer import and its required exports, and CommonJS/ESM pattern compatibility. The ZIP target and unchanged neighbor live inside one owned scratch directory; no user archive or library is touched. Logs: `.context/m6-continue/dependency-tests-before.log` and `dependency-tests-after.log`.

The old/new manifests are retained under `.context/m6-continue/`. Install uses `npm install --ignore-scripts`; it adds 12 package paths, removes 75, changes three retained versions and reports zero findings. Every production package path/version/source/integrity tuple compares unchanged. No unsafe postinstall script, test download, advisory ignore or forced downgrade is added. The exact lock remains required; this is not permission for arbitrary future major overrides.

## Advisory results

Fresh `npm audit --json` changes from 33 entries (27 moderate, six high) to **zero**; `npm audit --omit=dev --json` also reports zero. `npm ls --all` validates the installed graph. These are registry results, not proof that all code is vulnerability-free. Raw JSON and dependency tree are under `.context/m6-continue/`, with `npm-audit-before.json`, `npm-audit-after.json`, `npm-audit-production.json` and `npm-tree.txt`.

The existing Windows/macOS Foundation workflow now runs the full npm audit with an explicit low threshold, installs pinned cargo-audit 0.22.2 with its lock and runs the Rust audit before application validation. No advisory IDs or failed scans are suppressed. These are explicit networked CI gates; deterministic local `npm run check` remains usable without an advisory-service connection. Local execution verifies the audit commands; the changed workflow still requires fresh CI execution on both targets.

The workflow also retains `.context/m5-native-metadata`, closing the configured artifact omission recorded in M5 closeout. This adds the separate metadata spec's receipts/screenshots to future uploads, including failures; it does not create a missing historical CI receipt or prove a future upload succeeded.

Build `cargo-audit 0.22.2` into ignored workspace tooling with:

```sh
node scripts/cargo.mjs install cargo-audit --version 0.22.2 --locked --root .context/m6-continue/tools
.context/m6-continue/tools/bin/cargo-audit audit --json --db .context/m6-continue/advisory-db
```

The actual audit exits zero: 1,403 advisories, RustSec commit `965bf9ad083b58e6340d65829d85217b18be9290`, last updated `2026-11-03T21:34:28+01:00`, zero vulnerability entries. Two informational warnings remain: unmaintained `proc-macro-error 1.0.4` ([RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html)) and unsound `glib 0.18.5` ([RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html)). No warning is hidden or ignored. Both packages are absent from separately generated normal/build graphs for `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`; the cross-platform lock retains the Linux/GTK chain. Generating a Windows dependency graph is not native Windows execution. Linux is outside the selected release targets and would require its own dependency/support review.

Logs: `cargo-audit-install.log`, `cargo-audit.json`, `cargo-audit.stderr`, `cargo-graph-macos.txt` and `cargo-graph-windows.txt` under `.context/m6-continue/`. The scanner and advisory database are development evidence only and never bundled. Native helper/SQLite runtime, complete notice/legal review, installed-artifact provenance and clean-machine gates remain separately required.

## Validation boundary

### Current-main CI portability correction — 2026-11-04

Foundation run 41869615079 at `a7c9af1` fails the dependency ZIP test on both macOS and Windows, plus the native-path expectation on Windows. This supersedes any inference that the earlier local pass established these test assumptions on every host. The resolved browser helper's `unpackArchive` prefers native `tar.exe`/PowerShell/`unzip` before its JavaScript fallback; the symlink containment logic inspected during remediation belongs to exported `extractZipWithYauzl`. A native extractor successfully returning is not evidence that the JavaScript check failed, or that arbitrary native extraction is contained.

The regression now invokes that exact resolved JavaScript implementation and requires its specific nested containment error for parent escape, descent-then-parent escape and absolute targets. Missing optional tools or unrelated filesystem errors cannot satisfy it. Both default and JavaScript paths separately extract an ordinary generated ZIP, checking exact bytes and an unchanged generated neighbor. This benign default-path test grants no hostile-archive/native-extractor safety claim. Automatic native driver downloads remain disabled; no bespoke extractor, dependency change, advisory ignore or production authority is added.

The archive utility returns native separators. Its expected file uses `path.join`, retaining the comparison with the installed glob and the same Unicode/exclusion/brace assertions. No execution path is normalized or assertions skipped. All ten release/dependency Node cases pass locally on Windows after reproducing both failures. Fresh full checks and actual remote platform outcomes are recorded in the implementation log.

The first draft-PR run, 41896607946 at `ee77709`, then exposes the missing optional `yauzl` peer on the clean Windows runner. The nested local worktree had resolved it from its preserved ancestor checkout. Keep that failure: it is not a successful containment rejection. Pin test-only `yauzl 3.4.0`, which satisfies the browser helper's declared peer range, plus its locked `pend 1.2.0` dependency (both MIT). Registry integrity matches the committed lock. A new regression requires that the actual consumer resolve version 3.4.0 inside this checkout's node_modules. A clean `npm ci --ignore-scripts` and all eleven Node cases pass; every one of the 183 production package records and root production pins compare exactly unchanged. No framework/helper/Cargo change, implicit download, audit ignore or runtime permission is added. Fresh npm audit reports zero findings.

Full checks and expanded native preservation/desktop validation are recorded below. Prior M6's 22-case Foundation smoke and two selected opt-ins remain historical results on their earlier lock. Current Windows, signing, clean-machine, whole-app screen-reader and complete performance/preservation acceptance remain open. No release certification follows from fixing these dependency paths.

## Current standard and native preservation results

The clean install and complete serial `npm run check` pass: 482 frontend/harness cases in 76 files, all nine Node release/dependency cases, format/lint/strict types/bindings, Rust formatting, default-core and workspace/all-target Clippy with warnings denied, 508 core cases (zero failures/15 existing opt-ins; 285.50s), two desktop unit cases, all 29 integrations, zero doctests and production frontend build. Existing Vite large-chunk warnings remain. Logs: `npm-ci.log`, `check.log` and `release-check-before-native.log` under `.context/m6-continue/`.

All **13 local native preservation opt-ins** pass serially, zero failures, in 375.96s:

```sh
node scripts/cargo.mjs test --locked -p stillharbour-core --lib \
  --features fixture-probes,copy-workflow,backup-workflow -- \
  --ignored --skip backup::rclone_worker::tests::actual_pinned --test-threads=1
```

This executes live transfer pause/rate/cancel/publication controls, actual pinned-helper backup/restore with wrong-key and late-occupant guards, schedule disable/replay, both catalog activation/retained-return cases, damaged-media/opaque-companion preservation, all five scheduling/authority/two-root/coordination cases, and both process-exit matrices. Each process matrix covers all eight declared boundaries: staging, catalog snapshot, incomplete catalog-copy step, snapshotting, inclusion, comparison, snapshot recording and restore publication. One matrix uses controlled exits and the other actual parent-issued termination. Reopen checks retain source identities/digests and destination-only/restore neighbors, revoke prior grants and require explicit recovery. `native-preservation.log` records the actual run.

The skip is limited to two separate rclone server-dependent opt-ins. Neither is represented as passing here; their transport setup and earlier evidence remain in [M4 transport tests](M4_TRANSPORT_TESTS.md). Serial execution avoids conflicting changes to the shared application I/O budget. These generated local tests do not establish independent failure domains, physical device-loss behavior, every remote commit phase, full AC/PS certification or production execution authority.

## Current native desktop and artifact results

`npm run desktop:test:build` passes (35.23s native compile). With the updated dependency graph, `npm run desktop:test` passes **all five specs**, 26 cases and three existing Windows-specific skips, in 2m28s: Foundation 22 cases, live-library, M4 application, map and separate metadata export. `npm run desktop:test:catalog` passes its two Node lifecycle cases and all three native launch specs in 42s. No current Windows execution is inferred.

The map and separate metadata receipts each independently report nine unchanged source/companion entries, twelve exported artifacts, two reviewed XMP exports, unchanged destination neighbor and disabled original mutations. The map has six located/two unavailable photos; zero and antimeridian scopes each contain two. Copies of these and the M4 lifecycle/catalog receipts are retained under `.context/m6-continue/native-evidence/`, with eight receipt hashes in `native-receipt-pins.json`. The map receipt SHA-256 is `f09df9fa118a8c00a57b5fa4e0ee0ecfd02faef6aba0ef963a74c436dcbd4329`; separate metadata is `df69554a816f6746cae2e1d2a5680d02ae834b509a800e2b6a2cf506ede239f7`.

The instrumented executable SHA-256 is `67c360fe07984a03b0e56360d2f4f4111c4bf98532b9618b66da3d6f39980c6e`. Native host and both lockfile hashes are recorded in `native-host.json` and `native-build.sha256`. These are debug native assertions, not signed-package GUI or fresh screen-reader certification. [Package validation](M6_PACKAGE_VALIDATION.md) records the subsequent normal app build, precise resource inventory, actual SQLite and generated-library probes, the original unsigned signature failure and separate successful local ad-hoc sealing. Production frontend/resource preflight passes afterward. Source/runtime files and production operation gates remain unchanged.

## Clean-install recipe typing — 2026-11-06

The first main-integration attempt added a pinned development CLI to generate custom recipe types; CLI 3.37.0 was rejected for its Zod 3 advisory, while CLI 3.29.0 with patched Zod 4 passed the recorded audit/build checks. Latest main's direct typed recipe implementation supersedes that workaround. Remove the extra CLI dependency and restore main's package manifest/lockfile, retaining strict typing and the existing runtime/helper pins. Historical audit/build receipts remain in the implementation log; final fresh-install validation must use the combined current source with stock Chakra declarations. No new development tool or native helper is shipped by this merge.
