> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# M5 closeout — 2026-11-03

M5's development gate is complete for the implemented offline map, catalog metadata and reviewed exports into new files. This review closes the pending Windows execution of the repeated-XMP export correction. Format combinations without clean target-machine evidence remain **unverified**, as the [M5 gate](../tasks/M5_MAP_METADATA_FORMATS.md#gate) explicitly permits. This acceptance does not certify a first release or enable production original-file mutations. Stop before M6.

## Tested application and CI

[PR #16](https://github.com/gil00pita/Stillharbour/pull/16) merged as `e15a4b2f773782d98cb0d5938e2eef0eec829839`. Its final head, `80da716aa0eb18f0daf8b79c652fcad00fdc5719`, and that merge have the same Git tree: `891061e4920ec35cd924c8bac5434aedd8168110`. This closeout changes documentation only.

Both target-platform jobs pass in [Foundation 41631002416](https://github.com/gil00pita/Stillharbour/actions/runs/41631002416) and [transport 41631002379](https://github.com/gil00pita/Stillharbour/actions/runs/41631002379). The separate post-merge Foundation run `41652528689` was queued when inspected; its result is not needed to infer a new pass because the tested and merged trees are identical.

| Executed check                                                | macOS CI                                                                          | Windows CI                                                                           |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Complete `npm run check`                                      | Pass; 482 frontend tests, 508 core tests, two desktop unit cases, 29 integrations | Pass; 482 frontend tests, 470 core tests, two desktop unit cases, eight integrations |
| Default core opt-ins                                          | Fifteen ignored in the default run; selected native suites run separately         | Fourteen ignored in the default run; selected native suites run separately           |
| Native desktop suite                                          | Five specs pass, including M5 map and metadata/export                             | Five specs pass, including M5 map and metadata/export                                |
| Combined M5 map/metadata/export scenario                      | Pass, 18.3s                                                                       | Pass, 9.8s                                                                           |
| Separate M5 metadata/export scenario                          | Pass, 13.6s                                                                       | Pass, 7.6s                                                                           |
| Three-launch catalog recovery suite                           | Three specs pass                                                                  | Three specs pass                                                                     |
| Production resource exclusions and native cross-volume checks | Pass                                                                              | Pass                                                                                 |

CI checks include formatting, strict types, lint, binding drift, Rust formatting, default-core and workspace/all-target Clippy with warnings denied, workspace tests and frontend build. A CI runner with development tools is not a clean-machine installation. The separate transport workflow passes its generated restore/failure scenarios on both platforms; full M4 preservation certification remains independent.

Fresh local `npm run check` also passes completely: 482 frontend tests in 76 files, 508 core tests with zero failures/fifteen existing opt-ins (285.49s), two desktop unit cases, all 29 integrations, zero doctests, strict checks and production frontend build. `npm run release:check` then passes helper/resource pins and test-instrumentation exclusions. Logs are `.context/m5-final-review/local-check-prepared.log` and `release-check.log`. The initial missing-helper build failure and Python TLS preparation failure, followed by verified system-curl preparation, remain recorded in the [implementation log](IMPLEMENTATION_LOG.md). The existing large-chunk warning remains. No new local native desktop or clean-machine execution is claimed by this documentation closeout.

## Native evidence reviewed

Downloaded Foundation artifacts `13231216403` (`native-evidence-macos-15`) and `13230450852` (`native-evidence-windows-latest`) contain the combined scenario's `m5-native/map.json`. Both receipts pass a local replay of the existing plan-scoped export assertions and explicit checks of map membership, catalog focus, output safety and source preservation. The Windows map overview and macOS catalog-export review screenshots were also inspected. Downloaded evidence, workflow logs and the replay summary are retained under `.context/m5-final-review/` in this workspace.

| Receipt assertion    | Both platforms                                                                                                                                                                |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Valid/missing GPS    | Six located, two unavailable; zero and antimeridian queries each contain the expected two photos                                                                              |
| Native interactions  | Keyboard zoom, map/library navigation and minimum-width Catalog section focus pass                                                                                            |
| Catalog organization | Album membership, saved geographic search, parent tag category, Unicode caption and reject state are retained                                                                 |
| Export evidence      | Twelve output artifacts; two distinct reviewed four-file exports of the same source XMP, each retaining original and merged unknown-bearing packets                           |
| Portable catalog     | Schema 17, SQLite 3.54.1, integrity `ok`, eight media rows, unbound roots, zero native authorities and zero credential/schedule/approval rows                                 |
| Preservation         | Nine generated source/companion entries independently verified unchanged during native execution; destination-only neighbor unchanged; production original mutations disabled |
| Network observation  | No external HTTP(S) resources recorded in the main WebView; worker/OS traffic was not independently intercepted                                                               |

Receipt SHA-256 pins:

```text
macOS   51b20632bd4ac815c17cdc12dbe2d3656f4ef53a70e48e9058b3d8c229b9a5d7
Windows 4cd88300f141a55a0afb65c1b9c5df0c76b361a59fc57752d37ba42d14dcb021
```

The separate metadata scenario passes in the CI logs, but its `m5-native-metadata` directory is not included in these workflow artifacts. Its separate receipt was therefore not inspected here. The combined scenario retains the shared export assertions and its own independent source audit. Receipt replay does not rehash the former CI source trees, reopen their exported databases or constitute new native execution.

## Acceptance mapping and retained limits

| M5 requirement                                                                                                            | Evidence and outcome                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spatial queries, bounded clustering, zero/missing GPS, antimeridian, filter/selection navigation (AC-074/075, M5-G01–G06) | Core/component regressions and both-platform native scenarios pass. [Map implementation](M5_MAP.md) records bounds and migration evidence.                                                                                                                      |
| Production map data, attribution, offline rights and privacy (AC-076, M5-G07)                                             | Bundled pinned public-domain Natural Earth geometry and local MapLibre worker; no online provider, geocoder, tile downloads or cache. Native resource observations have the scope above.                                                                        |
| Accessible map/list and catalog controls (M5-G08, M5-A11; M5 portion of AC-079)                                           | Both-platform keyboard/focus checks pass. The previously recorded owner-performed VoiceOver review passes map names, heading announcements and Catalog labels/focus on macOS 27.0.1 arm64. Whole-application and other-screen-reader certification remain open. |
| Albums, hierarchical tags, ratings/favorites/rejects/captions, saved searches and provenance (M5-C and M5-X01–X03)        | Core/component/native acceptance passes. Schema 17 retains catalog-only edits, reversible tag history and explicit observed-keyword import overrides.                                                                                                           |
| Portable catalog and conflict-safe XMP export (AC-083, M5-X04–X08, M5-A12)                                                | Core conflict/parser/exit regressions and both-platform native scenarios pass, including deterministic repeated exports. Existing originals and sidecars remain read-only; export publishes only new reviewed outputs.                                          |
| Packaged format capability gates (M5-F01–F06)                                                                             | [Format support](FORMAT_SUPPORT.md) publishes exact local packaged macOS evidence and explicit unverified combinations. Preview success never establishes full RAW validation. No unverified clean-machine capability is advertised.                            |

The earlier [validation history](M5_VALIDATION.md) retains failures and subsequent repairs. In particular, the passing documentation-only rerun at `5a56a61` did not prove an export-assertion repair; the deterministic correction now has actual Windows and macOS execution at `80da716`.

Clean Windows/macOS installation without development tools, signing/notarization/updater validation, Intel/minimum-macOS coverage, broader camera/color/HDR/full-media validation, NVDA and whole-application accessibility remain unverified or release blockers under their existing requirements. The M5 gate permits explicit unverified format claims; this closeout does not waive AC-077/078/079 or M5-F06's eventual release coverage. M4 independent exact-version recovery and production original-file execution retain their separate gates. No M6, N-extension, editing, mobile or cloud work is started.
