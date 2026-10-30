# ADR-0008 — Opt-in P0 performance evidence

Date: 2026-10-28  
Status: accepted for P0 instrumentation and fixture diagnostics only

The owner authorised P0 inspection, bounded local measurements, disposable fixtures and a report. The application already has native catalog paging, row virtualisation, cached previews and bounded independent work queues. Their existence does not close later performance phases or establish measured performance.

Use the existing architecture without changing its query plans, schema, decoder, durability or UI. The Rust diagnostic recorder is compiled to no-ops in default production builds and requires explicit enablement in fixture-enabled builds. Retain at most 4096 numeric events; drop rather than block on recorder contention/full capacity. Logical filesystem-volume aliases are session-local and limited to16; unknown/overflow is0. Keep raw device identities private. Source/cache payload bytes, SQLite page misses/writes and OS process I/O are distinct measurements, not a universal physical endpoint budget.

Only desktop-test builds expose the private performance fixture/trace command and renderer harness. Retain its existing release compile prohibition and capability separation. The normal binary's standalone `--diagnostic-sqlite` flag reports linked SQLite provenance through an in-memory connection and exits before Tauri/user-catalog setup. It accepts no paths or SQL.

Use the existing decoder and native root guards against generated sources. A separate release core probe seeds fresh schema2 catalogs with synthetic records; no user catalog is opened or migrated. The harness independently hashes source bytes and inventory using SHA-256 before/after runs, including failed runs where available. Retain owned backend artifacts; do not purge caches or clean personal libraries to create cold results.

Run native UI diagnostics separately from release backend observations. Label debug/Strict Mode/driver overhead, partial process attribution, finite trace capacity, low sample counts and uncontrolled OS caches. Do not treat rAF or an image-load callback as proof of actual presentation. Backup contention remains blocked until the production adapter exists. Proposed P1 fixes require separate approval and preservation review; this ADR grants none.

Actual evidence, missing proposal references, acceptance statuses and commands belong in [PERFORMANCE_BASELINE.md](../PERFORMANCE_BASELINE.md).

## P4 evidence collection — 2026-10-29

The authorized local P4 slice adds consuming reads of the same bounded diagnostic recorder. Drain events and observed losses atomically while retaining enabled state and the session-local volume map; production calls remain no-ops. Frontend drains retain observer lifetime and the original timestamp origin. The native harness writes at most32 segments of at most4096 events per recorder; overflow and recorder contention still count as loss and fail the new scoped report. This avoids restarting the observer or increasing production working-set limits. Release automation remains forbidden; [PERFORMANCE_P4.md](../PERFORMANCE_P4.md) separates release core/worker probes from debug native UI results and open platform gates.
