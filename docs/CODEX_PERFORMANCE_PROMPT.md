# Codex prompt - Stillharbour performance work

You are working on Stillharbour, a local-first Windows/macOS photo and video library. Improve responsiveness without changing original files or weakening safety. This prompt authorises P0 only, followed by a report; it is not permission to rewrite the application or implement every phase.

Read the repository AGENTS.md and existing CODEX_SPEC.md, docs/PRESERVATION_CONTRACT.md and relevant accepted feature specifications. Then read this addendum's PERFORMANCE_PLAN.md, PERFORMANCE_ACCEPTANCE_TESTS.md and SOURCES.md. Existing preservation requirements override speed targets. The addendum is a proposal, not evidence of implemented performance.

## P0 scope

1. Inspect the real repository. Identify current gallery, database, thumbnail service, Tauri bridge, job scheduler and backup adapters. If the app or these components do not yet exist, report the prerequisites and add only appropriate fixture/instrumentation scaffolding; do not fabricate traces.
2. Record platform/toolchain/lockfile details, bundled SQLite version and source ID, release/debug mode, WebView versions, and baseline hardware. Check the actually linked SQLite build against the WAL-reset fix and current advisories. Do not upgrade dependencies or migrate a user catalog without a reviewed plan.
3. Add bounded, opt-in local performance instrumentation for catalog pages, thumbnail queue/decode/paint, UI frame work, IPC payload size, process-tree memory and per-endpoint I/O. Exclude photo content, exact paths, GPS and credentials from logs. Instrumentation must not create an unbounded log or telemetry upload.
4. Create disposable seeded fixtures for catalog sizes 10,000, 100,000 and a separate 1,000,000 synthetic rows. Include equal timestamps, unknown dates, dense days, extreme aspect ratios, offline roots, tags, small and large videos and representative image formats. Synthetic catalog tests are not full-media decode benchmarks.
5. Establish warm/cold-cache protocols and test scripts for opening a catalog, scrolling, random date jumps, resize, viewer navigation, selection and browsing while a bounded scan/backup runs. Record completed versus unrun tests honestly; do not claim native macOS/Windows evidence from browser-only mocks or this container.
6. Record current source fixtures' independent manifests before and after read-only runs. No original-file mutations, cleanup, auto-conversion or unsafe cache eviction.
7. Report the three largest measured bottlenecks, proposed smallest fixes, and prerequisites for P1. Keep the current UI and behaviour unless a diagnostic-only change is necessary. Do not introduce a new framework, SQL schema migration, decoder or filesystem write feature in this phase.

## Deliverable

Create docs/PERFORMANCE_BASELINE.md containing environment, fixture/trace identifiers, measurement definitions, p50/p95/p99 results where measured, memory trends, disk/network observations, failures, skipped tests and limitations. Link executable diagnostic scripts and their invocation commands. Mark each acceptance scenario passed, failed, blocked or not run only when supported by evidence.

Stop after P0. Later approval can authorise indexed catalog paging, row virtualisation, rendition caching and shared resource scheduling in that order. Never skip independent verification, source identity checks, durable recovery records or retained previous versions to meet a timing target.
