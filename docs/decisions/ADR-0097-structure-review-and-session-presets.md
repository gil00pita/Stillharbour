# ADR-0097: Structure review, illustrative examples and session presets

Date: 2026-11-07. Status: accepted for read-only planning and explicit simulation;
production restructuring remains disabled.

## Context

The owner supplied a local HTML prototype and requested a complete rules-to-review
flow while retaining Stillharbour's existing studio identity. ADR-0095 already
owns whole-library membership, bounded processing, global conflicts and stale
preview rejection; ADR-0096 owns metadata naming and custom folder chips. A
prototype's simulated success cannot establish native mutation or recovery.

## Decision

Separate folder categories from filename formatting. Date, Location, Tags,
Camera and Custom keep the existing metadata semantics. Folder cards preserve
the filename; Original, Date + original and Sequence number controls preserve
the folders. Keep custom filename input and an Advanced path rule disclosure.
Preserve extension spelling when changing filename stem case.

Add native `organization_template_examples`: four fixed, explicitly illustrative
sources pass through the pure naming resolver. They have no filesystem paths,
execution identities or catalog authority. The editor debounces updates and hides
stale results; Folder tree and Sample paths are two views of these examples.
Invalid rules show an error rather than a plausible replacement example.

Add MissingDatePolicy::Skip. When a rule needs a capture date or quarter and no
usable indexed capture date exists, retain the source row with a typed skip
reason, no target and its original global sequence number. Non-date rules do not
skip merely because a capture date is absent. Undated retains its existing
whole-path fallback, `Undated/original-name.ext`, including for sequence rules.
The legacy Block policy remains readable. Missing-date observations describe
indexed provenance, not proof that an original contains no date.

Skip uses preview policy v2 and saved template policy schema 2. Optional row
observations are omitted for legacy selected-photo previews, preserving old JSON
and plan digests. Saved examples cannot assert a target for a skipped source.
Selected-photo executable plan creation explicitly rejects Skip before native
destination planning; its UI explains the unsupported policy. No catalog schema
migration, identity change or new mutation authority is introduced.

Extend the native TEMP report with derived move/rename/folder/date/skip/search
columns and one cached complete summary per preview. Global summary counts
exclude blocked, colliding and skipped outputs from moves, renames and destination
folders; unchanged eligible rows remain separately counted. Destination folders
are distinct proposed output folders, not assertions about folders newly created
on disk. Needs attention includes conflicts, unavailable sources and missing
indexed capture dates; these can overlap skipped rows. Cache only complete
membership and validate its count. Catalog revision and session checks still
precede report access.

Add `organization_library_query` with a typed filter and literal Unicode-lowered
substring search over displayed before/after paths. Search is bounded to 256
characters/1024 bytes. SQL clauses come from the native enum; values are bound.
Search, matching count and filters cover the entire completed TEMP namespace,
with at most 240 rows returned. Search never renumbers sequences or changes the
global summary. The renderer fences out-of-order queries and hides failed results.

Session presets are cloned rule/category values in module memory, partitioned by
catalog ID. Navigation retains them; app reload clears them. Names are trimmed,
case-insensitively unique and limited to 80 characters; limits are 20 presets per
catalog and 32 catalogs, with explicit refusal instead of silent eviction.
Existing immutable catalog templates and saved plans remain distinct controls.

Review shows global metrics, missing-date notice, full-scope search/filters,
paged before/after paths and return-to-editing. A clearly labelled simulation
dialog revalidates the native preview before showing simulated completion. It
invokes only the read-only page command, never approval, execution, source I/O,
backup or job creation. Closing/unmounting retires pending completion. Actual
Run restructure stays disabled in every stage. Backup/sync prerequisites remain
visible observations; the future executor still needs atomic admission against
active work, unresolved recovery and every enabled backup schedule.

## Consequences and validation

Use colocated Chakra v3 recipes for the editor and portalled dialogs, existing
palette/spacing and focus restoration on stage changes. Provide a local CSS
bridge for legacy unlayered primary-button margins. No new dependency is needed.

Test native classification, literal filtered paging beyond 240 rows, skipped
sequence/fingerprint compatibility, illustrative resolution and persisted policy
versioning. Frontend tests cover examples, session boundaries, error/stale paths,
confirmation and simulation-only requests. Instrumented desktop tests use owned
generated sources, audit absence of organization mutation commands and verify
unchanged fixture bytes. These are not production execution, physical input,
Windows, embedded metadata coverage or packaged-release qualification. See the
dated implementation log for actual runs; M2/M4 preservation gates remain open.
