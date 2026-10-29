# ADR-0007 — Immediate-child Folders explorer

Date: 2026-10-28. Status: accepted for the owner-requested M1 browsing refinement.

## Context and decision

The owner wants Folders to behave like a file explorer: folder navigation first, then that folder's files/pictures. Existing logical folder filtering is recursive and its flattened strings are lossy display data, so it cannot define exact native folder scope. Existing directories/asset observations already retain native path bytes; browsing should use those rather than enumerate live original folders or migrate the catalog.

Add a read-only browse_directory operation over existing schema2 tables. A CatalogDirectory is an indexed observation, not proof that its filesystem object is online. Its opaque transport identifier encodes native relative-path bytes and is always paired with a validated root UUID. It is location-based, not a stable filesystem-object identity across rename. Display names/paths never become original-file execution inputs. Query known observations; refuse blocked/excluded/unknown IDs. Retain empty/partial/history/offline distinctions instead of inventing empty disk directories.

Return at most240 immediate entries, folders first then files in native-byte order. Use validated root/directory/search-bound row-value cursors for next/previous pages, not OFFSET, full-tree arrays or an unbounded history. A joined asset projection avoids per-file metadata queries. Filename filtering applies to direct file names and leaves navigation folders visible. No new index/schema migration is introduced; existing root/native directory keys and asset directory index are reused. Large-folder query latency remains an evidence gate.

Expose optional AssetQuery.directoryId for exact folder_native matching in media queries, counts, buckets/selection and viewer neighbors. Omitted means existing behavior, including recursive folder text filters. Exact queries must bypass whole-root cached inventory totals. This keeps the existing cached-preview viewer/annotation pipeline without allowing next/previous to escape into descendant folders.

Frontend Folders starts at registered roots and keeps one bounded page with virtual rows, responsive named Icons and image-free Details, Back/Up/breadcrumb navigation, bounded64-position history and explicit local failures/retry. Photos remains the chronological three-chunk feed. Media opens the existing viewer; non-media entries show catalog-only File details. No external launcher, arbitrary decoder, batch folder mutation, new format or original-file write is added. Cache generation uses existing read-only workers and limits.

## Alternatives and evidence

Reusing recursive gallery filters would include descendants and could collide on lossy native names. Enumerating source folders live would bypass indexed scan policy/offline behavior. Persisting new directory UUIDs would require a reviewed catalog migration without a current storage need. The byte-preserving read-only adapter avoids all three while preserving schema/UUIDs and the established preview/viewer boundary.

Generated hierarchy tests cover immediate scope, nested exclusion, empty folders, indexed non-media files, literal wildcard characters, page reversal, cursor misuse and exact media totals/neighbors. The host APFS volume rejected invalid UTF-8 names; distinct lossy-label identities are tested through explicit synthetic rows in an owned temporary catalog, without claiming native filename creation. Original fixture bytes and catalog identity remain unchanged; offline navigation reads retained records. Native UI tests cover actual hierarchy, media viewer scope/return, text-only work, file details, navigation and minimum-window containment. Final results and remaining Windows/accessibility/performance gates are in IMPLEMENTATION_LOG/M1_VALIDATION.

Primary SQL references checked: [SQLite row-value comparisons](https://www.sqlite.org/rowvalue.html) and [SELECT/compound queries](https://www.sqlite.org/lang_select.html). No third-party dependency or production permission is added.
