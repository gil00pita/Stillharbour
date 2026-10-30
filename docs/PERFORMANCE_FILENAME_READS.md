# P4 continuation — selective filename projections

Owner request: 2026-10-30, “Run the next stage.” [The bounded task](../tasks/P4_FILENAME_READS.md) continues the cold filtered-read costs left by [positive timeline totals](PERFORMANCE_POSITIVE_TOTALS.md). Preserve that earlier evidence and all wider [P4 limitations](PERFORMANCE_P4.md).

## Reproduction and decision

The initial 10k cold filename page reads 8,671,232 logical pager bytes and fails its diagnostic screen in 0.36 s. A name-only probe uses the existing assets_name covering index without INDEXED BY, reading fewer catalog columns. The release million-row probe records about 65–98 ms for a complete empty/narrow name-index pass, versus roughly 250 ms for the full-row count. The first candidate probes before the count and uses bounded rowids for counts/projections. It passes the 10k screen at 1,191,936 logical bytes and improves the narrow release page, but regresses the later-sorting broad control from 317.169 to 416.307 ms. Retain and reject that eager-count policy rather than optimizing only the favorable control.

The final policy waits for an existing exact count of 1–1,024 and applies only to an unscoped filename-only query. Root/directory, folder, tag, favorite, date, size and extension filters retain their ordinary plans. Copy the validated query only to check those existing inventory predicates with name cleared; do not change the query hash or user scope. A bounded name lookup retains at most 1,025 i64 rowids. If every raw name match fits, append a parameterized JSON rowid predicate to the full original query. Otherwise retain the complete original SQL scope. All original media/presence/name predicates still apply, including excluded/non-media observations that make raw names broader than the visible count.

The rowids are transient native SQL inputs within one database-actor request, never renderer IDs, persisted identity, selection authority or cached membership. The existing JSON1 adapter avoids a growing placeholder list and preserves i64 values without JavaScript transport. Counts, unknown/large timeline summaries, viewer-neighbor queries and frozen selection retain their existing SQL. Existing connection/epoch cache freshness, source guards, schema 2, FULL durability and open-time validation remain intact. No new index, migration, dependency, native helper or public IPC surface.

Three hypotheses were tested: wide full-row filtering, repeated ordered projection and count-cache overhead. The release candidate/index plans support the first two; a count-cached projection isolates the ordered read without changing cache invalidation. The minimized regression primes the exact count, empties only the disposable connection pager, then repeats the page. The pre-change implementation fails in 0.38 s at 5,410,816 logical pager bytes against a 2 MiB allowance. It is a cold projection with a known count, not a cold-count benchmark. Original cold-page and rejected eager-policy failures remain separate under .context/p4-name-scope/.

[SQLite describes covering-index scans and rowid lookup](https://sqlite.org/queryplanner.html). EXPLAIN is diagnostic evidence only; [its output format is not a runtime contract](https://sqlite.org/eqp.html). SQL LIKE/ESCAPE remains the existing [SQLite expression semantics](https://sqlite.org/lang_expr.html), using the shared escape_like helper. No Rust case-folding or literal-to-SQL interpolation is added.

## Commands and measurement scope

```sh
node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example filtered_probe
python3 scripts/performance/filtered.py --fixture-run UUID --rows 1000000
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes library::performance_probe::tests -- --nocapture
node scripts/cargo.mjs test --locked --release -p stillharbour-core --features fixture-probes library::performance_probe::tests -- --nocapture
node scripts/cargo.mjs clippy --locked -p stillharbour-core --all-targets -- -D warnings
python3 -m unittest discover -s scripts/performance -p 'test_*.py'
npm run check
npm run desktop:test:build
npm run desktop:test
npm run desktop:build
npm run release:check
node scripts/cargo.mjs run --locked --release -p stillharbour-core --features fixture-probes --example packaged_library_probe -- target/release/bundle/macos/Stillharbour.app
node scripts/performance/packaged-sqlite.mjs
git diff --check
```

The existing filtered runner still accepts only an owned generated UUID/row-count namespace, not a user-library path. This stage reuses fixture 475b54c4-2fe2-4337-a961-42fe7742ebac, with a million synthetic observations/offline roots and 33 generated source entries. No source scan, cleanup or OS-cache purge. Catalog opening remains outside call samples and may checkpoint app-owned fixture state. Run builds, measurements and native drivers serially.

The expanded protocol adds lateBroad/lateNarrow names and candidateProbe timings/plans to existing empty/narrow/broad and both request orders. bucketPlan remains the unmodified fallback/cold-timeline reference; pageFirstBucketPlan and pagePlan record actual known-count preparation outside stopwatch samples. Candidate output is capped even if the index scan must inspect the whole catalog. It is not a hard physical-I/O or time cap. Mixed scopes may remain slow; a small visible count with many excluded/non-media raw names can still pay the bounded-output probe before falling back.

Baseline df74d3f9-e2ed-4f03-9e48-8f7d9d8a39e5 uses the expanded timing protocol with the preceding stage's exact-positive-total behavior and no candidate optimization. Because that stage is still uncommitted locally, the control build reconstructs HEAD's store plus its already-validated positive-total block, saving exact baseline/candidate source copies and hashes and restoring the candidate in a guarded finally block. Baseline executable SHA-256 e6811cce9453a73ec6fefc47c2cc2ba8a1051487b4ff34fe22b55671cc64fac5; baseline-build-identity.json retains source identity. The initial three-control run a5874f1f-8bba-4c81-91a3-c5993d2e1366 and rejected eager run 4051ab42-22ce-4198-8f91-07cd33edabac remain separate. Earlier successful measurements are not relabeled.

Each scope/order has three samples; nearest-rank p95/p99 equal the maximum. Ordinary host load and OS caches are uncontrolled. A second page after a timeline is not whole-interaction latency. Release core queries are separate from debug native UI functional checks, release presentation, source-device throughput or complete memory attribution.

## Results and validation

Final release run c94615c5-b827-4fb4-ac8d-e47b5acc439d uses executable SHA-256 336eae9ac9cf4aaab73876fe500ad2dda9c43e0ee5f986b613cf2a3231b6184a. Host: macOS 27.0.1 arm64, Apple M1 Max, 32 GiB RAM and 10 logical CPUs, Rust 1.99.1/Node 26.3.1. Linked SQLite remains 3.54.1, schema 2 and FULL durability. Lock hashes remain unchanged. Independent lossless inventories of all 33 generated source entries match in baseline, rejected-candidate and final runs. Raw samples/provenance are in `.context/filtered/<run>/`; stage logs and source snapshots remain in .context/p4-name-scope/.

Milliseconds, n=3 per scope/order; each cell is p50 / p95, and p99 equals p95. “Cold page” resets derived summaries, not OS caches.

| Scope      | Matches   | Cold page before    | Cold page after   | Page after timeline before | Page after timeline after |
| ---------- | --------- | ------------------- | ----------------- | -------------------------- | ------------------------- |
| none       | 0         | 243.512 / 245.584   | 247.327 / 249.005 | 0.107 / 0.108              | 0.100 / 0.104             |
| narrow     | 999       | 521.215 / 521.399   | 372.345 / 388.042 | 245.865 / 246.812          | 96.891 / 98.419           |
| broad      | 1,000,000 | 259.461 / 260.950   | 254.764 / 260.018 | 2.008 / 2.429              | 1.939 / 2.144             |
| lateBroad  | 10,000    | 317.169 / 328.834   | 313.896 / 315.558 | 42.032 / 42.429            | 41.399 / 41.633           |
| lateNarrow | 100       | 1771.802 / 1792.083 | 371.725 / 379.486 | 1516.638 / 1525.170        | 97.056 / 97.239           |

Page-first subsequent timeline medians before→after (ms): none 0.046→0.070, narrow 282.013→97.165, broad 692.694→649.030, lateBroad 292.791→276.839, lateNarrow 272.249→96.405. Cold timeline-first aggregation retains its original scan; complete raw samples remain in the reports. The narrow page/timeline plans now show integer-primary-key rowid lookup with a bounded JSON subquery, while the covering-name-index probe remains a full index scan for a selective name. Plan collection occurs outside stopwatch samples; zero-result shortcuts may execute no projection/group SQL at all.

The final policy removes the measured late-broad regression without claiming every input improves. Cold selective pages remain above the 250 ms objective, and counts/unknown or large summaries still scan. Excluded/non-media raw matches can trigger a conservative fallback even when visible totals are small. No full P4, physical I/O, memory, native presentation or platform gate is promoted.

The first full repository check passes 177 frontend/harness and all 71 core unit cases but fails one of 21 library cases: change_journal_survives_restart_and_missing_folders_leave_normal_explorer reopens its generated catalog with CatalogInUse/OS35. Retain check.txt. Its focused invocation passes in 5.09 s, restart-focused.txt; this does not reproduce or fix the original failure. The fixture already obtains sole Catalog ownership and joins its actor before reopening. Its queries are unfiltered, so the new filename preparation does not change this read path. A full diagnostic rerun retains backtraces on failure; no lock bypass, Catalog::open retry, recovery change or speculative fixture fix is added. The diagnostic full rerun passes 177 frontend/harness and 94 Rust tests (71 unit, 21 library, 2 worker), strict types, lint/format, generated bindings, rustfmt, warnings-denied Clippy and frontend build. Five Python evidence tests also pass. The original intermittent CatalogInUse failure remains retained and unexplained; the passing rerun is not a lock-fix claim. All nine catalog regressions pass in release mode (1.75 s), including the projection budget and threshold/filter/native-rowid parity. Default-feature core all-target Clippy passes. The symbol-verified debug executable 37a1a04c018c5403f3f837997054c9bd7dcf3761a218d27c6e503a5189da0fdd passes all 15 macOS native cases in 29 s (33.593 s including runner startup). Three new independent read-only manifest pairs match at recorded precision; externally moved/restored fixture differences are restricted to two audited parent-directory mtimes. Native-source-audit.json retains the actual run identities and comparisons.

The unsigned default-feature macOS bundle builds at 34.44 MiB, executable SHA-256 e68f3b840fbe88b75476b2a279bf208a7fa773f2fbeb0acdfcf5786747389afc. Production capability/default-dependency/frontend test-route exclusions and reviewed helper checksums pass. The actual packaged decoder/ExifTool smoke indexes 20 generated media with JPEG/PNG/WebP previews and preserves 32 source entries. Packaged SQLite run 97c4f55a-d883-4545-a0cf-4a53f2fb9c59 remains 3.54.1 and reports userCatalogOpened=false using its in-memory provenance route. These are local unsigned functional checks, not signing, clean-machine, broad-format or release-presentation evidence. Exact commands, exits and durations are in validation-results.json alongside the full check/failure logs under .context/p4-name-scope/. Final changed-document formatting, relative links and Git whitespace checks pass. The focused projection regression and parity tests are run before broader checks. Parity compares the complete original SQL result to paged output in all five sorts, both stable-ID seek directions, neighbor IDs and bucket totals. It covers 1,024/1,025 matches, an excluded first entry that makes the visible count smaller than raw names, missing/non-media observations, literal percent/underscore/backslash/Unicode/quote text, i64 rowids near the maximum, mixed scope and external writes without revision bumps. Production source access is unchanged.

Stop at this bounded projection work. Cold exact counts, unknown/large timeline aggregation, mixed filters, prior trace loss/unavailable full memory, Windows/minimum OS/clean machines, native release presentation and real backup/NAS/HDD/pacing/preservation gates remain open. No branch rename, commit or push is included.

## Subsequent cold-summary continuation

The owner’s later P4 request authorizes [cold filename summaries](PERFORMANCE_COLD_SUMMARIES.md). Its shared predicate-order refinement and measurements are separate from this projection policy; retain the earlier rejected eager-probe and intermittent catalog-lock evidence unchanged.

## Subsequent catalog-lock diagnosis

The [local P4 closeout](PERFORMANCE_LOCAL_CLOSEOUT.md) reproduces CatalogInUse/OS35 with an owned child paused before exec and fixes explicit lock release after SQLite shutdown. The original isolated failure above lacks process evidence for a certain individual attribution; retain it unchanged alongside the new controlled regression and actual repeat/native outcomes.
