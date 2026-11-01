# ADR-0071: Explainable, pausable scans with filename inventory

Date: 2026-11-01
Status: accepted for this bounded M1 refinement; platform validation recorded in the implementation log

## Decision

Perform a separate handle-anchored names/attributes pass before authoritative indexing. It reads no file content, queues no unbounded path list, obeys existing filters/depth/identity/no-follow/placeholder gates and establishes an advisory eligible-file total. Inaccessible inventory falls back to unknown totals. Rewalk with existing native identity/content checks; only this authoritative scan can reconcile presence. Add nullable total_files/processed_files to scans in schema 10 through the existing retained pre-migration snapshot transaction. Preserve UUIDs and all original paths. Active percentages cap at 99 when the directory changes; completed/partial/failed states remain authoritative.

Attach pause flags to admitted scan/thumbnail jobs. Workers acknowledge pause only at safe entry boundaries, retain bounded cursor/queues and resume the same job. In-flight helper reads are allowed to finish. Cancellation takes priority and wakes paused workers, including removal/shutdown. Durable paused/pending rows become interrupted at startup. Paused jobs retain admission slots rather than allowing another scan to reconcile the same root. No operating-system thread/process suspension or persisted continuation cursor is claimed.

Retry resolves the historical job to its backend root and current saved scan filter, or the separate thumbnail generation adapter. It rejects active work and reuses valid metadata/cache entries. Intentional exclusions and unsupported formats are never overridden. Initial thumbnail jobs keep their existing root-bound durable identity; retry clears their old diagnostic rows before rerunning. Scan details record filter/package/data exclusions and unsupported entries within the existing 200-entry bound. Preview issue IDs resolve to local catalog display labels on read; no path is execution authority or telemetry.

Thumbnail remaining time uses observed item work duration after three observations, excludes boundary pause time and is explicitly approximate. Cached files, codec differences and source contention can change the rate. No timing SLA, total-duration forecast during filename enumeration or production mutation authority is introduced.

## Consequences and validation

The first pass adds directory I/O and delays metadata until enumeration completes. It provides a denominator without retaining the entire library in memory. Directory changes and inaccessible scopes remain explicit limits, not evidence of missing originals. Old jobs cannot retroactively gain unrecorded skip reasons. Tests cover schema snapshots/rollback, pause/resume/cancel/restart, generated filename totals/exclusions/retry and UI states. Native held-job receipts and independent original manifests remain required on each platform; see the implementation log for actual commands/outcomes. M2/M3 and all preservation gates remain unchanged.

## Branch integration — 2026-11-01

Merge 93fd459 into clarify-slash-request, retaining its preferred scan controls while keeping compatible M2/M3 code and acceptance history. This decision was ADR-0024 on the incoming branch; the target already uses that number for destination review, so retain both decisions and number this one ADR-0071. The target has schema-4 through schema-9 operation/inspection migrations; add scan counters at schema 10 rather than downgrading those catalogs. Existing scan-branch schema-4 catalogs instead have progress columns and lack journal indexes. Recognize that complete two-column layout, install the missing journal indexes before the later target migrations, preserve counters/history/UUID and retain an independent schema-4 snapshot. Standard schema-9 catalogs gain the counters after a schema-9 snapshot. An incomplete one-column layout fails with the source catalog unchanged. New regression fixtures cover both histories and the partial-layout failure. Historical evidence in the log retains its original branch's schema numbers; current behavior follows this integration note. M2/M3 fixture acceptance and production preservation gates remain unchanged; stop before M4.
