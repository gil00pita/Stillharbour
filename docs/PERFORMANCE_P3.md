# P3 — shared source admission baseline

Owner request: 2026-10-29, “Start new stage,” continuing the selected performance track. [Task scope](../tasks/P3_SHARED_RESOURCES.md) and [ADR-0014](decisions/ADR-0014-shared-source-admission.md) define the available read-only implementation. P2 and live-library evidence remain historical. This stage adds no original-file operation, schema/dependency upgrade, decoder or publication.

## Implemented scope

- One Library-owned scheduler replaces scan/rendition media semaphores. Two active steps globally; one source reader per known native volume; 64 queued requests; 384 MiB estimated reservations. Metadata reserves 64 MiB and renditions 192 MiB. These estimates limit admission, not whole-process RSS or native codec internals.
- Visible gallery/viewer requests precede unaged metadata/thumbnail-prebuild requests. Ready requests age after 500 ms into oldest-first service. Acquisition is atomic across reservations and source volume; independent volumes can progress while another volume is busy. A zero/unknown group conservatively conflicts with every active source.
- Queued cancellation is checked before starting work and at 20 ms waits. RAII releases admission on errors/panics. Existing helpers supervise running cancellation; a canceled UI generation alone does not prove a decoder stopped. Metadata/portable input reads check cancellation before each ≤64 KiB chunk. Original source/identity/revision revalidation, cache publication/ownership and maintenance exclusions remain intact.
- During gallery scrolling, request only visible 32 px previews; after the existing virtualizer settles, restore clear tiers and bounded overscan, visible first. Retain already displayed layers, viewport anchors, text-only behavior and viewer controls. No speculative original viewer reads are added, including offline roots.
- Existing opt-in fixture-only 4096-event diagnostics add interactive/background admission wait metrics. Production diagnostic calls remain no-ops. No paths, file names, content or credentials enter scheduling traces.

## Coverage limits

`backup::production_backup_available()` is still false. `scheduler_probe` uses four generated 256 KiB files on one actual local volume and 60 read steps per contender, each followed by a known 2 ms simulated helper step while holding admission. The “backup_read_fixture” label describes a synthetic read contender, not restic/rclone integration, upload, restore, safe publication or verification evidence. Source inputs are synthetic bytes, not image decode fixtures. OS caches are uncontrolled; no warm/cold media claim follows.

Native volume identity merges paths on the same volume but cannot map distinct volumes to one physical disk or multiple NAS aliases to one endpoint. No hard byte-rate limit, transfer profile, rate/pause setting or physical-I/O guarantee is exposed. Enumeration/stat/header reads, interactive catalog reads, cached-preview reads and native helper internals are not fully charged byte I/O. Helper-internal threads and allocations remain opaque; two admitted steps do not imply a two-thread native decoder limit. Disk/network physical throughput, complete process-tree memory trends, release presented frames, Windows, controlled NAS/HDD, minimum OS, clean-machine and signing remain separate unrun/blocked gates.

## Reproduce

```sh
export PATH="/Users/gilalvaro/.nvm/versions/node/v26.3.1/bin:$PATH"
npm run check
node scripts/cargo.mjs build --locked --release -p stillharbour-core --features fixture-probes --example scheduler_probe
python3 scripts/performance/scheduling.py
npm run desktop:test:build
```

[The fixture runner](../scripts/performance/scheduling.py) creates a UUID-owned directory under `.context/scheduling/`, four disposable source files, and independent lossless-stat SHA-256 manifests before/after the executable. It records raw timing samples, stdout/stderr, executable/lock SHA-256 and a percentile report. It performs no cleanup, backup or real-library operation. [The executable](../crates/core/examples/scheduler_probe.rs) accepts only `--run UUID` in this generated namespace and checks directory/marker ownership. A process failure or manifest mismatch is a failed run. The general native driver uses generated fixtures; never point it at a user library.

## Results

Host: macOS 27.0.1 (26A434), arm64 Mac13,1, 32 GiB RAM, 10 logical CPUs; rustc 1.99.1, Node 26.3.1. `.context/p3/environment.json` records fresh commands and lock hashes. Cargo SHA-256 `6f4e5ee5d3d6be2a21beb28b1da6472f39c079cbc5f31e08354ca0050ce30fe9`; npm `eab358a6345e96c3195256904292d24534df4033a1c4aa79f1b26086f1afeda0`, unchanged from the preceding live-library change. No linked SQLite/dependency upgrade. Earlier packaged SQLite provenance remains historical; no new advisory review or release claim follows from this stage.

Release synthetic run `dc86871e-1cb6-45fd-a6b9-6494cda5bce1`, `.context/scheduling/dc86871e-1cb6-45fd-a6b9-6494cda5bce1/report.json`, exits 0 with all four independent source entries identical and zero dropped trace events. Executable SHA-256 `e57349ab57fdbf9a7af1f565e7b15bb9848de9565e928335b1cc3d7a68eed059`. Each contender issues 15 MiB of logical source reads (60×256 KiB), 60 MiB total; this is not physical-device traffic. Measurements overlap repository checks on this development host and are diagnostic observations, not an idle-machine SLO comparison. No network transfer occurs and no process-tree sampler was run. Memory admission boundaries are tested independently, not measured RSS trends.

Percentiles use nearest rank `ceil(p*n)`, with 60 samples per row. Admission wait spans immediately before scheduler acquisition to lease return. Step duration spans the generated read and the deliberate 2 ms simulated helper sleep; OS scheduling can extend it. All values below are milliseconds, rounded to3 decimal places.

| Generated contender        | Wait p50 / p95 / p99 (ms) | Read + simulated step p50 / p95 / p99 (ms) |
| -------------------------- | ------------------------- | ------------------------------------------ |
| Visible                    | 0.001 / 3.843 / 12.362    | 2.841 / 8.197 / 23.650                     |
| Metadata                   | 8.484 / 68.534 / 98.150   | 3.091 / 14.113 / 38.006                    |
| Backup read fixture        | 9.539 / 69.664 / 96.725   | 2.796 / 12.579 / 36.370                    |
| Thumbnail prebuild fixture | 11.541 / 48.478 / 151.058 | 2.643 / 21.407 / 25.857                    |

These figures show service under this finite synthetic workload, not proof of indefinite fairness, actual backup throughput or faster gallery paint. The independent age/priority/volume/memory/cancellation/unwind/queue-bound tests establish policy behavior.

The first focused Rust invocation failed to compile a test because a cloned scheduler binding was moved into a thread and then reused; corrected before validation. Retain `.context/p3/scheduler-tests.txt` as failed evidence. `.context/p3/scheduler-tests-verified.txt` passes all six scheduler tests. `npm run check` passes 159 frontend and 80 Rust tests (58 unit, 20 library, 2 worker), strict types/lint/format, bindings, warnings-denied Clippy and frontend build; `.context/p3/check.txt`. The final warnings-denied Clippy, formatting and full Rust rerun also pass after interrupted-read handling was added (`.context/p3/rust-final.txt`).

## Native and final validation

`npm run desktop:test:build` passes; the guarded copy `target/debug/stillharbour-p3-test` includes the fixture-only symbol, SHA-256 `778bae94069158f85f77cf8fe9d22e85727a040cb8c179f85c8484bd5e01eff1`. Both embedded-driver service and capability select this copy to avoid the user's separate development watcher replacing the tested executable. No real user catalog/library is opened. Native run `.context/p3/native-live.txt` passes the one live-library scenario in 14.3 s, WDIO WebKit reported 605.1.15 (generic driver label, not exact engine provenance). Fixture `native-86637798-8a9b-4ba2-b3d7-5f01ec7f7a4a` has exact lossless read-only manifests: 32 entries after external rename, 31 while absent. The test's own external staging restores every file/path/native identity and file metadata; only two parent-directory mtimes change, explicitly retained in `restored-source-check.json`. Screenshots are copied under `.context/p3/`; the feature UI remains unchanged.

Python diagnostics pass two tests (`.context/p3/python-tests.txt`). React Doctor reports 73/100 and 11 warnings, unchanged from the preceding live-library state (`.context/p3/react-doctor.txt`); no rules disabled or numeric-score improvement claimed. The isolated native foundation run passes all 14 scenarios in 32 s (`.context/p3/native-foundation.txt`), including real gallery/timeline scroll, viewer return, list/selection, folder navigation, Settings, maintenance and unchanged generated originals. Together with the isolated live-library case this is 15 passing native cases across two sessions, not a combined-driver or Windows result. The final Rust rerun passes all 80 tests. No unsigned release application bundle was rebuilt in P3; the release build/measurements here apply to the fixture executable only.

## Acceptance disposition

| Scenario | Disposition                                               | Evidence / limit                                                                                                                                                                                        |
| -------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P3-01    | Passed for the implemented admission scope                | Independent volume/global/memory/queue-bound tests; metadata and both rendition callers share the same Library-owned scheduler. Physical-device mapping is outside this pass.                           |
| P3-02    | Passed for queue policy                                   | Deterministic aging/FIFO/priority checks and threaded source exclusion. This is not a wall-clock latency guarantee.                                                                                     |
| P3-03    | Passed for queued admission and current helper regression | Pre-cancel/queued cancel, retry/unwind, bounded-input fixture and existing worker supervision tests. Blocking kernel I/O and running native codecs are not instantly canceled.                          |
| P3-04    | Passed for component behavior                             | Scroll-visible-only tiny/settled bounded upgrade regressions;159 frontend tests including viewer/list/anchor behavior. Native outcomes are explicitly scoped above.                                     |
| P3-05    | Passed                                                    | Release generated read contender report, 240 steps, a matching independent inventory of all four sources, raw samples/hashes, zero trace drops. Not backup or media decode evidence.                    |
| P3-06    | Blocked for full platform acceptance                      | Repository checks/local native cases as reported; Windows/minimum OS/clean-machine and release presentation/full memory remain unrun.                                                                   |
| P3-07    | Blocked                                                   | No production backup/transfer adapter, reviewed complete physical-device/endpoint mapping, byte pacer or mutation-safe pause/commit integration exists. PS-031–PS-035 are not passed by these fixtures. |
| P3-08    | Not run                                                   | No controlled native release NAS/HDD, Windows, whole-process memory or presented-frame measurements.                                                                                                    |

Stop after this available read-only P3 slice. Before further scheduling work, review actual backup/transfer adapter prerequisites and the tighter preservation burst/accounting rules, then establish endpoint/physical-volume mapping and independent fake-clock pacing/failure tests. Keep exact-version recovery records and retained versions mandatory. P4 tuning or original mutations require separate authorisation and their existing gates.

Final owned-file/link audit checks 254 text files and 87 retained before/after pairs, with zero findings. Four full live-test restoration pairs differ only in the explicitly recorded parent-directory mtimes caused by external staging; read-only phase pairs match exactly at their recorded precision. All three retained P2 rendition source trees also match fresh inventories. This audit does not upgrade earlier legacy-JSON timestamp precision. Final formatting and `git diff --check` pass.

## Owner gallery follow-up — 2026-10-29

The owner's dark-line/loading/sticky-date request revises the scrolling behavior above: visible tiny keys can queue while clear keys perform cache-only lookups, already-loaded clear pixels stay mounted, settled admission uses 150 ms and pending polling 200 ms. Date sections retain full extent with virtual inner rows and native sticky headings; each date begins separately. [GALLERY_REFINEMENT.md](GALLERY_REFINEMENT.md) owns this refinement and its new evidence. Earlier P3 synthetic measurements remain historical and do not measure this updated gallery pipeline. Source/worker/resource limits, backup/physical-endpoint/pacing and platform gates are unchanged.
