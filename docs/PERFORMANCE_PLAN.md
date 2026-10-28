# Stillharbour - gallery and background-work performance plan

Research checked: 28 October 2026  
Document revision: P1  
Status: independent engineering proposal; not implemented, benchmarked, or a reconstruction of Google's current private code.

## 1. Scope and precedence

This is an additive companion to the Stillharbour v1.2.1 specification, especially its image pipeline, catalog, IPC, preservation contract and performance section. It does not replace that package, change its branding, grant permission to edit originals, or move later features into an approved implementation milestone.

Preservation and correctness take priority over performance targets. The existing verified-copy, recovery, journal, source-identity, companion-file and fresh-evidence rules remain binding. File browsing can be immediately useful while verification is incomplete, but it cannot imply that an asset is protected or verified.

Windows and macOS remain first. Keep the existing Rust + Tauri + React/TypeScript + local SQLite architecture unless measurements justify a change. No application source was supplied for this investigation; recommendations must be reconciled with the actual repository before implementation.

## 2. Evidence: what is documented and what is proposed

The 2018 first-person Google Photos web-grid account describes section summaries, bounded visible elements, justified layout, batched updates, adaptive prefetch and small-to-large image loading. It is historical evidence, not a claim about the exact 2026 implementation. [S01]

A separate first-person account by the former Android lead describes local SQLite metadata, paged UI reads, and background synchronisation. Web and native mobile choices should not be conflated. [S02]

All numeric settings, data contracts, priorities and targets below are Stillharbour proposals. They are not reported Google settings. Do not copy historical microbenchmark numbers into Stillharbour marketing.

The implementation should solve four different costs: catalog lookup, UI layout/rendering, image decoding, and storage/network contention. Changing language alone does not eliminate those costs. Profile each independently and together.

## 3. Architecture and first useful paint

The UI reads compact catalog pages and local cached renditions. It must not enumerate folders or wait for a NAS connection merely to render already catalogued items. The Rust core owns catalog access, a bounded job scheduler, image-generation workers and a read-only rendition endpoint. Originals stay outside the derived cache.

On opening a known catalog:

1. Open/migrate it through the existing safe catalog workflow. Report migration time separately; do not conceal a necessary migration.
2. Return cached section summaries and one compact visible-page projection. Do not run a full rescan first.
3. Lay out the visible rows from recorded dimensions and date information.
4. Display cached thumbnails; show an honest placeholder when a rendition is absent.
5. Start limited reconciliation and missing-thumbnail work after the initial interactive frame.

First import is different from warm browsing. Publish discovered records incrementally, prioritise visible-item metadata, then generate visible thumbnails. Full hashes, deep integrity decoding and similarity indexing cannot block initial discovery. Newly discovered source items remain unverified until the relevant checks actually run. Imported destination items cannot be marked complete before their required copy verification.

Separate renderer-observed readiness, original availability, decoder support, verification evidence and backup coverage. A cached image is not a substitute for its original.

## 4. Virtualise rows and retain a bounded working set

Windowing limits rendered elements to the visible area and a nearby buffer; ordinary infinite scrolling alone does not provide this bound. [S03]

Build a row model for the justified gallery, then virtualise rows rather than treating every photograph as an identical-height list item. A headless utility such as TanStack Virtual is a candidate for the windowing layer, not an off-the-shelf justified-gallery algorithm. [S16]

Initial tunable policy:

| Item                       | Starting policy                                                              |
| -------------------------- | ---------------------------------------------------------------------------- |
| Mounted area               | Visible rows plus at most one viewport above and below                       |
| Metadata request           | 256 items normally; enforce a server-side maximum of 512                     |
| Resident detailed metadata | Around 2,000 items, with byte-based eviction as the actual limit             |
| Grid target heights        | Existing Compact/Comfortable/Large choices; approximately 150/220/300 CSS px |
| Outstanding work           | Explicit queue, byte and worker limits; no task per catalog item             |

Do not use a universal hard-coded tile count: viewport size, zoom, accessibility and photo aspect ratios affect how many items fit. Measure that mounted items scale with viewport size, not total catalog size or the number of photos previously visited.

Use stable asset identifiers for selection and focus. Keep selection independently of mounted components. For select-all, store a backend selection expression or snapshot token with exclusions instead of materialising a million JS objects. Bind any later file-operation plan to an explicit reviewed scope and revision; a live search must not silently expand a destructive selection.

Unloading an offscreen row must release app-owned image references, object URLs and subscriptions. Bound frontend query caches as well as DOM nodes. Do not keep every fetched page in React state forever. Browser-native decoded-image retention is not completely controllable; use admission limits and native memory measurements rather than claiming exact control.

The details list stays genuinely thumbnail-free. Tab switches suspend offscreen viewers, videos and speculative gallery work.

## 5. Timeline navigation and stable justified geometry

Return small summary records containing a section ID, date range, item count, estimated aspect-ratio totals, revision and date-group count. Split very dense periods into bounded chunks; month boundaries alone are not a sufficient bound. Empty-date assets live in a separate, explicit group using the existing date policy.

Use summaries to represent the whole filtered timeline without loading every asset. Maintain estimated section heights in a prefix-sum structure. A date-rail jump resolves a section and an indexed anchor directly; it must not request all intermediate pages.

Compute exact row geometry only for visible/nearby sections. As estimates become exact, preserve an anchor consisting of asset ID and its offset within the viewport. Apply height corrections and scroll adjustment coherently. Do not let both browser scroll anchoring and custom anchoring double-compensate.

A useful row formula for a candidate set of aspect ratios a_i is:

    row_height = (available_width - gap * (item_count - 1)) / sum(a_i)

Implement a simple deterministic bounded justified-row solver first. Test a bounded dynamic-programming solver later only when the visual improvement justifies measured cost. Avoid globally recomputing the full library during resize. Cache layouts by section membership revision, width, density, date policy and layout algorithm version.

A giant logical gallery can exceed browser coordinate limits. The million-row stress fixture must test logical-to-physical scroll mapping. Use section-local/rebased scroll space if required; preserve the date rail, keyboard navigation and anchor. Do not rely on one arbitrarily tall CSS spacer.

Apply the active root/search/filter scope to both sections and results. Expensive new filter summaries can arrive progressively, but label them as updating and prevent stale responses from another filter being displayed. A catalog revision change triggers a coherent refresh rather than mixing generations.

## 6. Local thumbnail pyramid and memory policy

Store originals unchanged. Generate disposable, read-only-to-the-renderer renditions under an application-owned local cache outside indexed libraries, backup history and quarantine.

Suggested rendition tiers, generated on demand rather than all at import:

| Tier                 | Approximate bounding-box long edge | Use                                                     |
| -------------------- | ---------------------------------- | ------------------------------------------------------- |
| Tiny                 | 64-96 px                           | Fast-scroll visual context                              |
| Grid small           | 256 px                             | Dense gallery                                           |
| Grid medium          | 512 px                             | Normal gallery                                          |
| Grid large           | 1,024 px                           | Large thumbnails / high-DPI display                     |
| Viewer               | Up to 2,048 px initially           | Fit-to-window inspection                                |
| Original/high-detail | On explicit zoom/edit demand       | Full-resolution inspection through bounded decode paths |

Choose a tier that covers both rendered dimensions multiplied by device pixel ratio. Do not upscale unnecessarily. Account for aspect ratio when converting a row-height request into a long-edge tier. Large panoramas may need a distinct bounded rendition.

Keep original aspect ratio, apply the documented orientation policy, and use a versioned colour-management policy. Start with correctly colour-managed SDR gallery renditions where appropriate; HDR/full-quality viewing is a separate tested capability. A rendition is not archival conversion or proof of original fidelity. Never write converted images over originals.

Initial budgets to tune on the reference machine: about 256 MiB for app-owned decoded-thumbnail allocations/admission estimates and a configurable 10 GiB disk cache, subject to a free-space reserve. Viewer/decoder memory is separately reserved under the process-tree budget. A 10 GiB cache does not promise all 100,000 photos are cached; record hit rate and eviction behaviour.

Use byte-based least-recently-used eviction. Restrict eviction to the cache's validated namespace and manifest. Do not follow symlinks/reparse points out of that namespace. Never treat history, quarantine, edit recipes, tags or source files as expendable cache.

The rendition key includes catalog ID, asset ID, observed file revision, requested rendition tier, pipeline/decoder version, orientation/colour policy and edit-recipe revision. A known source change invalidates relevant results. Timestamp/size matching may support opportunistic display reuse but cannot establish unmodified bytes. Retain last-refresh and verification provenance; an integrity audit must read the relevant content independently.

Publish cache objects through exclusive staging and atomic publication where supported. Coalesce duplicate requests for the same key. Record persistent decoder errors with bounded retry/backoff rather than entering a render-triggered retry loop.

Image-library evaluation should include native shrink-on-load rather than always decoding full-resolution pixels and resizing afterward. libvips documents a combined thumbnail path and warns about dimensions, untrusted loaders and unusually expensive progressive images. It is an evaluation candidate, not a mandatory dependency or an assertion that Google uses it. [S13]

Use restricted image-worker processes for risky/native decoders where feasible; account for their internal thread counts. Apply dimension, pixel, memory and time budgets with a visible unsupported/too-large/error state. RAW embedded previews can make browsing useful but cannot count as a deep RAW integrity check. Animated images and videos use still posters in the gallery by default.

## 7. Request scheduling, prefetch and viewer transitions

Rank requests by user need, not enqueue time alone. Visible thumbnails and the explicitly opened viewer beat speculative work. Discard requests for obsolete view/filter generations. Reuse a close-enough existing thumbnail after a small window resize instead of regenerating every tile.

Start with this tunable prefetch behaviour:

- Slow scrolling: request the next viewport in the current direction after visible requests.
- Fast scrolling: prefer existing tiny renditions and postpone expensive new decodes.
- Large date jump: cancel queued intermediate work and load the target section.
- Scroll stop: upgrade visible renditions before extending prefetch.

Already-running native work may finish; cancellation of a request does not imply the underlying decode stopped. Ignore stale results for display, but a valid completed rendition may enter the bounded cache. Prevent retry/cancellation churn from increasing I/O.

Open the viewer immediately with the available cached rendition and explicit loading/original-availability state. Upgrade to a suitable viewer rendition, then request high detail only when needed. Prefetch at most the next and previous viewer item under the same memory/I/O budgets. Disable speculative original reads for an offline root or strict network profile.

Respect reduced-motion settings. Do not animate hundreds of blurred layers or permanently promote every tile into a GPU layer. Keep cache upgrades within fixed geometry, so quality changes do not move photographs.

## 8. Keep the interface thread small

Google's general rendering guidance explains the 16.7 ms frame interval at 60 Hz and the need to leave time for browser work. Its layout guidance recommends separating geometry reads from writes to avoid forced repeated layout. [S04, S05]

For Stillharbour, coalesce scroll-related updates into one scheduled animation-frame callback. Avoid per-scroll global React state updates and per-tile layout measurements. Keep stable props and fine-grained subscriptions; selecting one tile must not recreate all visible images. Profile before introducing more complicated memoisation.

Use a Web Worker for bounded layout or other measured JS computation that exceeds the renderer budget. Use Rust worker pools for file enumeration, metadata extraction, hashing and image analysis. Calling a function async does not automatically make CPU work cheap.

Tokio explicitly warns that spawn_blocking can create many threads, recommends limiting CPU concurrency, and does not support aborting already-running blocking tasks. [S12] Acquire resource permits before starting work. Use cooperative checkpoints for loops and process-level containment for uninterruptible decoding; never kill an original-mutating process mid-commit to meet a UI cancellation target.

Coalesce ordinary progress updates, initially to 5-10 updates/second. Deliver important state transitions reliably, and recover current job state through a query after a renderer reconnect. Progress updates are not the durable operation journal.

## 9. Catalog queries and Tauri transport

Return a lean gallery projection: IDs, display-date keys, width/height/orientation, observed revision, media type, duration when relevant, status bits and rendition identifiers. Fetch full EXIF, tags, paths, audit history and backup detail only when requested. Avoid SELECT * on a gallery hot path.

Use keyset pagination with a deterministic time-and-ID ordering and matching indexes. SQLite documents that deep OFFSET queries do work proportional to the skipped rows. [S06] Define NULL/unknown-date ordering explicitly. Provide separate indexed plans for all-roots and selected-root views; verify real EXPLAIN QUERY PLAN output. Tag, filename and metadata searches must have suitable indexes or a documented slower path rather than an assumption that one date index solves every filter.

Illustrative contract, not a drop-in API:

    query_gallery(scope, sort, cursor, limit, request_generation)
      -> items, next_cursor, catalog_revision, request_generation
    query_timeline_sections(scope, revision_hint)
      -> sections, catalog_revision, summary_state
    resolve_date_anchor(scope, date, sort)
      -> section_id, anchor_cursor, catalog_revision
    request_rendition(asset_id, observed_revision, tier, recipe_revision)
      -> ready_url | queued_request_id | unavailable_reason

Validate scopes, limits and enums in Rust; never accept arbitrary SQL or a raw filesystem path from these operations. Apply results only to the matching request generation. Avoid keeping a read transaction open while waiting for images, network I/O or UI interaction.

The active catalog and cache stay local. SQLite WAL allows readers and a writer to overlap, but it does not support a live database shared over a network filesystem. [S07] Use bounded writes and coordinated checkpoints. Do not weaken durable journal/annotation writes to improve a benchmark. SQLite documents that WAL with synchronous=NORMAL can lose committed transactions after a power failure; preserve the existing durability requirements and prefer FULL for authoritative records, subject to tested platform guarantees. [S08]

Release gate: inspect the SQLite version/source ID actually bundled with the application and helpers. The WAL-reset corruption fix is in SQLite 3.51.3, released 13 March 2026, and documented backports. Use a maintained non-withdrawn patched release, verify build provenance, and recheck advisories at release. Do not rely solely on the Rust crate version. [S07, S17]

For thumbnails, prefer an ID-scoped local asset/custom-protocol URL that the image element can load without carrying full image blobs through React state. Tauri documents JSON serialization overhead and an optimised binary response path; its channel mechanism is appropriate for bounded streamed messages. [S09] Compare transport implementations in native builds before selecting one. Do not claim zero-copy behaviour.

A custom endpoint must map opaque IDs to authorised rendition objects; no unrestricted filesystem reader, localhost server exposed to the LAN, path traversal or shell command interpolation. Tauri asset scopes are a restriction mechanism; an arbitrary custom protocol must implement its own equivalent checks. [S10] Revoke object URLs and limit outstanding requests. Keep photo content and sensitive paths out of performance logs.

## 10. One resource scheduler for disks, CPU and transfers

Every component competes for the same physical resources. A limit on backup upload alone does not bound thumbnail reads, verification reads or source hashing. Route app-owned work through shared CPU, memory and per-device/per-network-endpoint budgets.

| Class                                               | Treatment                                                                         |
| --------------------------------------------------- | --------------------------------------------------------------------------------- |
| In-progress safe commit/journal step                | Finish the bounded safety-critical step; do not interrupt it to improve scrolling |
| User-visible requests                               | Give responsive service within user-set hard resource caps                        |
| Interactive catalog/metadata requests               | Small, latency-sensitive reads                                                    |
| Nearby rendition prefetch                           | Opportunistic and cancelable                                                      |
| Backup, verification, indexing, reconciliation      | Bounded fair service with aging/reserved capacity; not starvation                 |
| Whole-library cache prebuild and similarity reindex | Idle/explicit mode; pause or slow during interaction                              |

Start conservatively: one active bulk source-reading job per HDD or network endpoint, and perhaps two per SSD after measurement. Map multiple paths/shares to the same resource where possible. When topology is unknown, use conservative endpoint grouping and allow an advanced override with an explanation.

For CPU tasks, start with 2-4 workers on the 16 GB reference machine, bounded by available CPUs and actual per-job memory reservations. Native codecs may already be multithreaded, so do not multiply an outer worker count by unrestricted inner pools.

Quiet mode should build on the existing proposed 10 MiB/s transfer profile and one bulk transfer. Define two controls clearly: transfer payload rate and aggregate application I/O budget per endpoint. Charge copy reads/writes, verification and history reads/writes to the appropriate endpoint budgets. Copying within one device consumes that device's budget for both directions; verification consumes additional work, not free throughput.

Use paced chunk dispatch/token buckets with bounded bursts; do not accumulate unlimited idle credit. A candidate starting burst is no more than 1 MiB at a 10 MiB/s limit, with bounded I/O chunks. Tune against both throughput and latency; a high file-operation rate on tiny files can be disruptive even at low byte rates.

Pause/resume at safe boundaries. OS, hardware and network buffering can still produce physical bursts, so label this as application-level pacing, not a guarantee of perfectly flat drive activity. Opaque backup adapters need measured controls and honest limitations; do not advertise a hard aggregate cap if their internal I/O cannot be accounted for or constrained. Never bypass required verification to maintain an apparent copy rate.

An explicit original request may have to wait behind a safe I/O step or the user's hard budget; show a truthful waiting state rather than silently exceeding limits. Automatic throttling can reduce concurrency but cannot increase a user-set maximum.

## 11. Incremental work, similarity and other screens

Rescan known roots incrementally. Treat filesystem notifications as hints and coalesce them. The notify crate documents missing network-filesystem events and other watcher limitations. [S14] Use scheduled/user-triggered reconciliation, identify overflow/gaps, and expose scan coverage. An unreachable folder is not an empty folder and never authorises deletion.

Compute visual fingerprints once per source revision and analysis-policy version. OpenCV documents perceptual image hashing suitable for image-similarity candidates. [S15] Use a metric/index appropriate to the chosen signature, then rerank candidates using higher-quality normalised renditions. Do not compare every image against every other image. For 100,000 items, an all-pairs scan has 4,999,950,000 pairs.

Benchmark resized/recompressed copies against difficult negatives: burst sequences, screenshots, near-identical scenes, solid colours and crops. Approximate search may miss matches; report measured recall/precision and let a slower thorough mode operate on a bounded scope. Do not label semantic resemblance or a perceptual hash match as byte identity. Results never authorise automatic deletion. Preserve unique RAW/Live Photo/sidecar components.

The map should request filtered geographic summaries and limited visible markers rather than creating a DOM marker for every photograph. The storage treemap should use catalog aggregates and drill-down requests, with size-accounting time and scan scope shown. Neither screen should start a surprise full NAS rescan.

## 12. Measurement contract and proposed targets

Carry forward the existing reference baseline: a recorded 4-core-or-better machine with 16 GB RAM and local SSD, a 100,000-asset representative catalog, and a separate 1,000,000-row synthetic catalog for query/selection/virtualisation stress. Record Windows/macOS versions, WebView versions, display scale/refresh, CPU, RAM, disk, source transport and exact fixture characteristics. Native process model and WebView differences require separate platform tests. [S11]

Do not describe a synthetic million-row test as evidence that a million real RAW images were decoded. Separate app-cold, OS-cache-cold where reproducible, thumbnail-cache-cold and warm-cache conditions. Use release builds; development-mode timings are diagnostic rather than release evidence.

| Metric                                           | Proposed baseline target                                                                                |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Known catalog, warm first visible cached gallery | p95 <= 2,000 ms; <= 1,000 ms stretch                                                                    |
| Ordinary indexed page/filter query, backend only | p95 < 250 ms; < 100 ms stretch                                                                          |
| Details-list first useful page, end-to-end       | p95 < 500 ms                                                                                            |
| Common action to next visual acknowledgement     | p95 < 100 ms, separately report result-completion time                                                  |
| Visible JS update work during warmed scrolling   | p95 <= 6 ms on a 60 Hz test display; tune on higher refresh                                             |
| Warm-cache scrolling                             | Target < 5% missed display frames over a specified 60-second route; no sustained app-induced freeze     |
| Text-only mode                                   | Zero thumbnail/original image-decode requests caused by that view                                       |
| Steady-state app memory                          | Below existing 1 GB objective for core + UI + visible/cache working set; report method and subprocesses |
| Cancellation acknowledgement                     | <= 1 second; actual safe stop has a distinct measured bound/state                                       |
| Original data/verification correctness           | Zero unexpected modifications or false verified-success states in the test suite                        |

These are proposed goals, not results or universal guarantees. Background-throttled and cold NAS runs must remain responsive and truthful even when image availability is much slower. Do not count a placeholder-only screen as a fully populated gallery metric.

Capture p50/p95/p99, frame timelines and memory trends, not just average FPS. Distinguish resource bytes transferred, app-level I/O bytes, OS-observed physical I/O where available, decode time, queue wait, IPC time, DB time, layout time and final paint. A requestAnimationFrame heartbeat alone does not prove presented-frame performance; use native/browser traces.

Test repeated forward/backward routes, random year jumps, resize/DPI changes, large selections, offline roots, slow/disconnected NAS, imports and simultaneous backup/hash tasks. Use independently generated disposable fixtures and before/after source manifests. Do not test destructive operations on a user's real library.

## 13. Implementation sequence

P0: inspect the repository and current preservation rules; instrument the existing baseline; add reproducible fixtures and a performance evidence report. Do not invent passing results.

P1: build compact catalog pages, indexed seeks and cached timeline summaries; make warm browsing independent of source-root enumeration.

P2: add row virtualisation, stable anchoring and progressive local rendition caching; measure before adopting a more complex layout solver.

P3: add shared resource budgets, truthful cancellation and adaptive prefetch; test NAS/HDD contention with existing backup adapters.

P4: run native release-build stress/failure tests, profile each remaining bottleneck, tighten budgets and implement only measured improvements.

Stop after the first bounded authorised phase. Never replace the whole interface or migrate existing catalog data merely to demonstrate a synthetic benchmark. See CODEX_PERFORMANCE_PROMPT.md and PERFORMANCE_ACCEPTANCE_TESTS.md.

## P0 repository reconciliation — 2026-10-28

The owner authorised P0 only. The earlier “no source supplied” statement describes the proposal's research context; the current repository contains the native M1 application. Actual inspection, bounded instrumentation, fixture evidence and unresolved gates are in [PERFORMANCE_BASELINE.md](PERFORMANCE_BASELINE.md). The previously missing [acceptance checklist](PERFORMANCE_ACCEPTANCE_TESTS.md) is explicitly reconstructed from the authorised P0 scope. Proposal S01–S17 citations were absent from the supplied register; checked P0 references are separately recorded in [SOURCES.md](SOURCES.md). Existing gallery/paging/cache code does not imply approval or completion of P1–P4.

## P1 authorisation — 2026-10-28

After the P0 report, the owner's “Move to next plan” instruction authorises P1 only. Use existing schema2 indexes, explicit display projections, versioned compact page transport and connection-owned bounded summary caching. Preserve current views, stable IDs, selection membership, full startup validation and durability. No user-catalog migration or source-root enumeration is needed for these reads. See [ADR-0009](decisions/ADR-0009-catalog-read-performance.md) and [P1 evidence](PERFORMANCE_P1.md). P2–P4 remain unapproved.

## Bounded P2 rendition authorisation — 2026-10-28

The owner's thumbnail/background-job request authorises only [progressive previews](PROGRESSIVE_PREVIEWS.md) and [ADR-0011](decisions/ADR-0011-progressive-renditions.md): tiny derived-cache generation, demand-driven clear renditions and bounded macOS JPEG/PNG/TIFF downsampling. No full P2 acceptance, new decoder dependency, schema migration or P3/P4 scheduling claim follows from this slice. Existing P0/P1 measurements remain historical and must not be presented as measurements of the new pipeline.
