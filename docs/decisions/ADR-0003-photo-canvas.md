# ADR-0003 — Read-only chronological canvas and viewer

Date: 2026-10-27. Status: accepted within the authorised M1 UI/read-only slice.

## Context

The expanded owner brief supersedes the earlier flush image-only gallery. It needs chronology, safe selection and immersive viewing while retaining bounded catalog/preview work and preservation gates. No similarity engine or full-resolution rendering pipeline currently exists.

## Decision

Use the existing pure row planner with aspect-preserving justified rows, 4 px gutters, actual day headings and 150/220/300 px density targets. Final incomplete rows remain left-aligned. Grid and Text list share stable-ID selection; Text list cancels background preview work and starts no nonempty batches. Use a bounded controller for query/page/month history and explicit selection, a viewport adapter for virtualization/anchors, and a modal viewer that leaves the collection mounted.

Extend generated typed IPC with filtered month buckets, validated year/month seeks, filtered previous/next neighbors and atomic additive tags. Neighbors compare the same allowlisted sort column plus stable ID in SQLite (including exact native integer sort values); no preceding page or whole-library ID list is required. Month/year seeks use existing bounded keyset queries. The schema stays at version 2. Annotation/selection operations write the catalog only. Add tags trims/deduplicates, unions existing JSON tags and rolls back the complete batch if any asset would exceed 64 tags.

Explicit IDs, shown-date selection and all-query backend tokens remain different scopes. Late all-query responses are discarded after filter changes. Compare accepts 2–4 explicit IDs, never hidden/token members. Similarity grouping is visibly unavailable pending M3 data/analysis; possible filename companions are not similarity or exact-duplicate evidence. Original-file edit/export/album actions remain disabled with reasons.

Use native dialog showModal for modality. The viewer queries neighbors under the original filters/sort, offers optional filmstrip/Info and fit-relative zoom/pan, and restores gallery focus plus pixel offset after native dialog close. Cached preview resolution remains 256/512 px; there is no new decoder, format or original-mutation capability. Viewer copy discloses this limit. Offline status is distinct from cached-preview availability.

Gallery and viewer share one backend preview-generation queue. Pause gallery requests completely while a modal owns it. Each request has at most 64 keys; a visible bounded page has at most 240 keys, submitted in at most four batches under one generation. Stale effect results are ignored and retry is explicit. Native queue capacity 64, two decoder workers, 256 MiB cache and existing decoder limits remain unchanged. Text-only cancellation can send an empty batch.

## Consequences and validation

No new dependency, schema migration, renderer filesystem access or capability is required. Generated fixtures cover filtered month/neighbor ordering, invalid periods/IDs, tag merge/rollback, stale selection, month paging, text-only work and original preservation. Native WKWebView checks cover real queries, viewer/zoom/navigation and scroll/focus return. Full-resolution viewing, similarity analysis, large-catalog UI performance, Windows, VoiceOver and original mutation safety remain separate gates. Actual outcomes are recorded in IMPLEMENTATION_LOG.md and M1_VALIDATION.md.
