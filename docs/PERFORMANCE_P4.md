# P4 — local stress evidence

Owner request: 2026-10-29, “Lets start on the next stage,” continuing the performance track. [Task scope](../tasks/P4_LOCAL_STRESS.md) authorizes the available local read-only checks. P3 production backup, physical-device/network mapping and accounted byte pacing remain blocked. P0–P3 evidence is historical; this stage never upgrades it to new measurements.

## Measurement boundary

Production compilation deliberately rejects desktop-test instrumentation. Keep that guard. Build and test the actual unsigned default-feature release application and its worker modes; use independent release core probes for catalog and scheduling measurements. Drive generated-source UI scale routes in the separately identified debug desktop-test application. Release core/worker timings and debug UI timings have distinct scopes. The native route's rAF observations cannot pass presented-frame targets, and the observed-descendant sampler does not establish complete WebContent/GPU/XPC attribution or a whole-application memory budget.

The existing native route could exhaust its 4096-event buffers during long routes. Add consuming diagnostic reads for recorded events/loss counts while keeping recording, observer lifetime, timestamps and volume aliases intact. Write up to 32 sequential segment files, each with at most 4096 frontend and 4096 backend events. Drain every ten scroll steps and each date jump. The P4 report accepts only explicitly selected P4 runs and rejects lost events, short routes, changed/non-lossless source manifests, missing resident/mount observations, over 720 summaries, incomplete jump/resize routes and nonempty text-mode thumbnail requests. Missing metrics remain unknown. No production cache/scheduler/decoder/resource setting changes.

## Reproduce

Run builds and native drivers serially; do not rebuild dist during a driver run. Commands use generated or UUID-owned fixtures only, with no real-library arguments or cleanup:

```sh
npm run desktop:build
npm run release:check
node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example performance_probe --example scheduler_probe
python3 scripts/performance/run.py --phase p4
python3 scripts/performance/scheduling.py
python3 scripts/renditions/p2_probe.py --build release --worker target/release/bundle/macos/Stillharbour.app/Contents/MacOS/stillharbour
node scripts/cargo.mjs test --locked --release -p stillharbour-core --features fixture-probes
npm run desktop:test:build
STILLHARBOUR_PERF_PHASE=P4 STILLHARBOUR_PERF_ROWS=10000 npm run perf:desktop
STILLHARBOUR_PERF_PHASE=P4 STILLHARBOUR_PERF_ROWS=100000 npm run perf:desktop
STILLHARBOUR_PERF_PHASE=P4 STILLHARBOUR_PERF_ROWS=1000000 npm run perf:desktop
python3 scripts/performance/p4_report.py --native-run native-UUID-10000 --native-run native-UUID-100000 --native-run native-UUID-1000000
npm run desktop:test
python3 -m unittest discover -s scripts/performance -p 'test_*.py'
npm run check
npm run release:check
git diff --check
```

The catalog runner's `--phase p4` reuses the P1 stopwatch/query protocol and consumes trace segments between timed groups. It rejects any recorded catalog trace loss; retain its fresh P4 run ID/build identity. Earlier P1-protocol diagnostic runs with dropped ancillary events remain historical failed-coverage evidence. For an isolated native copy, set STILLHARBOUR_PERF_APP to the symbol-verified debug desktop-test executable; the runner records its SHA-256 and verifies it again at completion. Each native invocation requires a fresh app session. The report accepts one to three explicit native run names, validates phase and segment bounds and writes a new report under .context/p4. The million-row catalog is synthetic observations, not a million decoded photos. OS caches are uncontrolled; no system-wide purge or cold-NAS inference.

## Results

Host: macOS 27.0.1 (26A434), arm64 Mac13,1, 32 GiB RAM, 10 logical CPUs, Rust 1.99.1, Node 26.3.1/npm 11.16.0. .context/p4/environment.json records toolchain, host/WebKit framework and lock/binary hashes. Cargo.lock SHA-256 6f4e5ee5d3d6be2a21beb28b1da6472f39c079cbc5f31e08354ca0050ce30fe9; package-lock.json eab358a6345e96c3195256904292d24534df4033a1c4aa79f1b26086f1afeda0; both unchanged. Workloads run serially on an ordinary development host, with uncontrolled OS caches. No cold-storage/quiet-machine guarantee or system cache purge.

The unsigned default-feature .app builds 34.42 MiB, executable SHA-256 347fd4eb0ec7239e5818b7997290942a4506faf3f681a107c5a4cdbe62bae609. Production capability/default-dependency/frontend exclusions and reviewed ExifTool checksums pass. Read-only packaged SQLite provenance run 24e09890-6e3d-486a-a6ac-37587b58592d reports 3.54.1, source ID 2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76 and userCatalogOpened=false. No dependency upgrade or new upstream advisory-free claim is made.

### Release catalog probes

Current-source P1-protocol diagnostic run4bd7942e-85d0-4e94-a4fe-d0752db4b933 completes all sizes with unchanged sources but drops 271 ancillary events per size. Preserve that incomplete-coverage run. P4-protocol run26c4b889-dc55-42c3-b84b-0bcbf1440183 repeats all sizes with the same stopwatch/query work, drains between timed groups and retains 4367 events per size in eleven bounded snapshots with zero drops. All independent source manifests match. Environment and executable hashes are in `.context/performance/<run>-environment.json`; raw metrics/trace segments/plans/source inventories remain in each `<run>-<rows>/baseline.json` namespace.

Nearest-rank percentiles; milliseconds. Ordinary queries/seek/cached buckets have n=100; uncached buckets have n=10, so p95/p99 equal their maximum. Connection/derived-cache reset definitions remain in the existing probe; seeding is excluded from query timers. The synthetic roots contain no corresponding photo originals.

| Synthetic rows | First page p50/p95/p99 | Indexed date seek p50/p95/p99 | Uncached month summary p95 | Cached month summary p95 |
| -------------- | ---------------------- | ----------------------------- | -------------------------- | ------------------------ |
| 10,000         | 1.095 / 1.214 / 1.309  | 1.218 / 1.575 / 2.259         | 5.435                      | 0.035                    |
| 100,000        | 1.101 / 1.272 / 1.376  | 1.436 / 1.702 / 1.739         | 62.819                     | 0.057                    |
| 1,000,000      | 1.126 / 1.381 / 1.593  | 1.501 / 1.660 / 1.805         | 633.771                    | 0.041                    |

The 100k ordinary page/seek observations meet the proposed backend 250 ms objective for this workload only. The largest measured catalog cost is the uncached million-row month aggregation (~634ms p95); its cached hit is ~0.041ms p95. EXPLAIN confirms boundary index searches without temporary B-trees. Retain the existing bounded summary cache; these results do not justify a schema migration, durability/startup-validation reduction or production resource-limit change. The finite native route does not measure known-catalog release first-paint percentiles. Observed descendant memory and process-accounted I/O remain partial and are retained in raw reports, without a full-memory/physical-I/O pass.

### Packaged worker and synthetic contention

Packaged worker run e7466674-6085-4702-b018-e6a4fca987c6 passes 19 cases/120 measured invocations over generated 3072×2048 RGB noise JPEG/PNG/TIFF and all five tiers, with matching independent inventories of six sources. Retain malformed/over-dimension/writable-descriptor/no-upscale cases. Each invocation starts a fresh process; OS caches remain uncontrolled. Eight samples per format/tier; p95/p99 equal the maximum. `.context/p2-renditions/<run>/` retains observations, summaries and manifests, with the packaged binary hash above.

| Format | 2048px p50/p95/p99 (ms)     | Sampled direct-child peak (MiB) |
| ------ | --------------------------- | ------------------------------- |
| JPG    | 165.067 / 170.380 / 170.380 | 80.39                           |
| PNG    | 116.886 / 123.698 / 123.698 | 89.73                           |
| TIF    | 94.553 / 111.809 / 111.809  | 105.86                          |

Direct-child sampling omits short allocation spikes and the app/WebView/GPU. No GUI queue latency, original integrity/fidelity, higher-tier camera matrix or complete process-tree claim follows.

Release synthetic scheduling run 8c868e20-5c91-4026-8a35-a7a87f929475 completes 240 read steps, four generated 256 KiB files, 60 MiB logical reads and a deliberate 2 ms simulated helper step per admission. All four lossless independent source entries match; zero trace drops. Visible wait p50/p95/p99=2.413/3.292/6.895ms (n=60). The unchanged P3 runner retains raw samples and executable/lock hashes in `.context/scheduling/<run>/`. This is synthetic read contention, with no real backup, transfer, physical-device throughput or media decode inference.

### Native routes and final checks

The three native routes pass their functional assertions: 120 scroll steps each, ten date jumps, four resizes, selection/viewer return, no nonempty text-mode image requests, and independently matching lossless source inventories. All use the symbol-verified debug desktop-test executable SHA-256 eb836118c733eee4844aefe09f57c472362435b7dfe4996c0132b864414965ae. This binary predates only the additional standalone P4 catalog-probe wrapper, which the GUI route does not call. Each run has 25 bounded trace segments.

| Synthetic rows / native UUID                     | Scroll duration (s) | IPC p50/p95/p99 (ms; n) | Date jump p50/p95/p99 (ms; n=10) | Peak mounted / retained | Frontend / backend drops |
| ------------------------------------------------ | ------------------- | ----------------------- | -------------------------------- | ----------------------- | ------------------------ |
| 10,000 / 6bf78bb8-4ba1-46e5-b459-ba4d291070bd    | 61.276              | 5 / 13 / 42; 454        | 107 / 108 / 108                  | 20 / 240                | 0 / 0                    |
| 100,000 / b34e3654-5d84-4bae-b90b-7e95895d68a0   | 61.113              | 8 / 20 / 190; 490       | 107 / 112 / 112                  | 43 / 480                | 0 / 2                    |
| 1,000,000 / c0a64100-933f-4ccc-bdbc-a24bdb202546 | 63.134              | 8 / 155 / 2692; 466     | 107 / 111 / 111                  | 27 / 480                | 0 / 0                    |

The strict combined report .context/p4/report-ea1ec531-3890-41ea-85ce-d3228d29b787.json exits 1: 10k/1m pass their scoped diagnostic checks, but 100k fails trace completeness. Its resize/text-entry and final date-jump segments lose one backend event each while containing 205 and 67 retained events, respectively, far below capacity. This identifies recorder contention rather than buffer overflow; keep the recorder's nonblocking drop policy and retain this failed evidence instead of changing safety limits or rerunning until green. The two earlier native report files are retained too. The report-command JSON records the exact invocation/exit status. Functional route success does not repair incomplete trace coverage.

The million-row seed takes406.360s wall time before browsing timers start. Its generated-source scan overlaps four of five sampled catalog queries (24–25ms). The later viewer/search phase contains multi-second IPC observations, with maximum 5005 ms; the backend records a 2922.876 ms date-bucket span and catalog queue waits up to 2749 ms. Source inspection confirms filename search uses escaped leading-wildcard LIKE. This is evidence of slow filtered aggregation/actor contention on the debug synthetic million-row workload, not a measured release endpoint attribution or ordinary indexed-page regression. A release reproduction and reviewed search/index design remain prerequisites to production tuning; do not hide this cost behind fast unfiltered page numbers. No full-catalog array or production schema change is introduced here.

Observed descendant RSS peaks at 169.80 MiB (n=120) and 176.19 MiB (n=129) for 10k/100k, with sampler error counts 3/2 and zero sampler drops. These include setup and cannot establish steady-state or full WebContent/GPU/XPC memory. The million-row sampler returns no parseable report; memory is unavailable, not zero or passed, and the prior runner discarded stderr so no cause is asserted. Preserve its failed memory status. The harness now retains bounded sampler stdout plus a separate stderr file and exit/signal/truncation metadata; validate that follow-up with a separately labeled short diagnostic route, without relabeling it as a60-second scale pass.

A controlled real-child regression reproduces an independent sampler teardown race: a descendant can keep stdout open after its parent exits, so parsing on exit misses the report. Register completion at spawn and wait for stream closure, consistent with [Node’s child-process documentation](https://nodejs.org/api/child_process.html#event-close). The old exit-based implementation fails; the close-based helper and repeated owned-child teardown pass. This establishes the collector defect without asserting the cause of the earlier million-row missing JSON. Keep its original unavailable status and sampler stderr/exit diagnostics for future runs.

The pinned embedded driver also logs a before/addTauriApi patchBrowserExecute bind warning while the direct native IPC and route assertions execute successfully. Retain driver logs; no dependency change or clean-machine/physical-input claim follows. Quantized layout and rAF observations do not prove actual presented frames. React profiling samples remain unavailable.

### Final validation and acceptance

The first full check passes 174 frontend/harness and 89 Rust tests. After the collector follow-up, the final npm run check passes 176 frontend/harness and 89 Rust tests (66 unit/21 library/2 worker), generated IPC drift, strict types, lint/format, rustfmt, denied-warning Clippy and production frontend build. Full release core tests also pass all 89, including actual worker timeout/replacement and scheduler cancellation/error/unwind behavior. Default-feature core all-target Clippy, five Python manifest/report tests and production exclusions/helper checksums pass. Logs and exact commands/durations are in .context/p4/validation-3a3372b0-64e0-4f63-9ef1-b9b9f868b699/ and check-final.txt. The initial TypeScript test-index failure and controlled sampler failure remain retained rather than relabeled.

The current debug native regression executable SHA-256 cceafbff86832d84778e4eb5e4bdd7d90530d9fe7d3e02d5f58bcba4f474b1f1 passes 15 cases in 31 s (14 foundation and one live-library case). Three independent read-only phase manifest pairs match at recorded precision; the deliberately externally moved/restored live fixture restores all files/identities with only two audited parent-directory mtimes changed. .context/p4/native-source-audit.json records the independent audit. No real library was a fixture.

The sampler follow-up uses a separate six-step diagnostic run native-06611535-41db-4ec1-8f74-e9ece594fa7b-10000: functional assertions/source manifests pass, the sampler exits0 without truncation, stderr is empty and 21 samples parse successfully. This verifies the new lifecycle/artifact collection; it is excluded from the 120-step scale acceptance and cannot repair earlier missing memory or trace data. Source code, harness and report regressions preserve the same bounds.

The final unsigned 34.42 MiB release bundle is byte-identical to the initial packaged executable above. Final SQLite provenance run 6c22ffa3-d70f-4277-a857-5d0cb62c2124 confirms the same version/source ID and userCatalogOpened=false. Packaged worker repeat 14cec51c-aa7d-46fe-bd0a-0db919158c6a again passes 19 cases/120 measured invocations and six unchanged independent sources. The earlier latency table retains its original run identity; the repeat has separate observations and makes no improvement claim. Production frontend/test-route/default-dependency exclusions and helper checksums pass after both native work and the final production build. No signing/clean-machine or new upstream advisory claim is made.

React Doctor changed-scope regression results are retained under .context/p4; the initial four-file scan reports 100/100 with no issues. The final changed-scope rerun also scans four files and reports 100/100. This is a changed-scope result, not a new whole-codebase score. No rules or safety checks are suppressed.

| Scenario | Disposition                                            | Evidence / remaining gate                                                                                                                                                                                                                  |
| -------- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P4-01    | Passed for collector/report behavior                   | Bounds, privacy/opt-in, loss accounting, aliases, consuming reads, real-child close regression and report phase/manifest/build checks pass. The recorder intentionally still drops on contention.                                          |
| P4-02    | Passed for local release synthetic evidence            | All three new P4 catalog sizes and the finite scheduler fixture complete with matching sources, retained identities and zero reported drops. No actual backup or physical throughput inference.                                            |
| P4-03    | Passed on this macOS host                              | Packaged worker cases/all tiers and 89 release core tests, including timeout/replacement/cancellation/unwind. Broader platform/media capability gates remain.                                                                              |
| P4-04    | Functional routes pass; trace-completeness gate failed | All three 120-step routes pass native functional/source checks;100k records two lost backend events. The strict combined report correctly exits1. Million-row memory is unavailable and slow filtered/viewer observations remain explicit. |
| P4-05    | Passed for local repository/regression checks          | Final 176 frontend/harness,89 Rust, five Python, default Clippy,15 native regression cases, short collector smoke, unsigned package and production exclusions. No production tuning or automation-guard relaxation.                        |
| P4-06    | Blocked / not run                                      | Real backup/physical endpoint/accounted pacing and native release presented frames, full WebContent/GPU/XPC memory, Windows/minimum OS/clean machines, physical input and controlled NAS/HDD remain unavailable.                           |

Stop at this bounded local P4 evidence gate. Before claiming full acceptance, address recorder trace coverage and memory attribution, reproduce the filtered aggregation/actor waits in release before choosing a reviewed search/index change, and obtain the missing backup/endpoint/platform/physical-media evidence. No further milestone or source mutation is authorized by this work.
