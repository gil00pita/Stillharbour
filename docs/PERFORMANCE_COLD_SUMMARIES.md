# P4 continuation — cold filename summaries

Owner request: 2026-10-30, “continue with P4.” [Task scope](../tasks/P4_COLD_SUMMARIES.md) continues [selective filename projections](PERFORMANCE_FILENAME_READS.md). Preserve all earlier P4 trace, memory, platform and preservation evidence; this is one read-only query refinement.

## Decision and reproduction

The existing cold exact count and unknown-count timeline both scan wide asset records. query_where previously evaluated media classification, initial-preview visibility, presence and root scope before filename matching. On the generated 10k catalog, the first empty-name regression executes 120,014 SQLite VM steps versus 40,014 for its initial filename-first reference and fails in 0.46 seconds. Retain .context/p4-cold-summaries/regression-before.txt. Refine the accepted ordering to keep media classification first: its empty-name reference takes 60,014 steps, and non-media observations retain their original early rejection.

Move the existing escaped filename predicate immediately after media classification, ahead of presence/preparation checks. The shared builder keeps validated parameters in matching SQL order. No index requirement, new candidate lookup, query hash/cache change, schema/dependency/IPC migration, full catalog renderer array or original-file access is introduced. Count, timeline, pages, seeks, neighbors and frozen selection retain the same predicates and stable IDs. Existing count-cache epochs, bounds, inventory lag and initial-ready behavior stay intact.

This reduces statement work for filename nonmatches. It does not remove the table scan or reduce its logical pager bytes, and it does not establish a universal speedup for every catalog/filter distribution. Broad searches still require their actual matching work.

## Rejected alternatives and measurement discipline

The first release comparison, run 2f410d2b-e991-4c30-a18b-f783480da215, uses the owned million-row fixture and independently unchanged 33-entry source inventories. Forcing assets_name or scanning the covering name index with a rowid join cuts narrow count medians from 274.78 ms to 97.88/97.45 ms, but regresses the all-matching count from 251.92 ms to 326.88/331.60 ms. Its broad timeline median also rises from 638.32 ms to 694.74/722.39 ms. Reject both as production policies; retain raw SQL, plans and samples under .context/filtered/2f410d2b-e991-4c30-a18b-f783480da215/ and diagnostic-v1.rs in .context/p4-cold-summaries/.

A second exploratory run, 5eaad725-5965-48aa-9080-439cfffcde25, tested fully filename-first predicates. A focused debug test build overlapped its later measurements, so retain it as exploratory evidence only. It is not the accepted final timing comparison. Keep its source snapshot as diagnostic-v2.rs. The final run uses media-first/name-early predicates, runs without concurrent builds, and compares legacy/current SQL in one release executable.

SQLite documents [covering index tradeoffs](https://www.sqlite.org/queryplanner.html#covering_indexes), [manual join ordering](https://www.sqlite.org/optoverview.html#manual_control_of_query_plans_using_cross_join), and the fact that [INDEXED BY is a requirement](https://www.sqlite.org/lang_indexedby.html), rather than a portable optimizer hint. The measured broad regression motivates retaining planner freedom here.

## Reproduction and scope

Build node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example filtered_probe, then run python3 scripts/performance/filtered.py --fixture-run 475b54c4-2fe2-4337-a961-42fe7742ebac --rows 1000000. The UUID refers only to the previously retained generated fixture; the runner rejects arbitrary library paths and validates ownership and row count. Do not scan a real library, purge OS caches or run builds concurrently with timing collection.

The existing runner retains three page-first/timeline-first endpoint samples plus separate summaryProbe legacyNameLast/currentNameEarly reference comparisons. Cold reference statements clear only the connection pager before each count and timeline measurement; OS filesystem caches are uncontrolled. They verify equal exact counts and complete month results, retain plans, SQLite VM steps, logical pager bytes, raw timings, linked SQLite/build/lock provenance and independent lossless source inventories. These are release backend synthetic-catalog measurements, not physical disk, real-media decode, full-memory or native release presentation evidence. Historical outputs without summaryProbe remain supported.

## Final release measurements

Run ae8b03e2-62b3-46c6-8cb2-3ef052e15419 compares three samples per statement/variant on the same million-row catalog. Host: macOS 27.0.1 arm64, Apple M1 Max, 32 GiB RAM, 10 logical CPUs; Rust 1.99.1 and Node 26.3.1. Linked SQLite is 3.54.1, schema 2 and synchronous 2/FULL. The release probe SHA-256 is fc88829bcfacebf7e8b0aa371a2df635f4d0229363b7afd0282a0fca96907bda; lock hashes are unchanged. Independent before/after manifests match all 33 generated source entries. Full provenance/raw samples are in .context/filtered/ae8b03e2-62b3-46c6-8cb2-3ef052e15419/; environment/build/check logs are under .context/p4-cold-summaries/.

Values below are p50 / p95 milliseconds, n=3 per cell. With three samples, p95/p99 select the maximum; these are small diagnostic samples, not release-wide target acceptance.

| Scope      | Matches   | Legacy count       | Current count      | Legacy timeline    | Current timeline  |
| ---------- | --------- | ------------------ | ------------------ | ------------------ | ----------------- |
| none       | 0         | 255.353 / 267.092  | 208.379 / 1035.144 | 246.054 / 297.456  | 211.096 / 228.359 |
| narrow     | 999       | 274.610 / 280.338  | 232.247 / 232.545  | 277.758 / 280.509  | 232.523 / 237.073 |
| broad      | 1,000,000 | 257.418 / 281.786  | 255.776 / 257.796  | 666.547 / 680.876  | 671.304 / 674.631 |
| lateBroad  | 10,000    | 401.249 / 1035.200 | 234.400 / 235.937  | 286.449 / 2548.694 | 238.220 / 239.303 |
| lateNarrow | 100       | 277.631 / 277.913  | 236.237 / 236.720  | 281.534 / 286.359  | 236.846 / 238.796 |

The narrow count median falls 274.610→232.247 ms and cold timeline 277.758→232.523 ms; late narrow falls 277.631→236.237 ms and 281.534→236.846 ms. Empty count work halves from 12,000,014 to 6,000,014 VM steps; narrow falls 12,001,013→6,007,007. All-matching count/timeline VM work is identical before/after (13,000,014/28,018,787 steps); its time remains broadly similar, with the timeline median slightly higher. Retain every sample, including the noisy late-broad legacy count, rather than claiming uniform improvements.

Both variants retain the same table-scan/group/sort plans and 456,265,728 logical pager-miss bytes per cold statement. These are connection pager observations, not physical disk traffic. The production page-first/timeline-first endpoint samples and candidate-lookup plans remain separately retained in the same report; do not equate the reference-statement timings with end-to-end UI latency.

## Validation and next gate

npm run check passes 191 frontend/harness tests across 34 files and 99 Rust tests (74 unit/23 library/2 worker), strict types/lint/format/bindings, rustfmt, warnings-denied Clippy and production frontend build. All 11 catalog regressions also pass in release mode; default-feature core all-target Clippy and five Python evidence tests pass. No React source changed, so no new React Doctor result is claimed.

The symbol-verified debug executable 9a778ec7baa8adcc7bc53026ee9563bb72f078ac1199e5b9b9a9c476f0612161 passes all 15 native macOS cases in 35 seconds (41.306 s including runner startup). Actual search/filter/selection, initial-ready collection, original viewer/navigation/annotations/return and live-library reconciliation remain functional. Independent read-only manifest pairs match at recorded precision: first-import 32 entries uses the harness’s numeric stat representation; live renamed/absent phases use lossless decimal stat fields. Deliberate external live-fixture staging restores file bytes/identities with two audited parent-directory mtime changes. Exact run paths/comparisons are in native-source-audit.json.

The unsigned 34.53 MiB production app builds, executable SHA-256 126a90cbc40b4cc5b28be5a843ec2c684534274a7967ad01de5bb26e3bb92ad8. Production instrumentation/default-dependency/frontend-route exclusions and reviewed helper checksums pass. The packaged decoder/ExifTool probe indexes 20 generated media, reads original JPEG/PNG/WebP, verifies exact JPEG source bytes and retains 32 unchanged source entries. Actual packaged in-memory SQLite remains 3.54.1 with userCatalogOpened=false. These are local unsigned functional checks; Windows, signing and clean-machine support are not established.

Changed-document formatting, all 131 local documentation links and git diff --check pass. Stop at this bounded predicate-order refinement. Cold table scans, unknown/large summaries and mixed-filter costs are not eliminated. Earlier trace loss, unavailable complete memory, Windows/minimum OS/clean-machine, native release presentation, backup/endpoint/pacing/NAS/HDD and all preservation/later-milestone gates remain open.
