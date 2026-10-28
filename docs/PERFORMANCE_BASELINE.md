# Stillharbour P0 performance baseline

Measured: 2026-10-28. Scope: instrumentation, disposable fixtures and evidence only. **P0 stops at this report; P1–P4 are not authorised or implemented here.** Preservation remains controlling. No user library/catalog was opened, scanned, relinked, migrated or cleared for these experiments. Existing production UI, query semantics, schema2, decoder, cache policy and durability are retained.

The original addendum's named acceptance file and S01–S17 register entries were absent. The [P0 checklist](PERFORMANCE_ACCEPTANCE_TESTS.md) identifies its reconstruction, and [SOURCES.md](SOURCES.md) records checked primary references without inventing the missing bibliography. The proposal is not evidence of implementation or performance.

This is historical P0 evidence. The owner subsequently approved P1; its changes, measurements and remaining gates are recorded separately in [PERFORMANCE_P1.md](PERFORMANCE_P1.md).

## Repository inspected

| Component           | Actual implementation and current limits                                                                                                                                                                                                                                                                                                                                                    |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gallery/list/viewer | [AssetBrowser](../src/library/AssetBrowser.tsx), [usePhotoViewport](../src/library/usePhotoViewport.ts), [useAssetFeed](../src/library/useAssetFeed.ts), PhotoViewer. TanStack row virtualisation; at most three240-summary chunks (720 resident); justified rows, timeline/grid/image-free text list and stable-ID selection already exist. No replacement UI or framework was introduced. |
| Database            | [catalog.rs](../crates/core/src/catalog.rs) actor: capacity16, one connection/writer thread, WAL, synchronous FULL, full integrity/foreign-key validation on open. [store.rs](../crates/core/src/library/store.rs) retains existing240-item keysets, indexes, exact-folder scope, query counts, date summaries and materialised frozen membership. No schema/index/query rewrite.           |
| Thumbnails          | [service.rs](../crates/core/src/library/service.rs), [workers.rs](../crates/core/src/library/workers.rs), [media.rs](../crates/core/src/media.rs): queue64/two workers,256/512px previews,256MiB indexed cache,32MiB input and128MiB decode allocation limits. Existing publication, source revision checks and read-back remain. The proposed pyramid/10GiB cache is not implemented.      |
| Tauri bridge        | [ipc.ts](../src/ipc.ts), library_request and the ID/revision/policy-scoped thumbnail protocol in [main.rs](../src-tauri/src/main.rs). JSON metadata IPC; image bodies stay out of React state. Existing native capability/root containment remain.                                                                                                                                          |
| Scheduling          | [jobs.rs](../crates/core/src/jobs.rs), one scan admission and two shared media permits per Library; bounded independent queues. No global per-device bandwidth/CPU scheduler or hard physical-I/O cap exists.                                                                                                                                                                               |
| Backup              | [backup.rs](../crates/core/src/backup.rs) contains reviewed helper argument boundaries and production_backup_available=false. There is no production restic/rclone backup executor to contend with browsing. Do not replace it with a mock/raw sync command.                                                                                                                                |

## Environment and build provenance

| Item                | Observed                                                                                                                                                                                                                                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Host                | Apple M1 Max,10 CPU cores,32GiB RAM, model Mac13,1; macOS27.0.1 build26A434; arm64. This exceeds the proposed4-core/16GB minimum and is not a minimum-machine result.                                                                                                                                                              |
| Storage             | Local source/catalog/cache workload on the host data volume. About15GiB free before runs; approximately11GiB during diagnostics. Backend catalogs/sources retained; no cleanup or purge used to create a cold result. Logical volume aliases do not establish physical-device independence.                                        |
| Toolchain           | Node26.3.1, npm11.16.0, Python3.14.0; rustc1.99.1 (58cf8f9c3,2026-10-13), Rust LLVM23.1.1; macOS SDK27.0. Bundled SQLite reports clang21.0.0.                                                                                                                                                                                      |
| Pinned dependencies | Tauri Rust2.12.5 / JS API2.12.4; React19.3.1; TanStack Virtual3.14.23; rusqlite0.41.0 / libsqlite3-sys0.39.1; image0.25.10; Tokio1.54.1. Existing lockfiles remain the authority.                                                                                                                                                  |
| Locks               | Cargo.lock SHA-256 `0b44ad7be9a2053441f85ac7c5e49ef0618cf5e4dad5c99de5e7fa948f3f528f`; package-lock.json `eab358a6345e96c3195256904292d24534df4033a1c4aa79f1b26086f1afeda0`. Both remain unchanged; no dependency upgrades.                                                                                                        |
| Release backend     | Optimised Rust core example with fixture-probes explicitly enabled. Executable SHA-256 `ade9ba29694773667fd9d12c0abc866d21b7e2b4d6d7e0b746fdaa6fdaf24423`. Media-worker executable `4c363544bfce6a99517a64d1d1eed01261e8e3356b0f6c03480707cab5191a03` from the existing release build; source guard/decoder behaviour unchanged.   |
| Native UI           | Actual macOS WKWebView via the pinned embedded driver, Rust debug build with optimised Vite desktop-test frontend. Later profiling runs explicitly use the installed react-dom/profiling variant. They are diagnostic observations, not production release UI benchmarks.                                                          |
| WebView/display     | WebKit framework22625.1.29.11.28 (short version22625). User-agent compatibility token AppleWebKit/605.1.15, which is not the framework build. Profiling100k run reports DPR1,3440×1440 screen,1180×748 content viewport; native window sizes also660×560 and900×700. Physical refresh/DPI changes were not independently measured. |

Environment evidence: `.context/performance/6edf9952-474f-4d13-bf69-77252a404479-environment.json` and `environment-supplement.json`. Some exploratory release measurements overlapped development compilation/test activity; this is an uncontrolled developer workstation, not an isolated benchmark lab. No CPU-frequency/thermal/power-state normalisation is claimed.

### Actually linked SQLite

The release core and native debug process report **SQLite3.54.1**, source ID `2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76`. This matches the checked upstream release, is not the withdrawn3.52.0, and includes the WAL-reset fix introduced in3.51.3. [SQLite WAL](https://sqlite.org/wal.html), [3.54.1 release](https://sqlite.org/releaselog/3_54_1.html).

Runtime catalog settings remain WAL, synchronous2/FULL, page size4096, schema2. Recorded compile options include THREADSAFE=1, ENABLE_STAT4, FTS5, RTree, API_ARMOR and DEFAULT_WAL_AUTOCHECKPOINT=1000; full options are in raw results. No checkpoint/durability tuning occurred. The checked advisory table's listed fixed releases precede this build; the3.54.1 release's32-bit issue does not apply to arm64. This is a dated provenance/advisory check, not a vulnerability-free guarantee. Recheck at release. [SQLite advisories](https://sqlite.org/cves.html), [release history](https://sqlite.org/changes.html).

A standalone `--diagnostic-sqlite` flag opens an in-memory connection and exits before Tauri/user-catalog setup. The packaged release provenance result is recorded in the final delivery evidence below; no personal catalog is needed for that check.

## Fixtures, identities and protocols

Release run ID: `6edf9952-474f-4d13-bf69-77252a404479`; separate new catalogs at10,000,100,000 and1,000,000 rows, seed20261028/version1. Directories `.context/performance/<run>-<rows>/` contain ownership metadata, baseline.json and independent manifest-before/after.json. Each source inventory has33 entries including the generated root wrapper. All three before/after inventories match.

Synthetic observations include two offline-state roots,5% unknown dates,20% in one dense equal-timestamp day, landscape/portrait12000:40 extremes, tags,10 extension categories and20% video records (64KiB or8GiB logical sizes). Offline is a catalog fixture state, not a physically disconnected NAS. No8GiB video payload was created or decoded. These are synthetic metadata rows, not proof of a million media decodes or valid RAW/HEIC/AVIF/video support.

The independent media corpus is the existing generated JPEG/PNG/WebP, hidden-file, companion, hostile-name, symlink and malformed/unsupported-marker fixtures. Native UI also scans these real generated sources. The marker file is not valid HEIC coverage. Real large video/RAW/HEIC decoding remains unavailable/unrun. All original-file handles, revision/identity checks and worker limits remain unchanged.

[manifest.py](../scripts/performance/manifest.py) independently uses SHA-256, separate from the native BLAKE3 checks. It records hashed relative names, entry types, bytes, mode, mtime, inode/device, uid/gid and link targets without following links. It excludes access-time changes and does not establish ACL/xattr/alternate-stream preservation. Backend fixtures are retained; native automation's existing disposable directories are released after the after-manifest. No personal source/cache cleanup is performed.

| Condition           | Protocol and actual coverage                                                                                                                                                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Warm/reopen         | Seed/checkpoint fixture, keep OS cache uncontrolled, open seven new Catalog connections with full validation; then repeated page/seek/neighbor/filter operations. Reopen is not first populated gallery paint.                                                                        |
| Fresh preview cache | New native-test app data directory; first generated-source Save/Scan to first decoded image is observed. This includes scan/helper/poll/driver work and is not a known-catalog warm-open target.                                                                                      |
| Warm UI route       | After seeding, reset bounded traces;120 scripted scrollTop steps of+330/−250px,500ms apart, then10 deterministic date-rail jumps, four window resizes, Text list, five query samples during a real rescan, selection and cached viewer next/return. Source manifests bracket the run. |
| App/OS cache cold   | Separate processes are used, but the OS cache is not purged. No true OS-cache-cold, reboot/cold NAS/HDD or production app-cold populated-gallery result is claimed. A future controlled machine/corpus must specify these separately.                                                 |
| Background backup   | Blocked: no production adapter. No substitute transfer was run.                                                                                                                                                                                                                       |

## Measurement definitions and release catalog results

All timings below are milliseconds, reported **p50 / p95 / p99**, nearest rank `ceil(q*n)-1`. Small n7/10/3 tails are maxima, not calibrated long-tail guarantees. Page limit240. Core durations include catalog actor scheduling and result creation; driver/browser/IPC work is excluded. Reopen includes full integrity/foreign-key validation and close. Raw data is retained, including actual EXPLAIN QUERY PLAN output.

| Operation                                       | n per size | 10k                   | 100k                       | 1m synthetic                   |
| ----------------------------------------------- | ---------- | --------------------- | -------------------------- | ------------------------------ |
| Catalog connection reopen + full validation     | 7          | 58.21 / 59.37 / 59.37 | 790.05 / 1047.62 / 1047.62 | 11250.99 / 11589.21 / 11589.21 |
| First 240-item page                             | 100        | 1.40 / 1.73 / 1.96    | 1.58 / 1.86 / 2.17         | 1.39 / 1.77 / 1.93             |
| Next keyset page                                | 100        | 1.46 / 1.80 / 1.92    | 1.54 / 1.93 / 1.96         | 1.35 / 1.69 / 1.89             |
| Root-scoped page                                | 50         | 1.38 / 1.77 / 1.90    | 1.63 / 2.11 / 2.37         | 1.43 / 1.67 / 2.25             |
| Filename contains filter                        | 30         | 3.95 / 5.27 / 5.55    | 30.56 / 33.90 / 34.05      | 290.05 / 300.53 / 316.01       |
| Tag filter                                      | 30         | 4.34 / 5.42 / 5.50    | 31.24 / 35.43 / 38.15      | 286.50 / 303.03 / 313.43       |
| Seeded random-year seek                         | 100        | 1.56 / 1.89 / 2.18    | 1.87 / 2.17 / 2.66         | 1.71 / 2.18 / 2.24             |
| Timeline bucket summaries                       | 10         | 4.35 / 5.18 / 5.18    | 56.44 / 63.25 / 63.25      | 672.97 / 1099.86 / 1099.86     |
| Viewer boundary neighbors                       | 100        | 6.59 / 8.75 / 22.08   | 125.32 / 138.98 / 146.20   | 1669.39 / 2018.03 / 2738.20    |
| Create + release selection (fixture round trip) | 3          | 26.78 / 27.53 / 27.53 | 351.54 / 354.33 / 354.33   | 4142.33 / 4262.09 / 4262.09    |
| Selection creation only (trace)                 | 3          | 12.19 / 15.19 / 15.19 | 170.83 / 174.74 / 174.74   | 1931.49 / 2054.92 / 2054.92    |

The selection fixture round trip includes releasing only the just-created test membership; it is not a source cleanup operation. The separate trace row isolates creation/commit, the actual user-facing freeze boundary. Ordinary page queries are fast in this synthetic cohort. Filename/tag filters scale to about300ms p95 at1m, and buckets exceed1s p95; the100k reference-size query measurements stay below the proposed250ms backend threshold. This does not pass release end-to-end/real-media targets.

Representative EXPLAIN statements show the date page scanning the existing assets_timeline index, bucket aggregation scanning assets with a temporary GROUP BY B-tree, and tag count scanning assets plus json_each. These explain statements match the base query shapes, not a complete capture of every parameterised production statement. Fixtures run ANALYZE explicitly after seeding; normal scan workflows do not automatically do so. P0 did not change application plans or indexes. The parameterised neighbor plans still need separate diagnosis before choosing a rewrite.

### Three largest measured release boundaries and smallest proposed fixes

| Boundary                        | Measured1m p95    | Smallest proposed next step, not implemented                                                                                                                                                                                                 |
| ------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Catalog reopen/full validation  | 11,589.21ms (n7)  | Profile validation separately and show a truthful native loading surface while it runs. Keep integrity/foreign-key checks and catalogue admission intact; do not skip verification for a faster number.                                      |
| Frozen selection creation       | 2,054.92ms (n3)   | Review bounded membership construction/admission with a coherent frozen revision, cancellation and exact membership. Preserve hidden-member/action scope and durable records; do not replace it with a live expanding destructive selection. |
| Viewer boundary-neighbor lookup | 2,018.03ms (n100) | Inspect both parameterised directions and boundary/no-result cases, then seek through existing date/ID indexes where correct. Preserve equal/unknown-date ordering and all root/filter/directory constraints.                                |

These rank comparable release catalog boundaries, not debug UI operations or synthetic seed setup. Code inspection identifies plausible causes; this is not a causal CPU-profile proof or permission to implement those fixes.

## Memory, disk and transport observations

| Release rows | Peak observed RSS MiB | First → last observed RSS MiB | Catalog MiB after selection | 240-item page JSON bytes | OS-accounted read / write MiB |
| ------------ | --------------------- | ----------------------------- | --------------------------- | ------------------------ | ----------------------------- |
| 10,000       | 26.6                  | 0.0 → 20.1                    | 14.6                        | 172,644                  | 0.0 / 43.1                    |
| 100,000      | 34.1                  | 6.9 → 27.2                    | 146.0                       | 171,265                  | 8.3 / 553.7                   |
| 1,000,000    | 44.3                  | 6.3 → 44.3                    | 1469.3                      | 171,322                  | 26.9 / 5392.2                 |

These process samples span seeding and measurement. They are core plus observed descendants, sampled every500ms; RSS sums can include shared pages and are not unique physical memory. libproc disk counters cover observed process lifetime traffic, including seed/selection writes, not a physical endpoint breakdown. Short-lived children can be missed. XPC WebContent/GPU ownership is not inferred, so neither this table nor the native samples can pass the whole-application<1GB objective.

P0 adds separate source/cache payload counters and SQLite pager-miss/page-write estimates with at most16 opaque logical filesystem-volume aliases. The initial release trace cohort preceded the final pager/alias extension; final native diagnostic traces exercise it. Pager bytes are not OS disk reads, and volume IDs are not proof of physical-device topology. Read errors, memory mapping, OS buffering/fsync, unknown aliases and dropped events limit coverage. No aggregate hard-I/O cap is claimed. [SQLite connection status](https://sqlite.org/c3ref/c_dbstatus_options.html).

IPC sizes are transient UTF-8 JSON object sizes, excluding transport envelopes/escaping/copies. The page projection is roughly171–173kB for240 rows in these fixtures. No photo contents, filenames/paths, EXIF/GPS or credentials are retained in traces. No content network/backup workload was requested; loopback automation traffic exists, and OS socket/network bytes were not measured.

The bounded worker tests make10 attempts per format: generated JPEG/PNG/WebP all succeed; malformed/marker inputs all fail without source changes. Worker wall time includes process startup, pipes, decode/resize/encode and JSON. At100k the JPEG worker p50/p95/p99 is20.23/33.90/33.90ms; in-process JPEG decode/resize/encode is2.12/3.04/3.20ms (n30). These are small generated-image results, not full-resolution camera/video throughput. The10k JPEG first-worker outlier333.84ms is retained, not discarded.

## Native UI results, failures and acceptance

Completed full native routes (source manifests matched in every listed run):

| Synthetic rows | Trace ID                                            | Route duration | First generated decoded image | Observed core/descendant RSS first → last / peak MiB | Scan overlap observations              |
| -------------- | --------------------------------------------------- | -------------- | ----------------------------- | ---------------------------------------------------- | -------------------------------------- |
| 10,000         | native-d77b4b8f-d54d-4835-9311-ffd5d1489893-10000   | 60.59s         | 886ms                         | 143.5 →166.7 /170.5                                  | 1 of5 queries began while scan running |
| 100,000        | native-0bfaea31-4171-4b71-a9ae-2a8a4df3c067-100000  | 60.49s         | 889ms                         | 141.3 →168.9 /179.6                                  | 1 of5                                  |
| 1,000,000      | native-a4b929a3-d140-4e7b-a5d7-0e9f6907b7a4-1000000 | 60.42s         | 881ms                         | 146.0 →150.0 /182.1                                  | 2 of5                                  |

The10k route used ordinary optimised React with a Rust debug backend;100k/1m rows here use the explicit React profiling variant. The10k display was DPR2 on1296×2304; later profiling runs were DPR1 on3440×1440. These are different diagnostic conditions, so do not infer scaling ratios. First-image values have n1 and include new-root scan; they are not warm-open p95. Peak memory remains an incomplete process-model observation. The scans were brief rescans, not sustained whole-library concurrent ingest.

| Native metric (ms p50/p95/p99)          | 10k                 | 100k profiling      | 1m profiling           |
| --------------------------------------- | ------------------- | ------------------- | ---------------------- |
| React render actualDuration             | Not collected       | 0 /1 /2 (n221)      | 0 /1 /1 (n286)         |
| Justified-row solver                    | 0 /0 /0 (n28)       | 0 /0 /0 (n22)       | 0 /0 /0 (n2)           |
| Aggregate instrumented IPC completion   | 5 /18 /55 (n103)    | 6 /232 /1454 (n110) | 3 /23914 /26915 (n147) |
| Date-jump command to target rows        | 108 /109 /109 (n10) | 143 /213 /213 (n10) | 309 /2305 /2305 (n10)  |
| Resize command + deliberate100ms settle | 106 /115 /115 (n4)  | 106 /112 /112 (n4)  | 106 /116 /116 (n4)     |
| Image load to double-rAF opportunity    | 48 /65 /65 (n164)   | 29 /35 /36 (n116)   | 29 /35 /35 (n33)       |

**A recorded0ms is below these clocks' useful resolution, not zero work.** React actualDuration covers rendering, not the full browser frame/commit/presentation. Resize includes an explicit100ms settle and driver time, so it cannot establish a<100ms visual-acknowledgement target.

**Severe additional native diagnostic finding:** the1m debug run's IPC tail is about24–27seconds, even though sampled React render work is small. The retained core prefix contains catalog queue-wait p50/p95/p99 of0.61/1735.76/19008.59ms (n781), with a19,007.61ms maximum catalog work item. This is evidence of long actor work/backpressure, not proof that every delay comes from JSON transport. That frontend phase has147 IPC-duration samples but only42 response-size samples; rejected calls, phase boundaries and clipping prevent an exact success rate or per-operation attribution. Do not describe the functional route pass as a responsiveness pass. Before changing scheduling or SQL, add operation/outcome correlation and inspect the responsible query/scan statement. P0 made no fix to this backlog.

Observed mounted-asset maxima were31/27/27; retained-summary maxima were240 in these routes. This supports a bounded observed working set, not traversal of all records or every720-summary edge. The separate existing960-row native regression exercises bidirectional720-summary paging. Ten date jumps reached target rows at every size, including the million-row fixture; the huge catalog is not represented by a million mounted DOM items. Exhaustive logical scroll-coordinate/DPI stress remains unrun.

Trace limits were exercised honestly: warm frontend drops0/267/2124; warm backend retained4096 events and dropped7161/10409/10286. Core dropped counts include recorder contention. These are clipped prefixes, not complete60-second backend timelines. The three release catalog traces each retain2119 events with zero drops. Sampler drops were zero, with1/4/2 transient process-read errors; short helpers can exit between discovery and sampling.

The first10k script failed on an ambiguous strict selector after scrolling; its manifest matched. Two1m attempts hit the pinned evaluator's30-second setup deadline; a promise/poll attempt hit180seconds. All failed source inventories matched. The final1m case passed in4m14s after allowing600seconds for fixture setup (900-second case cap); the longer setup limit is not an application speed improvement. The prior failures and a six-step smoke result are retained and are not counted as passing60-second routes. Artifacts remain in `.context/performance/native-*/`.

| Scenario                                       | Status  | Supported scope / remaining limitation                                                                                                                               |
| ---------------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0-01 repository inspection                    | Passed  | Source paths/components and unavailable backup identified above.                                                                                                     |
| P0-02 provenance/advisories                    | Passed  | Runtime version/source/options and immutable locks recorded; packaged runtime check is included in final delivery evidence. Recheck at release.                      |
| P0-03 opt-in/bounds/privacy                    | Passed  | Cap/privacy tests and drop evidence; production exclusion checked during final release validation. No telemetry path.                                                |
| P0-04 seeded catalog coverage                  | Passed  | All three sizes and specified synthetic cases; real large-video/RAW/full-media benchmarks are blocked/unrun.                                                         |
| P0-05 source preservation                      | Passed  | All release/full native/failed native source inventories match; ACL/xattr/streams are outside this manifest proof.                                                   |
| P0-06 backend queries (100k synthetic)         | Passed  | Recorded operations below250ms p95 except unrelated open/selection;1m filename/tag filters exceed250ms and buckets exceed1s. Not a full-media/UI gate.               |
| P0-07 warm populated gallery release paint     | Not run | Fresh-root first-image proxy is not known-catalog populated paint; needs controlled native release/corpus/presentation tracing.                                      |
| P0-08 release text first-page timing           | Not run | Native regression has zero images/nonempty thumbnail batches in text mode. No known-catalog release first-page p95.                                                  |
| P0-08a thumbnail-free text functionality       | Passed  | Existing13-case native regression checks no images or nonempty thumbnail requests caused by Text list.                                                               |
| P0-09 release presentation/whole-frame targets | Not run | Actual60-second native routes; clipped/coarse debug measurements cannot pass<5% missed presentation or full JS-work target.                                          |
| P0-09a scripted scroll routes/bounds           | Passed  | All three sizes completed actual60-second routes with the observed working-set bounds; clipped data is not complete frame evidence.                                  |
| P0-10 jumps/resize/viewer/selection            | Passed  | Native target-row/viewer/selection assertions passed;1m IPC stalls and coarse/driver timings preclude a fast-response claim. DPI/exhaustive coordinate test not run. |
| P0-10b next visual acknowledgement             | Not run | Completion timestamps and explicit driver settling are not a visual acknowledgement measurement. Long IPC tails are separately flagged.                              |
| P0-11 whole-app<1GB                            | Blocked | XPC/GPU and short-lived helper attribution incomplete; observed core/descendant trends only.                                                                         |
| P0-12 brief scan contention                    | Passed  | Real bounded fixture rescans overlapped1/1/2 of five queries. No cancellation acknowledgement/stop experiment was collected.                                         |
| P0-12b cancellation acknowledgement/stop       | Not run | No cancellation timing experiment was collected; no success is inferred from the brief rescans.                                                                      |
| P0-13 backup contention                        | Blocked | Production backup adapter absent; no mock/raw substitute.                                                                                                            |
| P0-14 scoped logical/process I/O               | Passed  | Logical role/volume counters, pager estimates and OS process totals; clipping/errors/OS buffering limit completeness.                                                |
| P0-14b physical endpoint/network attribution   | Blocked | Requires complete process/VFS/device/socket attribution and real transport workloads.                                                                                |
| P0-15 cold/transport/platform matrix           | Not run | No true OS-cache-cold, NAS/HDD, Windows, physical display/DPI or production app-cold benchmark.                                                                      |
| P0-16 report/P1 boundary                       | Passed  | Costs, proposed smallest fixes and prerequisites recorded; P0 stops here.                                                                                            |

The initially optimised React frontend emitted no Profiler samples because ordinary production React disables profiling. The explicit test-only profiling alias fixes that measurement setup; normal production React remains unchanged. [React Profiler](https://react.dev/reference/react/Profiler). Image load-to-double-rAF is only a paint-opportunity proxy, never actual presentation. Observed JS clocks are coarse; a recorded0ms does not establish zero work. The rAF cadence varied around20–33ms across these harness/display conditions and is not evidence of a60Hz display or missed-frame percentage.

## Executable diagnostics and next gate

- [Release fixture example](../crates/core/examples/performance_probe.rs) and [measurement implementation](../crates/core/src/library/performance_probe.rs).
- [Owned runner](../scripts/performance/run.py), [independent manifest](../scripts/performance/manifest.py), [bounded process sampler](../scripts/performance/process_stats.py) and [summary script](../scripts/performance/summarize.py).
- [Native route](../tests/performance/gallery.e2e.mjs), [native configuration](../wdio.performance.conf.mjs) and [packaged SQLite check](../scripts/performance/packaged-sqlite.mjs).

```sh
npm run desktop:build
node scripts/performance/packaged-sqlite.mjs
npm run perf:catalog:build
npm run perf:catalog -- --rows 10000 100000 1000000
STILLHARBOUR_REACT_PROFILE=1 npm run desktop:test:build
STILLHARBOUR_PERF_ROWS=100000 npm run perf:desktop
STILLHARBOUR_PERF_ROWS=1000000 npm run perf:desktop
python3 scripts/performance/summarize.py --catalog-run RUN_UUID
```

Replace RUN_UUID with the UUID printed by the catalog runner; the recorded run above can be summarised directly.

Core and renderer traces each retain4096 numeric events, drop on capacity/recorder contention and expose drop counts. Process sampling and identity tracking each cap at4096, with a fixed runtime bound. Instrumentation is off until explicitly started; the production renderer/test bridge is absent. Counting/observing adds overhead, so these are instrumented observations. No log upload service or automatic on-disk logging is installed.

P1 requires separate approval of read-only catalog projections/indexed seeks/summary work, reproduction of both-direction neighbor plans, equality/unknown-date/root-scope regressions and independent preservation evidence. Native release UI/presentation, a representative full-media corpus, complete process attribution, controlled cold-cache/transport workloads and Windows still need their own evidence. Later virtualisation/rendition/resource-scheduling changes follow the owner's sequence; none are implemented by this P0 baseline. Backup contention additionally needs the reviewed production adapter. Existing M1/platform/preservation/recovery gates remain open.

## Final P0 validation and delivery

Full `npm run check` passes:104 frontend tests,45 Rust tests (28 unit,15 library,2 media-process), formatting, lint, strict types, binding drift, Rust fmt/clippy with warnings denied and production frontend build. The existing13-case native regression suite passes in15.6seconds. Selected full performance routes at10k/100k/1m pass with source manifests matched; raw attempts, including all four failures, remain available. Python scripts compile, and local ownership/whitespace/link checks are recorded in IMPLEMENTATION_LOG. React Doctor reports73/100 with the same ten incumbent warnings (six complexity, the existing finally-reset false positive, three imperative viewport-scroll warnings); no new diagnostics or rule suppressions.

The normal unsigned macOS bundle builds (33.84MiB), and release:check passes normal capability/dependency/helper-resource/renderer test-exclusion checks. Actual packaged execution of `--diagnostic-sqlite` reports3.54.1 and the exact source ID above, `userCatalogOpened=false`, release binary SHA-256 `82e439e60d07c32a4f2cba20c6c1bf2c94813be0b31931a9b1f5bece54508a38`. Evidence: `.context/evidence/performance-packaged-sqlite-d17a8569-97fa-4951-bcc8-b138fe6e3c0e.json`. This closes the host packaged SQLite provenance check without accessing a personal catalog. No normal-app restart, user scan/cache clear, dependency upgrade, commit, push or branch rename was performed.

Validation logs use `.context/evidence/performance-*`; the consolidated numeric index is `.context/performance/6edf9952-474f-4d13-bf69-77252a404479-summary.json`. Core release traces were taken before the final logical-pager/alias extension; later native traces and the bounded-recorder tests cover that extension. All numbers are scoped to their recorded build, display, corpus, cache condition and finite trace coverage. P0 is complete as an evidence/report phase; remaining acceptance gates and P1 work stay explicitly unapproved.
