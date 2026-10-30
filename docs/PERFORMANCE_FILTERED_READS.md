# P4 continuation — empty filtered reads

Owner request: 2026-10-29, “Start the new step,” after publishing the local P4 work. [Task scope](../tasks/P4_EMPTY_RESULTS.md) continues the measured filtered-query issue. [P4's earlier evidence and failed/unavailable gates](PERFORMANCE_P4.md) remain historical. This step changes only proven-empty display reads; other filtered-query costs remain open.

## Reproduction and fix

A current release core probe of the previously generated million-row fixture reproduces the no-match query at 1545–1600ms (n=3). Month summaries take 227–236ms after that page. Narrow and broad matches are retained as controls. The catalog contains synthetic observations, not a million real media files, and no source folder is scanned.

The minimal regression exercises the actual query/cache seam with 10k generated rows. After caching an exact zero count and clearing only SQLite's connection pager, a repeated page reads 11,915,264 logical pager bytes and fails a 64 KiB budget in 0.39 s. The timeline-first case reads 15,175,680 bytes and fails in 0.37 s. These are SQLite logical misses, not physical I/O, and the tests impose no wall-clock CI threshold. Failed outputs remain in .context/p4-filtered/empty-before.txt and summary-before.txt.

Three ranked hypotheses were considered against the release measurements: redundant page projection after an exact zero count, repeated empty summary scans, and conservative cache invalidation during bookkeeping. The first two are reproduced and fixed. Cache invalidation remains conservative; the third is not relaxed to improve timings.

When a validated query has an exact zero count, return the same empty page with current catalog revision and no continuation, without running its ordered asset projection. An empty timeline and an exact zero count can share knowledge through the existing query-hashed cache in either request order. Keep 16 entries/512 KiB encoded retention and all connection-write/data-version invalidation. Invalid limits, cursors and filters still fail before the shortcut.

Unfiltered inventory counters are deliberately excluded: rows can become visible before a streaming scan refreshes the root count. A regression sets inventory counts to zero while retaining 10k observations and requires the page to return 240 rows and the timeline to retain all 10k. Existing frozen-selection and original-operation authority do not use this shortcut. No schema, dependency, index, IPC, source-access, decoder or durability change.

## Reproduce

```sh
node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example filtered_probe
python3 scripts/performance/filtered.py --fixture-run UUID --rows 1000000
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes empty_pages -- --nocapture
npm run check
npm run desktop:test:build
npm run desktop:test
npm run desktop:build
npm run release:check
node scripts/cargo.mjs run --locked --release -p stillharbour-core --features fixture-probes --example packaged_library_probe -- target/release/bundle/macos/Stillharbour.app
```

Supply a UUID created by scripts/performance/run.py; the retained fixture used here is 26c4b889-dc55-42c3-b84b-0bcbf1440183. The new filtered runner checks the bounded ownership marker and ordinary directories, creates a fresh .context/filtered/UUID output and independently inventories generated sources before/after, including failure. The fixed executable checks regular files, rejects links/hardlinks where supported and opens only Fixture.stillharbour under the generated namespace. It retains full catalog opening validation. These development ownership checks are not a race-proof production authorization mechanism. Neither runner accepts a source-library path or cleans fixtures. A retained catalog's app-owned SQLite/WAL state can change when opened; media manifests cover the generated sources separately.

Run builds, measurements and native drivers serially. No OS cache purge; source storage and OS caches are uncontrolled. Catalog opening/validation is outside per-call samples. Three samples per scope; nearest-rank p95/p99 equal the maximum and do not establish a population SLO. The existing P4 renderer report is separate and no earlier failure is relabeled.

## Results

Baseline raw data, exact failed regressions and independent source inventories are in .context/p4-filtered/. After-fix release run d4c3b33d-9428-4677-acfa-c69e51f6a614 retains raw samples, query plans, current bundled SQLite provenance and executable/lock hashes under `.context/filtered/<run>/`. All generated source entries match with lossless stat transport. Cargo.lock/package-lock.json remain unchanged; linked SQLite remains 3.54.1. No new advisory-free or platform-support claim.

| Scope  | Matches   | Before page p50/p95/p99 (ms)   | After page p50/p95/p99 (ms) | Before timeline p50/p95/p99 (ms) | After timeline p50/p95/p99 (ms) |
| ------ | --------- | ------------------------------ | --------------------------- | -------------------------------- | ------------------------------- |
| none   | 0         | 1545.576 / 1599.890 / 1599.890 | 228.256 / 230.365 / 230.365 | 231.586 / 235.999 / 235.999      | 0.048 / 0.049 / 0.049           |
| narrow | 999       | 493.027 / 508.941 / 508.941    | 500.657 / 503.625 / 503.625 | 258.803 / 276.628 / 276.628      | 264.942 / 286.719 / 286.719     |
| broad  | 1,000,000 | 244.251 / 245.285 / 245.285    | 239.584 / 248.742 / 248.742 | 612.200 / 624.405 / 624.405      | 617.774 / 646.016 / 646.016     |

The no-match improvement is about 85% for the page median; its cold exact count still scans the filtered catalog. The empty timeline reuses that exact result. Positive-match scans/grouping remain unchanged and expensive at this size. The initial all-scope 250 ms diagnostic screen remains unmet for positive filters and is not promoted to a passing full performance gate. Source/worker/native presentation/full-memory/Windows/NAS/backup limitations remain as recorded in P4.

## Validation and next gate

The focused regressions pass in debug and release after the fix, including all sort orders, inventory lag and invalidation after actor/external writes. Repeated empty pages and timeline-first/page use 8,192 logical pager bytes; page-first/timeline uses 4,096 bytes. The full repository check passes 176 frontend/harness and 91 Rust tests (68 unit/21 library/2 worker), strict types/lint/format, generated bindings, denied-warning Clippy and production frontend build. Default-feature core Clippy and five existing Python regressions also pass. The native foundation test now exercises an escaped literal no-match search, its honest empty state/absent timeline, and successful return to populated photos. The first native run passes 13 foundation cases (including the added empty-search/return flow) but times out waiting for the maintenance removal-review button to enable. The following live-library case then encounters the still-open review. Retain this failed run in .context/p4-filtered/validation-51916672-7131-4291-a23f-1eff3548b25a/native-regressions.txt. Maintenance uses its existing separate admission path; no production gate is weakened or retry added.

A diagnostic-only harness follow-up captures review alerts/button state and job kind/state on the first review failure. The fresh diagnostic run passes all 15 native cases in 30 s, .context/p4-filtered/native-diagnostic.txt. It does not reproduce the timeout, so its original cause remains unknown and the earlier failure is not relabeled. The additional captured state will survive any recurrence. Final-source npm run check passes again after the test-only diagnostic capture: 176 frontend/harness and 91 Rust tests. The complete command/exit/duration record is .context/p4-filtered/final-1598e596-c8da-44bc-8989-488453e1bb31/results.json; earlier failed native evidence remains separate.

The passing native run uses debug executable SHA-256 8cd293837a50e566d5e9dd948e566d6a0cf4d72132e878f6cdf103897927ea42. Four independent read-only native manifest pairs match at recorded precision across the retained runs. The completed live fixture restores all files/identities, with only two audited externally changed parent-directory mtimes. .context/p4-filtered/native-source-audit.json records the audit; the release filtered probe uses lossless-stat strings for its source inventories. No real library is used.

The unsigned macOS 34.42 MiB application builds with SHA-256 aecf3e95c1e48cfbd3ff800025e64be4dfc0b2315d89affe44c866b30f3a3d37. Production permissions/default dependency graph/frontend test-hook exclusions and reviewed helper checksums pass. The actual packaged decoder and ExifTool smoke indexes 20 generated media, reports JPEG/PNG/WebP previews and 32 unchanged source entries. These are functional format samples, not a newly certified broad camera/color/OS matrix. In-memory packaged SQLite provenance run 1fe53688-b151-4832-aa00-4b7c6c489781 confirms 3.54.1 and userCatalogOpened=false; no dependency upgrade or new advisory-free claim is made.

React Doctor changed-scope scan covers seven files and reports 100/100/no findings; no full-codebase score or suppression is claimed. Final formatting, documentation-link and whitespace checks run after this report update. Stop after this bounded read optimization; broader positive-filter/index design, complete traces/memory and platform/backup gates require their own evidence.

| Check                            | Outcome                                                                                                                |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Empty-result regressions         | Both pass in debug and release, all five sorts; correct inventory lag, input failures and write invalidation.          |
| Release filtered fixture         | No-match page median1545.576→228.256ms; timeline median231.586→0.048ms, n=3. Sources unchanged; positive costs remain. |
| Repository/default Clippy/Python | 176 frontend/harness,91 Rust and five Python tests plus documented types/lint/format/bindings/build checks pass.       |
| Native                           | Final15 cases pass; retain the initial maintenance-review timeout/cascade and unknown cause.                           |
| Production package               | Unsigned app, exclusion/checksum checks and actual bundled worker/metadata smoke pass on this macOS host.              |
| Wider acceptance                 | P4 trace/memory/positive-filter/Windows/NAS/backup/release-presentation gates remain open.                             |
