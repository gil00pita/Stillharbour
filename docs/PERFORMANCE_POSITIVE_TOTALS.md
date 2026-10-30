# P4 continuation — positive timeline totals

Owner request: 2026-10-30, “Continue to next stage.” [Task scope](../tasks/P4_POSITIVE_TOTALS.md) continues the positive-filter issue left by [exact-empty reads](PERFORMANCE_FILTERED_READS.md). Earlier [P4 trace, memory and platform failures/limits](PERFORMANCE_P4.md) remain open. This stage reuses an already completed positive timeline; it does not accelerate every cold filtered read.

The subsequent “Run the next stage” request continues [selective filename projections](PERFORMANCE_FILENAME_READS.md). Its evidence and narrower policy do not replace the measurements below.

## Reproduction and change

The existing release filtered probe now records both page-first and timeline-first request orders, resetting only derived summaries before each pair. A newly generated million-row fixture reproduces a redundant count after the positive timeline: broad/narrow page medians are 244.699/493.724 ms. The first investigative run 6b9d0d8a-2116-4ee6-a19c-01d219641c9c overlaps a short debug regression build; retain it separately. The comparison baseline 8f328934-0601-4776-bd75-a3121ec2ece1 and after run e39a494c-5b5f-46d1-9cff-7be5db0afbd5 run serially without concurrent builds/tests. OS caches and ordinary development-host load remain uncontrolled.

The first 10k regression's absolute 128 KiB screen also includes an expensive capture projection. Minimize the signal by comparing identical page/seek reads against their already-count-cached reference, after clearing only the disposable connection pager. This fails in 0.39 s: 19,968,000 versus 16,674,816 logical pager bytes, over the 64 KiB allowance. It isolates 3,293,184 redundant logical bytes without treating the remaining projection as a bounded index seek. Raw initial and minimized failures remain in .context/p4-positive/.

Ranked hypotheses: ignored positive bucket totals; ordered projection work; conservative invalidation. The minimized test holds projections constant and makes no writes, isolating the first. Existing cached_count now checks the fully query-hashed, epoch-validated bucket summary when an exact count is required, adds year counts with checked u32 arithmetic and retains the result through the existing count cache. A miss/eviction/oversized unretained timeline falls back to the original SQL. No eager timeline calculation, SQL/index/schema/dependency/IPC change or new cache.

Visible unfiltered inventory totals still use root counters, which may lag a scan; a complete positive timeline must not silently replace that existing behavior. Include-missing counts remain exact. Positive tag/root scope, actor/external invalidation without revision bumps and inventory lag are covered in the existing strengthened regressions. All five sorts compare complete page responses and seek identity/totals. Selection membership and preservation evidence do not use this cache.

SQLite documents [count(*) as the number of rows in each group](https://www.sqlite.org/lang_aggfunc.html). Existing freshness checks remain [connection total changes](https://www.sqlite.org/c3ref/total_changes.html) plus [data_version for external commits](https://www.sqlite.org/pragma.html#pragma_data_version); none is weakened for a measurement. Failed timeline parsing never supplies a count.

## Commands and scope

```sh
node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example filtered_probe
python3 scripts/performance/filtered.py --fixture-run UUID --rows 1000000
node scripts/cargo.mjs test --locked -p stillharbour-core --features fixture-probes positive_timeline_supplies_exact_page_and_seek_counts -- --nocapture
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

Use an existing generated UUID namespace as documented in DEVELOPMENT.md. This workspace starts without retained performance fixtures. A seed-only invocation of the existing performance_probe creates 475b54c4-2fe2-4337-a961-42fe7742ebac-1000000 (43.160 s). The local harness deliberately closes stdin at its independent-manifest handshake; its expected exit 1 is recorded in seed-stop.json, not claimed as a passing measurement route. Independent lossless sources match around that stop and both filtered measurements. No user-library path, source scan, cache purge or cleanup is accepted. Catalog opening retains full validation and may checkpoint the app-owned fixture catalog; generated media manifests are separate.

Builds and measurements run serially. Three samples per scope/order; nearest-rank p95/p99 equal the maximum and cannot establish a population SLO. The first call of a pair owns the expensive scan. A fast second page does not imply a fast total timeline-first interaction, and the UI does not enforce this request order. Production query behavior with a cold/evicted cache falls back normally.

## Release results

Host macOS 27.0.1 arm64, Apple M1 Max, 32 GiB RAM and 10 logical CPUs; toolchain details are in .context/p4-positive/environment.json. The million rows are synthetic catalog observations with offline roots, not a million decoded originals. Independent source manifests cover 33 generated entries. No physical-storage/cold-NAS or native UI inference. Linked SQLite remains 3.54.1, schema 2, synchronous 2/FULL. Lock hashes remain unchanged. Raw samples, plans, binary/lock/SQLite provenance and independent lossless source inventories remain in each `.context/filtered/<run>/`. Comparison probe SHA-256 before f3fb7fc4bfed310753c009e7464a0cae4bafa1bb4a6f80adda9ff6f702b4b1ae; after bb7ef84df075ccff5f2b21c3d45cc0a42756b0815da2b71471563c552c1c457b. No new advisory-free or OS-support claim.

Milliseconds, n=3 per scope/order. Table values are p50/p95; p99 equals p95.

| Scope  | Matches   | Page after timeline, before | Page after timeline, after | Cold page-first, before | Cold page-first, after |
| ------ | --------- | --------------------------- | -------------------------- | ----------------------- | ---------------------- |
| Empty  | 0         | 0.095 / 0.095               | 0.084 / 0.114              | 235.246 / 236.039       | 267.761 / 301.684      |
| Narrow | 999       | 493.724 / 500.517           | 237.311 / 238.466          | 514.684 / 516.526       | 533.184 / 549.254      |
| Broad  | 1,000,000 | 244.699 / 252.228           | 2.010 / 2.327              | 240.731 / 241.404       | 253.373 / 253.500      |

Cold timeline-first medians before→after: empty 254.469→257.794 ms; narrow 262.672→273.959 ms; broad 624.771→660.962 ms. Page-first subsequent timeline medians: 0.061→0.066 ms, 271.174→269.384 ms and 618.302→640.091 ms. Those scans/aggregations are unchanged; retain observed timing drift rather than claiming a general latency pass. The optimization removes the duplicated exact-count scan when a usable timeline is already retained. It does not meet the 250 ms screen for all scopes, and narrow capture projection still scans.

The focused regression now passes in 0.57 s. After timeline and already-count-cached reference have identical logical pager reads: capture 16,674,816; name 45,056; size/modified/added 49,152 each. These are logical cache-miss bytes, not physical disk traffic. Full validation and remaining gates are recorded below after execution.

## Validation and next gate

The full repository check passes 177 frontend/harness and 92 Rust tests (69 unit, 21 library and 2 worker), formatting/lint/strict types, generated bindings, rustfmt, warnings-denied Clippy and production frontend build. All seven catalog performance regressions also pass in release mode (1.51 s), including the new five-sort case with identical pager observations. Default-feature core all-target Clippy and five Python evidence regressions pass.

The symbol-verified debug native executable ed66738f3b6bacc08e44c51b363c0d850d54aa5ee74a75294fb92b2ca4f69034 passes all 15 macOS native cases in 29 s (34.389 s including runner startup). Three independent read-only source-manifest pairs match at recorded precision; the external move/restore fixture retains only two explicitly audited parent-directory mtime differences. Raw runs, identities and native-source-audit.json remain in .context/p4-positive/. Native functional checks do not establish release UI performance or Windows support.

The unsigned default-feature macOS application builds at 34.42 MiB, executable SHA-256 f0394b45041fb0ec2f878db0daa74b435befc2aa6b46660e103f558065146dc0. Production capability/default-dependency/frontend test-route exclusions and reviewed helper checksums pass. The actual packaged decoder/ExifTool smoke indexes 20 generated media and preserves 32 source entries, with JPEG/PNG/WebP previews. In-memory packaged SQLite provenance run 1f2d3e13-3391-4e62-a84e-51cda89fbee9 confirms 3.54.1 and userCatalogOpened=false. These are local unsigned functional checks, not signing/clean-machine or broad-format certification. Exact commands, exits and durations are in .context/p4-positive/validation-results.json; full repository/Python logs are alongside it.

Final changed-document formatting, relative links and git whitespace checks pass. Stop after this exact-total reuse. Cold filtered projection/aggregation, prior trace loss/unavailable full memory, Windows/minimum OS/clean machines, release presentation and real backup/NAS/HDD/pacing/preservation gates remain open. Save locally for review; no branch rename, commit or push is included.
