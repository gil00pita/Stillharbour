> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0082 — Catalog metadata and virtual organization

Date: 2026-11-02. Status: implemented bounded M5 continuation; whole-milestone acceptance remains open.

## Context

The owner requested continuing M5 after the offline map increment. Favorites, ratings and flat catalog tags already exist. Captions, reject flags, virtual albums, saved searches and category relationships need durable backend behavior without source-file writes or new native permissions.

## Decision

Add schema 16 through the existing transactional migration path. Preserve compiled historical schemas, validate a bound catalog before upgrading, and retain a consistent independently openable prior snapshot. No persisted native identity or Keychain authority changes.

Use the existing catalog actor and typed `library_request` boundary. A nested catalog-metadata request accepts validated IDs/data; it exposes no filesystem, SQL, executable, schedule or permission inputs. Optional query fields preserve old serialized filters when absent. All gallery/map paging, count and neighbor paths share the same predicates.

Store caption/reject state separately from raw source observations. Save with an expected per-item version in one transaction with append-only note history and global revision. A stale save fails and retains the frontend draft. History survives catalog asset removal as retained audit data; re-adding a file does not reconnect it to an old asset ID. Embedded keyword observations remain separate from manual tags; rescans cannot silently restore a manually removed catalog tag.

Virtual albums contain stable asset IDs and never relocate media. Membership requires an exact reviewed selection count and the expected collection version. Explicit IDs must all exist, be distinct media and stay within the existing 500-ID boundary. Frozen selections expire after one hour and permit up to 50,000 members; creating an album after freezing does not invalidate the frozen set. Missing members cause an atomic refusal. Archive is reversible and retains memberships. Limit collections to 200, including archived entries.

Saved searches contain at most 8192 bytes of validated `AssetQuery` data. Unknown fields and invalid filters fail. They grant no authority and run only when explicitly viewed; no automatic scans or jobs. Replacing filters is a distinct visible action.

Tag categories arrange literal manual labels using explicit parent IDs. Preserve case, Unicode and slashes, using SQLite's binary label comparisons and existing trimming rules. Do not infer hierarchy from slash-separated labels or change existing tags. Bound the graph to 512 categories and sixteen levels; reject missing parents, cycles and stale reparenting. Rename/merge/undo and provenance-aware import overrides remain separate work.

## Consequences and validation

The new native dialog and inspector provide loaded, busy, failed and stale-edit states, native modal focus and reload actions. The selection toolbar opens album review. No new dependency, capability or helper is needed. Existing read-only roots and unsupported-preview videos can still carry catalog organization.

[M5_METADATA.md](../M5_METADATA.md) records limits and executed evidence. Consistent snapshots retain the new organization. This does not implement portable catalog export, XMP round trips, catalog encryption, whole-N2 support, Windows native acceptance or M4 preservation certification. Stop before M6.
