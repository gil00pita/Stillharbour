# ADR-0005 — Gallery filter panel and bounded scroll feed

Date: 2026-10-28. Status: accepted for the owner-requested M1 gallery refinement.

## Context and decision

The owner's annotated screen removes stacked controls, the gallery Add folder button and explicit pagination. Preserve the incumbent photo canvas and safety boundaries. Search and Filter remain fixed; a native modal right panel contains query/sort/display controls, facets, active chips, count, availability and applied-query Select all. Draft Apply/Close/Escape avoids repeated partial queries; controls do not save application defaults. Settings retains indexed-folder registration and management. Contextual batch actions still affect only catalog annotations.

Use existing keyset queries with chunks of at most 240 summaries and a renderer window of at most three chunks (720). Serialize continuations and discard responses from older query generations. Append/prepend and evict the distant end while capturing the visible stable-ID/row offset before committing the new window. Virtualized DOM and preview limits remain separate bounds. Near-bottom scrolling loads forward; upward scrolling (including scrollbar movement), wheel or keyboard navigation near the top requests earlier rows. Track programmatic restored offsets separately so the corresponding scroll event cannot start an opposite-direction load. Date-rail seeks are direct and do not automatically prepend earlier dates. A failed continuation preserves loaded rows and requires explicit retry. Revision refresh rereads up to three chunks from the retained first ID; an empty obsolete window with remaining results falls back to the query head.

Add a typed `seek_asset` catalog operation with validated UUID, query, position (`at`/`before`) and maximum-240 limit. Choose sort columns from the existing allowlist and read the native sort value inside SQLite, preserving large integer size keys. Reverse bounded before-results into ordinary order, and reuse query-bound cursors. The anchor may no longer match current filters during a live refresh; all returned rows must match the query. Total lookup reuses the existing bounded query/count path; additional filtered counts are not a universal constant-time claim. No original path access is added.

## Consequences and validation

No dependency, catalog-schema migration or production permission change. Existing media queue capacity 64, two workers, 256 MiB cache and 256/512 px preview policies stay intact. Range/date selection covers shown IDs, never silently unloaded members; all-results selection freezes native membership only for the applied query. Similarity grouping remains unavailable pending separately authorized M3.

Core fixtures compare directional seeks against all ordinary supported sorts/filters, including size keys beyond JavaScript exact-integer precision and cursor/error boundaries. Component tests cover bounded windows, stale/duplicate requests, retry, revisions and panel drafts/selection. Native tests use the generated library plus 960 explicitly unavailable synthetic catalog rows to traverse both directions without source changes. The synthetic fixture command accepts no paths and is excluded from production alongside existing test instrumentation. Actual checks and remaining native/platform/performance gates are recorded in IMPLEMENTATION_LOG.md and M1_VALIDATION.md.
