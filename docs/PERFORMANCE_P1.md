# Stillharbour P1 catalog-read results

Measured: 2026-10-28. The owner's “Move to next plan” instruction after [P0](PERFORMANCE_BASELINE.md) authorises this phase only. Compact pages, existing-index seeks and bounded cached timeline/count summaries are implemented. Current views, exact selection membership, schema2, dependency locks, FULL durability, full startup validation and original-file guards remain. No personal library/catalog/cache was opened or cleared for these experiments. P2–P4 remain unapproved.

## Changes and regression evidence

[ADR-0009](decisions/ADR-0009-catalog-read-performance.md) defines the choices. [store.rs](../crates/core/src/library/store.rs) now projects27 display fields instead of SELECT* for pages/seeks/neighbors. Simple inventory queries require an existing ordered index; selective queries retain normal planning. The old actual-neighbor test failed after16,510,976 logical SQLite pager-miss bytes on10k. Its parameterized plan used assets_root_timeline with ANY(root_id) plus a temporary ORDER BY sort. Matching the partial-index predicate or selecting IDs alone still sorted. The same regression now passes at45,056 bytes, with correct previous/next identities. Red/green evidence: .context/performance/p1-neighbors-red.txt and p1-neighbors-green.txt; the final unit log also covers the retained assertion.

PackedAssetPage format1 removes repeated object keys. Rust owns the typed tuple/order; [pageTransport.ts](../src/pageTransport.ts) reconstructs the unchanged UI model once at IPC. Every configurable table/viewer field remains, including nullable fields and decimal byte strings above JavaScript's safe integer range. A [shared golden fixture](../tests/fixtures/catalog-page-v1.json) is checked by Rust and TypeScript. Incompatible formats/oversized pages fail explicitly. There is no catalog/preference migration.

[read_cache.rs](../crates/core/src/library/read_cache.rs) retains at most16 count/bucket entries and512KiB encoded keys/values per database worker. This is not an exact heap/RSS bound. Full validated query hashes separate scopes/sorts. Connection total_changes64 and same-connection PRAGMA data_version invalidate after worker writes or external commits; tests include writes without a library revision bump. It is deliberately conservative during preview/selection/scan work. No selection membership, integrity result or preservation evidence is cached. Seek totals and rows share one actor call. No source enumeration is performed by these catalog reads; all seeded roots are explicitly offline synthetic observations, not proof of real source availability.

## Environment and identity

Release run **c030a54d-619b-4fbc-91a0-002347850265** has separate10k/100k/1m catalogs under .context/performance/<run>-<rows>/baseline.json, ownership.json, sources and independent manifest-before/after.json. All three33-entry inventories match. Catalog seeding is metadata-only; generated decoder fixtures remain small JPEG/PNG/WebP plus explicit malformed/unsupported cases. Logical large-video/RAW/HEIC records are not real decode benchmarks.

Same host/toolchain as P0: M1 Max10 cores/32GiB, Mac13,1, macOS27.0.1 build26A434, arm64; Rust1.99.1/LLVM23.1.1, Node26.3.1/npm11.16.0, Python3.14.0, WebKit22625.1.29.11.28. Approximately11GiB free before the new retained fixtures; the runner enforces a4GiB reserve and performs no cleanup. Exact environment/binary hashes: .context/performance/c030a54d-619b-4fbc-91a0-002347850265-environment.json.

Linked SQLite remains3.54.1, source ID `2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76`; WAL/page4096/synchronous2/schema2. The upstream WAL-reset/advisory review and dependency versions are in P0; no upgrade occurred. Cargo.lock SHA256 remains0b44ad7be9a2053441f85ac7c5e49ef0618cf5e4dad5c99de5e7fa948f3f528f; package-lock remains eab358a6345e96c3195256904292d24534df4033a1c4aa79f1b26086f1afeda0. Release probe hash ebd75cf48c3b383406392d2c8e7879a5cdcdf6db1361dc44a25eb71597230f96 identifies the measured new core. The unchanged generated-media decoder subtest used the retained P0 release app binary82e439e60d07c32a4f2cba20c6c1bf2c94813be0b31931a9b1f5bece54508a38; it is not P1 release-UI evidence.

## Measurement definitions and results

No builds/tests ran concurrently with this release probe. This is a developer workstation, not a controlled minimum-spec lab. Seeding warms filesystem caches; OS-cache-cold is not claimed. First/next pages return240 display records. Boundary-neighbor samples use the actual store method for the first and last items independently. Random seeks use the fixed P0 PRNG/year sequence. Uncached bucket/tag measurements reset only the owned in-memory derived summaries before each sample; cached measurements reuse unchanged query results. The one logical-pager measurement uses PRAGMA shrink_memory on this disposable connection only, not an OS purge. Timing includes actor round trips and optional diagnostic overhead. Percentiles use nearest rank; n10 p95/p99 are both the maximum.

| Metric (samples per size)     | 10k p50 / p95 / p99 ms | 100k p50 / p95 / p99 ms    | 1m p50 / p95 / p99 ms       |
| ----------------------------- | ---------------------- | -------------------------- | --------------------------- |
| firstBoundaryNeighbors (n100) | 0.089 / 0.119 / 0.142  | 0.100 / 0.151 / 0.178      | 0.089 / 0.136 / 0.194       |
| firstPage (n100)              | 1.069 / 1.319 / 1.421  | 1.219 / 1.559 / 2.524      | 1.264 / 4.502 / 6.492       |
| lastBoundaryNeighbors (n100)  | 0.089 / 0.151 / 0.188  | 0.101 / 0.225 / 0.565      | 0.089 / 0.119 / 0.134       |
| nextPage (n100)               | 1.078 / 1.378 / 1.440  | 1.301 / 3.223 / 6.312      | 1.076 / 1.538 / 1.678       |
| packedPageEncode (n100)       | 0.065 / 0.067 / 0.075  | 0.068 / 0.148 / 0.428      | 0.064 / 0.067 / 0.079       |
| randomDateSeek (n100)         | 1.277 / 1.587 / 1.655  | 1.593 / 3.895 / 4.985      | 1.435 / 1.800 / 2.786       |
| tagPageCachedCount (n100)     | 1.209 / 1.610 / 2.819  | 2.047 / 3.321 / 5.419      | 5.118 / 6.502 / 6.792       |
| tagPageUncachedCount (n10)    | 4.319 / 5.408 / 5.408  | 35.059 / 52.400 / 52.400   | 289.528 / 322.651 / 322.651 |
| timelineBucketsCached (n100)  | 0.029 / 0.052 / 0.061  | 0.034 / 0.051 / 0.063      | 0.029 / 0.057 / 0.067       |
| timelineBucketsUncached (n10) | 4.819 / 5.392 / 5.392  | 76.543 / 137.843 / 137.843 | 688.566 / 772.742 / 772.742 |

For240 rows, legacy versus packed response bytes are172,667→103,076 (10k),171,288→101,697 (100k), and171,345→101,754 (1m): about40–41% smaller, without removing display fields. These are actual serialized fixture pages, not an all-library/network compression guarantee. Post-shrink first-boundary logical misses are45,056/49,152/57,344 bytes. Both parameterized capture directions use assets_visible_timeline range SEARCH, with no temporary sort.

P0's1m first-boundary-neighbor p95 was2,018.03ms; P1's separately sampled first/last boundaries are0.136/0.119ms. Cached bucket p95 is0.057ms at1m, but uncached aggregation remains772.742ms; cached tag-count pages are6.502ms versus322.651ms when recounting. Do not compare cached and uncached numbers as equivalent work. First-page1m p95 is4.502ms versus P0's1.77ms; this short-run increase is reported, and the evidence does not support claiming every route became faster. Ordinary reference100k measured backend reads remain below the proposed250ms target; no end-to-end release paint claim follows.

Full open-time validation and exact frozen selection are unchanged and were not rebenchmarked. Their P0 costs remain prerequisites for separate reviewed investigation; removing validation or replacing exact membership with cached counts is not an acceptable fix.

## Memory, I/O and trace limits

Bounded500ms libproc/ps observations cover core and observed descendants, excluding unowned XPC/WebContent/GPU and short-lived processes. Peak observed tree RSS is21.45/29.44/32.59MiB, with6/14/159 samples and2/1/2 transient sampler errors. Final observed RSS is about21.45/29.44/23.14MiB; the1m probe does not demonstrate an unbounded rising trend over this finite route. This is not a whole-application1GB pass or long-duration leak test.

Final observed process disk read/write counters are4,096/31,342,592 bytes (10k),81,920/346,980,352 (100k), and40,120,320/3,390,382,080 (1m). They include fixture creation/seeding/WAL and decoder work, have partial process coverage and are not per-original or physical-device throughput. SQLite logical pager bytes are a different measurement. No network workload was requested; OS socket traffic was not instrumented. No NAS/HDD/cold-network evidence is inferred. Each core catalog trace retained4096 events and dropped271; reported percentile sample loops completed independently, but clipped trace prefixes cannot reconstruct all timing/outcome counts. Media traces retain their own limits. No unbounded recorder or upload is introduced.

## Native checks and acceptance

The13 existing native desktop regressions pass (15.9seconds), including configurable text columns/sorts, image-free text mode, directory scope, bounded bidirectional feed, viewer return and preservation of generated originals. The app uses Rust debug plus optimized Vite desktop-test JavaScript with explicit React profiling, not the production release UI. Build identity: .context/performance/p1-native-build-identity.json (binary SHA2568099e14cd806c2a1499b055d7e26f47b33dd55ca2a2f941af3f361067f79ef0c).

The100k full native route **native-414bcfe5-f667-41c8-acde-194e844a3929-100000** passes in1m15s. Scripted scrolling lasted60,552ms (120 steps), followed by10 date jumps,4 resizes, text/grid changes, viewer/selection/return and five catalog queries with one observed real rescan overlap. Source32-entry manifests match. Date-jump command-to-target p50/p95/p99 is10/108/108ms (n10); resize command timing106/112/112ms (n4). Mounted tiles peak27 and resident summaries240; this route does not traverse all100k rows, while the existing960-record native regression exercises the720-summary window. Fresh-root scan-to-first-decoded-image873ms is n1, not a known-catalog warm paint measurement.

Renderer aggregate IPC p50/p95/p99 is3/990/2178ms (n105; max8447ms): substantial debug/overlap tails remain. This spans mixed commands and phase boundaries, without outcome attribution; request/response counts differ and cannot establish a success rate. The duration excludes subsequent page unpacking/rendering. React render p50/p95/p99 is0/1/2ms (n189, max5ms); coarse solver timers record0ms (n23), which does not mean no layout work. rAF intervals20/22/27ms (n3084, max117ms) are not presented-frame or dropped-frame measurements. The renderer retained its observations without drops; the backend retained4096 and dropped7550. These clipped prefixes cannot prove an end-to-end responsiveness target. Native observed RSS peaks172.09MiB, starts140.08MiB and ends146.20MiB across129 samples (one transient sampler error). Final observed disk read/write counters10,760,192/1,225,617,408 bytes include seeding; XPC/GPU/short-lived children remain incompletely attributed.

The separate1m native attempt **native-11313275-26d1-4316-a5b4-044d01f8b89c-1000000** failed its existing600-second seed-completion bound (case10m19.6s). No post-seed scrolling/viewer result was collected. A one-second local sample observed seed_catalog→SQLite WAL-checkpoint/read/write functions while the seed promise remained pending; raw paths/addresses/stacks were not retained. This does not prove the cause or physical-device throughput. Numeric/function-only note: .context/performance/p1-seed-stall.json. Keep the failure,32-entry matching source manifests and partial sampler evidence (1075 samples,9 errors, observed peak505.75MiB). Initial generated-image readiness was14,892ms (n1), before synthetic seeding; the evidence does not support attributing that delay to a specific code change. The smaller native attempt used the same app binary and completed normally. The timeout and FULL durability were not relaxed. A native1m functional/performance pass remains open even though the separate release1m catalog probe completed.

The harness now overwrites one small progress.json per owned run with an allowlisted phase/stage and elapsed milliseconds, rather than creating a log stream. Summary generation requires an explicit matching phase (P0 is the historical default); a standard-library regression test rejects cross-phase reports and retains unmeasured fields as null. Aggregate P1 evidence: .context/performance/c030a54d-619b-4fbc-91a0-002347850265-summary.json. P0 summaries were not overwritten.

| Check | Status and supported scope                                                                                                                                                                                                |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1-01 | Passed: shared Rust/TypeScript wire fixture, null/large-byte/empty/malformed/version/bounds checks and native UI decoding.                                                                                                |
| P1-02 | Passed: real pager red/green regression; exhaustive10k pages/all five sorts against independent Rust ordering; native filter/root/exact-directory/seek regressions; release plans and first/last boundaries at all sizes. |
| P1-03 | Passed: LRU entry/encoded-byte eviction/oversize rejection and worker/external-write invalidation; root/tag/missing/date/sort scope tests. No safety/selection cache is introduced.                                       |
| P1-04 | Passed for catalog-only seeded offline-root observations at all sizes. Actual offline NAS/HDD performance is not run.                                                                                                     |
| P1-05 | Passed for defined release backend loops at10k/100k/1m, binary/SQLite identity, payloads, manifests and disclosed sampler/trace limits. Whole-process memory/release presentation targets are not run.                    |
| P1-06 | Passed for13 native functional regressions and100k full route; failed1m fixture setup at600seconds, with its browsing scenarios not run. No release-frame/Windows pass.                                                   |
| P1-07 | Passed for five new independent before/after manifest pairs and retained preservation/durability/schema/dependency checks. Real-library mutations remain disabled.                                                        |

Release presented-frame/paint, Windows, true cold NAS/HDD, full process-tree memory, sustained cancellation and backup contention remain unrun or blocked. The production backup adapter is still absent; no mock or raw sync substitutes for it. The remaining measured costs are uncached aggregation/counting and native mixed-command/scan tails; P0's full-open and frozen-selection costs were not remeasured. Selective queries still depend on planner choices. Before further optimization, reproduce the1m native setup on sufficient local storage and collect complete native release traces with command/outcome attribution. P2 requires separate approval; retain existing row/window bounds and source guards before considering rendition work.

## Final validation

Full npm run check passes107 frontend tests and50 Rust tests (33 unit/15 library/2 media-process), strict types, lint, format checks, generated IPC drift, Rust fmt/clippy with denied warnings and frontend production build. Standard-library summary regression (one test) and Python compile checks pass. React Doctor remains73/100 with the same ten incumbent warnings; no new warning or suppression was introduced.

Normal unsigned macOS bundle builds33.86MiB. release:check passes capability/default dependency graph, helper resource hashes and test-bridge/renderer-harness exclusion. The packaged --diagnostic-sqlite result confirms the same SQLite version/source/options and userCatalogOpened=false; no normal user-catalog startup occurred. Artifact: .context/evidence/performance-packaged-sqlite-06701d11-8cee-4a84-a46a-a5583891321a.json; packaged binary SHA256e02e08b50a6dd1b244f54a75690f0dbbef1718941128b0e03e3146b4767598d7. This is build/provenance evidence, not signed distribution or release UI performance.

The local node_modules executable links were missing, so npm ci --ignore-scripts restored the existing lock without upgrades. npm audit reports nine high-severity findings in the WDIO/test-download dependency chain (.context/performance/p1-npm-audit.json); no automatic audit fix was applied. These remain a separate tooling/dependency review, not a claim that production dependencies are vulnerability-free. Initial SQLite unsigned-row conversion, test-file discovery and clippy type-complexity failures were corrected; the retained native1m setup timeout remains unresolved.

Owned-file whitespace/local-link checks, unchanged lock hashes, retained source-manifest equality and fresh independent re-inventory of the three P1 source trees supplement git diff --check because this scaffold is still untracked. Final result:210 owned text files/local links,17 matching retained source-manifest pairs and three matching fresh P1 re-inventories, with zero unexpected findings (.context/evidence/performance-p1-owned-validation.txt). Files are saved locally; no commit, push or branch rename was performed.

## Reproduction

See [DEVELOPMENT.md](DEVELOPMENT.md) for tools/build isolation. Scripts accept only supported row counts and create fresh UUID-owned fixtures, never arbitrary user-library/catalog inputs. Retain source manifests and failed runs.

```sh
npm run perf:catalog:build
npm run perf:p1 -- --rows 10000 100000 1000000
python3 scripts/performance/summarize.py --phase P1 --catalog-run UUID_FROM_RUN
python3 -m unittest discover -s scripts/performance -p 'test_*.py'
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes boundary_neighbors_use_a_bounded_index_seek -- --nocapture
npm test -- --run src/pageTransport.test.tsx
STILLHARBOUR_REACT_PROFILE=1 npm run desktop:test:build
npm run desktop:test
STILLHARBOUR_PERF_PHASE=P1 STILLHARBOUR_PERF_ROWS=100000 npm run perf:desktop
# Separate million-row attempt: setup timed out in the recorded run.
STILLHARBOUR_PERF_PHASE=P1 STILLHARBOUR_PERF_ROWS=1000000 npm run perf:desktop
npm run check
npm run desktop:build
npm run release:check
node scripts/performance/packaged-sqlite.mjs
```

The P0 Python process sampler and independent SHA256 manifest definitions/limitations remain in [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md). Preserve previous reports/evidence. Stop at the P1 gate; do not advance to P2 or any original-file operation without approval.
