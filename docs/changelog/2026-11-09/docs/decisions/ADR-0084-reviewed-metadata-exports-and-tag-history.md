> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0084: Reviewed metadata exports and reversible tag changes

Date: 2026-11-02. Status: implemented; actual acceptance evidence and remaining platform gates are recorded in [M5 validation](../M5_VALIDATION.md).

The owner's “Finish M5” request authorizes the remaining M5 implementation. It does not authorize M6, production original-file mutations, a metadata editor, cloud import or mobile work.

## Decision

Schema 17 adds exact-label tag overrides, immutable asset-tag history, reviewed metadata plans, category history and an immutable export journal. Keep the compiled definitions of schemas 14–16 intact. Bound startup validates the historical schema before transactional migration and retains its consistent pre-migration copy. Existing catalog UUIDs, native roots and Keychain identities stay unchanged.

Preview tag rename/merge/remove against a retained catalog revision and category graph, then apply the exact reviewed digest atomically. Case, Unicode and literal slashes remain distinct labels. Undo checks current tags/categories against the recorded after-state and writes a new history event; category versions increase monotonically. Category-only changes remain visible in history. Explicit observed-keyword import respects removal overrides unless the user explicitly restores removed labels. A rescan does not import source keywords or revoke manual intent.

Metadata export is a separate backend adapter, restricted to **new outputs** in a natively selected folder outside indexed roots and application data. The renderer carries IDs and enums, never paths, SQL, executables or export bytes. Preview materializes bounded private outputs and retains their digests, catalog revision, native folder identity, source witnesses, fresh companion inventory, picker session and process-instance nonce. Approval expires after one hour and cannot replay after restart. Native read/write leases coordinate owned operations. Publication uses exclusive stages, native no-replace publication, durable journal phases and independent read-back. Cancellation/failure retains evidence and partial outputs; only a completed journal receipt represents success. There is no overwrite, automatic resume, cleanup or original-sidecar synchronization.

Portable catalogs use SQLite's consistent Backup API for the private source snapshot, then copy an allowlist into a fresh database initialized exclusively by compiled schema. Reopen independently and verify schema, integrity, foreign keys and UUID. Preserve catalog annotations, albums, saved filters, categories, tag history and read-only inspection/baseline evidence. Keep a bounded sanitized summary of prior jobs. Export no credential references, schedules, runtime approvals, active jobs, executable journals or root/destination authority. Roots are unbound and need explicit relinking. Raw metadata uses a documented observation allowlist. The source catalog and existing full recovery snapshots retain the fuller history; this portable export is not a full disaster-recovery replacement or a new in-app import workflow.

XMP exports preserve source packets and all possible companions byte-for-byte as separately named artifacts, plus an association manifest. A bounded namespace-aware XML merge retains unknown content and updates supported catalog fields. DTDs, entities, malformed packets, ambiguous shared-basename variants and unsupported complex qualifiers fail explicitly. Fresh descriptor-anchored enumeration includes hidden, excluded and unindexed companions. Recheck source identity/revision, companion digests and membership before and after publication. Never modify or claim to round-trip arbitrary editor instructions in the source packet.

Owned stream reads, stage writes and read-back share the existing 10 MiB/s budget and cancellation. SQLite internal page-copy I/O and filesystem metadata calls are not independently byte-metered; disclose this limit. SQLite source snapshot has a 512 MiB admission bound and a 30-second stepping deadline. No native helper or duplicate state library is introduced. The already locked quick-xml 0.42.0 (MIT) becomes a pinned direct dependency for namespace-aware parsing.

## Consequences and gates

The ordinary M4 original-file preservation policy stays fail-closed. Exporting new metadata outputs grants no authority to rename, replace, quarantine or rewrite originals. Unknown imported schema and imported approvals do not execute. Platform/format/accessibility evidence remains scoped to actual tests. Stop before M6.

Native metadata acceptance runs independently from the graphics-dependent map case, using the real map photo-list alternative to establish its bounded query. On this host an occluded test window reports `document.visibilityState = hidden` and suspends requestAnimationFrame even though its bundled worker responds. Foreground, keep-awake and temporary scheduling/worker experiments did not resolve that host condition and were removed. Neither source requirements nor graphics assertions are weakened to claim a passing map run. Production scheduling, worker and CSP remain unchanged. Fresh foreground graphics acceptance remains explicitly unverified when the recorded run cannot render.
