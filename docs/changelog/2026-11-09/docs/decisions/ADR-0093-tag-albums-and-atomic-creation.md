> Archived 2026-11-09 from `4f293da`. This is historical material; use the
> [current documentation](../../../../README.md) for implemented behavior and active instructions.
> Earlier failures, limitations and unmet safety/release obligations remain preserved.

# ADR-0093: Tag albums and atomic custom album creation

Date: 2026-11-06. Status: accepted for the owner-requested Albums / Tags slice.

The catalog already stores versioned custom collections, unique memberships, source keyword observations and tag-removal overrides. The old Tags route reused the photo browser and required explicit observed-keyword import. The owner requests a unified album grid and automatic tag grouping during scans, retaining unchanged originals.

Project exact catalog tag labels as read-only albums using the existing asset query. Reuse custom collections and memberships; do not introduce another album database or copy originals into album folders. Merge bounded source keyword observations during fresh/cached scan processing, respecting persistent removal overrides and retaining raw observations/history. Existing observations become grouped on an authorized rescan. No catalog schema/UUID or source-file migration occurs.

Add typed paged album projection and atomic creation requests through the existing database actor. Save a custom album and its initial explicit membership in one transaction, with backend name/ID/count validation. Reuse optimistic versions for subsequent additions. Custom names use trimmed Unicode lowercase comparison, including existing tag names and archived custom collections; source tags retain their documented exact case/Unicode identity. Later observed source tags never lose their exact label to resolve a name collision with an older custom album.

Keep cover loading bounded by viewport visibility and existing cached-preview policy. Reuse the virtual photo browser for detail/picking and Chakra v3 menu primitives with a native-dialog picker. Native tests use a separate constructor-owned tagged fixture and test-only flag, excluded from production builds. Windows gains keyword observations through existing pinned image/EXIF/XML dependencies, with explicit parser bounds and unsupported encodings. No new dependency, original mutation authority, cloud source or release gate follows.

See [Albums / Tags](../ALBUMS_TAGS.md) for behavior, limits and validation commands.
