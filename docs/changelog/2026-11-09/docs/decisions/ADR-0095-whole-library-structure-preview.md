> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0095: Whole-library structure previews

Date: 2026-11-07
Status: Accepted for read-only preview; execution remains gated

The owner clarified that Structure Formater uses the entire library. Users configure rules rather than selecting photos. The existing 1–240 explicit-ID planner cannot represent that scope, and independently planning batches would miss collisions and restart sequence numbers.

Add typed `organization_library_start`, `organization_library_advance`, `organization_library_page` and `organization_library_discard` requests. The native catalog actor captures every indexed media ID, ordered by root ID, lossless native relative path and asset ID, into session-only SQLite TEMP membership. Gallery queries, selections and availability do not filter it. One opaque UUID identifies the current preview in that catalog session. Starting again replaces it; late discard affects only the matching handle. Closing the catalog removes TEMP state. No main schema, stored identity, dependency or file-operation authority changes.

Each advance snapshots at most 240 sources through the existing catalog-only adapter, renders the shared portable naming rules outside the database actor, and commits bounded rows with the expected cursor and catalog revision. Changed catalogs, superseded handles and duplicate/concurrent advances fail closed. The original revision must remain current throughout; this is an observation, not a durable frozen execution plan. Scanning or metadata changes can require restarting the preview.

Store results in TEMP tables with indexed exact and ASCII-folded targets. Check exact outputs, ancestors and descendants against the complete accumulated namespace, marking all participants including earlier pages and rows already blocked for another reason. ASCII case conflicts remain advisory; this does not emulate native Unicode or destination case rules. Sequence is global rather than restarting at a page boundary. Missing, excluded and offline media stay in scope as blocked entries; sidecars remain indexed companion observations requiring separate bundle review. Final counts and rows appear only after the complete namespace is processed. IPC and rendered pages contain at most 240 rows.

The frontend opens directly on rules, offers progress/cancel, stale/error/empty states and bounded previous/next pages. Rule changes, navigation away and cancellation discard native state and reject late responses. Cleanup failure leaves only replaceable TEMP data, never an operation or approval. Catalog switching remounts the page. Saved templates and existing saved plans remain accessible. The separate explicit-photo name-preview/copy-plan workflow is retained. Whole-library result pages cannot be passed as complete executable plans.

**Run restructure remains disabled.** Whole-library execution, destination and companion validation, immutable review/approval, retained-version recovery and native failure/recovery qualification require a subsequent authorized increment. This read-only adapter neither opens originals/destinations nor grants new mutation commands.

SQLite owns full membership/output storage, while each Rust render/IPC allocation is bounded. Initial membership insertion and result conflict updates run on the single catalog actor; very large catalogs can occupy that actor. No 100,000/1,000,000-item latency, disk/RSS budget or full native performance acceptance is claimed. TEMP storage placement follows SQLite configuration. Large-catalog performance qualification remains open; do not treat bounded pages as a proven whole-library resource ceiling.

Focused verification covers more than 240 media rows, stable global sequences, cross-batch conflicts updating early pages, ancestor/case collisions, unavailable/excluded sources, empty scope, catalog changes, superseded/racing handles and strict IPC fields. Generated native desktop checks cover over 960 catalog items, actual paged IPC, no-selection entry, native window fit and unchanged fixture originals. Actual command outcomes are recorded separately in IMPLEMENTATION_LOG; both-platform CI and all existing preservation/release gates remain binding.
