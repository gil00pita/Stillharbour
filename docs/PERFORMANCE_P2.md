# P2 gallery and rendition evidence

Owner authorisation: 2026-10-29, “start next stage,” explicitly confirmed as “Finish P2: gallery and preview performance.” This follows P1 and the earlier bounded progressive-preview slice. P3 scheduling, preservation operations and a catalog/dependency migration are outside this stage. Preserve historical P0/P1 results and the preceding timeline repair.

## Implementation and bounds

[ADR-0012](decisions/ADR-0012-bounded-rendition-pyramid.md) records the rendition/transport decision. Reuse the existing virtualizer, balanced rows, indexed catalog seeks and stable-ID/pixel anchors. Keep 240 summaries per page, three retained pages/720 summaries, up to240 preview assets/480 keys,64 keys per backend batch, two media permits and the existing256MiB indexed disk cache.

Gallery demand selects256/512/1024px from actual row geometry, fit/crop and device pixel ratio (clamped1–3). Reserve a256px estimate for every admitted image before admitting larger tiers. Clear plus tiny decoded-pixel estimates are capped at128MiB; this is admission accounting, not a WebView/GPU memory quota. The opened viewer requests2048px with512/32px fallbacks. Both gallery and viewer retain smaller usable images until actual larger-image load, including offline/unsupported larger tiers. Backend fallbackEdge identifies an indexed smaller cache rendition without adding renderer requests or new decode work.

The private worker response uses SHRP/version 2, a ≤4096-byte JSON metadata header and raw PNG bytes. Validate exact frame/body lengths, allowed tier, PNG header dimensions/8-bit RGBA, source dimensions and output budgets. PNG caps remain2MiB for32/256/512, with 5MiB for1024 and 17MiB for2048. The renderer uses the existing ID/revision/policy cache protocol. Legacy diagnostic JSON modes remain limited to512px. Unknown worker modes fail before GUI/catalog startup.

Keep source/file-input limits, read-only handle validation, native containment, revision revalidation, orientation/ICC/SDR policy, no upscale,12-second timeout and sampled512MiB child RSS watchdog. Portable output now explicitly uses RGBA8. Pipeline policy p2-v2-bounded-pyramid-srgb8-preview retries obsolete negative observations while retaining indexed p2-v1/M5-v2/M1 positives. Cache cleanup validates the new maximum size only inside its positively indexed, no-follow cache namespace; original/history/quarantine boundaries stay intact.

## Generated worker protocol check

Debug run 954b5869-df49-44f3-8bf2-254ed0a93caf passes 19 cases and48 invocations, including legacy 512px comparisons. Deterministic3072×2048 RGB noise exercises output size across JPEG/PNG/TIFF and all five tiers. Additional cases reject malformed/over-dimension images and writable descriptors and preserve12×8 no-upscale output. Independent SHA256/native-identity/stat manifests match for all six source fixtures.

The 512px PNG bytes are identical to the saved legacy worker for each format. Wire bytes fall from1,740,537→478,938 (JPEG),1,735,241→477,664 (PNG) and1,730,031→476,471 (TIFF), approximately 72.5% less. These are protocol bytes, not disk/network throughput or end-to-end UI latency. Debug 2048px p50 times are4066/3946/3824ms respectively with n=3; p95/p99 are the maximum at this sample size. Each invocation starts a fresh decoder process; source generation/preceding reads warm the OS cache and no OS-cache-cold claim follows. Direct-child sampled RSS peaks85–106MB omit WebView/GPU/XPC and are not full process-tree evidence. Release results are recorded below.

## Executable checks

```sh
npm run bindings:check
npm test
npm run desktop:test:build
npm run desktop:test
npm run desktop:test:formats
node scripts/cargo.mjs build --locked --release -p stillharbour-core --bin stillharbour-media-worker
python3 scripts/renditions/p2_probe.py --build release --worker target/release/stillharbour-media-worker
STILLHARBOUR_PERF_PHASE=P2 STILLHARBOUR_PERF_ROWS=10000 npm run perf:desktop
npm run check
npm run desktop:build
npm run release:check
```

Use the verified isolated native test binary procedure in [DEVELOPMENT.md](DEVELOPMENT.md) when a live development watcher shares target/debug. Run native drivers and frontend builds serially. The worker probe accepts binary paths, never a source library, creates owned fixtures below .context/p2-renditions, enforces a 4GiB reserve, retains evidence and performs no cache cleanup. Each summary identifies build, binary SHA256, samples and its measurement scope. An optional --legacy-worker records only the supplied old binary's512px JSON reference; compare durations only within appropriately matched build/workload scopes.

## Acceptance and remaining gates

Statuses apply only to their stated evidence. [P2-01–P2-08](PERFORMANCE_ACCEPTANCE_TESTS.md#p2--gallery-and-rendition-follow-through) and final check outcomes are recorded below. Existing native drivers synthesize pointer/wheel events; they cannot certify physical OS input. Debug/native functional results do not pass production presentation or whole-process memory targets. Windows, minimum-OS/clean-machine, broad higher-tier camera variants, true OS-cache-cold/NAS and release presented-frame validation require separate environments/evidence. The missing backup adapter remains a P3 contention prerequisite. Stop before P3.

## Release worker results

Run bcccd81c-c586-4768-994c-ee88ea20a376 passes 19 cases/120 invocations and six independent source manifests. Rust release/optimized standalone worker SHA256 and full observations are retained in that run's summary. Same Mac13,1 arm64 host,32GiB RAM/10 logical CPUs, macOS27.0.1 build26A434, Rust 1.99.1/LLVM 23.1.1, Node 26.3.1 and npm 11.16.0. Current lock hashes are recorded in .context/p2/environment.json and remain unchanged relative to this stage's start. The older P1 hash record is historical, not a claim about a later baseline.

Times below include process startup, decode/resize/PNG encoding, bounded stdout transfer and direct-child sampling overhead. Each cell is p50/p95/p99 milliseconds with n=8, nearest rank (p95/p99 therefore equal the maximum). Source generation/prior reads warm OS caches; each worker process is fresh. Builds/native suites were complete before this probe; this developer workstation is not a controlled minimum-spec lab. It measures no native UI paint, queue wait, warm application-cache hit latency, physical storage throughput or complete process-tree memory.

| Source format | 512px                 | 1024px                | 2048px                | Sampled2048px child peak |
| ------------- | --------------------- | --------------------- | --------------------- | ------------------------ |
| JPG           | 192.0 / 247.7 / 247.7 | 209.1 / 511.6 / 511.6 | 376.2 / 801.2 / 801.2 | 79.7MiB                  |
| PNG           | 133.1 / 173.5 / 173.5 | 168.0 / 219.0 / 219.0 | 242.3 / 334.7 / 334.7 | 101.3MiB                 |
| TIF           | 107.4 / 135.9 / 135.9 | 138.9 / 229.6 / 229.6 | 223.4 / 342.0 / 342.0 | 105.0MiB                 |

The 2048px PNG bodies are approximately10.3–11.0MB on these noise fixtures. Read-only inherited descriptor,12,000px source-dimension bound, per-tier body cap, no upscale and malformed-input checks pass. These are native common-image preview observations; they do not certify untested higher-tier camera/OS variants or original-file fidelity. The existing sampled512MiB watchdog is unchanged and is not an instantaneous quota.

## Fixture audit and retained failures

Initial native load-only checks missed a viewer fade bug. A stronger native test records593px image/canvas height and1180px width, but both layers at opacity0 with document visibility hidden. Disable this fade when hidden and crossfade the smaller layer in visible windows. The corrected HEIC test verifies loaded image geometry and final opacity; a new screenshot visibly contains the image. Preserve the red trace/native source manifests and exclude the first interrupted10k run from passing evidence.

Initial fixture-version1 scale routes at10k/100k return passed, but the1m run times out after seeding (234.8s total, independent32-file manifest unchanged). A0.90s10k service regression reveals the fixture error: accessible empty roots marked offline reconnect through normal status recovery and scan synthetic observations away. These earlier native runs are excluded from catalog-scale acceptance. Version2 retains the newly created empty fixture directories at different owned paths using no-replace publication, leaving their registered paths genuinely unavailable. No production recovery behavior is changed. New routes assert seed version, offline root state, mounted synthetic IDs and retained cardinality through the bounded source scan. Historical P0/P1 native scale claims require re-audit; direct catalog-only release probes do not run root recovery and are a separate scope.

## Measured hot paths and targeted corrections

A diagnostic-only catalog counter reproduces 1,152 actor calls for 64 preview keys. Batched indexed cache lookup/touch and a narrow asset/root status projection reduce this to 2 calls in the same regression. The counter is fixture-probes-only, absent from production IPC. Retain revision, policy, offline, generation, maintenance and queue bounds; cache filesystem validation remains outside the actor. This measures call count, not a latency percentile.

A separate native geometry route (3f1f4e5d-0b7b-4655-bdb3-1fa7e9a884bf,24 steps) finds652 mounted assets in13 rows, with the first image height2.265px. The existing balanced solver amortised distortion across too many photos. A failing720-photo mixed-aspect regression now passes after rejecting multi-image candidates below half the requested row height. Single panoramas may be shorter; singleton rows stay at or below target height. Keep the solver's64-item search bound. Visible rows receive preview admission before overscan, with a component regression for the demand order. Neither change raises the720-summary/240-preview limits.

These are independent of the approximately 72.5% worker wire-byte reduction. Avoid adding the improvements together or presenting them as a measured gallery speedup. The three measured issues are worker JSON expansion, repeated cache-status actor calls and collapsed dense-day rows. Their smallest fixes are the bounded binary frame, batched status reads and the existing solver's candidate-height floor.

## Final environment and functional checks

Native tests use debug desktop-test instrumentation in macOS WKWebView, DPR2 at1180×748 viewport. WebKit framework22625.1.29.11.28 (bundle short version 22625); the driver reports webkit605.1.15 and the generic UA AppleWebKit/605.1.15. These identifiers are recorded separately; the generic UA does not identify an exact WebKit engine revision. Host/toolchain details above are local observations, not Windows evidence.

Both locks are unchanged: Cargo.lock SHA2563e8b4a0d8149fa06461b381b9cab6ae7a577c9ec0a85f6b060207edaf4729078 and package-lock.json SHA256eab358a6345e96c3195256904292d24534df4033a1c4aa79f1b26086f1afeda0. Native test binary SHA256989f45027a8f283c4b5103203632467b2e9dfdf232dfa800ac2d31bb4c167944; isolate it from the user's live watcher as documented.

The final full npm run check passes 149 frontend and 67 Rust tests (47 unit/18 library/2 media), strict types, lint, generated bindings, formatting, denied-warning Clippy and production frontend build. A subsequent type-only named-props extraction passes strict types/lint and14 focused gallery tests. Cached React Doctor0.9.18 reports the 10 incumbent warnings after this extraction; no new warnings remain. Its score API times out, so no final numeric score is available. The initial latest-package invocation stalls; retained outputs distinguish the cached local scan from a successful latest fetch.

Native final functional runs pass 14 desktop cases in 44.7s (b394f7ad-1893-4628-b957-f4f79fcc39ad) and the separate HEIC gallery/viewer case in 11.4s (50f9a8fd-e5d1-44b6-8a8f-c4fd3c8fa294). Independent32/36-source manifests match. Assertions cover wheel/timeline/hidden scrollbar behavior, bounded paging, date navigation, selection, viewer return, text-only requests, focus and protected maintenance. The HEIC check waits for nonzero image geometry and final visible opacity, not just decode completion. Native synthetic input does not establish physical mouse/trackpad behavior.

Reproduction sources: [worker probe](../scripts/renditions/p2_probe.py), [native scale route](../tests/performance/gallery.e2e.mjs), [native gallery checks](../tests/desktop/foundation.e2e.mjs), [native format/viewer check](../tests/formats/native.e2e.mjs), [independent source manifests](../scripts/performance/manifest.py) and [process sampler](../scripts/performance/process_stats.py). Generated sources and private traces remain local under .context; do not commit photographs, catalogs or cache entries.

## Measurement definitions and limits

The final native scale routes use version 2 synthetic fixtures, an existing indexed-root scan of 32 generated entries and 120 scripted scrollTop steps over at least60 seconds. The catalog is newly seeded in the same session; seeding warms database/OS caches. Synthetic media roots are genuinely offline and contain no decoded source media. Real generated previews have already been exercised before the main route. This is neither a cold catalog-open benchmark nor a full-media library benchmark. Random date-jump time ends when the expected catalog date appears in the DOM, not at presented paint; resize measurements include a scripted100ms wait. The fresh-import timing covers Save/scan through first decoded image on the32-file fixture, not all thumbnails complete.

Worker-cold means a new process for every invocation. OS-cache-cold was not run: fixture generation, source manifests and earlier reads warm caches; no system cache purge is attempted. Application-cache reuse is covered by targeted warm/offline service and native tests, not a separately controlled warm-latency distribution.

Frontend rAF intervals are JavaScript scheduling observations. Layout timers use millisecond resolution and very small sample counts;0ms means below timer resolution, not zero work. React profiling is disabled, so react_render_ms has zero samples. Bounded frontend/backend rings can drop events; reported trace percentiles describe retained samples and may omit later work. They do not pass presented-frame targets. IPC samples aggregate endpoint calls across the route and phase boundaries; do not interpret them as one query's database latency.

Memory sampling covers the app and observed descendants, omitting OS-owned WebKit/GPU/XPC services and short-lived children between samples. RSS first/last/peak values include fixture startup and seeding. They cannot establish a complete application budget or a steady-state leak trend. OS-accounted disk counters include synthetic seeding, are not physical storage throughput and may omit unobserved helpers. Backend endpoint counters are logical owned-I/O observations. All endpoints in these fixtures are local; network throughput, NAS behavior and real offline reconnection are untested. A bounded scan runs during browsing; no backup adapter exists to establish backup-contention evidence.

## Final native scale routes

All three version 2 routes pass120 steps (61.83s scrolling each), date jumps, four resize commands, selection/viewer navigation, list mode and bounded scan overlap. The three 32-entry source manifests match. Run identifiers are db03880a-9f78-4cbf-803e-53ba97721111 (10k),49173a49-85d8-4298-b96e-caf22c79bc1c (100k) and21984181-eef8-43d1-bc7f-00ba11965d3d (1m), under .context/performance/native-<UUID>-<rows>. The million-row seed finishes at 213.4s elapsed; the full route takes302.2s. Seeding is excluded from the scrolling/date-jump durations. Earlier valid version 2 runs precede the cache/layout corrections and are retained separately.

Quantiles use nearest rank. Date jumps have n=10 each, so p95/p99 equal the maximum. IPC distributions and other trace metrics describe retained observations only.

| Rows      | Date jump p50/p95/p99 (ms) | IPC p50/p95/p99 (ms; n) | Peak mounted / retained | Fresh import to first decode (ms ; n=1) |
| --------- | -------------------------- | ----------------------- | ----------------------- | --------------------------------------- |
| 10,000    | 139 / 221 / 221            | 13 / 77 / 153 ; n=170   | 29 / 480                | 1149                                    |
| 100,000   | 129 / 173 / 173            | 13 / 50 / 85 ; n=184    | 45 / 480                | 1009                                    |
| 1,000,000 | 137 / 286 / 286            | 13 / 61 / 117 ; n=169   | 27 / 480                | 1060                                    |

| Rows      | Observed RSS first / last / peak (MiB) | Accounted reads / writes (MiB) | Frontend / backend drops | Sampler errors / drops |
| --------- | -------------------------------------- | ------------------------------ | ------------------------ | ---------------------- |
| 10,000    | 144.1 / 147.9 / 183.8                  | 0.52 / 37.29                   | 687 / 1                  | 11 / 0                 |
| 100,000   | 143.1 / 183.2 / 183.2                  | 0.75 / 317.52                  | 665 / 4                  | 9 / 0                  |
| 1,000,000 | 143.1 / 129.3 / 166.7                  | 255.98 / 3169.19               | 683 / 2                  | 7 / 0                  |

Sampler errors are reported counts, without a retained per-error cause; do not assume all are harmless child-exit races. Sampling limitations and missing WebKit/GPU coverage prevent a full-memory pass. Raw source/endpoint identifiers remain private aliases.

| Rows      | Logical source read (bytes) | Cache read / write (bytes) | SQLite page misses / writes (bytes) |
| --------- | --------------------------- | -------------------------- | ----------------------------------- |
| 10,000    | 2656                        | 15032 / 4675               | 8208384 / 741376                    |
| 100,000   | 6012                        | 14574 / 7018               | 14655488 / 753664                   |
| 1,000,000 | 3014                        | 9899 / 2343                | 25464832 / 696320                   |

These logical counters cover the retained main-route trace, not physical I/O or the entire run. SQLite page misses may be satisfied by OS caches. At least one query starts during the bounded generated-source scan in each run (one/one/three of five overlap observations); overlap is brief, and four/four/two other queries respectively start after the scan finishes. No backup run is claimed. Text-mode no-image behavior and viewer/selection have a separate trace phase.

Retained rAF p50/p95/p99 are 20/30/50ms (n3061),20/31/48ms (n3041) and20/31/48ms (n3060). Layout samples are0/0/0ms (n5),1/1/1ms (n1) and0/0/0ms (n5), quantised by the timer; they cannot establish a render budget. React profiling samples are zero. IPC response maxima are 91,565/92,307/89,753 bytes. A complete endpoint-specific IPC or presented-paint distribution was not measured.

## Acceptance disposition and next gate

| Scenario | Status                               | Evidence and scope                                                                                                                                                                                                                                                   |
| -------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P2-01    | Passed locally                       | Planner/admission/DPR tests, visible-before-overscan regression, dense-row regression and native bounded mounts. Text-only checks remain enabled.                                                                                                                    |
| P2-02    | Passed locally                       | Viewer retained/failure/offline 1024-only component cases; final native HEIC geometry/opacity and desktop navigation/return checks.                                                                                                                                  |
| P2-03    | Passed locally                       | Frame unit tests cover truncation/version/length/tier/body/dimension/trailing errors; worker fixtures and fail-closed dispatch retained.                                                                                                                             |
| P2-04    | Passed locally                       | Cache/maintenance/revision/failure integration checks,1024→offline2048 fallback and 64-key two-call regression. Existing retention remains 256MiB.                                                                                                                   |
| P2-05    | Passed on this macOS host            | Generated common-format all-tier worker cases, exact legacy 512px PNG equality in the debug comparison, read-only descriptor/limits/no-upscale and independent manifests. Broader high-tier camera variants remain unrun. Final packaged results are recorded below. |
| P2-06    | Passed locally                       | Final version 2 native 10k/100k/1m routes,≤480 retained/≤45 mounted observed, viewer/selection/resize/date/list checks and unchanged manifests. Original 720-summary cap remains.                                                                                    |
| P2-07    | Passed for reporting                 | Definitions, sample sizes, percentiles, drops, errors, I/O and memory limitations recorded. Production presented-frame/full-memory and controlled cold-cache targets are not run.                                                                                    |
| P2-08    | Blocked for full platform acceptance | Local full repository/native checks pass. Windows, minimum OS/clean machine/signing, physical input and production presentation/full-process memory evidence remain unavailable; do not infer support from these runs.                                               |

The initial hidden-viewer and synthetic-root failures are resolved by stronger passing regressions; their retained failing artifacts are not passing evidence. Native performance before the cache/layout corrections is historical comparison evidence, not the final result. Trace loss and sampler errors remain limitations rather than silent passes.

Stop at the P2 implementation/report gate. P3 needs a reviewed shared-resource scheduling design that retains endpoint/volume limits, source identity and preservation behavior; a real bounded backup adapter or explicitly scoped adapter fixtures for contention; and independent scheduler/failure tests. Complete the outstanding platform/presentation/memory validation before claiming full performance acceptance. No later milestone, catalog migration, dependency change, original-file mutation or publication is included.

## Final packaged application

The unsigned macOS release .app builds 34.24MiB. Production capability/dependency/test-route exclusions and vendored-helper checksums pass. Read-only packaged SQLite provenance run df326a12-7ece-4ea2-a81b-efb524f91db1 reports version 3.54.1, source ID `2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76` and userCatalogOpened=false. Executable SHA256 `4196ae7cee507e1de48df03e8a33b63bfeb8afd6f21d5c5048269ea4dc477450`. Existing source/advisory review remains in the P0/P1 reports; this run confirms linked provenance and makes no new advisory-free claim. No dependency or catalog upgrade is performed.

React Doctor local diagnostics completed with the 10 incumbent warnings. Lingering score-service retries were terminated after retaining results; the full command did not finish with a successful score response. No numeric-score pass is claimed.

Final packaged worker run 9941631a-9e63-4ba8-b538-ec353ed2e969 passes 19 cases/120 measured invocations across all five tiers, plus the runner handshake. Six independent source manifests match. Its binary hash matches the packaged SQLite executable above. Fresh-process/OS-warm measurement definitions are unchanged. Unknown worker-mode rejection separately returns exit 2 before app startup.

| Format | 512px p50/p95/p99 (ms) | 1024px p50/p95/p99 (ms) | 2048px p50/p95/p99 (ms) | Sampled2048px child peak (MiB) |
| ------ | ---------------------- | ----------------------- | ----------------------- | ------------------------------ |
| JPG    | 204.4 / 489.8 / 489.8  | 276.9 / 413.6 / 413.6   | 337.2 / 415.1 / 415.1   | 85.6                           |
| PNG    | 131.0 / 214.5 / 214.5  | 217.2 / 253.3 / 253.3   | 280.7 / 443.8 / 443.8   | 96.1                           |
| TIF    | 174.6 / 540.7 / 540.7  | 139.0 / 205.4 / 205.4   | 254.1 / 280.1 / 280.1   | 106.6                          |

Each tier/format has n=8; p95/p99 equal its maximum. Sampled direct-child peaks exclude the app/WebView/GPU process tree. 2048px wire responses are 10,327,263 bytes for JPEG and 11,015,012 for PNG/TIFF on these noise fixtures; these are disposable PNG previews, not source fidelity or throughput measurements.

Release uses the existing thin LTO and symbol stripping in Cargo.toml. Debug/native runs use the desktop-test feature; the packaged release uses default production features. Native worker benchmarks ran without parallel builds/native suites.

Final packaged format regression run 39e08180-d769-45f1-aa12-b877829c8a25 passes all 18 expected cases with unchanged independent sources, using the same packaged executable hash. Reuse is restricted to previously owned/manifested fixture 45d3ec0b-3636-4012-846b-1d74f3482a17. Generated common formats/P3 patches and pinned CR2/NEF/ARW/DNG/CR3/RAF samples pass at the established 512px diagnostic tier; this does not certify all higher-tier camera variants. RAF reports one dropped backend event; retain that coverage limit.

Reproduce packaged probes after npm run desktop:build:

```sh
python3 scripts/renditions/p2_probe.py --build release --worker target/release/bundle/macos/Stillharbour.app/Contents/MacOS/stillharbour --samples 8
npm run formats:build
python3 scripts/formats/run.py --packaged --camera-samples
node scripts/performance/packaged-sqlite.mjs
```

The format command downloads only the pinned CC0 camera fixtures described by its manifest. For retained sources, use --reuse-fixtures <owned-UUID> after its identity/manifest checks; no real library path is accepted.

Final owned-file verification passes 237 text/local-link checks, 70 retained independent before/after manifest pairs and fresh inventories of all three P2 rendition source trees, with zero findings. Both lockfiles remain unchanged. Repository formatting and git diff --check pass. All requested source/specification/evidence changes are saved locally; no commit or push.
